//! Application state for evaluating action conditions.

/// Conditions for when an action should be available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum ActionCondition {
    /// Always available (no condition)
    #[default]
    Always,
    /// Branch is ahead of remote (can push)
    BranchAhead,
    /// Branch is behind remote (should pull)
    BranchBehind,
    /// Branch has upstream configured
    HasUpstream,
    /// Branch has no upstream configured
    NoUpstream,
    /// Has staged changes ready to commit
    HasStagedChanges,
    /// Has unstaged changes in working directory
    HasWorkingChanges,
    /// Has untracked files
    HasUntrackedFiles,
}

/// Snapshot of app state for evaluating action conditions
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub ahead: usize,
    pub behind: usize,
    pub has_upstream: bool,
    pub staged_count: usize,
    pub working_count: usize,
    pub untracked_count: usize,
}

impl AppState {
    /// Check if a condition is satisfied
    #[must_use]
    pub const fn satisfies(&self, condition: ActionCondition) -> bool {
        match condition {
            ActionCondition::Always => true,
            ActionCondition::BranchAhead => self.ahead > 0,
            ActionCondition::BranchBehind => self.behind > 0,
            ActionCondition::HasUpstream => self.has_upstream,
            ActionCondition::NoUpstream => !self.has_upstream,
            ActionCondition::HasStagedChanges => self.staged_count > 0,
            ActionCondition::HasWorkingChanges => self.working_count > 0,
            ActionCondition::HasUntrackedFiles => self.untracked_count > 0,
        }
    }
}
