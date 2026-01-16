//! Action menu for contextual actions.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem};
use ratatui::Frame;

use super::{Menu, MenuAction, MenuItem, MenuResult};
use crate::actions::{Action, ActionType, Context};
use crate::command::{CommandRequest, CommandSource};

/// Menu for displaying contextual actions
pub struct ActionMenu {
    context: Context,
    actions: Vec<Action>,
    repo_path: PathBuf,
    selected: usize,
}

impl ActionMenu {
    /// Create a new action menu
    pub fn new(context: Context, actions: Vec<Action>, repo_path: PathBuf) -> Self {
        Self {
            context,
            actions,
            repo_path,
            selected: 0,
        }
    }

    /// Get context name for display
    fn context_name(&self) -> &str {
        self.context.display_name()
    }
}

impl Menu for ActionMenu {
    fn title(&self) -> &str {
        // Use context name directly (can't format with lifetime)
        self.context.display_name()
    }

    fn items(&self) -> Vec<MenuItem> {
        self.actions
            .iter()
            .map(|action| {
                let type_indicator = match &action.action_type {
                    ActionType::App(_) => "",
                    ActionType::Cli(_) => "⌘ ",
                    ActionType::Alias(_) => "⎇ ",
                };
                MenuItem::new(format!(
                    "{}{}",
                    type_indicator, action.label
                ))
                .with_key(&action.key)
            })
            .collect()
    }

    fn selected(&self) -> usize {
        self.selected
    }

    fn set_selected(&mut self, idx: usize) {
        if idx < self.actions.len() {
            self.selected = idx;
        }
    }

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

            // Execute selected action
            KeyCode::Enter => {
                if let Some(action) = self.actions.get(self.selected) {
                    match &action.action_type {
                        ActionType::App(app_action) => {
                            MenuResult::Execute(MenuAction::App(*app_action))
                        }
                        ActionType::Cli(cmd) => {
                            if let Some(request) = CommandRequest::from_input(cmd) {
                                MenuResult::Execute(MenuAction::Command(
                                    request.with_source(CommandSource::ActionMenu),
                                ))
                            } else {
                                MenuResult::Close
                            }
                        }
                        ActionType::Alias(alias) => {
                            let request = CommandRequest::git_alias(
                                &alias.name,
                                &alias.command,
                                &self.repo_path,
                            )
                            .with_source(CommandSource::ActionMenu);
                            MenuResult::Execute(MenuAction::Command(request))
                        }
                    }
                } else {
                    MenuResult::Continue
                }
            }

            // Close menu
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('m') => MenuResult::Close,

            _ => MenuResult::Continue,
        }
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        frame.render_widget(Clear, area);

        let mut items: Vec<ListItem<'_>> = Vec::new();

        for (i, action) in self.actions.iter().enumerate() {
            let is_selected = i == self.selected;
            let prefix = if is_selected { "▸ " } else { "  " };

            let style = if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Type indicator
            let type_indicator = match &action.action_type {
                ActionType::App(_) => "",
                ActionType::Cli(_) => "⌘ ",
                ActionType::Alias(_) => "⎇ ",
            };

            items.push(ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled(
                    format!("{:<8}", action.key),
                    style.fg(Color::Cyan),
                ),
                Span::raw(type_indicator),
                Span::styled(action.label.clone(), style),
            ])));
        }

        // Add footer hint
        items.push(ListItem::new(Line::from("")));
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                "  j/k ",
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                "navigate  ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                "Enter ",
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                "execute  ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                "m/Esc ",
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                "close",
                Style::default().fg(Color::DarkGray),
            ),
        ])));

        let title = format!(" {} Actions ", self.context_name());
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(title)
                .title_style(Style::default().fg(Color::Cyan).bold()),
        );

        frame.render_widget(list, area);
    }
}
