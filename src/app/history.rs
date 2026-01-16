use crate::feedback::PopupContent;
use crate::section::SectionId;

use super::{App, HistoryMode};

impl App {
    /// Toggle history mode between reflog and commit log
    pub(super) fn toggle_history_mode(&mut self) {
        self.history_mode = match self.history_mode {
            HistoryMode::Reflog => HistoryMode::CommitLog,
            HistoryMode::CommitLog => HistoryMode::Reflog,
        };
        self.history_page = 0; // Reset to first page when switching modes
        self.refresh_activity();
        self.update_sections();
    }

    /// Go to next history page
    pub(super) fn next_history_page(&mut self) {
        if self.history_page + 1 < self.history_total_pages {
            self.history_page += 1;
            self.refresh_activity();
            self.select_first_history_commit();
            self.update_sections();
        }
    }

    /// Go to previous history page
    pub(super) fn prev_history_page(&mut self) {
        if self.history_page > 0 {
            self.history_page -= 1;
            self.refresh_activity();
            self.select_first_history_commit();
            self.update_sections();
        }
    }

    /// Select the first commit in the history section
    pub(super) fn select_first_history_commit(&mut self) {
        if !self.activity.is_empty() {
            let files_total =
                self.status.staged_changes().len() + self.status.working_changes().len();
            // First commit is after header: 1 (command) + files_total + 1 (header)
            self.selected = Some(1 + files_total + 1);
        }
    }

    /// Go to first history page
    #[allow(dead_code)]
    pub(super) fn first_history_page(&mut self) {
        if self.history_page != 0 {
            self.history_page = 0;
            self.refresh_activity();
            self.update_sections();
        }
    }

    /// Check if selection is on the History header
    pub(super) fn is_on_history_header(&self) -> bool {
        let Some(selected) = self.selected else {
            return false;
        };
        selected == self.history_start_index()
    }

    /// Check if selection is in the history commits section (not on header)
    pub(super) fn is_in_history(&self) -> bool {
        if self.history_collapsed {
            return false;
        }
        let Some(selected) = self.selected else {
            return false;
        };
        // History header is at history_start_index(), commits start at +1
        let history_commits_start = self.history_start_index() + 1;
        let history_commits_end = history_commits_start + self.activity.len();
        selected >= history_commits_start && selected < history_commits_end
    }

    /// Get the index where history section starts (the header)
    pub(super) fn history_start_index(&self) -> usize {
        let counts = self.section_item_counts();
        self.section_registry
            .section_start_index(SectionId::History, &counts)
            .unwrap_or(0)
    }

    /// Check if currently selected item is the expanded commit
    pub(super) fn is_on_expanded_commit(&self) -> bool {
        if let Some(expanded_sha) = &self.expanded_commit {
            if let Some(selected_sha) = self.selected_activity_sha() {
                return expanded_sha == &selected_sha;
            }
        }
        false
    }

    /// Open diff popup for a file in a specific commit
    pub(super) fn open_commit_file_diff(&mut self, commit_sha: &str, file_path: &str) {
        match self
            .repo
            .commit_file_diff(commit_sha, file_path)
        {
            Ok(diff_content) => {
                self.feedback
                    .popup
                    .open(PopupContent::Diff {
                        path: file_path.to_string(),
                        content: diff_content,
                        is_staged: false,
                    });
            }
            Err(e) => {
                self.feedback.error = Some(format!("Failed to get diff: {e}"));
            }
        }
    }

    /// Collapse the History section
    pub(super) fn collapse_history(&mut self) {
        self.history_collapsed = true;
        self.close_expanded_commit();
        self.update_sections();
    }

    /// Expand the History section
    pub(super) fn expand_history(&mut self) {
        self.history_collapsed = false;
        // Collapse any expanded branch when expanding history
        self.collapse_branch();
        self.update_sections();
    }

    /// Close expanded commit detail
    pub(super) fn close_expanded_commit(&mut self) {
        if self.expanded_commit.is_none()
            && self.expanded_detail.is_none()
            && self.expanded_file_idx.is_none()
        {
            return;
        }

        self.expanded_commit = None;
        self.expanded_detail = None;
        self.expanded_file_idx = None;
        self.update_sections();
    }

    pub(super) fn toggle_commit_detail(&mut self, sha: &str) {
        if self.expanded_commit.as_deref() == Some(sha) {
            self.close_expanded_commit();
            return;
        }

        self.expanded_commit = Some(sha.to_string());
        self.expanded_detail = self.repo.commit_detail(sha).ok();
        self.expanded_file_idx = None;
        self.update_sections();
    }

    /// Get the selected commit's SHA (from history or expanded branch)
    pub(super) fn selected_activity_sha(&self) -> Option<String> {
        let selected = self.selected?;

        // Check if in history commits (not collapsed)
        let history_header_idx = self.history_start_index();
        if !self.history_collapsed && selected > history_header_idx {
            let history_commits_start = history_header_idx + 1;
            let history_commits_end = history_commits_start + self.activity.len();

            if selected >= history_commits_start && selected < history_commits_end {
                let commit_idx = selected - history_commits_start;
                return self
                    .activity
                    .get(commit_idx)
                    .and_then(|cmd| cmd.sha.clone());
            }
        }

        // Check if in expanded branch commits
        if let Some(commit_idx) = self.branch_commit_index_for_selection() {
            return self
                .expanded_branch_commits
                .get(commit_idx)
                .and_then(|cmd| cmd.sha.clone());
        }

        None
    }
}
