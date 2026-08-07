//! Command mode key handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, ViewMode};

impl App {
    /// Handle keyboard input in command mode
    pub(in crate::app) fn handle_command_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            // Cancel command mode
            KeyCode::Esc => {
                self.exit_command_mode();
            }
            KeyCode::Char('c')
                if key
                    .modifiers
                    .contains(KeyModifiers::CONTROL) =>
            {
                self.exit_command_mode();
            }

            // Execute command
            KeyCode::Enter => {
                self.view_mode = ViewMode::Normal;
                self.execute_command();
                self.command_draft = None;
                self.update_sections();
            }

            // Type characters
            KeyCode::Char(c) => {
                self.command_input.push(c);
                self.command_draft = None;
                self.command_history.reset_navigation();
                self.update_sections();
            }

            // Backspace
            KeyCode::Backspace => {
                self.command_input.pop();
                self.command_draft = None;
                self.command_history.reset_navigation();
                self.update_sections();
            }

            // History navigation / exit
            KeyCode::Up => {
                self.history_prev();
            }
            KeyCode::Down => {
                // When browsing history, Down moves toward newer entries and eventually
                // restores the in-progress command. Once we're back at the draft (not
                // navigating), Down should release focus back to the main list.
                if self.command_history.is_navigating() || self.command_draft.is_some() {
                    self.history_next();
                } else {
                    self.exit_command_mode();
                    self.select_next();
                }
            }

            _ => {}
        }
    }
}
