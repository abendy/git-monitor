//! Popup mode key handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;

impl App {
    /// Handle keyboard input in popup mode
    pub(in crate::app) fn handle_popup_key(&mut self, key: KeyEvent) {
        match key.code {
            // Close popup
            KeyCode::Esc | KeyCode::Char('q') => {
                self.feedback.popup.close();
            }

            // Scroll down
            KeyCode::Char('j') | KeyCode::Down => {
                self.feedback.popup.scroll_down(1);
            }

            // Scroll up
            KeyCode::Char('k') | KeyCode::Up => {
                self.feedback.popup.scroll_up(1);
            }

            // Page down
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.feedback.popup.page_down();
            }

            // Page up
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.feedback.popup.page_up();
            }

            // Jump to top
            KeyCode::Char('g') => {
                self.feedback.popup.scroll_to_top();
            }

            // Jump to bottom
            KeyCode::Char('G') => {
                self.feedback.popup.scroll_to_bottom();
            }

            _ => {}
        }
    }
}
