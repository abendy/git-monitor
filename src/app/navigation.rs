use super::App;
use crate::section::SectionId;

impl App {
    /// Clamp selection to valid bounds
    ///
    /// Called after item counts change (file changes, commits, etc.) to ensure
    /// the selection index remains valid.
    pub(super) fn clamp_selection(&mut self) {
        if let Some(idx) = self.selected {
            let total = self.total_count();
            if total == 0 {
                self.selected = None;
            } else if idx >= total {
                self.selected = Some(total - 1);
            }
        }
    }

    /// Jump to working area (first file in staged or working changes)
    pub(super) fn jump_to_working(&mut self) {
        let files_total = self.status.staged_changes().len() + self.status.working_changes().len();
        if files_total == 0 {
            return;
        }

        // Select first file (index 1, after any spacer)
        self.selected = Some(1);
        self.close_expanded_commit();
    }

    /// Jump to history and expand it, landing on most recent commit
    pub(super) fn jump_to_history(&mut self) {
        if self.activity.is_empty() {
            return;
        }

        // Expand history (collapses any expanded branch)
        self.expand_history();

        let staged_len = self.status.staged_changes().len();
        let working_len = self.status.working_changes().len();
        let files_total = staged_len + working_len;

        // Select first commit (skip header)
        let history_header_idx = 1 + files_total;
        self.selected = Some(history_header_idx + 1);
        self.close_expanded_commit();
    }

    /// Jump to first branch and expand it, landing on first commit
    pub(super) fn jump_to_branches(&mut self) {
        let other_branches = self.other_branches();
        if other_branches.is_empty() {
            return;
        }

        // Get first branch name and expand it
        let first_branch_name = other_branches[0].name.clone();
        self.expand_branch(&first_branch_name);

        // Select first commit in the branch
        let branches_start = self.branches_start_index();
        self.selected = Some(branches_start);
        self.close_expanded_commit();
    }

    /// Select next item with accordion auto-expand/collapse
    pub(super) fn select_next(&mut self) {
        let len = self.total_count();
        if len == 0 {
            return;
        }

        match self.selected {
            None => {
                self.selected = Some(0);
            }
            Some(idx) => {
                self.close_expanded_commit();

                // Are we on the last history commit about to move to branches?
                let on_last_history_commit = !self.history_collapsed
                    && !self.activity.is_empty()
                    && idx == self.history_start_index() + self.activity.len();

                if on_last_history_commit {
                    // Moving from last history commit to first branch's first commit
                    let other_branches = self.other_branches();
                    if let Some(first_branch) = other_branches.first() {
                        let branch_name = first_branch.name.clone();
                        self.expand_branch(&branch_name);
                        self.selected = Some(self.branches_start_index());
                        return;
                    }
                }

                // Don't go past the last item if no branches to expand
                if idx >= len - 1 {
                    return;
                }

                // Normal navigation
                let new_idx = idx + 1;
                self.selected = Some(new_idx);
            }
        }
    }

    /// Select previous item with accordion auto-expand/collapse
    pub(super) fn select_prev(&mut self) {
        match self.selected {
            Some(idx) if idx > 0 => {
                self.close_expanded_commit();

                if self.history_collapsed
                    && !self.activity.is_empty()
                    && idx == self.branches_start_index()
                {
                    self.expand_history();
                    let history_header_idx = self.history_start_index();
                    self.selected = Some(history_header_idx + self.activity.len());
                    return;
                }

                // Normal navigation
                self.selected = Some(idx - 1);
            }
            _ => {}
        }
    }

    /// Select first item
    pub(super) fn select_first(&mut self) {
        self.selected = Some(0);
        self.close_expanded_commit();
    }

    /// Select last item
    pub(super) fn select_last(&mut self) {
        let len = self.total_count();
        if len > 0 {
            self.selected = Some(len - 1);
            self.close_expanded_commit();
        }
    }

    pub(super) fn select_default_section(&mut self) {
        let counts = self.section_item_counts();

        if counts.working > 0 {
            if let Some(start) = self
                .section_registry
                .section_start_index(SectionId::Working, &counts)
            {
                self.selected = Some(start);
                return;
            }
        }

        if counts.staged > 0 {
            if let Some(start) = self
                .section_registry
                .section_start_index(SectionId::Staged, &counts)
            {
                self.selected = Some(start);
                return;
            }
        }

        self.selected = Some(0);
    }
}
