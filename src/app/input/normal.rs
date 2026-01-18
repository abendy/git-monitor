//! Normal mode key handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::actions::AppAction;
use crate::app::{App, PAGE_SIZE};
use crate::command::ExternalCommand;

impl App {
    /// Handle keyboard input in normal mode
    #[allow(clippy::too_many_lines)] // Key dispatch is naturally verbose
    pub(in crate::app) fn handle_normal_key(&mut self, key: KeyEvent) {
        // Try declarative keymap lookup first
        let context = self.current_context();
        if let Some(action) = self.keymap.lookup(key, context) {
            self.execute_app_action(action);
            return;
        }

        // Auto-enter command mode when typing on command section
        // (except for quit, help, and special keys)
        if self.selected == Some(0) {
            if let KeyCode::Char(c) = key.code {
                if !matches!(c, 'q' | '?' | ':' | 'o')
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    self.enter_command_mode();
                    self.command_input.push(c);
                    self.update_sections();
                    return;
                }
            }
        }

        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }

            // Enter command mode
            KeyCode::Char(':') => {
                self.enter_command_mode();
            }

            // Show aliases (universal shortcut)
            KeyCode::Char('a') => {
                self.enter_alias_browser();
            }

            // Open action menu for current context
            KeyCode::Char('m') => {
                self.open_action_menu();
            }

            // Help
            KeyCode::Char('?') => {
                self.show_help = true;
            }

            // Refresh
            KeyCode::Char('r') => {
                self.refresh_status();
            }

            // History pagination: next page (only in history section)
            KeyCode::Char(']') => {
                if self.is_in_history() || self.is_on_history_header() {
                    // Use >= because remote-only commits may add extra items beyond PAGE_SIZE
                    if self.activity.len() >= PAGE_SIZE {
                        self.next_history_page();
                    }
                }
            }

            // History pagination: previous page (only in history section)
            KeyCode::Char('[') => {
                if self.is_in_history() || self.is_on_history_header() {
                    self.prev_history_page();
                }
            }

            // Select first item (vim-style)
            KeyCode::Char('g') => {
                self.select_first();
            }

            // Jump to branches
            KeyCode::Char('b') => {
                self.jump_to_branches();
            }

            // Jump to working area
            KeyCode::Char('w') => {
                self.jump_to_working();
            }

            // Stage/Unstage
            KeyCode::Char('s') => {
                self.toggle_stage();
            }

            // Push current branch (universal shortcut in normal mode)
            KeyCode::Char('P') => {
                self.show_push_confirm(false);
            }

            // Pull (lowercase p)
            KeyCode::Char('p') => {
                self.execute_pull();
            }

            // Fetch
            KeyCode::Char('f') => {
                self.execute_fetch();
            }

            // Diff (inline popup)
            KeyCode::Char('d') => {
                // Check if we're on a file in an expanded commit
                if let (Some(sha), Some(file_idx)) =
                    (self.expanded_commit.clone(), self.expanded_file_idx)
                {
                    let file_path = self
                        .expanded_detail
                        .as_ref()
                        .and_then(|d| d.files.get(file_idx))
                        .map(|f| f.path.clone());

                    if let Some(path) = file_path {
                        self.open_commit_file_diff(&sha, &path);
                    }
                } else {
                    // Working directory file diff
                    self.show_diff();
                }
            }

            // External difftool (uses gitconfig diff.tool)
            KeyCode::Char('M') => {
                // First check if we're on a working/staged file
                if let Some((path, is_staged, _)) = self.selected_file_info() {
                    self.pending_external = Some(ExternalCommand::FileDiffTool {
                        file_path: path.to_string_lossy().to_string(),
                        staged: is_staged,
                    });
                } else if let (Some(sha), Some(file_idx)) =
                    (self.expanded_commit.clone(), self.expanded_file_idx)
                {
                    // Commit file difftool
                    let file_path = self
                        .expanded_detail
                        .as_ref()
                        .and_then(|d| d.files.get(file_idx))
                        .map(|f| f.path.clone());

                    if let Some(path) = file_path {
                        self.pending_external = Some(ExternalCommand::DiffTool {
                            commit_sha: sha,
                            file_path: path,
                        });
                    }
                }
            }
            KeyCode::Enter => {
                if self.selected == Some(0) {
                    // Activate command mode when on command section
                    self.enter_command_mode();
                } else if self.is_on_branch_header().is_some() {
                    // Checkout selected branch
                    self.checkout_selected_branch();
                } else {
                    self.show_diff();
                }
            }

            // List navigation (with expanded commit support)
            KeyCode::Char('j') | KeyCode::Down => {
                if self.is_on_expanded_commit() {
                    // Navigate within expanded commit
                    let file_count = self
                        .expanded_detail
                        .as_ref()
                        .map_or(0, |d| d.files.len());

                    match self.expanded_file_idx {
                        None if file_count > 0 => {
                            // Move from header to first file
                            self.expanded_file_idx = Some(0);
                            self.update_sections();
                        }
                        Some(idx) if idx + 1 < file_count => {
                            // Move to next file
                            self.expanded_file_idx = Some(idx + 1);
                            self.update_sections();
                        }
                        Some(_) | None => {
                            // At last file or no files - move to next commit
                            self.expanded_file_idx = None;
                            self.select_next();
                        }
                    }
                } else {
                    self.select_next();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected == Some(0) && !self.command_history.commands().is_empty() {
                    // On command section - enter command mode and show history
                    self.enter_command_mode();
                    self.history_prev();
                } else if self.is_on_expanded_commit() {
                    // Navigate within expanded commit
                    match self.expanded_file_idx {
                        Some(0) => {
                            // Move from first file back to header
                            self.expanded_file_idx = None;
                            self.update_sections();
                        }
                        Some(idx) => {
                            // Move to previous file
                            self.expanded_file_idx = Some(idx - 1);
                            self.update_sections();
                        }
                        None => {
                            // On header - move to previous item
                            self.select_prev();
                        }
                    }
                } else {
                    self.select_prev();
                }
            }
            KeyCode::Char('G') => {
                self.select_last();
            }

            // Copy sha to clipboard (for history items) - 'y' copies short sha
            KeyCode::Char('y') => {
                if let Some(sha) = self.selected_activity_sha() {
                    if self.copy_to_clipboard(&sha) {
                        self.feedback.error = Some(format!("Copied: {sha}"));
                    } else {
                        self.feedback.error = Some("Failed to copy to clipboard".to_string());
                    }
                }
            }

            // Copy full sha to clipboard (for history or branch commit items)
            KeyCode::Char('c') => {
                // Works for both history commits and expanded branch commits
                if let Some(sha) = self.selected_activity_sha() {
                    // If expanded, copy full sha from detail
                    if let Some(detail) = &self.expanded_detail {
                        if self.expanded_commit.as_ref() == Some(&sha) {
                            if self.copy_to_clipboard(&detail.full_sha) {
                                self.feedback.error =
                                    Some(format!("Copied: {}", detail.full_sha));
                            } else {
                                self.feedback.error =
                                    Some("Failed to copy to clipboard".to_string());
                            }
                            return;
                        }
                    }
                    // Copy short sha
                    if self.copy_to_clipboard(&sha) {
                        self.feedback.error = Some(format!("Copied: {sha}"));
                    } else {
                        self.feedback.error = Some("Failed to copy to clipboard".to_string());
                    }
                }
            }

            // Interactive rebase onto selected commit (history only)
            KeyCode::Char('R') => {
                if self.is_in_history() {
                    self.execute_app_action(AppAction::InteractiveRebase);
                }
            }

            // Toggle expand/collapse for headers, or show commit details
            KeyCode::Char(' ') => {
                // First check if we're on a staged/working file - open in pager
                if let Some((path, is_staged, _)) = self.selected_file_info() {
                    self.pending_external = Some(ExternalCommand::FilePagerDiff {
                        file_path: path.to_string_lossy().to_string(),
                        staged: is_staged,
                    });
                } else if self.is_on_history_header() {
                    // Toggle history section expand/collapse
                    if self.history_collapsed {
                        self.expand_history();
                    } else {
                        self.collapse_history();
                    }
                } else if let Some(branch_name) = self.is_on_branch_header() {
                    // Toggle branch expand/collapse
                    if self.expanded_branch.as_deref() == Some(&branch_name) {
                        self.collapse_branch();
                        self.update_sections();
                    } else {
                        self.expand_branch(&branch_name);
                    }
                } else if let Some(sha) = self.selected_activity_sha() {
                    // Commit item handling (in history or expanded branch)
                    if self.expanded_commit.as_ref() == Some(&sha) {
                        if let Some(file_idx) = self.expanded_file_idx {
                            // Open external pager diff (uses gitconfig core.pager)
                            let file_path = self
                                .expanded_detail
                                .as_ref()
                                .and_then(|d| d.files.get(file_idx))
                                .map(|f| f.path.clone());

                            if let Some(path) = file_path {
                                self.pending_external = Some(ExternalCommand::PagerDiff {
                                    commit_sha: sha,
                                    file_path: path,
                                });
                            }
                        } else {
                            self.toggle_commit_detail(&sha);
                        }
                    } else {
                        self.toggle_commit_detail(&sha);
                    }
                }
            }

            // Open output popup (when on command section with output)
            KeyCode::Char('o') => {
                if self.selected == Some(0) && self.feedback.has_output() {
                    self.open_output_popup();
                }
            }

            // Expand error details to popup (when error is showing)
            KeyCode::Char('e') => {
                self.feedback.expand_error();
            }

            _ => {}
        }
    }

    /// Handle keyboard input when help overlay is shown
    #[allow(clippy::missing_const_for_fn)] // Mutates self
    pub(in crate::app) fn handle_help_key(&mut self) {
        // Any key closes help
        self.show_help = false;
    }
}
