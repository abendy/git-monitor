//! Menu mode key handling.

use crossterm::event::KeyEvent;

use crate::app::App;
use crate::menu::{MenuAction, MenuResult};

impl App {
    /// Handle key events when menu stack is active
    pub(in crate::app) fn handle_menu_key(&mut self, key: KeyEvent) {
        // Get the result from the active menu
        let result = if let Some(menu) = self.menu_stack.current_mut() {
            menu.handle_key(key)
        } else {
            return;
        };

        // Process the result
        match result {
            MenuResult::Continue => {}
            MenuResult::Close | MenuResult::Pop => {
                self.menu_stack.pop();
            }
            MenuResult::Execute(action) => {
                self.menu_stack.clear();
                match action {
                    MenuAction::Command(request) => {
                        self.run_command(request);
                    }
                    MenuAction::App(app_action) => {
                        self.execute_app_action(app_action);
                    }
                    MenuAction::Custom(callback) => {
                        callback();
                    }
                }
            }
            MenuResult::Push(menu) => {
                self.menu_stack.push(menu);
            }
            MenuResult::CloseAll => {
                self.menu_stack.clear();
            }
        }
    }
}
