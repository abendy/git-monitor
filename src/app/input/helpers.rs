//! Input mode helper methods.

use crate::actions::Action;
use crate::app::{App, ViewMode};
use crate::menu::{ActionMenu, AliasSectionMenu};

impl App {
    // ─────────────────────────────────────────────────────────────────────────
    // ViewMode helper methods
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if in command mode
    pub const fn is_command_mode(&self) -> bool {
        matches!(self.view_mode, ViewMode::Command)
    }

    /// Enter command mode
    pub fn enter_command_mode(&mut self) {
        self.view_mode = ViewMode::Command;
        self.command_input.clear();
        self.command_draft = None;
        self.command_history.reset_navigation();
        self.update_sections();
    }

    /// Exit command mode (return to normal)
    pub fn exit_command_mode(&mut self) {
        self.view_mode = ViewMode::Normal;
        self.command_input.clear();
        self.command_draft = None;
        self.command_history.reset_navigation();
        self.update_sections();
    }

    /// Enter alias browser using the menu stack
    pub fn enter_alias_browser(&mut self) {
        if self.config.sections.is_empty() {
            return;
        }

        let menu = AliasSectionMenu::new(
            self.config.sections.clone(),
            self.repo_path.clone(),
        );
        self.menu_stack.push(Box::new(menu));
    }

    /// Open action menu for current context using the menu stack
    pub fn open_action_menu(&mut self) {
        let context = self.current_context();
        let state = self.app_state();
        let actions: Vec<Action> = self
            .action_registry
            .actions_for_context(context, &state)
            .into_iter()
            .cloned()
            .collect();

        if actions.is_empty() {
            return;
        }

        let menu = ActionMenu::new(context, actions, self.repo_path.clone());
        self.menu_stack.push(Box::new(menu));
    }
}
