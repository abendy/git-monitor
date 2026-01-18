use super::App;
use crate::actions::{AppState, Context};
use crate::git::FileState;
use crate::section::{
    BranchesSectionData, CommandSectionData, HistorySectionData, SectionItemCounts,
    SectionRegistry, StagedSectionData, WorkingSectionData,
};

impl App {
    // ─────────────────────────────────────────────────────────────────────────
    // Section management
    // ─────────────────────────────────────────────────────────────────────────

    /// Update all sections with current app state
    ///
    /// Called after state changes (refresh, mode change, etc.) to keep
    /// section data in sync with app state.
    pub fn update_sections(&mut self) {
        // Update command section
        self.command_section
            .update(CommandSectionData {
                input: self.command_input.clone(),
                is_active: self.is_command_mode(),
                output: self.feedback.output().cloned(),
                menu_active: self.menu_stack.is_active(),
            });

        // Update staged section
        self.staged_section
            .update(StagedSectionData {
                files: self
                    .status
                    .staged_changes()
                    .into_iter()
                    .cloned()
                    .collect(),
                command_mode_active: self.is_command_mode(),
                action_registry: Some(self.action_registry.clone()),
                app_state: Some(self.app_state()),
            });

        // Update working section
        self.working_section
            .update(WorkingSectionData {
                files: self
                    .status
                    .working_changes()
                    .into_iter()
                    .cloned()
                    .collect(),
                command_mode_active: self.is_command_mode(),
                action_registry: Some(self.action_registry.clone()),
                app_state: Some(self.app_state()),
            });

        // Update history section
        self.history_section
            .update(HistorySectionData {
                activity: self.activity.clone(),
                history_mode: self.history_mode,
                is_collapsed: self.history_collapsed,
                page: self.history_page,
                total_items: self.history_total_items,
                total_pages: self.history_total_pages,
                current_branch: self
                    .branches
                    .iter()
                    .find(|b| b.is_current)
                    .cloned(),
                status: self.status.clone(),
                expanded_commit: self.expanded_commit.clone(),
                expanded_detail: self.expanded_detail.clone(),
                expanded_file_idx: self.expanded_file_idx,
                command_mode_active: self.is_command_mode(),
                action_registry: Some(self.action_registry.clone()),
                app_state: Some(self.app_state()),
            });

        // Update branches section
        self.branches_section
            .update(BranchesSectionData {
                branches: self.branches.clone(),
                expanded_branch: self.expanded_branch.clone(),
                expanded_branch_commits: self.expanded_branch_commits.clone(),
                expanded_commit: self.expanded_commit.clone(),
                expanded_detail: self.expanded_detail.clone(),
                expanded_file_idx: self.expanded_file_idx,
                command_mode_active: self.is_command_mode(),
                action_registry: Some(self.action_registry.clone()),
                app_state: Some(self.app_state()),
            });
    }

    /// Get item counts for all sections (for registry calculations)
    ///
    /// Computes directly from app state to ensure consistency even if
    /// section data is stale. This is critical for index calculations.
    #[must_use]
    pub fn section_item_counts(&self) -> SectionItemCounts {
        SectionItemCounts {
            command: 1, // Command section always has 1 item
            staged: self.status.staged_changes().len(),
            working: self.status.working_changes().len(),
            history: if self.history_collapsed {
                1 // Just the header when collapsed
            } else {
                1 + self.activity.len() // Header + commits
            },
            branches: self.branch_items_len(),
        }
    }

    /// Get the section registry for iteration and index lookups
    #[must_use]
    pub fn section_registry(&self) -> &SectionRegistry {
        &self.section_registry
    }

    /// Determine the current context based on selection and state
    #[must_use]
    pub fn current_context(&self) -> Context {
        // Handle special modes first
        if self.is_command_mode() {
            return Context::Command;
        }

        let Some(selected) = self.selected else {
            return Context::Global;
        };

        // Check if in expanded commit files (special case not in registry)
        if self.expanded_commit.is_some() && self.expanded_file_idx.is_some() {
            return Context::CommitFiles;
        }

        if self
            .branch_header_for_selection()
            .is_some()
        {
            return Context::BranchHeader;
        }

        // Use registry for standard section context resolution
        let counts = self.section_item_counts();
        self.section_registry
            .context_for_index(selected, &counts)
    }

    /// Get current app state for condition evaluation
    #[must_use]
    pub fn app_state(&self) -> AppState {
        AppState {
            ahead: self.status.ahead,
            behind: self.status.behind,
            has_upstream: self.status.upstream.is_some(),
            staged_count: self.status.staged_changes().len(),
            working_count: self.status.working_changes().len(),
            untracked_count: self
                .status
                .files
                .iter()
                .filter(|f| f.working == FileState::Untracked)
                .count(),
        }
    }

    /// Total count of all selectable items
    pub fn total_count(&self) -> usize {
        let counts = self.section_item_counts();
        self.section_registry
            .total_items(&counts)
    }
}
