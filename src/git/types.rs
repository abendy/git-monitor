use std::path::PathBuf;

use chrono::{DateTime, Local};
use git2::RepositoryState;

pub use super::command_type::CommandType;

/// Status of a file in the repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStatus {
    /// Path relative to repository root
    pub path: PathBuf,
    /// Status in working directory
    pub working: FileState,
    /// Status in staging area (index)
    pub staged: FileState,
    /// Lines inserted in working directory changes
    pub working_insertions: usize,
    /// Lines deleted in working directory changes
    pub working_deletions: usize,
    /// Lines inserted in staged changes
    pub staged_insertions: usize,
    /// Lines deleted in staged changes
    pub staged_deletions: usize,
}

/// State of a file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileState {
    #[default]
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    #[allow(dead_code)]
    Ignored,
    Conflicted,
}

impl FileState {
    /// Character representation (like git status --short)
    pub const fn as_char(self) -> char {
        match self {
            Self::Unmodified => ' ',
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Untracked => '?',
            Self::Ignored => '!',
            Self::Conflicted => 'U',
        }
    }

    pub const fn is_changed(self) -> bool {
        !matches!(self, Self::Unmodified | Self::Ignored)
    }
}

/// Repository state (merging, rebasing, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepoState {
    #[default]
    Normal,
    Merge,
    Rebase,
    RebaseInteractive,
    CherryPick,
    Revert,
    Bisect,
}

impl From<RepositoryState> for RepoState {
    fn from(state: RepositoryState) -> Self {
        match state {
            RepositoryState::Merge => Self::Merge,
            RepositoryState::Rebase | RepositoryState::RebaseMerge => Self::Rebase,
            RepositoryState::RebaseInteractive => Self::RebaseInteractive,
            RepositoryState::CherryPick | RepositoryState::CherryPickSequence => Self::CherryPick,
            RepositoryState::Revert | RepositoryState::RevertSequence => Self::Revert,
            RepositoryState::Bisect => Self::Bisect,
            _ => Self::Normal,
        }
    }
}

/// Complete git status snapshot
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    /// Current branch name (None if detached HEAD)
    pub branch: Option<String>,
    /// Upstream branch name if tracking
    pub upstream: Option<String>,
    /// Commits ahead of upstream
    pub ahead: usize,
    /// Commits behind upstream
    pub behind: usize,
    /// Files with changes
    pub files: Vec<FileStatus>,
    /// Repository state
    #[allow(dead_code)]
    pub state: RepoState,
}

impl GitStatus {
    /// Files changed in working directory (not staged)
    pub fn working_changes(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| f.working.is_changed())
            .collect()
    }

    /// Files staged for commit
    pub fn staged_changes(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| f.staged.is_changed())
            .collect()
    }
}

/// A decoration (branch, tag, etc.) attached to a commit
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefDecoration {
    /// HEAD pointer
    Head,
    /// Local branch
    LocalBranch(String),
    /// Remote branch (e.g., origin/main)
    RemoteBranch(String),
    /// Tag
    Tag(String),
}

/// A git command/action from the reflog
#[derive(Debug, Clone)]
pub struct GitCommand {
    /// When the command was executed
    pub timestamp: DateTime<Local>,
    /// Type of command
    pub command_type: CommandType,
    /// Command message/description
    pub message: String,
    /// Short SHA if available
    pub sha: Option<String>,
    /// Decorations (branches, tags) pointing to this commit
    pub decorations: Vec<RefDecoration>,
    /// True if this commit only exists on remote (not reachable from HEAD)
    pub is_remote_only: bool,
}

/// Detailed commit information for expanded view
#[derive(Debug, Clone)]
pub struct CommitDetail {
    /// Full 40-character SHA
    pub full_sha: String,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Author timestamp
    pub author_time: DateTime<Local>,
    /// Committer name
    pub committer_name: String,
    /// Committer email
    pub committer_email: String,
    /// Committer timestamp
    #[allow(dead_code)]
    pub committer_time: DateTime<Local>,
    /// Full commit message (summary + body)
    pub message: String,
    /// GPG signature status (if signed)
    pub gpg_status: Option<String>,
    /// Files changed in this commit
    pub files: Vec<CommitFile>,
    /// Total lines added
    pub insertions: usize,
    /// Total lines deleted
    pub deletions: usize,
}

/// A file changed in a commit
#[derive(Debug, Clone)]
pub struct CommitFile {
    /// File path
    pub path: String,
    /// Change status
    pub status: FileState,
    /// Lines added in this file
    pub insertions: usize,
    /// Lines deleted in this file
    pub deletions: usize,
}

/// Information about a local branch
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Whether this is the current (checked out) branch
    pub is_current: bool,
    /// Whether this is a remote tracking branch
    pub is_remote: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod file_state {
        use super::*;

        #[test]
        fn as_char_returns_correct_characters() {
            assert_eq!(FileState::Unmodified.as_char(), ' ');
            assert_eq!(FileState::Modified.as_char(), 'M');
            assert_eq!(FileState::Added.as_char(), 'A');
            assert_eq!(FileState::Deleted.as_char(), 'D');
            assert_eq!(FileState::Renamed.as_char(), 'R');
            assert_eq!(FileState::Untracked.as_char(), '?');
            assert_eq!(FileState::Ignored.as_char(), '!');
            assert_eq!(FileState::Conflicted.as_char(), 'U');
        }

        #[test]
        fn is_changed_returns_false_for_unchanged_states() {
            assert!(!FileState::Unmodified.is_changed());
            assert!(!FileState::Ignored.is_changed());
        }

        #[test]
        fn is_changed_returns_true_for_changed_states() {
            assert!(FileState::Modified.is_changed());
            assert!(FileState::Added.is_changed());
            assert!(FileState::Deleted.is_changed());
            assert!(FileState::Renamed.is_changed());
            assert!(FileState::Untracked.is_changed());
            assert!(FileState::Conflicted.is_changed());
        }
    }

    mod git_status {
        use super::*;

        fn make_file(path: &str, working: FileState, staged: FileState) -> FileStatus {
            FileStatus {
                path: PathBuf::from(path),
                working,
                staged,
                working_insertions: 0,
                working_deletions: 0,
                staged_insertions: 0,
                staged_deletions: 0,
            }
        }

        #[test]
        fn working_changes_filters_modified_files() {
            let status = GitStatus {
                files: vec![
                    make_file(
                        "changed.txt",
                        FileState::Modified,
                        FileState::Unmodified,
                    ),
                    make_file(
                        "unchanged.txt",
                        FileState::Unmodified,
                        FileState::Unmodified,
                    ),
                    make_file(
                        "added.txt",
                        FileState::Added,
                        FileState::Unmodified,
                    ),
                ],
                ..Default::default()
            };
            let changes = status.working_changes();
            assert_eq!(changes.len(), 2);
            assert_eq!(
                changes[0].path,
                PathBuf::from("changed.txt")
            );
            assert_eq!(
                changes[1].path,
                PathBuf::from("added.txt")
            );
        }

        #[test]
        fn staged_changes_filters_staged_files() {
            let status = GitStatus {
                files: vec![
                    make_file(
                        "staged.txt",
                        FileState::Unmodified,
                        FileState::Modified,
                    ),
                    make_file(
                        "unstaged.txt",
                        FileState::Modified,
                        FileState::Unmodified,
                    ),
                    make_file(
                        "both.txt",
                        FileState::Modified,
                        FileState::Added,
                    ),
                ],
                ..Default::default()
            };
            let staged = status.staged_changes();
            assert_eq!(staged.len(), 2);
            assert_eq!(
                staged[0].path,
                PathBuf::from("staged.txt")
            );
            assert_eq!(
                staged[1].path,
                PathBuf::from("both.txt")
            );
        }
    }
}
