//! Menu stack for nested menu navigation.

use super::Menu;

/// Manages a stack of menus for nested navigation
#[derive(Default)]
pub struct MenuStack {
    /// Stack of menus (last is active)
    stack: Vec<Box<dyn Menu>>,
}

impl MenuStack {
    /// Create an empty menu stack
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a menu onto the stack
    pub fn push(&mut self, menu: Box<dyn Menu>) {
        self.stack.push(menu);
    }

    /// Pop the top menu from the stack
    pub fn pop(&mut self) -> Option<Box<dyn Menu>> {
        self.stack.pop()
    }

    /// Get reference to current (top) menu
    #[must_use]
    pub fn current(&self) -> Option<&dyn Menu> {
        self.stack.last().map(AsRef::as_ref)
    }

    /// Get mutable reference to current (top) menu
    pub fn current_mut(&mut self) -> Option<&mut Box<dyn Menu>> {
        self.stack.last_mut()
    }

    /// Check if any menu is active
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.stack.is_empty()
    }

    /// Clear all menus from the stack
    pub fn clear(&mut self) {
        self.stack.clear();
    }

    /// Get the depth of the menu stack
    #[must_use]
    #[allow(dead_code)] // API for future use
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

impl std::fmt::Debug for MenuStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuStack")
            .field("depth", &self.stack.len())
            .field("is_active", &self.is_active())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::{MenuItem, MenuResult};
    use crossterm::event::{KeyCode, KeyEvent};
    use ratatui::layout::Rect;

    /// Simple test menu for unit testing
    struct TestMenu {
        title: String,
        selected: usize,
    }

    impl TestMenu {
        fn new(title: &str) -> Self {
            Self {
                title: title.to_string(),
                selected: 0,
            }
        }
    }

    impl Menu for TestMenu {
        fn title(&self) -> &str {
            &self.title
        }

        fn items(&self) -> Vec<MenuItem> {
            vec![
                MenuItem::new("Item 1"),
                MenuItem::new("Item 2"),
                MenuItem::new("Item 3"),
            ]
        }

        fn selected(&self) -> usize {
            self.selected
        }

        fn set_selected(&mut self, idx: usize) {
            self.selected = idx;
        }

        fn handle_key(&mut self, key: KeyEvent) -> MenuResult {
            match key.code {
                KeyCode::Esc => MenuResult::Close,
                _ => MenuResult::Continue,
            }
        }

        fn render(&self, _frame: &mut ratatui::Frame<'_>, _area: Rect) {
            // No-op for testing
        }
    }

    #[test]
    fn new_creates_empty_stack() {
        let stack = MenuStack::new();

        assert!(!stack.is_active());
        assert_eq!(stack.depth(), 0);
        assert!(stack.current().is_none());
    }

    #[test]
    fn push_adds_menu() {
        let mut stack = MenuStack::new();

        stack.push(Box::new(TestMenu::new("Menu 1")));

        assert!(stack.is_active());
        assert_eq!(stack.depth(), 1);
        assert!(stack.current().is_some());
        assert_eq!(stack.current().expect("menu").title(), "Menu 1");
    }

    #[test]
    fn push_multiple_menus() {
        let mut stack = MenuStack::new();

        stack.push(Box::new(TestMenu::new("Menu 1")));
        stack.push(Box::new(TestMenu::new("Menu 2")));
        stack.push(Box::new(TestMenu::new("Menu 3")));

        assert_eq!(stack.depth(), 3);
        assert_eq!(stack.current().expect("menu").title(), "Menu 3");
    }

    #[test]
    fn pop_removes_top_menu() {
        let mut stack = MenuStack::new();

        stack.push(Box::new(TestMenu::new("Menu 1")));
        stack.push(Box::new(TestMenu::new("Menu 2")));

        let popped = stack.pop();

        assert!(popped.is_some());
        assert_eq!(popped.expect("menu").title(), "Menu 2");
        assert_eq!(stack.depth(), 1);
        assert_eq!(stack.current().expect("menu").title(), "Menu 1");
    }

    #[test]
    fn pop_empty_returns_none() {
        let mut stack = MenuStack::new();

        let popped = stack.pop();

        assert!(popped.is_none());
    }

    #[test]
    fn clear_removes_all_menus() {
        let mut stack = MenuStack::new();

        stack.push(Box::new(TestMenu::new("Menu 1")));
        stack.push(Box::new(TestMenu::new("Menu 2")));
        stack.push(Box::new(TestMenu::new("Menu 3")));

        stack.clear();

        assert!(!stack.is_active());
        assert_eq!(stack.depth(), 0);
    }

    #[test]
    fn current_mut_allows_modification() {
        let mut stack = MenuStack::new();

        stack.push(Box::new(TestMenu::new("Menu 1")));

        if let Some(menu) = stack.current_mut() {
            menu.set_selected(2);
        }

        assert_eq!(stack.current().expect("menu").selected(), 2);
    }

    #[test]
    fn is_active_reflects_stack_state() {
        let mut stack = MenuStack::new();

        assert!(!stack.is_active());

        stack.push(Box::new(TestMenu::new("Menu")));
        assert!(stack.is_active());

        stack.pop();
        assert!(!stack.is_active());
    }
}
