//! Modular menu and prompt system.
//!
//! Provides composable menus with stack-based navigation.

mod confirm;
mod select;
mod stack;

pub use confirm::{Checkbox, ConfirmMenu};
pub use select::{SelectItem, SelectMenu};
pub use stack::MenuStack;

use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

/// A menu item for display
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// Display label
    pub label: String,
    /// Optional description/help text
    pub description: Option<String>,
    /// Optional keyboard shortcut hint
    pub key_hint: Option<String>,
    /// Whether this item can be selected
    pub enabled: bool,
    /// Visual style
    pub style: ItemStyle,
}

impl MenuItem {
    /// Create a normal menu item
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            key_hint: None,
            enabled: true,
            style: ItemStyle::Normal,
        }
    }

    /// Add a description
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a key hint
    #[must_use]
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key_hint = Some(key.into());
        self
    }

    /// Set enabled state
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set style
    #[must_use]
    pub fn with_style(mut self, style: ItemStyle) -> Self {
        self.style = style;
        self
    }

    /// Create a separator
    #[must_use]
    pub fn separator() -> Self {
        Self {
            label: String::new(),
            description: None,
            key_hint: None,
            enabled: false,
            style: ItemStyle::Separator,
        }
    }

    /// Create a header item
    #[must_use]
    pub fn header(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            key_hint: None,
            enabled: false,
            style: ItemStyle::Header,
        }
    }

    /// Create a checkbox item
    #[must_use]
    pub fn checkbox(label: impl Into<String>, checked: bool) -> Self {
        Self {
            label: label.into(),
            description: None,
            key_hint: None,
            enabled: true,
            style: ItemStyle::Checkbox { checked },
        }
    }
}

/// Visual style for menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStyle {
    /// Normal selectable item
    Normal,
    /// Disabled/grayed out
    Disabled,
    /// Visual separator line
    Separator,
    /// Section header (not selectable)
    Header,
    /// Checkbox with state
    Checkbox { checked: bool },
}

/// Result of menu interaction
pub enum MenuResult {
    /// Keep menu open, no action
    Continue,
    /// Close this menu only
    Close,
    /// Close menu and execute action
    Execute(MenuAction),
    /// Push a new menu onto the stack
    Push(Box<dyn Menu>),
    /// Pop back to previous menu
    Pop,
    /// Clear entire menu stack
    CloseAll,
}

/// Action to execute after menu closes
pub enum MenuAction {
    /// Execute a command
    Command(crate::command::CommandRequest),
    /// Trigger an app action
    App(crate::actions::AppAction),
    /// Custom callback (use sparingly)
    Custom(Box<dyn FnOnce() + Send>),
}

/// A menu that can be rendered and interacted with
pub trait Menu: Send {
    /// Title for the menu overlay
    fn title(&self) -> &str;

    /// Items to display
    fn items(&self) -> Vec<MenuItem>;

    /// Currently selected index
    fn selected(&self) -> usize;

    /// Set selection index
    fn set_selected(&mut self, idx: usize);

    /// Handle a key event
    fn handle_key(&mut self, key: KeyEvent) -> MenuResult;

    /// Render the menu (default implementation provided)
    fn render(&self, frame: &mut Frame<'_>, area: Rect)
    where
        Self: Sized,
    {
        default_render(self, frame, area);
    }

    /// Move selection up
    fn select_prev(&mut self) {
        let items = self.items();
        let current = self.selected();

        // Find previous selectable item
        for i in (0..current).rev() {
            if items.get(i).map_or(false, |item| item.enabled) {
                self.set_selected(i);
                return;
            }
        }
    }

    /// Move selection down
    fn select_next(&mut self) {
        let items = self.items();
        let current = self.selected();

        // Find next selectable item
        for i in (current + 1)..items.len() {
            if items.get(i).map_or(false, |item| item.enabled) {
                self.set_selected(i);
                return;
            }
        }
    }
}

/// Default menu rendering
fn default_render(menu: &dyn Menu, frame: &mut Frame<'_>, area: Rect) {
    use ratatui::{
        style::{Color, Modifier, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, List, ListItem},
    };

    frame.render_widget(Clear, area);

    let items = menu.items();
    let selected = menu.selected();

    let list_items: Vec<ListItem<'_>> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == selected && item.enabled;
            let prefix = if is_selected { "▸ " } else { "  " };

            let style = match item.style {
                ItemStyle::Separator => return ListItem::new(Line::from("  ────────")),
                ItemStyle::Header => {
                    return ListItem::new(Line::from(Span::styled(
                        format!("  {}", item.label),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    )));
                }
                ItemStyle::Disabled => Style::default().fg(Color::DarkGray),
                ItemStyle::Normal => {
                    if is_selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    }
                }
                ItemStyle::Checkbox { checked } => {
                    let check = if checked { "☑" } else { "☐" };
                    return ListItem::new(Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(
                            format!("{check} "),
                            if is_selected {
                                Style::default().fg(Color::Cyan)
                            } else {
                                Style::default()
                            },
                        ),
                        Span::styled(
                            item.label.clone(),
                            if is_selected {
                                Style::default().add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            },
                        ),
                    ]));
                }
            };

            let mut spans = vec![Span::raw(prefix)];

            if let Some(ref key) = item.key_hint {
                spans.push(Span::styled(
                    format!("{key:<8}"),
                    if is_selected {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ));
            }

            spans.push(Span::styled(item.label.clone(), style));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" {} ", menu.title()))
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    );

    frame.render_widget(list, area);
}
