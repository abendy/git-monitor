use std::path::Path;

use anyhow::{Context, Result};
use tracing::debug;

use super::GitRepo;

impl GitRepo {
    /// Get diff for a file (working directory changes)
    pub fn diff_file(&self, path: &Path, staged: bool) -> Result<String> {
        use std::fmt::Write;

        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.pathspec(path);

        let diff = if staged {
            // Staged: diff HEAD to index
            let head = self.repo.head().ok();
            if head.is_none() {
                debug!("No HEAD found for staged diff (empty repo?)");
            }
            let head_tree = head.and_then(|h| {
                h.peel_to_tree()
                    .map_err(|e| {
                        debug!("Failed to peel HEAD to tree: {}", e);
                        e
                    })
                    .ok()
            });
            self.repo.diff_tree_to_index(
                head_tree.as_ref(),
                None,
                Some(&mut diff_opts),
            )
        } else {
            // Unstaged: diff index to workdir
            self.repo
                .diff_index_to_workdir(None, Some(&mut diff_opts))
        }
        .context("Failed to get diff")?;

        let mut output = String::new();

        diff.print(
            git2::DiffFormat::Patch,
            |_delta, _hunk, line| {
                let prefix = match line.origin() {
                    '+' => "+",
                    '-' => "-",
                    ' ' => " ",
                    _ => "",
                };
                let content = std::str::from_utf8(line.content()).unwrap_or("");
                let _ = write!(output, "{prefix}{content}");
                true
            },
        )
        .context("Failed to print diff")?;

        if output.is_empty() {
            output = String::from("(no changes)");
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use tempfile::TempDir;

    /// Create a test repo with an initial commit
    fn create_test_repo() -> (TempDir, GitRepo) {
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
        fs::write(&file_path, "line one\nline two\n").expect("Failed to write file");

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
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        )
        .expect("Failed to commit");

        drop(tree);
        drop(repo);

        let git_repo = GitRepo::open(dir.path()).expect("Failed to open repo");
        (dir, git_repo)
    }

    mod diff_file {
        use super::*;

        #[test]
        fn returns_working_diff() {
            let (dir, repo) = create_test_repo();

            // Modify the file
            fs::write(
                dir.path().join("file.txt"),
                "line one\nmodified line\n",
            )
            .expect("Failed to write");

            let diff = repo
                .diff_file(Path::new("file.txt"), false)
                .expect("Failed to get diff");

            assert!(diff.contains("-line two"));
            assert!(diff.contains("+modified line"));
        }

        #[test]
        fn returns_staged_diff() {
            let (dir, repo) = create_test_repo();

            // Modify and stage the file
            fs::write(
                dir.path().join("file.txt"),
                "line one\nstaged change\n",
            )
            .expect("Failed to write");
            repo.stage(Path::new("file.txt"))
                .expect("Failed to stage");

            let diff = repo
                .diff_file(Path::new("file.txt"), true)
                .expect("Failed to get diff");

            assert!(diff.contains("-line two"));
            assert!(diff.contains("+staged change"));
        }

        #[test]
        fn returns_no_changes_for_unchanged_file() {
            let (_dir, repo) = create_test_repo();

            let diff = repo
                .diff_file(Path::new("file.txt"), false)
                .expect("Failed to get diff");

            assert_eq!(diff, "(no changes)");
        }

        #[test]
        fn handles_new_untracked_file() {
            let (dir, repo) = create_test_repo();

            // Create a new untracked file
            fs::write(
                dir.path().join("new.txt"),
                "new content\n",
            )
            .expect("Failed to write");

            // For untracked files, diff_file won't show anything (it's not in index)
            let diff = repo
                .diff_file(Path::new("new.txt"), false)
                .expect("Failed to get diff");

            // Untracked files show no changes in working diff
            assert_eq!(diff, "(no changes)");
        }

        #[test]
        fn handles_staged_new_file() {
            let (dir, repo) = create_test_repo();

            // Create and stage a new file
            fs::write(
                dir.path().join("new.txt"),
                "new content\n",
            )
            .expect("Failed to write");
            repo.stage(Path::new("new.txt"))
                .expect("Failed to stage");

            let diff = repo
                .diff_file(Path::new("new.txt"), true)
                .expect("Failed to get diff");

            // Staged new file should show additions
            assert!(diff.contains("+new content"));
        }
    }
}
