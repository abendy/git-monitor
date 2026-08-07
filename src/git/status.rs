use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use git2::{Diff, DiffDelta, DiffHunk, DiffLine, DiffOptions, Status, StatusOptions};

use super::{FileState, FileStatus, GitRepo, GitStatus};

impl GitRepo {
    /// Get current git status
    pub fn status(&self) -> Result<GitStatus> {
        let mut status = GitStatus {
            state: self.repo.state().into(),
            ..Default::default()
        };

        // Get branch info
        self.populate_branch_info(&mut status)?;

        // Get file statuses
        self.populate_file_statuses(&mut status)?;

        Ok(status)
    }

    /// Populate branch and upstream info
    #[allow(clippy::unnecessary_wraps)] // Result for API consistency
    fn populate_branch_info(&self, status: &mut GitStatus) -> Result<()> {
        // Get HEAD reference
        let Ok(head) = self.repo.head() else {
            return Ok(()); // No commits yet
        };

        // Get branch name
        if head.is_branch() {
            status.branch = head.shorthand().ok().map(String::from);

            // Get upstream tracking info
            if let Some(branch_name) = &status.branch {
                if let Ok(branch) = self
                    .repo
                    .find_branch(branch_name, git2::BranchType::Local)
                {
                    if let Ok(upstream) = branch.upstream() {
                        status.upstream = upstream
                            .name()
                            .ok()
                            .flatten()
                            .map(String::from);

                        // Get ahead/behind counts
                        if let (Some(local_oid), Some(upstream_oid)) =
                            (head.target(), upstream.get().target())
                        {
                            if let Ok((ahead, behind)) = self
                                .repo
                                .graph_ahead_behind(local_oid, upstream_oid)
                            {
                                status.ahead = ahead;
                                status.behind = behind;
                            }
                        }
                    }
                }
            }
        } else {
            // Detached HEAD - show short commit hash
            if let Some(oid) = head.target() {
                status.branch = Some(format!("{oid:.7}"));
            }
        }

        Ok(())
    }

    /// Populate file statuses
    fn populate_file_statuses(&self, status: &mut GitStatus) -> Result<()> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false)
            .include_unmodified(false);

        let statuses = self
            .repo
            .statuses(Some(&mut opts))
            .context("Failed to get git status")?;

        // Compute diff stats for all files
        let diff_stats = self
            .compute_diff_stats()
            .unwrap_or_default();

        for entry in statuses.iter() {
            let path = entry.path().map_or_else(
                |_| PathBuf::from("<invalid>"),
                PathBuf::from,
            );

            let git_status = entry.status();

            // Look up diff stats for this file
            let (working_ins, working_del, staged_ins, staged_del) = diff_stats
                .get(&path)
                .copied()
                .unwrap_or((0, 0, 0, 0));

            let file_status = FileStatus {
                path,
                working: working_state_from_git2(git_status),
                staged: staged_state_from_git2(git_status),
                working_insertions: working_ins,
                working_deletions: working_del,
                staged_insertions: staged_ins,
                staged_deletions: staged_del,
            };

            // Only add if there's an actual change
            if file_status.working.is_changed() || file_status.staged.is_changed() {
                status.files.push(file_status);
            }
        }

        // Sort by path
        status
            .files
            .sort_by(|a, b| a.path.cmp(&b.path));

        Ok(())
    }

    /// Compute diff stats for all files with changes
    ///
    /// Returns a map of path to `(working_insertions, working_deletions, staged_insertions,
    /// staged_deletions)`
    #[allow(clippy::unnecessary_wraps)] // Result for API consistency
    fn compute_diff_stats(&self) -> Result<HashMap<PathBuf, (usize, usize, usize, usize)>> {
        let mut stats: HashMap<PathBuf, (usize, usize, usize, usize)> = HashMap::new();

        // Get HEAD tree for staged diff baseline (if available)
        let head_tree = self
            .repo
            .head()
            .ok()
            .and_then(|head| head.peel_to_tree().ok());

        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);

        // Staged changes: diff from HEAD to index
        if let Ok(staged_diff) = self.repo.diff_tree_to_index(
            head_tree.as_ref(),
            None,
            Some(&mut diff_opts),
        ) {
            collect_diff_stats(&staged_diff, &mut stats, false);
        }

        // Working changes: diff from index to workdir
        let mut workdir_opts = DiffOptions::new();
        workdir_opts.include_untracked(true);

        if let Ok(working_diff) = self
            .repo
            .diff_index_to_workdir(None, Some(&mut workdir_opts))
        {
            collect_diff_stats(&working_diff, &mut stats, true);
        }

        Ok(stats)
    }
}

/// Extract working directory state from git2 status
fn working_state_from_git2(status: Status) -> FileState {
    if status.is_wt_new() {
        FileState::Untracked
    } else if status.is_wt_modified() {
        FileState::Modified
    } else if status.is_wt_deleted() {
        FileState::Deleted
    } else if status.is_wt_renamed() {
        FileState::Renamed
    } else if status.is_conflicted() {
        FileState::Conflicted
    } else {
        FileState::Unmodified
    }
}

/// Extract staged (index) state from git2 status
fn staged_state_from_git2(status: Status) -> FileState {
    if status.is_index_new() {
        FileState::Added
    } else if status.is_index_modified() {
        FileState::Modified
    } else if status.is_index_deleted() {
        FileState::Deleted
    } else if status.is_index_renamed() {
        FileState::Renamed
    } else {
        FileState::Unmodified
    }
}

/// Collect line stats from a diff into the stats map
///
/// If `is_working` is true, updates `working_insertions/deletions`,
/// otherwise updates `staged_insertions/deletions`.
fn collect_diff_stats(
    diff: &Diff<'_>,
    stats: &mut HashMap<PathBuf, (usize, usize, usize, usize)>,
    is_working: bool,
) {
    use std::cell::RefCell;

    // Use RefCell to allow mutable access from the line callback
    let stats_cell = RefCell::new(stats);

    // Intentionally ignore foreach result - we're collecting stats via callbacks,
    // and partial results are acceptable (stats are optional enhancement)
    let _ = diff.foreach(
        &mut |_delta: DiffDelta<'_>, _progress: f32| true, // file callback
        None,                                              // binary callback
        None,                                              // hunk callback
        Some(
            &mut |delta: DiffDelta<'_>, _hunk: Option<DiffHunk<'_>>, line: DiffLine<'_>| {
                if let Some(path) = delta
                    .new_file()
                    .path()
                    .or_else(|| delta.old_file().path())
                {
                    let mut stats_ref = stats_cell.borrow_mut();
                    let entry = stats_ref
                        .entry(PathBuf::from(path))
                        .or_insert((0, 0, 0, 0));

                    match line.origin() {
                        '+' => {
                            if is_working {
                                entry.0 += 1; // working_insertions
                            } else {
                                entry.2 += 1; // staged_insertions
                            }
                        }
                        '-' => {
                            if is_working {
                                entry.1 += 1; // working_deletions
                            } else {
                                entry.3 += 1; // staged_deletions
                            }
                        }
                        _ => {}
                    }
                }
                true
            },
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    /// Create a test repo with an initial commit
    fn create_test_repo() -> (TempDir, GitRepo) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(dir.path()).expect("Failed to init repo");

        // Configure git user for commits
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

        // Create initial file and commit
        let file_path = dir.path().join("initial.txt");
        fs::write(&file_path, "initial content\n").expect("Failed to write file");

        let mut index = repo
            .index()
            .expect("Failed to get index");
        index
            .add_path(Path::new("initial.txt"))
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

    mod status {
        use super::*;

        #[test]
        fn clean_repo_has_no_file_changes() {
            let (_dir, repo) = create_test_repo();

            let status = repo
                .status()
                .expect("Failed to get status");

            assert!(status.files.is_empty());
        }

        #[test]
        fn detects_untracked_file() {
            let (dir, repo) = create_test_repo();

            // Create untracked file
            fs::write(
                dir.path().join("untracked.txt"),
                "new file\n",
            )
            .expect("Failed to write file");

            let status = repo
                .status()
                .expect("Failed to get status");

            assert_eq!(status.files.len(), 1);
            assert_eq!(
                status.files[0].path,
                PathBuf::from("untracked.txt")
            );
            assert_eq!(
                status.files[0].working,
                FileState::Untracked
            );
            assert_eq!(
                status.files[0].staged,
                FileState::Unmodified
            );
        }

        #[test]
        fn detects_modified_file() {
            let (dir, repo) = create_test_repo();

            // Modify tracked file
            fs::write(
                dir.path().join("initial.txt"),
                "modified content\n",
            )
            .expect("Failed to write file");

            let status = repo
                .status()
                .expect("Failed to get status");

            assert_eq!(status.files.len(), 1);
            assert_eq!(
                status.files[0].path,
                PathBuf::from("initial.txt")
            );
            assert_eq!(
                status.files[0].working,
                FileState::Modified
            );
            assert_eq!(
                status.files[0].staged,
                FileState::Unmodified
            );
        }

        #[test]
        fn detects_staged_file() {
            let (dir, repo) = create_test_repo();

            // Create and stage a new file
            fs::write(
                dir.path().join("staged.txt"),
                "staged content\n",
            )
            .expect("Failed to write file");
            repo.stage(Path::new("staged.txt"))
                .expect("Failed to stage");

            let status = repo
                .status()
                .expect("Failed to get status");

            assert_eq!(status.files.len(), 1);
            assert_eq!(
                status.files[0].path,
                PathBuf::from("staged.txt")
            );
            assert_eq!(
                status.files[0].working,
                FileState::Unmodified
            );
            assert_eq!(status.files[0].staged, FileState::Added);
        }

        #[test]
        fn detects_staged_modification() {
            let (dir, repo) = create_test_repo();

            // Modify and stage tracked file
            fs::write(
                dir.path().join("initial.txt"),
                "modified content\n",
            )
            .expect("Failed to write file");
            repo.stage(Path::new("initial.txt"))
                .expect("Failed to stage");

            let status = repo
                .status()
                .expect("Failed to get status");

            assert_eq!(status.files.len(), 1);
            assert_eq!(
                status.files[0].path,
                PathBuf::from("initial.txt")
            );
            assert_eq!(
                status.files[0].working,
                FileState::Unmodified
            );
            assert_eq!(
                status.files[0].staged,
                FileState::Modified
            );
        }

        #[test]
        fn detects_both_staged_and_working_changes() {
            let (dir, repo) = create_test_repo();

            // Modify and stage, then modify again
            fs::write(
                dir.path().join("initial.txt"),
                "staged version\n",
            )
            .expect("Failed to write file");
            repo.stage(Path::new("initial.txt"))
                .expect("Failed to stage");
            fs::write(
                dir.path().join("initial.txt"),
                "working version\n",
            )
            .expect("Failed to write file");

            let status = repo
                .status()
                .expect("Failed to get status");

            assert_eq!(status.files.len(), 1);
            assert_eq!(
                status.files[0].working,
                FileState::Modified
            );
            assert_eq!(
                status.files[0].staged,
                FileState::Modified
            );
        }

        #[test]
        fn detects_deleted_file() {
            let (dir, repo) = create_test_repo();

            // Delete tracked file
            fs::remove_file(dir.path().join("initial.txt")).expect("Failed to delete file");

            let status = repo
                .status()
                .expect("Failed to get status");

            assert_eq!(status.files.len(), 1);
            assert_eq!(
                status.files[0].path,
                PathBuf::from("initial.txt")
            );
            assert_eq!(
                status.files[0].working,
                FileState::Deleted
            );
        }

        #[test]
        fn files_sorted_by_path() {
            let (dir, repo) = create_test_repo();

            // Create files in non-alphabetical order
            fs::write(dir.path().join("zebra.txt"), "z\n").expect("Failed to write");
            fs::write(dir.path().join("alpha.txt"), "a\n").expect("Failed to write");
            fs::write(dir.path().join("middle.txt"), "m\n").expect("Failed to write");

            let status = repo
                .status()
                .expect("Failed to get status");

            assert_eq!(status.files.len(), 3);
            assert_eq!(
                status.files[0].path,
                PathBuf::from("alpha.txt")
            );
            assert_eq!(
                status.files[1].path,
                PathBuf::from("middle.txt")
            );
            assert_eq!(
                status.files[2].path,
                PathBuf::from("zebra.txt")
            );
        }

        #[test]
        fn computes_working_diff_stats() {
            let (dir, repo) = create_test_repo();

            // Modify file with known insertions/deletions
            fs::write(
                dir.path().join("initial.txt"),
                "line one\nline two\nline three\n",
            )
            .expect("Failed to write file");

            let status = repo
                .status()
                .expect("Failed to get status");

            assert_eq!(status.files.len(), 1);
            // Original had 1 line, new has 3 lines = 3 insertions, 1 deletion
            assert_eq!(status.files[0].working_insertions, 3);
            assert_eq!(status.files[0].working_deletions, 1);
        }

        #[test]
        fn computes_staged_diff_stats() {
            let (dir, repo) = create_test_repo();

            // Modify and stage file
            fs::write(
                dir.path().join("initial.txt"),
                "new line one\nnew line two\n",
            )
            .expect("Failed to write file");
            repo.stage(Path::new("initial.txt"))
                .expect("Failed to stage");

            let status = repo
                .status()
                .expect("Failed to get status");

            assert_eq!(status.files.len(), 1);
            // Original had 1 line, new has 2 lines = 2 insertions, 1 deletion
            assert_eq!(status.files[0].staged_insertions, 2);
            assert_eq!(status.files[0].staged_deletions, 1);
            // No working changes after staging
            assert_eq!(status.files[0].working_insertions, 0);
            assert_eq!(status.files[0].working_deletions, 0);
        }
    }

    mod branch_info {
        use super::*;

        #[test]
        fn returns_branch_name() {
            let (_dir, repo) = create_test_repo();

            let status = repo
                .status()
                .expect("Failed to get status");

            // Default branch after init is usually "master" or "main"
            assert!(status.branch.is_some());
            let branch = status
                .branch
                .expect("should have branch");
            assert!(branch == "master" || branch == "main");
        }

        #[test]
        fn detached_head_shows_short_sha() {
            let (dir, _repo) = create_test_repo();

            // Detach HEAD by checking out a commit directly
            {
                let git_repo = Repository::open(dir.path()).expect("Failed to open");
                let head = git_repo
                    .head()
                    .expect("Failed to get HEAD");
                let commit = head
                    .peel_to_commit()
                    .expect("Failed to get commit");
                git_repo
                    .set_head_detached(commit.id())
                    .expect("Failed to detach HEAD");
            }

            // Re-open to get fresh state
            let repo = GitRepo::open(dir.path()).expect("Failed to open");
            let status = repo
                .status()
                .expect("Failed to get status");

            assert!(status.branch.is_some());
            let branch = status
                .branch
                .expect("should have branch");
            // Should be a 7-char SHA, not a branch name
            assert_eq!(branch.len(), 7);
            assert!(branch
                .chars()
                .all(|c| c.is_ascii_hexdigit()));
        }

        #[test]
        fn no_upstream_by_default() {
            let (_dir, repo) = create_test_repo();

            let status = repo
                .status()
                .expect("Failed to get status");

            assert!(status.upstream.is_none());
            assert_eq!(status.ahead, 0);
            assert_eq!(status.behind, 0);
        }
    }
}
