//! Confirmation dialog implementation.
//!
//! Not yet integrated - provides generic `ConfirmMenu` for future use.

#![allow(dead_code)] // Module reserved for future generic confirm dialogs

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{Menu, MenuItem, MenuResult};

/// A confirmation dialog with optional checkboxes
pub struct ConfirmMenu {
    /// Title for the dialog
    title: String,
    /// Message to display
    message: String,
    /// Info lines to display (label: value pairs)
    info: Vec<(String, String)>,
    /// Checkbox options
    checkboxes: Vec<Checkbox>,
    /// Currently focused element (0 = confirm button, 1+ = checkboxes)
    focus: usize,
}

/// A checkbox option in a confirmation dialog
pub struct Checkbox {
    /// Label for the checkbox
    pub label: String,
    /// Key identifier
    pub key: String,
    /// Current checked state
    pub checked: bool,
    /// Keyboard shortcut to toggle
    pub shortcut: Option<char>,
}

impl Checkbox {
    /// Create a new checkbox
    pub fn new(label: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            key: key.into(),
            checked: false,
            shortcut: None,
        }
    }

    /// Set initial checked state
    #[must_use]
    pub const fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set keyboard shortcut
    #[must_use]
    pub const fn with_shortcut(mut self, shortcut: char) -> Self {
        self.shortcut = Some(shortcut);
        self
    }
}

impl ConfirmMenu {
    /// Create a new confirmation dialog
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            info: Vec::new(),
            checkboxes: Vec::new(),
            focus: 0,
        }
    }

    /// Add an info line
    #[must_use]
    pub fn with_info(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.info
            .push((label.into(), value.into()));
        self
    }

    /// Add a checkbox
    #[must_use]
    pub fn with_checkbox(mut self, checkbox: Checkbox) -> Self {
        self.checkboxes.push(checkbox);
        self
    }

    /// Get checkbox value by key
    pub fn get_checkbox(&self, key: &str) -> bool {
        self.checkboxes
            .iter()
            .find(|c| c.key == key)
            .is_some_and(|c| c.checked)
    }

    /// Toggle the focused checkbox
    fn toggle_focused(&mut self) {
        if self.focus > 0 {
            let idx = self.focus - 1;
            if let Some(cb) = self.checkboxes.get_mut(idx) {
                cb.checked = !cb.checked;
            }
        }
    }

    /// Handle shortcut key
    fn handle_shortcut(&mut self, c: char) -> bool {
        for cb in &mut self.checkboxes {
            if cb.shortcut == Some(c) {
                cb.checked = !cb.checked;
                return true;
            }
        }
        false
    }
}

impl Menu for ConfirmMenu {
    fn title(&self) -> &str {
        &self.title
    }

    fn items(&self) -> Vec<MenuItem> {
        // Not used for custom rendering
        Vec::new()
    }

    fn selected(&self) -> usize {
        self.focus
    }

    fn set_selected(&mut self, idx: usize) {
        let max = self.checkboxes.len() + 1; // +1 for confirm button
        if idx < max {
            self.focus = idx;
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> MenuResult {
        match key.code {
            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                if self.focus > 0 {
                    self.focus -= 1;
                }
                MenuResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.checkboxes.len();
                if self.focus < max {
                    self.focus += 1;
                }
                MenuResult::Continue
            }

            // Toggle checkbox (when focused on one)
            KeyCode::Char(' ') => {
                self.toggle_focused();
                MenuResult::Continue
            }

            // Confirm (Enter on button or 'y')
            KeyCode::Enter => {
                if self.focus == 0 {
                    // Confirm button is focused
                    MenuResult::Close
                } else {
                    // Toggle checkbox
                    self.toggle_focused();
                    MenuResult::Continue
                }
            }
            KeyCode::Char('y') => MenuResult::Close,

            // Cancel
            KeyCode::Esc | KeyCode::Char('n' | 'q') => MenuResult::CloseAll,

            // Shortcuts
            KeyCode::Char(c) => {
                self.handle_shortcut(c);
                MenuResult::Continue
            }

            _ => MenuResult::Continue,
        }
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        frame.render_widget(Clear, area);

        let mut lines = Vec::new();

        // Message
        lines.push(Line::from(Span::styled(
            &self.message,
            Style::default().fg(Color::White),
        )));
        lines.push(Line::from(""));

        // Info lines
        for (label, value) in &self.info {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{label}: "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(value, Style::default().fg(Color::Cyan)),
            ]));
        }

        if !self.info.is_empty() {
            lines.push(Line::from(""));
        }

        // Checkboxes
        for (i, cb) in self.checkboxes.iter().enumerate() {
            let is_focused = self.focus == i + 1;
            let check = if cb.checked { "☑" } else { "☐" };
            let prefix = if is_focused { "▸ " } else { "  " };

            let mut spans = vec![
                Span::raw(prefix),
                Span::styled(
                    format!("{check} "),
                    if is_focused {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(
                    &cb.label,
                    if is_focused {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
            ];

            if let Some(shortcut) = cb.shortcut {
                spans.push(Span::styled(
                    format!(" ({shortcut})"),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            lines.push(Line::from(spans));
        }

        if !self.checkboxes.is_empty() {
            lines.push(Line::from(""));
        }

        // Confirm button
        let button_focused = self.focus == 0;
        let button_style = if button_focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        lines.push(Line::from(vec![
            Span::raw(if button_focused { "▸ " } else { "  " }),
            Span::styled(" Confirm (y) ", button_style),
            Span::styled("  ", Style::default()),
            Span::styled(
                "Cancel (n/Esc)",
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        let para = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(format!(" {} ", self.title))
                .title_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
        );

        frame.render_widget(para, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod checkbox {
        use super::*;

        #[test]
        fn new_creates_unchecked_checkbox() {
            let cb = Checkbox::new("Test", "test_key");

            assert_eq!(cb.label, "Test");
            assert_eq!(cb.key, "test_key");
            assert!(!cb.checked);
            assert!(cb.shortcut.is_none());
        }

        #[test]
        fn checked_sets_initial_state() {
            let cb = Checkbox::new("Test", "key").checked(true);

            assert!(cb.checked);
        }

        #[test]
        fn with_shortcut_sets_shortcut() {
            let cb = Checkbox::new("Test", "key").with_shortcut('t');

            assert_eq!(cb.shortcut, Some('t'));
        }

        #[test]
        fn builder_chain_works() {
            let cb = Checkbox::new("Force", "force")
                .checked(false)
                .with_shortcut('f');

            assert_eq!(cb.label, "Force");
            assert_eq!(cb.key, "force");
            assert!(!cb.checked);
            assert_eq!(cb.shortcut, Some('f'));
        }
    }

    mod confirm_menu {
        use super::*;

        #[test]
        fn new_creates_menu() {
            let menu = ConfirmMenu::new("Confirm", "Are you sure?");

            assert_eq!(menu.title(), "Confirm");
            assert_eq!(menu.selected(), 0); // Focus on confirm button
        }

        #[test]
        fn with_info_adds_info_line() {
            let menu = ConfirmMenu::new("Confirm", "Message").with_info("Branch", "main");

            assert_eq!(menu.info.len(), 1);
            assert_eq!(
                menu.info[0],
                ("Branch".to_string(), "main".to_string())
            );
        }

        #[test]
        fn with_checkbox_adds_checkbox() {
            let menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Option", "opt"));

            assert_eq!(menu.checkboxes.len(), 1);
        }

        #[test]
        fn get_checkbox_returns_false_for_unchecked() {
            let menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Option", "opt"));

            assert!(!menu.get_checkbox("opt"));
        }

        #[test]
        fn get_checkbox_returns_true_for_checked() {
            let menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Option", "opt").checked(true));

            assert!(menu.get_checkbox("opt"));
        }

        #[test]
        fn get_checkbox_returns_false_for_unknown_key() {
            let menu = ConfirmMenu::new("Confirm", "Message");

            assert!(!menu.get_checkbox("unknown"));
        }

        #[test]
        fn set_selected_updates_focus() {
            let mut menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Opt1", "opt1"))
                .with_checkbox(Checkbox::new("Opt2", "opt2"));

            menu.set_selected(1);

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn set_selected_ignores_out_of_bounds() {
            let mut menu =
                ConfirmMenu::new("Confirm", "Message").with_checkbox(Checkbox::new("Opt", "opt"));

            menu.set_selected(10);

            // Max is 2 (confirm button + 1 checkbox)
            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn handle_key_y_closes_with_confirm() {
            let mut menu = ConfirmMenu::new("Confirm", "Message");

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('y')));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn handle_key_n_closes_all() {
            let mut menu = ConfirmMenu::new("Confirm", "Message");

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('n')));

            assert!(matches!(result, MenuResult::CloseAll));
        }

        #[test]
        fn handle_key_esc_closes_all() {
            let mut menu = ConfirmMenu::new("Confirm", "Message");

            let result = menu.handle_key(KeyEvent::from(KeyCode::Esc));

            assert!(matches!(result, MenuResult::CloseAll));
        }

        #[test]
        fn handle_key_enter_on_button_closes() {
            let mut menu = ConfirmMenu::new("Confirm", "Message");
            // Focus is on button by default (0)

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn handle_key_enter_on_checkbox_toggles() {
            let mut menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Option", "opt"));
            menu.set_selected(1); // Focus on checkbox

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            assert!(matches!(result, MenuResult::Continue));
            assert!(menu.get_checkbox("opt"));
        }

        #[test]
        fn handle_key_space_toggles_focused_checkbox() {
            let mut menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Option", "opt"));
            menu.set_selected(1); // Focus on checkbox

            menu.handle_key(KeyEvent::from(KeyCode::Char(' ')));

            assert!(menu.get_checkbox("opt"));
        }

        #[test]
        fn handle_key_shortcut_toggles_checkbox() {
            let mut menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Force", "force").with_shortcut('f'));

            menu.handle_key(KeyEvent::from(KeyCode::Char('f')));

            assert!(menu.get_checkbox("force"));
        }

        #[test]
        fn handle_key_down_navigates() {
            let mut menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Option", "opt"));

            menu.handle_key(KeyEvent::from(KeyCode::Down));

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn handle_key_up_navigates() {
            let mut menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Option", "opt"));
            menu.set_selected(1);

            menu.handle_key(KeyEvent::from(KeyCode::Up));

            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn handle_key_j_navigates_down() {
            let mut menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Option", "opt"));

            menu.handle_key(KeyEvent::from(KeyCode::Char('j')));

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn handle_key_k_navigates_up() {
            let mut menu = ConfirmMenu::new("Confirm", "Message")
                .with_checkbox(Checkbox::new("Option", "opt"));
            menu.set_selected(1);

            menu.handle_key(KeyEvent::from(KeyCode::Char('k')));

            assert_eq!(menu.selected(), 0);
        }
    }
}
