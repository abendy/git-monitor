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

#[cfg(test)]
mod tests {
    use super::*;

    mod action_condition {
        use super::*;

        #[test]
        fn default_is_always() {
            let condition = ActionCondition::default();
            assert_eq!(condition, ActionCondition::Always);
        }
    }

    mod app_state {
        use super::*;

        #[test]
        fn default_has_zero_values() {
            let state = AppState::default();

            assert_eq!(state.ahead, 0);
            assert_eq!(state.behind, 0);
            assert!(!state.has_upstream);
            assert_eq!(state.staged_count, 0);
            assert_eq!(state.working_count, 0);
            assert_eq!(state.untracked_count, 0);
        }

        #[test]
        fn satisfies_always_is_true() {
            let state = AppState::default();

            assert!(state.satisfies(ActionCondition::Always));
        }

        #[test]
        fn satisfies_branch_ahead_when_ahead() {
            let state = AppState {
                ahead: 3,
                ..Default::default()
            };

            assert!(state.satisfies(ActionCondition::BranchAhead));
        }

        #[test]
        fn satisfies_branch_ahead_false_when_not_ahead() {
            let state = AppState::default();

            assert!(!state.satisfies(ActionCondition::BranchAhead));
        }

        #[test]
        fn satisfies_branch_behind_when_behind() {
            let state = AppState {
                behind: 2,
                ..Default::default()
            };

            assert!(state.satisfies(ActionCondition::BranchBehind));
        }

        #[test]
        fn satisfies_branch_behind_false_when_not_behind() {
            let state = AppState::default();

            assert!(!state.satisfies(ActionCondition::BranchBehind));
        }

        #[test]
        fn satisfies_has_upstream_when_true() {
            let state = AppState {
                has_upstream: true,
                ..Default::default()
            };

            assert!(state.satisfies(ActionCondition::HasUpstream));
        }

        #[test]
        fn satisfies_has_upstream_false_when_no_upstream() {
            let state = AppState::default();

            assert!(!state.satisfies(ActionCondition::HasUpstream));
        }

        #[test]
        fn satisfies_no_upstream_when_no_upstream() {
            let state = AppState::default();

            assert!(state.satisfies(ActionCondition::NoUpstream));
        }

        #[test]
        fn satisfies_no_upstream_false_when_has_upstream() {
            let state = AppState {
                has_upstream: true,
                ..Default::default()
            };

            assert!(!state.satisfies(ActionCondition::NoUpstream));
        }

        #[test]
        fn satisfies_has_staged_changes_when_staged() {
            let state = AppState {
                staged_count: 5,
                ..Default::default()
            };

            assert!(state.satisfies(ActionCondition::HasStagedChanges));
        }

        #[test]
        fn satisfies_has_staged_changes_false_when_none() {
            let state = AppState::default();

            assert!(!state.satisfies(ActionCondition::HasStagedChanges));
        }

        #[test]
        fn satisfies_has_working_changes_when_working() {
            let state = AppState {
                working_count: 3,
                ..Default::default()
            };

            assert!(state.satisfies(ActionCondition::HasWorkingChanges));
        }

        #[test]
        fn satisfies_has_working_changes_false_when_none() {
            let state = AppState::default();

            assert!(!state.satisfies(ActionCondition::HasWorkingChanges));
        }

        #[test]
        fn satisfies_has_untracked_files_when_untracked() {
            let state = AppState {
                untracked_count: 2,
                ..Default::default()
            };

            assert!(state.satisfies(ActionCondition::HasUntrackedFiles));
        }

        #[test]
        fn satisfies_has_untracked_files_false_when_none() {
            let state = AppState::default();

            assert!(!state.satisfies(ActionCondition::HasUntrackedFiles));
        }
    }
}
