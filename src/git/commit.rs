use anyhow::{Context, Result};
use chrono::{DateTime, Local, Offset};

use super::{CommitDetail, CommitFile, FileState, GitRepo};

/// Convert a `git2::Time` to a local `DateTime`.
fn git_time_to_local(time: git2::Time) -> DateTime<Local> {
    let secs = time.seconds();
    let offset_mins = time.offset_minutes();
    let offset =
        chrono::FixedOffset::east_opt(offset_mins * 60).unwrap_or_else(|| chrono::Utc.fix());
    DateTime::from_timestamp(secs, 0).map_or_else(Local::now, |dt| {
        dt.with_timezone(&offset)
            .with_timezone(&Local)
    })
}

impl GitRepo {
    /// Get detailed commit information for a given SHA
    #[allow(clippy::similar_names)] // stats/status are distinct concepts
    #[allow(clippy::too_many_lines)] // Complex function, refactoring would reduce clarity
    pub fn commit_detail(&self, short_sha: &str) -> Result<CommitDetail> {
        use std::cell::RefCell;
        use std::collections::HashMap;

        // Parse the short SHA to find the commit
        let obj = self
            .repo
            .revparse_single(short_sha)
            .with_context(|| format!("Failed to find commit {short_sha}"))?;
        let commit = obj
            .peel_to_commit()
            .with_context(|| format!("Object {short_sha} is not a commit"))?;

        let oid = commit.id();
        let full_sha = format!("{oid}");

        // Extract author info
        let author = commit.author();
        let author_name = author
            .name()
            .unwrap_or("Unknown")
            .to_string();
        let author_email = author.email().unwrap_or("").to_string();
        let author_time = git_time_to_local(author.when());

        // Extract committer info
        let committer = commit.committer();
        let committer_name = committer
            .name()
            .unwrap_or("Unknown")
            .to_string();
        let committer_email = committer
            .email()
            .unwrap_or("")
            .to_string();
        let committer_time = git_time_to_local(committer.when());

        // Get full commit message
        let message = commit
            .message()
            .unwrap_or("")
            .to_string();

        // Check for GPG signature
        let gpg_status = commit.raw_header().and_then(|header| {
            if header.contains("gpgsig") {
                Some("Signed".to_string())
            } else {
                None
            }
        });

        // Get files changed by diffing against parent
        let tree = commit
            .tree()
            .context("Failed to get commit tree")?;
        let parent_tree = commit
            .parent(0)
            .ok()
            .and_then(|p| p.tree().ok());

        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .context("Failed to diff trees")?;

        // Get overall stats
        let stats = diff
            .stats()
            .context("Failed to get diff stats")?;
        let insertions = stats.insertions();
        let deletions = stats.deletions();

        // Collect files with per-file stats using RefCell for interior mutability
        let file_stats: RefCell<HashMap<String, (FileState, usize, usize)>> =
            RefCell::new(HashMap::new());

        diff.foreach(
            &mut |delta, _progress| {
                let path = delta
                    .new_file()
                    .path()
                    .or_else(|| delta.old_file().path())
                    .map_or_else(
                        || "<unknown>".to_string(),
                        |p| p.to_string_lossy().to_string(),
                    );

                let status = match delta.status() {
                    git2::Delta::Added | git2::Delta::Copied => FileState::Added,
                    git2::Delta::Deleted => FileState::Deleted,
                    git2::Delta::Renamed => FileState::Renamed,
                    _ => FileState::Modified,
                };

                file_stats
                    .borrow_mut()
                    .insert(path, (status, 0, 0));
                true
            },
            None,
            None,
            Some(&mut |delta, _hunk, line| {
                if let Some(path) = delta
                    .new_file()
                    .path()
                    .or_else(|| delta.old_file().path())
                    .map(|p| p.to_string_lossy().to_string())
                {
                    if let Some(entry) = file_stats.borrow_mut().get_mut(&path) {
                        match line.origin() {
                            '+' => entry.1 += 1,
                            '-' => entry.2 += 1,
                            _ => {}
                        }
                    }
                }
                true
            }),
        )?;

        // Convert to Vec and sort
        let mut files: Vec<CommitFile> = file_stats
            .into_inner()
            .into_iter()
            .map(
                |(path, (status, ins, del))| CommitFile {
                    path,
                    status,
                    insertions: ins,
                    deletions: del,
                },
            )
            .collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(CommitDetail {
            full_sha,
            author_name,
            author_email,
            author_time,
            committer_name,
            committer_email,
            committer_time,
            message,
            gpg_status,
            files,
            insertions,
            deletions,
        })
    }

    /// Get diff for a specific file in a commit (vs its parent)
    pub fn commit_file_diff(&self, commit_sha: &str, file_path: &str) -> Result<String> {
        let obj = self
            .repo
            .revparse_single(commit_sha)
            .with_context(|| format!("Failed to find commit {commit_sha}"))?;
        let commit = obj
            .peel_to_commit()
            .with_context(|| format!("Object {commit_sha} is not a commit"))?;

        let tree = commit
            .tree()
            .context("Failed to get commit tree")?;
        let parent_tree = commit
            .parent(0)
            .ok()
            .and_then(|p| p.tree().ok());

        // Create diff with path filter
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(file_path);

        let diff = self
            .repo
            .diff_tree_to_tree(
                parent_tree.as_ref(),
                Some(&tree),
                Some(&mut opts),
            )
            .context("Failed to diff trees")?;

        // Format as patch
        let mut output = String::new();
        diff.print(
            git2::DiffFormat::Patch,
            |_delta, _hunk, line| {
                let prefix = match line.origin() {
                    '+' | '-' | ' ' => format!("{}", line.origin()),
                    _ => String::new(),
                };
                if let Ok(content) = std::str::from_utf8(line.content()) {
                    output.push_str(&prefix);
                    output.push_str(content);
                }
                true
            },
        )?;

        if output.is_empty() {
            output = format!("(No changes for {file_path})");
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    /// Create a test repo with an initial commit
    fn create_test_repo() -> (TempDir, GitRepo, String) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(dir.path()).expect("Failed to init repo");

        let mut config = repo
            .config()
            .expect("Failed to get config");
        config
            .set_str("user.name", "Test User")
            .expect("Failed to set user.name");
        config
            .set_str("user.email", "test@example.com")
            .expect("Failed to set user.email");
        drop(config);

        // Create initial commit
        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "initial content\n").expect("Failed to write file");

        let mut index = repo
            .index()
            .expect("Failed to get index");
        index
            .add_path(Path::new("file.txt"))
            .expect("Failed to add file");
        index
            .write()
            .expect("Failed to write index");

        let tree_id = index
            .write_tree()
            .expect("Failed to write tree");
        let tree = repo
            .find_tree(tree_id)
            .expect("Failed to find tree");
        let sig = repo
            .signature()
            .expect("Failed to get signature");
        let commit_oid = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Initial commit",
                &tree,
                &[],
            )
            .expect("Failed to commit");

        let short_sha = format!("{commit_oid:.7}");

        drop(tree);
        drop(repo);

        let git_repo = GitRepo::open(dir.path()).expect("Failed to open repo");
        (dir, git_repo, short_sha)
    }

    /// Add a commit that modifies a file
    fn add_modifying_commit(dir: &TempDir, message: &str) -> String {
        let repo = Repository::open(dir.path()).expect("Failed to open");

        // Modify file
        let file_path = dir.path().join("file.txt");
        let content = fs::read_to_string(&file_path).unwrap_or_default();
        fs::write(
            &file_path,
            format!("{content}added line\n"),
        )
        .expect("Failed to write");

        let mut index = repo
            .index()
            .expect("Failed to get index");
        index
            .add_path(Path::new("file.txt"))
            .expect("Failed to add");
        index
            .write()
            .expect("Failed to write index");

        let tree_id = index
            .write_tree()
            .expect("Failed to write tree");
        let tree = repo
            .find_tree(tree_id)
            .expect("Failed to find tree");
        let sig = repo
            .signature()
            .expect("Failed to get signature");
        let head = repo.head().expect("Failed to get HEAD");
        let parent = head
            .peel_to_commit()
            .expect("Failed to get commit");

        let commit_oid = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                message,
                &tree,
                &[&parent],
            )
            .expect("Failed to commit");

        format!("{commit_oid:.7}")
    }

    mod commit_detail {
        use super::*;

        #[test]
        fn returns_commit_info() {
            let (_dir, repo, sha) = create_test_repo();

            let detail = repo
                .commit_detail(&sha)
                .expect("Failed to get detail");

            assert_eq!(detail.full_sha.len(), 40);
            assert!(detail.full_sha.starts_with(&sha));
            assert_eq!(detail.author_name, "Test User");
            assert_eq!(detail.author_email, "test@example.com");
            assert_eq!(detail.message, "Initial commit");
        }

        #[test]
        fn returns_files_changed() {
            let (dir, _repo, _sha) = create_test_repo();

            let sha = add_modifying_commit(&dir, "Modify file");

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");
            let detail = repo
                .commit_detail(&sha)
                .expect("Failed to get detail");

            assert_eq!(detail.files.len(), 1);
            assert_eq!(detail.files[0].path, "file.txt");
            assert_eq!(
                detail.files[0].status,
                FileState::Modified
            );
        }

        #[test]
        fn returns_diff_stats() {
            let (dir, _repo, _sha) = create_test_repo();

            let sha = add_modifying_commit(&dir, "Add line");

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");
            let detail = repo
                .commit_detail(&sha)
                .expect("Failed to get detail");

            // We added one line
            assert!(detail.insertions >= 1);
            assert_eq!(detail.files[0].insertions, 1);
        }

        #[test]
        fn returns_error_for_invalid_sha() {
            let (_dir, repo, _sha) = create_test_repo();

            let result = repo.commit_detail("invalid");

            assert!(result.is_err());
        }

        #[test]
        fn detects_added_file() {
            let (dir, _repo, _sha) = create_test_repo();

            // Add a new file in second commit
            {
                let git_repo = Repository::open(dir.path()).expect("Failed to open");
                fs::write(dir.path().join("new.txt"), "new file\n").expect("Failed to write");

                let mut index = git_repo
                    .index()
                    .expect("Failed to get index");
                index
                    .add_path(Path::new("new.txt"))
                    .expect("Failed to add");
                index
                    .write()
                    .expect("Failed to write index");

                let tree_id = index
                    .write_tree()
                    .expect("Failed to write tree");
                let tree = git_repo
                    .find_tree(tree_id)
                    .expect("Failed to find tree");
                let sig = git_repo
                    .signature()
                    .expect("Failed to get signature");
                let head = git_repo
                    .head()
                    .expect("Failed to get HEAD");
                let parent = head
                    .peel_to_commit()
                    .expect("Failed to get commit");

                git_repo
                    .commit(
                        Some("HEAD"),
                        &sig,
                        &sig,
                        "Add new file",
                        &tree,
                        &[&parent],
                    )
                    .expect("Failed to commit");
            }

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");
            let commits = repo
                .commit_log(0, 1)
                .expect("Failed to get log");
            let sha = commits[0].sha.as_ref().expect("sha");

            let detail = repo
                .commit_detail(sha)
                .expect("Failed to get detail");

            let new_file = detail
                .files
                .iter()
                .find(|f| f.path == "new.txt");
            assert!(new_file.is_some());
            assert_eq!(
                new_file.expect("file").status,
                FileState::Added
            );
        }
    }

    mod commit_file_diff {
        use super::*;

        #[test]
        fn returns_diff_for_file() {
            let (dir, _repo, _sha) = create_test_repo();

            let sha = add_modifying_commit(&dir, "Modify file");

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");
            let diff = repo
                .commit_file_diff(&sha, "file.txt")
                .expect("Failed to get diff");

            assert!(diff.contains("+added line"));
        }

        #[test]
        fn returns_no_changes_for_unchanged_file() {
            let (dir, _repo, _sha) = create_test_repo();

            // Add a different file
            {
                let git_repo = Repository::open(dir.path()).expect("Failed to open");
                fs::write(dir.path().join("other.txt"), "other\n").expect("Failed to write");

                let mut index = git_repo
                    .index()
                    .expect("Failed to get index");
                index
                    .add_path(Path::new("other.txt"))
                    .expect("Failed to add");
                index
                    .write()
                    .expect("Failed to write index");

                let tree_id = index
                    .write_tree()
                    .expect("Failed to write tree");
                let tree = git_repo
                    .find_tree(tree_id)
                    .expect("Failed to find tree");
                let sig = git_repo
                    .signature()
                    .expect("Failed to get signature");
                let head = git_repo
                    .head()
                    .expect("Failed to get HEAD");
                let parent = head
                    .peel_to_commit()
                    .expect("Failed to get commit");

                git_repo
                    .commit(
                        Some("HEAD"),
                        &sig,
                        &sig,
                        "Add other file",
                        &tree,
                        &[&parent],
                    )
                    .expect("Failed to commit");
            }

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");
            let commits = repo
                .commit_log(0, 1)
                .expect("Failed to get log");
            let sha = commits[0].sha.as_ref().expect("sha");

            let diff = repo
                .commit_file_diff(sha, "file.txt")
                .expect("Failed to get diff");

            assert!(diff.contains("No changes"));
        }

        #[test]
        fn returns_error_for_invalid_commit() {
            let (_dir, repo, _sha) = create_test_repo();

            let result = repo.commit_file_diff("invalid", "file.txt");

            assert!(result.is_err());
        }
    }
}
