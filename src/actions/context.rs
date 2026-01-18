//! Application context for location-aware actions.

/// Application context - represents where the user currently is
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Context {
    /// On command input section
    Command,
    /// Selection is in staged files
    StagedFiles,
    /// Selection is in working files
    WorkingFiles,
    /// On the History section header
    HistoryHeader,
    /// On a commit in history
    HistoryCommits,
    /// On a file within expanded commit
    CommitFiles,
    /// On a branch header
    BranchHeader,
    /// On a commit in expanded branch
    BranchCommits,
    /// Actions available everywhere
    Global,
}

impl Context {
    /// Display name for the context
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Command => "Command",
            Self::StagedFiles => "Staged Files",
            Self::WorkingFiles => "Working Files",
            Self::HistoryHeader | Self::HistoryCommits => "History",
            Self::CommitFiles => "Commit Files",
            Self::BranchHeader => "Branch",
            Self::BranchCommits => "Branch Commits",
            Self::Global => "Global",
        }
    }
}
