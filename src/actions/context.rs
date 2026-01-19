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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_command() {
        assert_eq!(Context::Command.display_name(), "Command");
    }

    #[test]
    fn display_name_staged_files() {
        assert_eq!(Context::StagedFiles.display_name(), "Staged Files");
    }

    #[test]
    fn display_name_working_files() {
        assert_eq!(Context::WorkingFiles.display_name(), "Working Files");
    }

    #[test]
    fn display_name_history_header() {
        assert_eq!(Context::HistoryHeader.display_name(), "History");
    }

    #[test]
    fn display_name_history_commits() {
        assert_eq!(Context::HistoryCommits.display_name(), "History");
    }

    #[test]
    fn display_name_commit_files() {
        assert_eq!(Context::CommitFiles.display_name(), "Commit Files");
    }

    #[test]
    fn display_name_branch_header() {
        assert_eq!(Context::BranchHeader.display_name(), "Branch");
    }

    #[test]
    fn display_name_branch_commits() {
        assert_eq!(Context::BranchCommits.display_name(), "Branch Commits");
    }

    #[test]
    fn display_name_global() {
        assert_eq!(Context::Global.display_name(), "Global");
    }

    #[test]
    fn contexts_are_distinct() {
        assert_ne!(Context::Command, Context::StagedFiles);
        assert_ne!(Context::StagedFiles, Context::WorkingFiles);
        assert_ne!(Context::HistoryHeader, Context::HistoryCommits);
        assert_ne!(Context::BranchHeader, Context::BranchCommits);
    }
}
