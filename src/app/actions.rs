use crate::actions::AppAction;
use crate::command::ExternalCommand;

use super::App;

impl App {
    /// Execute a built-in app action
    pub(super) fn execute_app_action(&mut self, action: AppAction) {
        match action {
            // File actions
            AppAction::ToggleStage => self.toggle_stage(),
            AppAction::ShowDiff => self.show_diff(),
            AppAction::FilePagerDiff => {
                if let Some((path, is_staged, _)) = self.selected_file_info() {
                    self.pending_external = Some(ExternalCommand::FilePagerDiff {
                        file_path: path.to_string_lossy().to_string(),
                        staged: is_staged,
                    });
                }
            }
            AppAction::FileDiffTool => {
                if let Some((path, is_staged, _)) = self.selected_file_info() {
                    self.pending_external = Some(ExternalCommand::FileDiffTool {
                        file_path: path.to_string_lossy().to_string(),
                        staged: is_staged,
                    });
                }
            }
            AppAction::OpenEditor => self.open_editor(),

            // History actions
            AppAction::ToggleHistoryMode => self.toggle_history_mode(),
            AppAction::ExpandCommit => {
                if let Some(sha) = self.selected_activity_sha() {
                    self.toggle_commit_detail(&sha);
                }
            }
            AppAction::CopyShortSha => {
                if let Some(sha) = self.selected_activity_sha() {
                    if self.copy_to_clipboard(&sha) {
                        self.feedback.error = Some(format!("Copied: {sha}"));
                    }
                }
            }
            AppAction::CopyFullSha => {
                if let Some(sha) = self.selected_activity_sha() {
                    if let Some(detail) = &self.expanded_detail {
                        if self.expanded_commit.as_ref() == Some(&sha) {
                            if self.copy_to_clipboard(&detail.full_sha) {
                                self.feedback.error = Some(format!("Copied: {}", detail.full_sha));
                            }
                            return;
                        }
                    }
                    if self.copy_to_clipboard(&sha) {
                        self.feedback.error = Some(format!("Copied: {sha}"));
                    }
                }
            }
            AppAction::NextPage => self.next_history_page(),
            AppAction::PrevPage => self.prev_history_page(),
            AppAction::InteractiveRebase => {
                // Start interactive rebase onto the selected commit
                if let Some(sha) = self.selected_activity_sha() {
                    self.pending_external = Some(ExternalCommand::InteractiveRebase { onto: sha });
                }
            }

            // Commit file actions
            AppAction::PagerDiff => {
                if let (Some(sha), Some(file_idx)) = (
                    self.expanded_commit.clone(),
                    self.expanded_file_idx,
                ) {
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
                }
            }
            AppAction::InlineDiff => {
                if let (Some(sha), Some(file_idx)) = (
                    self.expanded_commit.clone(),
                    self.expanded_file_idx,
                ) {
                    let file_path = self
                        .expanded_detail
                        .as_ref()
                        .and_then(|d| d.files.get(file_idx))
                        .map(|f| f.path.clone());

                    if let Some(path) = file_path {
                        self.open_commit_file_diff(&sha, &path);
                    }
                }
            }
            AppAction::DiffTool => {
                if let (Some(sha), Some(file_idx)) = (
                    self.expanded_commit.clone(),
                    self.expanded_file_idx,
                ) {
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

            // Branch actions
            AppAction::Checkout => self.checkout_selected_branch(),
            AppAction::ExpandBranch => {
                if let Some(branch_name) = self.is_on_branch_header() {
                    if self.expanded_branch.as_deref() == Some(&branch_name) {
                        self.collapse_branch();
                        self.update_sections();
                    } else {
                        self.expand_branch(&branch_name);
                    }
                }
            }

            // Global actions
            AppAction::Push => self.show_push_confirm(false),
            AppAction::Pull => self.execute_pull(),
            AppAction::Fetch => self.execute_fetch(),
            AppAction::Refresh => self.refresh_status(),
            AppAction::EnterCommandMode => self.enter_command_mode(),
            AppAction::BrowseAliases => self.enter_alias_browser(),
            AppAction::ShowHelp => self.show_help = true,
            AppAction::Quit => self.running = false,

            // Navigation
            AppAction::JumpToWorking => self.jump_to_working(),
            AppAction::JumpToHistory => self.jump_to_history(),
            AppAction::JumpToBranches => self.jump_to_branches(),
        }
    }
}
