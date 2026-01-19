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

#[cfg(test)]
mod tests {
    use super::*;

    mod select_item {
        use super::*;

        #[test]
        fn new_creates_enabled_item() {
            let item = SelectItem::new("Test", 42);

            assert_eq!(item.label, "Test");
            assert_eq!(item.value, 42);
            assert!(item.enabled);
            assert!(item.key.is_none());
            assert!(item.description.is_none());
        }

        #[test]
        fn with_key_sets_key() {
            let item = SelectItem::new("Test", 1).with_key("k");

            assert_eq!(item.key, Some("k".to_string()));
        }

        #[test]
        fn with_description_sets_description() {
            let item = SelectItem::new("Test", 1).with_description("A description");

            assert_eq!(item.description, Some("A description".to_string()));
        }

        #[test]
        fn enabled_sets_state() {
            let item = SelectItem::new("Test", 1).enabled(false);

            assert!(!item.enabled);
        }

        #[test]
        fn builder_chain_works() {
            let item = SelectItem::new("Action", "value")
                .with_key("a")
                .with_description("Does something")
                .enabled(true);

            assert_eq!(item.label, "Action");
            assert_eq!(item.value, "value");
            assert_eq!(item.key, Some("a".to_string()));
            assert_eq!(item.description, Some("Does something".to_string()));
            assert!(item.enabled);
        }
    }

    mod select_menu {
        use super::*;

        #[test]
        fn new_creates_menu_with_items() {
            let items = vec![
                SelectItem::new("One", 1),
                SelectItem::new("Two", 2),
                SelectItem::new("Three", 3),
            ];
            let menu = SelectMenu::new("Test Menu", items);

            assert_eq!(menu.title(), "Test Menu");
            assert_eq!(menu.items().len(), 3);
            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn selected_value_returns_current_value() {
            let items = vec![
                SelectItem::new("One", 1),
                SelectItem::new("Two", 2),
            ];
            let menu = SelectMenu::new("Test", items);

            assert_eq!(menu.selected_value(), Some(&1));
        }

        #[test]
        fn set_selected_updates_selection() {
            let items = vec![
                SelectItem::new("One", 1),
                SelectItem::new("Two", 2),
            ];
            let mut menu = SelectMenu::new("Test", items);

            menu.set_selected(1);

            assert_eq!(menu.selected(), 1);
            assert_eq!(menu.selected_value(), Some(&2));
        }

        #[test]
        fn set_selected_ignores_out_of_bounds() {
            let items = vec![SelectItem::new("One", 1)];
            let mut menu = SelectMenu::new("Test", items);

            menu.set_selected(5);

            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn items_converts_to_menu_items() {
            let items = vec![
                SelectItem::new("Enabled", 1),
                SelectItem::new("Disabled", 2).enabled(false),
            ];
            let menu = SelectMenu::new("Test", items);

            let menu_items = menu.items();

            assert_eq!(menu_items.len(), 2);
            assert!(menu_items[0].enabled);
            assert!(!menu_items[1].enabled);
        }

        #[test]
        fn items_includes_key_and_description() {
            let items = vec![
                SelectItem::new("Item", 1)
                    .with_key("i")
                    .with_description("An item"),
            ];
            let menu = SelectMenu::new("Test", items);

            let menu_items = menu.items();

            assert_eq!(menu_items[0].key_hint, Some("i".to_string()));
            assert_eq!(menu_items[0].description, Some("An item".to_string()));
        }

        #[test]
        fn handle_key_esc_closes() {
            let items = vec![SelectItem::new("One", 1)];
            let mut menu = SelectMenu::new("Test", items);

            let result = menu.handle_key(KeyEvent::from(KeyCode::Esc));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn handle_key_q_closes() {
            let items = vec![SelectItem::new("One", 1)];
            let mut menu = SelectMenu::new("Test", items);

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('q')));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn handle_key_enter_on_enabled_closes() {
            let items = vec![SelectItem::new("One", 1)];
            let mut menu = SelectMenu::new("Test", items);

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn handle_key_enter_on_disabled_continues() {
            let items = vec![SelectItem::new("One", 1).enabled(false)];
            let mut menu = SelectMenu::new("Test", items);

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            assert!(matches!(result, MenuResult::Continue));
        }

        #[test]
        fn handle_key_down_navigates() {
            let items = vec![
                SelectItem::new("One", 1),
                SelectItem::new("Two", 2),
            ];
            let mut menu = SelectMenu::new("Test", items);

            menu.handle_key(KeyEvent::from(KeyCode::Down));

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn handle_key_j_navigates_down() {
            let items = vec![
                SelectItem::new("One", 1),
                SelectItem::new("Two", 2),
            ];
            let mut menu = SelectMenu::new("Test", items);

            menu.handle_key(KeyEvent::from(KeyCode::Char('j')));

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn handle_key_up_navigates() {
            let items = vec![
                SelectItem::new("One", 1),
                SelectItem::new("Two", 2),
            ];
            let mut menu = SelectMenu::new("Test", items);
            menu.set_selected(1);

            menu.handle_key(KeyEvent::from(KeyCode::Up));

            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn handle_key_k_navigates_up() {
            let items = vec![
                SelectItem::new("One", 1),
                SelectItem::new("Two", 2),
            ];
            let mut menu = SelectMenu::new("Test", items);
            menu.set_selected(1);

            menu.handle_key(KeyEvent::from(KeyCode::Char('k')));

            assert_eq!(menu.selected(), 0);
        }
    }
}
