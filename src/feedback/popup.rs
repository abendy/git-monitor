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
}

impl PopupContent {
    /// Check if popup is active
    #[must_use]
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Get the number of lines
    #[must_use]
    pub fn line_count(&self) -> usize {
        match self {
            Self::None => 0,
            Self::CommandOutput { output, .. } => output.lines().count(),
            Self::Diff { content, .. } => content.lines().count(),
        }
    }

    /// Get the title for the popup
    #[must_use]
    pub fn title(&self) -> String {
        match self {
            Self::None => String::new(),
            Self::CommandOutput { command, .. } => format!(" Output: {command} "),
            Self::Diff { path, .. } => format!(" Diff: {path} "),
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
        }
    }

    /// Check if this is a command output popup
    #[must_use]
    #[allow(dead_code)] // API for future use
    pub fn is_command_output(&self) -> bool {
        matches!(self, Self::CommandOutput { .. })
    }

    /// Get success status for command output
    #[must_use]
    #[allow(dead_code)] // API for future use
    pub fn is_success(&self) -> bool {
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
    pub fn is_open(&self) -> bool {
        self.content.is_active()
    }

    /// Get the content
    #[must_use]
    #[allow(dead_code)] // API for future use
    pub fn content(&self) -> &PopupContent {
        &self.content
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        let max_offset = self.content.line_count().saturating_sub(self.visible_height);
        self.scroll_offset = (self.scroll_offset + n).min(max_offset);
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Jump to top
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Jump to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.content.line_count().saturating_sub(self.visible_height);
    }

    /// Page down (scroll by visible height)
    pub fn page_down(&mut self) {
        self.scroll_down(self.visible_height.saturating_sub(2));
    }

    /// Page up (scroll by visible height)
    pub fn page_up(&mut self) {
        self.scroll_up(self.visible_height.saturating_sub(2));
    }

    /// Set visible height (called by renderer)
    #[allow(dead_code)] // API for future use
    pub fn set_visible_height(&mut self, height: usize) {
        self.visible_height = height;
    }
}
