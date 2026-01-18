use super::App;
use crate::git::BranchInfo;
use crate::section::SectionId;

impl App {
    /// Check if selection is on a branch header
    pub(super) fn is_on_branch_header(&self) -> Option<String> {
        self.branch_header_for_selection()
            .map(|branch| branch.name.clone())
    }

    /// Get the index where branches section starts
    pub(super) fn branches_start_index(&self) -> usize {
        let counts = self.section_item_counts();
        self.section_registry
            .section_start_index(SectionId::Branches, &counts)
            .unwrap_or(0)
    }

    /// Get branches excluding the current one (for accordion display)
    pub fn other_branches(&self) -> Vec<&BranchInfo> {
        self.branches
            .iter()
            .filter(|b| !b.is_current)
            .collect()
    }

    /// Total selectable items in the branches section (headers + expanded commits).
    pub(super) fn branch_items_len(&self) -> usize {
        let branches = self.other_branches();
        let header_count = branches.len();
        if header_count == 0 {
            return 0;
        }

        let commit_count = if self
            .expanded_branch_index(&branches)
            .is_some()
        {
            self.expanded_branch_commits.len()
        } else {
            0
        };

        header_count + commit_count
    }

    /// Local selection index within the branches section (if selected).
    pub(super) fn branch_local_selection(&self) -> Option<usize> {
        let selected = self.selected?;
        let counts = self.section_item_counts();
        let lookup = self
            .section_registry
            .lookup_index(selected, &counts)?;
        if lookup.section_id == SectionId::Branches {
            Some(lookup.local_index)
        } else {
            None
        }
    }

    /// Index of the expanded branch within the other-branches list.
    pub(super) fn expanded_branch_index(&self, branches: &[&BranchInfo]) -> Option<usize> {
        let expanded = self.expanded_branch.as_deref()?;
        branches
            .iter()
            .position(|b| b.name == expanded)
    }

    /// Return the branch header at a local index, if the index targets a header.
    pub(super) fn branch_header_for_local_index(&self, local_index: usize) -> Option<&BranchInfo> {
        let branches = self.other_branches();
        if branches.is_empty() {
            return None;
        }

        let commit_len = if self.expanded_branch.is_some() {
            self.expanded_branch_commits.len()
        } else {
            0
        };

        let Some(expanded_idx) = self.expanded_branch_index(&branches) else {
            return branches.get(local_index).copied();
        };

        if commit_len == 0 {
            return branches.get(local_index).copied();
        }

        let commit_start = expanded_idx + 1;
        let commit_end = commit_start + commit_len;

        if local_index < commit_start {
            branches.get(local_index).copied()
        } else if local_index >= commit_end {
            branches
                .get(local_index - commit_len)
                .copied()
        } else {
            None
        }
    }

    /// Return the expanded-branch commit index at a local index, if any.
    pub(super) fn branch_commit_index_for_local_index(&self, local_index: usize) -> Option<usize> {
        let branches = self.other_branches();
        let expanded_idx = self.expanded_branch_index(&branches)?;
        let commit_len = self.expanded_branch_commits.len();
        if commit_len == 0 {
            return None;
        }

        let commit_start = expanded_idx + 1;
        let commit_end = commit_start + commit_len;

        if (commit_start..commit_end).contains(&local_index) {
            Some(local_index - commit_start)
        } else {
            None
        }
    }

    /// Return the selected branch header, if selection is on a header.
    pub(super) fn branch_header_for_selection(&self) -> Option<&BranchInfo> {
        let local_index = self.branch_local_selection()?;
        self.branch_header_for_local_index(local_index)
    }

    /// Return the selected commit index within expanded branch (if any).
    pub(super) fn branch_commit_index_for_selection(&self) -> Option<usize> {
        let local_index = self.branch_local_selection()?;
        self.branch_commit_index_for_local_index(local_index)
    }

    /// Expand a branch to show its commits
    pub(super) fn expand_branch(&mut self, branch_name: &str) {
        // Collapse history when expanding a branch
        self.history_collapsed = true;

        // Collapse any previously expanded branch
        if self.expanded_branch.as_deref() == Some(branch_name) {
            return; // Already expanded
        }

        // Fetch commits unique to this branch
        match self
            .repo
            .commit_log_for_branch(branch_name)
        {
            Ok(commits) => {
                self.expanded_branch = Some(branch_name.to_string());
                self.expanded_branch_commits = commits;
            }
            Err(e) => {
                self.feedback.error = Some(format!(
                    "Failed to load branch history: {e}"
                ));
            }
        }
        self.update_sections();
    }

    /// Collapse the currently expanded branch
    ///
    /// Note: Does NOT call `update_sections()` - callers are responsible for updating.
    pub(super) fn collapse_branch(&mut self) {
        self.expanded_branch = None;
        self.expanded_branch_commits.clear();
    }

    /// Get the selected branch info (if on a branch header)
    pub(super) fn selected_branch(&self) -> Option<&BranchInfo> {
        self.branch_header_for_selection()
    }

    /// Checkout the currently selected branch
    pub(super) fn checkout_selected_branch(&mut self) {
        let branch_name = match self.selected_branch() {
            Some(branch) if branch.is_current => {
                self.feedback.error = Some("Already on this branch".to_string());
                return;
            }
            Some(branch) => branch.name.clone(),
            None => return,
        };

        match self.repo.checkout_branch(&branch_name) {
            Ok(()) => {
                self.feedback.error = Some(format!(
                    "Switched to branch '{branch_name}'"
                ));
                self.refresh_status();
            }
            Err(e) => {
                self.feedback.error = Some(e.to_string());
            }
        }
    }
}
