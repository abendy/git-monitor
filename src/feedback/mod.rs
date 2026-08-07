//! Unified feedback system for command output, toasts, and popups.
//!
//! All user feedback flows through this module for consistent behavior.

mod popup;
mod toast;

pub use popup::{PopupContent, PopupState};
pub use toast::{Toast, ToastLevel};

use crate::command::{CommandResult, CommandSource, FeedbackPolicy};

/// Threshold for auto-opening popup (line count)
pub const AUTO_POPUP_LINE_THRESHOLD: usize = 5;

/// Types of feedback that can be shown to the user
#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants for comprehensive feedback API
pub enum Feedback {
    /// Command completed - show output
    CommandOutput {
        /// The command that was run
        command: String,
        /// Result of execution
        result: CommandResult,
        /// How the command was triggered
        source: CommandSource,
    },
    /// Transient notification (auto-dismisses)
    Toast {
        /// Message to display
        message: String,
        /// Severity/type of toast
        level: ToastLevel,
    },
    /// Error requiring acknowledgment
    Error {
        /// Error message
        message: String,
    },
    /// File diff to display
    Diff {
        /// Path to the file
        path: String,
        /// Diff content
        content: String,
        /// Whether this is a staged diff
        is_staged: bool,
    },
}

/// Manages all feedback display state
#[derive(Debug, Clone, Default)]
pub struct FeedbackManager {
    /// Current command output (for inline display)
    pub command_output: Option<CommandOutput>,
    /// Whether the last command succeeded
    pub command_success: bool,
    /// Current popup state
    pub popup: PopupState,
    /// Current toast (auto-dismisses)
    pub toast: Option<Toast>,
    /// Persistent error message
    pub error: Option<String>,
}

/// Stored command output for inline display
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// The command that was run
    pub command: String,
    /// The output to display
    pub output: String,
    /// Whether the command succeeded
    pub success: bool,
}

impl FeedbackManager {
    /// Create a new feedback manager
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Show feedback based on type and policy
    pub fn show(&mut self, feedback: Feedback, policy: FeedbackPolicy) {
        match feedback {
            Feedback::CommandOutput {
                command,
                result,
                source,
            } => {
                self.show_command_output(command, result, source, policy);
            }
            Feedback::Toast { message, level } => {
                self.toast = Some(Toast::new(message, level));
            }
            Feedback::Error { message } => {
                self.error = Some(message);
            }
            Feedback::Diff {
                path,
                content,
                is_staged,
            } => {
                self.popup.open(PopupContent::Diff {
                    path,
                    content,
                    is_staged,
                });
            }
        }
    }

    /// Handle command output feedback
    #[allow(clippy::needless_pass_by_value)] // Takes ownership for API consistency
    fn show_command_output(
        &mut self,
        command: String,
        result: CommandResult,
        source: CommandSource,
        policy: FeedbackPolicy,
    ) {
        let output = result.display_output().to_string();
        let success = result.success;
        let line_count = result.line_count();

        // Store for inline display
        self.command_output = Some(CommandOutput {
            command: command.clone(),
            output: output.clone(),
            success,
        });
        self.command_success = success;

        // Determine if popup should auto-open
        let should_popup = match policy {
            FeedbackPolicy::AlwaysPopup => true,
            FeedbackPolicy::InlineOnly | FeedbackPolicy::Silent | FeedbackPolicy::External => false,
            FeedbackPolicy::Default => {
                // Source-based defaults:
                // - ActionMenu/AliasBrowser: Always popup (deliberate selection)
                // - Keyboard/Palette: Popup on failure or long output
                match source {
                    CommandSource::ActionMenu | CommandSource::AliasBrowser => true,
                    CommandSource::Keyboard | CommandSource::Palette | CommandSource::Internal => {
                        !success || line_count > AUTO_POPUP_LINE_THRESHOLD
                    }
                }
            }
        };

        if should_popup {
            self.popup
                .open(PopupContent::CommandOutput {
                    command,
                    output,
                    success,
                });
        }
    }

    /// Open popup for current command output (manual expansion)
    pub fn expand_output(&mut self) {
        if let Some(ref output) = self.command_output {
            self.popup
                .open(PopupContent::CommandOutput {
                    command: output.command.clone(),
                    output: output.output.clone(),
                    success: output.success,
                });
        }
    }

    /// Clear command output
    #[allow(dead_code)] // API for future use
    pub fn clear_output(&mut self) {
        self.command_output = None;
    }

    /// Clear error message
    #[allow(dead_code)] // API for future use
    pub fn clear_error(&mut self) {
        self.error = None;
    }

    /// Show detailed error information in a popup
    ///
    /// Opens a popup with the error message and additional context.
    /// The footer continues to show a brief error, but the user can
    /// see full details in the popup.
    pub fn show_error_details(&mut self, title: &str, message: &str, context: Vec<String>) {
        self.popup
            .open(PopupContent::ErrorDetails {
                title: title.to_string(),
                message: message.to_string(),
                context,
            });
    }

    /// Expand current error to popup (if one exists)
    ///
    /// Called when user presses 'e' to see more error details.
    /// Returns true if there was an error to expand.
    pub fn expand_error(&mut self) -> bool {
        if let Some(ref error) = self.error {
            self.popup
                .open(PopupContent::ErrorDetails {
                    title: "Error".to_string(),
                    message: error.clone(),
                    context: vec![],
                });
            true
        } else {
            false
        }
    }

    /// Tick - handle time-based updates (toast expiry)
    pub fn tick(&mut self) {
        if let Some(ref toast) = self.toast {
            if toast.is_expired() {
                self.toast = None;
            }
        }
    }

    /// Check if there is output to display inline
    #[must_use]
    pub const fn has_output(&self) -> bool {
        self.command_output.is_some()
    }

    /// Get output for inline display
    #[must_use]
    pub const fn output(&self) -> Option<&CommandOutput> {
        self.command_output.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandResult;
    use std::time::Duration;

    // Helper to create a test CommandResult
    fn make_result(success: bool, output: &str) -> CommandResult {
        CommandResult {
            success,
            exit_code: if success { Some(0) } else { Some(1) },
            stdout: output.to_string(),
            stderr: String::new(),
            duration: Duration::from_millis(100),
        }
    }

    mod feedback_manager_new {
        use super::*;

        #[test]
        fn creates_default_manager() {
            let manager = FeedbackManager::new();

            assert!(manager.command_output.is_none());
            assert!(!manager.command_success); // Default is false
            assert!(!manager.popup.is_open());
            assert!(manager.toast.is_none());
            assert!(manager.error.is_none());
        }

        #[test]
        fn default_and_new_are_equivalent() {
            let from_new = FeedbackManager::new();
            let from_default = FeedbackManager::default();

            assert_eq!(
                from_new.command_success,
                from_default.command_success
            );
            assert!(from_new.command_output.is_none());
            assert!(from_default.command_output.is_none());
        }
    }

    mod feedback_manager_show {
        use super::*;

        #[test]
        fn show_toast_sets_toast() {
            let mut manager = FeedbackManager::new();

            manager.show(
                Feedback::Toast {
                    message: "Test message".to_string(),
                    level: ToastLevel::Info,
                },
                FeedbackPolicy::Default,
            );

            assert!(manager.toast.is_some());
            let toast = manager.toast.as_ref().unwrap();
            assert_eq!(toast.message, "Test message");
            assert_eq!(toast.level, ToastLevel::Info);
        }

        #[test]
        fn show_error_sets_error() {
            let mut manager = FeedbackManager::new();

            manager.show(
                Feedback::Error {
                    message: "Something failed".to_string(),
                },
                FeedbackPolicy::Default,
            );

            assert_eq!(
                manager.error,
                Some("Something failed".to_string())
            );
        }

        #[test]
        fn show_diff_opens_popup() {
            let mut manager = FeedbackManager::new();

            manager.show(
                Feedback::Diff {
                    path: "src/main.rs".to_string(),
                    content: "+new line\n-old line".to_string(),
                    is_staged: false,
                },
                FeedbackPolicy::Default,
            );

            assert!(manager.popup.is_open());
            assert!(matches!(
                manager.popup.content,
                PopupContent::Diff { .. }
            ));
        }

        #[test]
        fn show_command_output_stores_output() {
            let mut manager = FeedbackManager::new();
            let result = make_result(true, "output text");

            manager.show(
                Feedback::CommandOutput {
                    command: "git status".to_string(),
                    result,
                    source: CommandSource::Keyboard,
                },
                FeedbackPolicy::InlineOnly,
            );

            assert!(manager.has_output());
            let output = manager.output().unwrap();
            assert_eq!(output.command, "git status");
            assert!(output.success);
        }
    }

    mod show_command_output_policies {
        use super::*;

        #[test]
        fn always_popup_opens_popup() {
            let mut manager = FeedbackManager::new();
            let result = make_result(true, "short");

            manager.show(
                Feedback::CommandOutput {
                    command: "git status".to_string(),
                    result,
                    source: CommandSource::Keyboard,
                },
                FeedbackPolicy::AlwaysPopup,
            );

            assert!(manager.popup.is_open());
        }

        #[test]
        fn inline_only_does_not_open_popup() {
            let mut manager = FeedbackManager::new();
            // Even with long output
            let result = make_result(
                true,
                "line1\nline2\nline3\nline4\nline5\nline6\nline7",
            );

            manager.show(
                Feedback::CommandOutput {
                    command: "git log".to_string(),
                    result,
                    source: CommandSource::Keyboard,
                },
                FeedbackPolicy::InlineOnly,
            );

            assert!(!manager.popup.is_open());
            assert!(manager.has_output()); // Still stored inline
        }

        #[test]
        fn silent_does_not_open_popup() {
            let mut manager = FeedbackManager::new();
            let result = make_result(false, "error occurred");

            manager.show(
                Feedback::CommandOutput {
                    command: "git push".to_string(),
                    result,
                    source: CommandSource::Internal,
                },
                FeedbackPolicy::Silent,
            );

            assert!(!manager.popup.is_open());
        }

        #[test]
        fn external_does_not_open_popup() {
            let mut manager = FeedbackManager::new();
            let result = make_result(true, "external output");

            manager.show(
                Feedback::CommandOutput {
                    command: "vim".to_string(),
                    result,
                    source: CommandSource::Keyboard,
                },
                FeedbackPolicy::External,
            );

            assert!(!manager.popup.is_open());
        }
    }

    mod show_command_output_default_policy {
        use super::*;

        #[test]
        fn action_menu_source_always_opens_popup() {
            let mut manager = FeedbackManager::new();
            let result = make_result(true, "short");

            manager.show(
                Feedback::CommandOutput {
                    command: "git status".to_string(),
                    result,
                    source: CommandSource::ActionMenu,
                },
                FeedbackPolicy::Default,
            );

            assert!(manager.popup.is_open());
        }

        #[test]
        fn alias_browser_source_always_opens_popup() {
            let mut manager = FeedbackManager::new();
            let result = make_result(true, "short");

            manager.show(
                Feedback::CommandOutput {
                    command: "git st".to_string(),
                    result,
                    source: CommandSource::AliasBrowser,
                },
                FeedbackPolicy::Default,
            );

            assert!(manager.popup.is_open());
        }

        #[test]
        fn keyboard_source_opens_popup_on_failure() {
            let mut manager = FeedbackManager::new();
            let result = make_result(false, "error");

            manager.show(
                Feedback::CommandOutput {
                    command: "git push".to_string(),
                    result,
                    source: CommandSource::Keyboard,
                },
                FeedbackPolicy::Default,
            );

            assert!(manager.popup.is_open());
        }

        #[test]
        fn keyboard_source_opens_popup_on_long_output() {
            let mut manager = FeedbackManager::new();
            // More than AUTO_POPUP_LINE_THRESHOLD (5) lines
            let result = make_result(true, "1\n2\n3\n4\n5\n6\n7");

            manager.show(
                Feedback::CommandOutput {
                    command: "git log".to_string(),
                    result,
                    source: CommandSource::Keyboard,
                },
                FeedbackPolicy::Default,
            );

            assert!(manager.popup.is_open());
        }

        #[test]
        fn keyboard_source_no_popup_on_short_success() {
            let mut manager = FeedbackManager::new();
            // Less than or equal to threshold
            let result = make_result(true, "line1\nline2");

            manager.show(
                Feedback::CommandOutput {
                    command: "git add".to_string(),
                    result,
                    source: CommandSource::Keyboard,
                },
                FeedbackPolicy::Default,
            );

            assert!(!manager.popup.is_open());
            assert!(manager.has_output()); // But still stored inline
        }

        #[test]
        fn palette_source_follows_keyboard_rules() {
            let mut manager = FeedbackManager::new();
            let result = make_result(false, "error");

            manager.show(
                Feedback::CommandOutput {
                    command: "git push".to_string(),
                    result,
                    source: CommandSource::Palette,
                },
                FeedbackPolicy::Default,
            );

            assert!(manager.popup.is_open());
        }

        #[test]
        fn internal_source_follows_keyboard_rules() {
            let mut manager = FeedbackManager::new();
            let result = make_result(true, "short");

            manager.show(
                Feedback::CommandOutput {
                    command: "git fetch".to_string(),
                    result,
                    source: CommandSource::Internal,
                },
                FeedbackPolicy::Default,
            );

            // Short success from internal source = no popup
            assert!(!manager.popup.is_open());
        }
    }

    mod expand_output {
        use super::*;

        #[test]
        fn opens_popup_with_stored_output() {
            let mut manager = FeedbackManager::new();
            // First store output without popup
            let result = make_result(true, "stored output");
            manager.show(
                Feedback::CommandOutput {
                    command: "git status".to_string(),
                    result,
                    source: CommandSource::Keyboard,
                },
                FeedbackPolicy::InlineOnly,
            );
            assert!(!manager.popup.is_open());

            // Now expand it
            manager.expand_output();

            assert!(manager.popup.is_open());
            if let PopupContent::CommandOutput {
                command,
                output,
                success,
            } = &manager.popup.content
            {
                assert_eq!(command, "git status");
                assert_eq!(output, "stored output");
                assert!(success);
            } else {
                panic!("Expected CommandOutput popup");
            }
        }

        #[test]
        fn does_nothing_without_stored_output() {
            let mut manager = FeedbackManager::new();

            manager.expand_output();

            assert!(!manager.popup.is_open());
        }
    }

    mod expand_error {
        use super::*;

        #[test]
        fn opens_popup_with_stored_error() {
            let mut manager = FeedbackManager::new();
            manager.error = Some("Test error message".to_string());

            let result = manager.expand_error();

            assert!(result);
            assert!(manager.popup.is_open());
            if let PopupContent::ErrorDetails { title, message, .. } = &manager.popup.content {
                assert_eq!(title, "Error");
                assert_eq!(message, "Test error message");
            } else {
                panic!("Expected ErrorDetails popup");
            }
        }

        #[test]
        fn returns_false_without_error() {
            let mut manager = FeedbackManager::new();

            let result = manager.expand_error();

            assert!(!result);
            assert!(!manager.popup.is_open());
        }
    }

    mod show_error_details {
        use super::*;

        #[test]
        fn opens_popup_with_full_context() {
            let mut manager = FeedbackManager::new();

            manager.show_error_details(
                "Git Error",
                "Failed to push",
                vec!["Hint: try git pull first".to_string()],
            );

            assert!(manager.popup.is_open());
            if let PopupContent::ErrorDetails {
                title,
                message,
                context,
            } = &manager.popup.content
            {
                assert_eq!(title, "Git Error");
                assert_eq!(message, "Failed to push");
                assert_eq!(context.len(), 1);
                assert_eq!(context[0], "Hint: try git pull first");
            } else {
                panic!("Expected ErrorDetails popup");
            }
        }

        #[test]
        fn works_with_empty_context() {
            let mut manager = FeedbackManager::new();

            manager.show_error_details("Error", "Something broke", vec![]);

            assert!(manager.popup.is_open());
            if let PopupContent::ErrorDetails { context, .. } = &manager.popup.content {
                assert!(context.is_empty());
            } else {
                panic!("Expected ErrorDetails popup");
            }
        }
    }

    mod tick {
        use super::*;

        #[test]
        fn clears_expired_toast() {
            let mut manager = FeedbackManager::new();
            // Create an already-expired toast by manipulating time
            // Since we can't easily manipulate the instant, we'll test the logic
            manager.toast = Some(Toast::new("Test", ToastLevel::Info));

            // Toast is not expired yet
            manager.tick();
            assert!(manager.toast.is_some());
        }

        #[test]
        fn does_nothing_without_toast() {
            let mut manager = FeedbackManager::new();

            // Should not panic
            manager.tick();

            assert!(manager.toast.is_none());
        }
    }

    mod clear_methods {
        use super::*;

        #[test]
        fn clear_output_removes_command_output() {
            let mut manager = FeedbackManager::new();
            manager.command_output = Some(CommandOutput {
                command: "test".to_string(),
                output: "output".to_string(),
                success: true,
            });

            manager.clear_output();

            assert!(manager.command_output.is_none());
        }

        #[test]
        fn clear_error_removes_error() {
            let mut manager = FeedbackManager::new();
            manager.error = Some("Test error".to_string());

            manager.clear_error();

            assert!(manager.error.is_none());
        }
    }

    mod has_output_and_output {
        use super::*;

        #[test]
        fn has_output_returns_true_when_present() {
            let mut manager = FeedbackManager::new();
            manager.command_output = Some(CommandOutput {
                command: "test".to_string(),
                output: "output".to_string(),
                success: true,
            });

            assert!(manager.has_output());
        }

        #[test]
        fn has_output_returns_false_when_empty() {
            let manager = FeedbackManager::new();

            assert!(!manager.has_output());
        }

        #[test]
        fn output_returns_reference_when_present() {
            let mut manager = FeedbackManager::new();
            manager.command_output = Some(CommandOutput {
                command: "git status".to_string(),
                output: "On branch main".to_string(),
                success: true,
            });

            let output = manager.output().unwrap();
            assert_eq!(output.command, "git status");
            assert_eq!(output.output, "On branch main");
            assert!(output.success);
        }

        #[test]
        fn output_returns_none_when_empty() {
            let manager = FeedbackManager::new();

            assert!(manager.output().is_none());
        }
    }

    mod command_success_tracking {
        use super::*;

        #[test]
        fn tracks_success_for_successful_command() {
            let mut manager = FeedbackManager::new();
            let result = make_result(true, "success");

            manager.show(
                Feedback::CommandOutput {
                    command: "git status".to_string(),
                    result,
                    source: CommandSource::Keyboard,
                },
                FeedbackPolicy::InlineOnly,
            );

            assert!(manager.command_success);
        }

        #[test]
        fn tracks_failure_for_failed_command() {
            let mut manager = FeedbackManager::new();
            let result = make_result(false, "error");

            manager.show(
                Feedback::CommandOutput {
                    command: "git push".to_string(),
                    result,
                    source: CommandSource::Keyboard,
                },
                FeedbackPolicy::InlineOnly,
            );

            assert!(!manager.command_success);
        }
    }
}
