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
        self.stack.last().map(|m| m.as_ref())
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
