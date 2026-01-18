//! Selection menu implementation.
//!
//! Not yet integrated - provides generic `SelectMenu` for future use.

#![allow(dead_code)] // Module reserved for future selection menus

use crossterm::event::{KeyCode, KeyEvent};

use super::{ItemStyle, Menu, MenuItem, MenuResult};

/// A menu for selecting from a list of items
pub struct SelectMenu<T> {
    /// Title for the menu
    title: String,
    /// Items with their associated values
    items: Vec<SelectItem<T>>,
    /// Currently selected index
    selected: usize,
}

/// An item in a selection menu
pub struct SelectItem<T> {
    /// Display label
    pub label: String,
    /// Optional key hint
    pub key: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Whether this item is enabled
    pub enabled: bool,
    /// The value associated with this item
    pub value: T,
}

impl<T> SelectItem<T> {
    /// Create a new select item
    pub fn new(label: impl Into<String>, value: T) -> Self {
        Self {
            label: label.into(),
            key: None,
            description: None,
            enabled: true,
            value,
        }
    }

    /// Add a key hint
    #[must_use]
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Add a description
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set enabled state
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<T: Clone + Send + 'static> SelectMenu<T> {
    /// Create a new selection menu
    pub fn new(title: impl Into<String>, items: Vec<SelectItem<T>>) -> Self {
        Self {
            title: title.into(),
            items,
            selected: 0,
        }
    }

    /// Get the currently selected value
    pub fn selected_value(&self) -> Option<&T> {
        self.items
            .get(self.selected)
            .map(|i| &i.value)
    }
}

impl<T: Clone + Send + 'static> Menu for SelectMenu<T> {
    fn title(&self) -> &str {
        &self.title
    }

    fn items(&self) -> Vec<MenuItem> {
        self.items
            .iter()
            .map(|item| {
                let mut menu_item = MenuItem::new(&item.label);
                if let Some(ref key) = item.key {
                    menu_item = menu_item.with_key(key);
                }
                if let Some(ref desc) = item.description {
                    menu_item = menu_item.with_description(desc);
                }
                menu_item = menu_item.enabled(item.enabled);
                if !item.enabled {
                    menu_item = menu_item.with_style(ItemStyle::Disabled);
                }
                menu_item
            })
            .collect()
    }

    fn selected(&self) -> usize {
        self.selected
    }

    fn set_selected(&mut self, idx: usize) {
        if idx < self.items.len() {
            self.selected = idx;
        }
    }

    #[allow(clippy::option_if_let_else)] // if-let-else is clearer here
    fn handle_key(&mut self, key: KeyEvent) -> MenuResult {
        match key.code {
            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_prev();
                MenuResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                MenuResult::Continue
            }

            // Selection
            KeyCode::Enter => {
                if let Some(item) = self.items.get(self.selected) {
                    if item.enabled {
                        // Return the selected value - caller will handle it
                        MenuResult::Close
                    } else {
                        MenuResult::Continue
                    }
                } else {
                    MenuResult::Continue
                }
            }

            // Close
            KeyCode::Esc | KeyCode::Char('q' | 'm') => MenuResult::Close,

            _ => MenuResult::Continue,
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        super::render_menu(self, frame, area);
    }
}
