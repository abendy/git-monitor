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
