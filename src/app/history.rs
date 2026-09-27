use tracing::warn;

use super::selection::SelectionKey;
use super::{App, PageCursor, PageLanding};
use crate::feedback::PopupContent;
use crate::git::HistoryMode;
use crate::section::SectionId;

impl App {
    /// Toggle history mode between reflog and commit log
    pub(super) fn toggle_history_mode(&mut self) {
        self.history_mode = match self.history_mode {
            HistoryMode::Reflog => HistoryMode::CommitLog,
            HistoryMode::CommitLog => HistoryMode::Reflog,
        };
        self.history_page = 0; // Reset to first page when switching modes
        self.load_snapshot();
        self.update_sections();
    }

    /// Go to next history page, back to where the cursor was on it last time
    pub(super) fn next_history_page(&mut self) {
        self.turn_history_page(true, PageLanding::Remembered);
    }

    /// Go to previous history page, back to where the cursor was on it last time
    pub(super) fn prev_history_page(&mut self) {
        self.turn_history_page(false, PageLanding::Remembered);
    }

    /// Turn one history page and place the cursor. Returns false at either end.
    pub(super) fn turn_history_page(&mut self, forward: bool, landing: PageLanding) -> bool {
        let target = if forward {
            if self.history_page + 1 >= self.snapshot.history_pages {
                return false;
            }
            self.history_page + 1
        } else {
            let Some(target) = self.history_page.checked_sub(1) else {
                return false;
            };
            target
        };

        self.remember_history_cursor();
        self.history_page = target;
        let _ = self.load_snapshot();

        match landing {
            PageLanding::Remembered => {
                if !self.restore_history_cursor() {
                    self.select_first_history_commit();
                }
            }
            PageLanding::First => self.select_first_history_commit(),
            PageLanding::Last => self.select_last_history_commit(),
        }
        self.update_sections();
        true
    }

    /// Save the selected commit and scroll position for the current page
    fn remember_history_cursor(&mut self) {
        if !self.is_in_history() {
            return;
        }
        if let Some(sha) = self.selected_activity_sha() {
            self.history_page_cursors.insert(
                (self.history_mode, self.history_page),
                PageCursor {
                    sha,
                    scroll: self.body_scroll,
                },
            );
        }
    }

    /// Put the cursor back where it was on this page. Returns false if there is no saved spot.
    fn restore_history_cursor(&mut self) -> bool {
        let Some(saved) = self
            .history_page_cursors
            .get(&(self.history_mode, self.history_page))
        else {
            return false;
        };
        let scroll = saved.scroll;
        let Some(index) = self.index_for_key(&SelectionKey::HistoryCommit(
            saved.sha.clone(),
        )) else {
            return false;
        };
        self.selected = Some(index);
        self.body_scroll = scroll;
        true
    }

    /// Select the last commit on the current history page
    pub(super) fn select_last_history_commit(&mut self) {
        if !self.snapshot.history.is_empty() {
            self.selected = Some(self.history_start_index() + self.snapshot.history.len());
        }
    }

    /// Select the first commit in the history section
    pub(super) fn select_first_history_commit(&mut self) {
        if !self.snapshot.history.is_empty() {
            let files_total = self
                .snapshot
                .status
                .staged_changes()
                .len()
                + self
                    .snapshot
                    .status
                    .working_changes()
                    .len();
            // First commit is after header: 1 (command) + files_total + 1 (header)
            self.selected = Some(1 + files_total + 1);
        }
    }

    /// Go to first history page
    #[allow(dead_code)]
    pub(super) fn first_history_page(&mut self) {
        if self.history_page != 0 {
            self.history_page = 0;
            self.load_snapshot();
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
        let history_commits_end = history_commits_start + self.snapshot.history.len();
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
        self.expanded_detail = self
            .repo
            .commit_detail(sha)
            .map_err(|e| {
                warn!(
                    "Failed to get commit detail for {}: {}",
                    sha, e
                );
                e
            })
            .ok();
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
            let history_commits_end = history_commits_start + self.snapshot.history.len();

            if selected >= history_commits_start && selected < history_commits_end {
                let commit_idx = selected - history_commits_start;
                return self
                    .snapshot
                    .history
                    .get(commit_idx)
                    .and_then(|cmd| cmd.sha.clone());
            }
        }

        // Check if in expanded branch commits
        if let Some(commit_idx) = self.branch_commit_index_for_selection() {
            return self
                .expanded_branch_commits()
                .get(commit_idx)
                .and_then(|cmd| cmd.sha.clone());
        }

        None
    }
}
