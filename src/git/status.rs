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
            status.branch = head.shorthand().map(String::from);

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
                || PathBuf::from("<invalid>"),
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
