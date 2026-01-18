//! Key event handling for different input modes.
//!
//! Dispatches keyboard input based on current application state:
//! - Menu mode: modal dialogs capture all input
//! - Command mode: text input for git commands
//! - Popup mode: scrollable overlays (diff, output)
//! - Normal mode: standard navigation and actions

mod command;
mod helpers;
mod menu;
mod normal;
mod popup;

use crossterm::event::KeyEvent;

use super::{App, ViewMode};

impl App {
    /// Handle keyboard input - main entry point
    pub(super) fn handle_key(&mut self, key: KeyEvent) {
        // Menu stack takes priority when active
        if self.menu_stack.is_active() {
            self.handle_menu_key(key);
            return;
        }

        // ViewMode-based dispatch
        match &self.view_mode {
            ViewMode::Command => {
                self.handle_command_mode_key(key);
                return;
            }
            ViewMode::Normal => {}
        }

        // Popup mode captures keys (full-screen overlays)
        if self.feedback.popup.is_open() {
            self.handle_popup_key(key);
            return;
        }

        // Help overlay captures keys
        if self.show_help {
            self.handle_help_key();
            return;
        }

        // Normal mode handling
        self.handle_normal_key(key);
    }
}
