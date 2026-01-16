use std::path::PathBuf;

use anyhow::{Context, Result};
use git2::{Status, StatusOptions};

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
    fn populate_branch_info(&self, status: &mut GitStatus) -> Result<()> {
        // Get HEAD reference
        let head = match self.repo.head() {
            Ok(head) => head,
            Err(_) => return Ok(()), // No commits yet
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
                status.branch = Some(format!("{:.7}", oid));
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

        for entry in statuses.iter() {
            let path = entry
                .path()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("<invalid>"));

            let git_status = entry.status();

            let file_status = FileStatus {
                path,
                working: working_state_from_git2(git_status),
                staged: staged_state_from_git2(git_status),
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
