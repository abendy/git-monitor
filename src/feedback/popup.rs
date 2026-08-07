//! Popup state and content management.

/// Content displayed in the popup
#[derive(Debug, Clone, Default)]
pub enum PopupContent {
    /// No popup active
    #[default]
    None,
    /// Command output
    CommandOutput {
        /// The command that was run
        command: String,
        /// The output content
        output: String,
        /// Whether the command succeeded
        success: bool,
    },
    /// File diff
    Diff {
        /// Path to the file
        path: String,
        /// Diff content
        content: String,
        /// Whether this is a staged diff
        #[allow(dead_code)]
        is_staged: bool,
    },
    /// Error details with context
    ErrorDetails {
        /// Short error title
        title: String,
        /// Primary error message
        message: String,
        /// Additional context lines (stack trace, suggestions, etc.)
        context: Vec<String>,
    },
}

impl PopupContent {
    /// Check if popup is active
    #[must_use]
    pub const fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Get the number of lines
    #[must_use]
    pub fn line_count(&self) -> usize {
        match self {
            Self::None => 0,
            Self::CommandOutput { output, .. } => output.lines().count(),
            Self::Diff { content, .. } => content.lines().count(),
            // Title line + blank + message lines + blank + context lines
            Self::ErrorDetails {
                message, context, ..
            } => 1 + 1 + message.lines().count() + 1 + context.len(),
        }
    }

    /// Get the title for the popup
    #[must_use]
    pub fn title(&self) -> String {
        match self {
            Self::None => String::new(),
            Self::CommandOutput { command, .. } => format!(" Output: {command} "),
            Self::Diff { path, .. } => format!(" Diff: {path} "),
            Self::ErrorDetails { title, .. } => format!(" Error: {title} "),
        }
    }

    /// Get the content for iteration
    #[must_use]
    #[allow(dead_code)] // API for future use
    pub fn content(&self) -> &str {
        match self {
            Self::None => "",
            Self::CommandOutput { output, .. } => output,
            Self::Diff { content, .. } => content,
            Self::ErrorDetails { message, .. } => message,
        }
    }

    /// Check if this is a command output popup
    #[must_use]
    #[allow(dead_code)] // API for future use
    pub const fn is_command_output(&self) -> bool {
        matches!(self, Self::CommandOutput { .. })
    }

    /// Get success status for command output
    #[must_use]
    #[allow(dead_code)] // API for future use
    pub const fn is_success(&self) -> bool {
        match self {
            Self::CommandOutput { success, .. } => *success,
            _ => true,
        }
    }
}

/// Popup state with scroll position
#[derive(Debug, Clone, Default)]
pub struct PopupState {
    /// Content being displayed
    pub content: PopupContent,
    /// Scroll offset (line number at top of view)
    pub scroll_offset: usize,
    /// Visible height from last render (for scroll calculations)
    pub visible_height: usize,
}

impl PopupState {
    /// Open popup with content
    pub fn open(&mut self, content: PopupContent) {
        self.content = content;
        self.scroll_offset = 0;
    }

    /// Close popup
    pub fn close(&mut self) {
        self.content = PopupContent::None;
        self.scroll_offset = 0;
    }

    /// Check if popup is open
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.content.is_active()
    }

    /// Get the content
    #[must_use]
    #[allow(dead_code)] // API for future use
    pub const fn content(&self) -> &PopupContent {
        &self.content
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        let max_offset = self
            .content
            .line_count()
            .saturating_sub(self.visible_height);
        self.scroll_offset = (self.scroll_offset + n).min(max_offset);
    }

    /// Scroll up by n lines
    pub const fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Jump to top
    pub const fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Jump to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self
            .content
            .line_count()
            .saturating_sub(self.visible_height);
    }

    /// Page down (scroll by visible height)
    pub fn page_down(&mut self) {
        self.scroll_down(self.visible_height.saturating_sub(2));
    }

    /// Page up (scroll by visible height)
    pub const fn page_up(&mut self) {
        self.scroll_up(self.visible_height.saturating_sub(2));
    }

    /// Set visible height (called by renderer)
    #[allow(dead_code)] // API for future use
    pub const fn set_visible_height(&mut self, height: usize) {
        self.visible_height = height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod popup_content {
        use super::*;

        #[test]
        fn none_is_not_active() {
            let content = PopupContent::None;

            assert!(!content.is_active());
        }

        #[test]
        fn command_output_is_active() {
            let content = PopupContent::CommandOutput {
                command: "git status".to_string(),
                output: "On branch main".to_string(),
                success: true,
            };

            assert!(content.is_active());
        }

        #[test]
        fn diff_is_active() {
            let content = PopupContent::Diff {
                path: "file.txt".to_string(),
                content: "+new line".to_string(),
                is_staged: false,
            };

            assert!(content.is_active());
        }

        #[test]
        fn none_line_count_is_zero() {
            let content = PopupContent::None;

            assert_eq!(content.line_count(), 0);
        }

        #[test]
        fn command_output_line_count() {
            let content = PopupContent::CommandOutput {
                command: "test".to_string(),
                output: "line1\nline2\nline3".to_string(),
                success: true,
            };

            assert_eq!(content.line_count(), 3);
        }

        #[test]
        fn diff_line_count() {
            let content = PopupContent::Diff {
                path: "file.txt".to_string(),
                content: "+a\n-b".to_string(),
                is_staged: false,
            };

            assert_eq!(content.line_count(), 2);
        }

        #[test]
        fn command_output_title() {
            let content = PopupContent::CommandOutput {
                command: "git status".to_string(),
                output: String::new(),
                success: true,
            };

            assert_eq!(content.title(), " Output: git status ");
        }

        #[test]
        fn diff_title() {
            let content = PopupContent::Diff {
                path: "src/main.rs".to_string(),
                content: String::new(),
                is_staged: false,
            };

            assert_eq!(content.title(), " Diff: src/main.rs ");
        }

        #[test]
        fn error_details_title() {
            let content = PopupContent::ErrorDetails {
                title: "Failed".to_string(),
                message: "Something went wrong".to_string(),
                context: vec![],
            };

            assert_eq!(content.title(), " Error: Failed ");
        }

        #[test]
        fn is_success_true_for_successful_command() {
            let content = PopupContent::CommandOutput {
                command: "test".to_string(),
                output: String::new(),
                success: true,
            };

            assert!(content.is_success());
        }

        #[test]
        fn is_success_false_for_failed_command() {
            let content = PopupContent::CommandOutput {
                command: "test".to_string(),
                output: String::new(),
                success: false,
            };

            assert!(!content.is_success());
        }

        #[test]
        fn is_success_true_for_non_command() {
            let content = PopupContent::Diff {
                path: String::new(),
                content: String::new(),
                is_staged: false,
            };

            assert!(content.is_success());
        }
    }

    mod popup_state {
        use super::*;

        #[test]
        fn default_is_closed() {
            let state = PopupState::default();

            assert!(!state.is_open());
        }

        #[test]
        fn open_sets_content() {
            let mut state = PopupState::default();

            state.open(PopupContent::CommandOutput {
                command: "test".to_string(),
                output: "output".to_string(),
                success: true,
            });

            assert!(state.is_open());
        }

        #[test]
        fn open_resets_scroll() {
            let mut state = PopupState {
                scroll_offset: 10,
                ..PopupState::default()
            };

            state.open(PopupContent::CommandOutput {
                command: "test".to_string(),
                output: "output".to_string(),
                success: true,
            });

            assert_eq!(state.scroll_offset, 0);
        }

        #[test]
        fn close_clears_content() {
            let mut state = PopupState::default();
            state.open(PopupContent::CommandOutput {
                command: "test".to_string(),
                output: "output".to_string(),
                success: true,
            });

            state.close();

            assert!(!state.is_open());
        }

        #[test]
        fn scroll_down_increases_offset() {
            let mut state = PopupState::default();
            state.open(PopupContent::CommandOutput {
                command: "test".to_string(),
                output: "1\n2\n3\n4\n5\n6\n7\n8\n9\n10".to_string(),
                success: true,
            });
            state.visible_height = 5;

            state.scroll_down(2);

            assert_eq!(state.scroll_offset, 2);
        }

        #[test]
        fn scroll_down_respects_max() {
            let mut state = PopupState::default();
            state.open(PopupContent::CommandOutput {
                command: "test".to_string(),
                output: "1\n2\n3".to_string(), // 3 lines
                success: true,
            });
            state.visible_height = 2;

            state.scroll_down(100);

            // Max offset = 3 - 2 = 1
            assert_eq!(state.scroll_offset, 1);
        }

        #[test]
        fn scroll_up_decreases_offset() {
            let mut state = PopupState {
                scroll_offset: 5,
                ..PopupState::default()
            };

            state.scroll_up(2);

            assert_eq!(state.scroll_offset, 3);
        }

        #[test]
        fn scroll_up_stops_at_zero() {
            let mut state = PopupState {
                scroll_offset: 2,
                ..PopupState::default()
            };

            state.scroll_up(100);

            assert_eq!(state.scroll_offset, 0);
        }

        #[test]
        fn scroll_to_top_resets_offset() {
            let mut state = PopupState {
                scroll_offset: 10,
                ..PopupState::default()
            };

            state.scroll_to_top();

            assert_eq!(state.scroll_offset, 0);
        }

        #[test]
        fn scroll_to_bottom_sets_max_offset() {
            let mut state = PopupState::default();
            state.open(PopupContent::CommandOutput {
                command: "test".to_string(),
                output: "1\n2\n3\n4\n5".to_string(), // 5 lines
                success: true,
            });
            state.visible_height = 3;

            state.scroll_to_bottom();

            // Max offset = 5 - 3 = 2
            assert_eq!(state.scroll_offset, 2);
        }
    }
}
