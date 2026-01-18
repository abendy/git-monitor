use std::path::PathBuf;

use super::App;
use crate::command::{CommandRequest, CommandSource, ExternalCommand};
use crate::feedback::{Feedback, PopupContent, Toast};
use crate::git::FileState;
use crate::menu::PushConfirmMenu;

impl App {
    pub(super) fn open_editor(&mut self) {
        let editor = std::env::var("EDITOR")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let Some(editor) = editor else {
            self.feedback.toast = Some(Toast::warning(
                "EDITOR not set. Example: export EDITOR=vim",
            ));
            return;
        };

        let path = self
            .editor_target_path()
            .unwrap_or_else(|| ".".to_string());

        self.pending_external = Some(ExternalCommand::OpenEditor { editor, path });
    }

    pub(super) fn editor_target_path(&self) -> Option<String> {
        if let (Some(detail), Some(file_idx)) = (
            self.expanded_detail.as_ref(),
            self.expanded_file_idx,
        ) {
            if let Some(file) = detail.files.get(file_idx) {
                return Some(file.path.clone());
            }
        }

        self.selected_file_info()
            .map(|(path, _is_staged, _state)| path.to_string_lossy().to_string())
    }

    /// Open the output popup with current command output
    pub(super) fn open_output_popup(&mut self) {
        self.feedback.expand_output();
    }

    /// Open a diff in the popup
    pub(super) fn open_diff_popup(&mut self, path: String, content: String, is_staged: bool) {
        self.feedback
            .popup
            .open(PopupContent::Diff {
                path,
                content,
                is_staged,
            });
    }

    /// Show push confirmation menu
    pub(super) fn show_push_confirm(&mut self, force: bool) {
        let branch = match &self.status.branch {
            Some(b) => b.clone(),
            None => {
                self.feedback.error = Some("No branch checked out".to_string());
                return;
            }
        };

        // Extract remote name from upstream (e.g., "origin/main" -> "origin")
        let (remote, has_upstream) = match &self.status.upstream {
            Some(upstream) => {
                let remote = upstream
                    .split('/')
                    .next()
                    .unwrap_or("origin")
                    .to_string();
                (remote, true)
            }
            None => ("origin".to_string(), false),
        };

        let menu = PushConfirmMenu::new(
            branch,
            remote,
            has_upstream,
            self.status.ahead,
            force,
        );
        self.menu_stack.push(Box::new(menu));
    }

    /// Execute git pull
    pub(super) fn execute_pull(&mut self) {
        let request = CommandRequest::git(["pull"]).with_source(CommandSource::Keyboard);
        self.run_command(request);
    }

    /// Execute git fetch
    pub(super) fn execute_fetch(&mut self) {
        let request = CommandRequest::git(["fetch"]).with_source(CommandSource::Keyboard);
        self.run_command(request);
    }

    /// Get the selected file info
    /// Returns (path, is_staged, file_state) or None if selection is on command or activity
    pub(super) fn selected_file_info(&self) -> Option<(PathBuf, bool, FileState)> {
        let selected = self.selected?;

        // Index 0 is command section
        if selected == 0 {
            return None;
        }

        let staged = self.status.staged_changes();
        let working = self.status.working_changes();
        let staged_len = staged.len();

        // Adjust for command section at index 0
        let file_idx = selected - 1;

        if file_idx < staged_len {
            // Selected is in staged
            staged
                .get(file_idx)
                .map(|f| (f.path.clone(), true, f.staged))
        } else if file_idx < staged_len + working.len() {
            // Selected is in working
            let working_idx = file_idx - staged_len;
            working
                .get(working_idx)
                .map(|f| (f.path.clone(), false, f.working))
        } else {
            // Selected is in activity or branches
            None
        }
    }

    /// Copy text to clipboard (cross-platform)
    pub(super) fn copy_to_clipboard(&self, text: &str) -> bool {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(text))
            .is_ok()
    }

    /// Toggle stage/unstage for selected file
    pub(super) fn toggle_stage(&mut self) {
        let Some((path, is_staged, _state)) = self.selected_file_info() else {
            return; // Can't stage/unstage activity items
        };

        let result = if is_staged {
            self.repo.unstage(&path)
        } else {
            self.repo.stage(&path)
        };

        if let Err(e) = result {
            self.feedback.error = Some(format!("Error: {e}"));
        } else {
            self.refresh_status();
        }
    }

    /// Show diff for selected file (using new popup system)
    pub(super) fn show_diff(&mut self) {
        let Some((path, is_staged, state)) = self.selected_file_info() else {
            return; // Can't show diff for activity items
        };

        let path_str = path.to_string_lossy().to_string();

        // For untracked files, show the file content instead of diff
        if state == FileState::Untracked {
            let full_path = self.repo_path.join(&path);
            match std::fs::read_to_string(&full_path) {
                Ok(content) => {
                    // Format as a "new file" diff-like view
                    let formatted = content
                        .lines()
                        .map(|line| format!("+{line}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let header = format!("(new file)\n\n{formatted}");
                    self.open_diff_popup(path_str, header, is_staged);
                }
                Err(e) => {
                    self.feedback.error = Some(format!("Error reading file: {e}"));
                }
            }
            return;
        }

        match self.repo.diff_file(&path, is_staged) {
            Ok(content) => {
                self.open_diff_popup(path_str, content, is_staged);
            }
            Err(e) => {
                self.feedback.error = Some(format!("Diff error: {e}"));
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Unified Command Execution
    // ─────────────────────────────────────────────────────────────────────────

    /// Execute a command using the unified framework
    ///
    /// This is the single entry point for all command execution. It handles:
    /// - Running the command via CommandExecutor
    /// - Recording in history
    /// - Displaying output (inline or popup based on source and result)
    /// - Refreshing git status if requested
    pub(super) fn run_command(&mut self, request: CommandRequest) {
        // Execute via the executor
        let result = self.executor.execute(&request);

        // Record in history (handles deduplication and persistence)
        self.command_history
            .add(request.display_name.clone());

        // Show feedback (handles inline output, popup logic, etc.)
        self.feedback.show(
            Feedback::CommandOutput {
                command: request.display_name.clone(),
                result,
                source: request.source,
            },
            request.feedback,
        );

        // Refresh git status if requested
        if request.refresh_after {
            self.refresh_status();
        }
    }

    /// Execute command from the command palette (: mode)
    pub(super) fn execute_command(&mut self) {
        let input = self.command_input.trim();
        if input.is_empty() {
            return;
        }

        // Parse command - for now still restrict to git commands for safety
        // TODO: Make this configurable for allowed command prefixes
        let Some(request) = CommandRequest::from_input(input) else {
            return;
        };

        if request.program != "git" {
            self.feedback.error = Some(String::from(
                "Only git commands are allowed",
            ));
            self.command_input.clear();
            self.command_draft = None;
            self.command_history.reset_navigation();
            self.update_sections();
            return;
        }

        // Execute using unified framework (source = Palette)
        self.run_command(request.with_source(CommandSource::Palette));

        // Clear input and reset history navigation
        self.command_input.clear();
        self.command_draft = None;
        self.command_history.reset_navigation();
        self.update_sections();
    }

    /// Navigate command history (older)
    pub(super) fn history_prev(&mut self) {
        // Capture whatever the user was typing before we start browsing history.
        if self.command_draft.is_none() {
            self.command_draft = Some(self.command_input.clone());
        }

        if let Some(cmd) = self.command_history.navigate_older() {
            self.command_input = cmd.to_string();
        }

        self.update_sections();
    }

    /// Navigate command history (newer)
    pub(super) fn history_next(&mut self) {
        match self.command_history.navigate_newer() {
            Some(cmd) => {
                self.command_input = cmd.to_string();
            }
            None => {
                // Past end of history: restore the user's draft (if any), otherwise clear.
                if let Some(draft) = self.command_draft.take() {
                    self.command_input = draft;
                } else {
                    self.command_input.clear();
                }
            }
        }

        self.update_sections();
    }
}
