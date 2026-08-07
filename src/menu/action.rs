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
    pub const fn new(context: Context, actions: Vec<Action>, repo_path: PathBuf) -> Self {
        Self {
            context,
            actions,
            repo_path,
            selected: 0,
        }
    }

    /// Get context name for display
    const fn context_name(&self) -> &str {
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
            KeyCode::Esc | KeyCode::Char('q' | 'm') => MenuResult::Close,

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, ActionCondition, ActionType, AppAction, Context};
    use crate::config::Alias;
    use crate::input::KeyBinding;
    use crossterm::event::KeyCode;
    use std::path::PathBuf;

    fn test_repo_path() -> PathBuf {
        PathBuf::from("/test/repo")
    }

    fn create_app_action(key: &str, label: &str, action: AppAction) -> Action {
        Action {
            key: key.to_string(),
            binding: Some(KeyBinding::char(
                key.chars().next().unwrap_or('x'),
            )),
            label: label.to_string(),
            action_type: ActionType::App(action),
            contexts: vec![Context::WorkingFiles],
            priority: 10,
            condition: ActionCondition::default(),
        }
    }

    fn create_cli_action(key: &str, label: &str, cmd: &str) -> Action {
        Action {
            key: key.to_string(),
            binding: None,
            label: label.to_string(),
            action_type: ActionType::Cli(cmd.to_string()),
            contexts: vec![Context::WorkingFiles],
            priority: 20,
            condition: ActionCondition::default(),
        }
    }

    fn create_alias_action(alias_name: &str, alias_cmd: &str) -> Action {
        let alias = Alias {
            name: alias_name.to_string(),
            command: alias_cmd.to_string(),
        };
        Action::from_alias(&alias, vec![Context::WorkingFiles])
    }

    mod action_menu_new {
        use super::*;

        #[test]
        fn creates_menu_with_context_and_actions() {
            let actions = vec![
                create_app_action("s", "Stage", AppAction::ToggleStage),
                create_app_action("d", "Diff", AppAction::ShowDiff),
            ];
            let menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            assert_eq!(menu.context, Context::WorkingFiles);
            assert_eq!(menu.actions.len(), 2);
            assert_eq!(menu.selected, 0);
        }

        #[test]
        fn stores_repo_path() {
            let path = PathBuf::from("/custom/path");
            let menu = ActionMenu::new(
                Context::StagedFiles,
                vec![],
                path.clone(),
            );

            assert_eq!(menu.repo_path, path);
        }
    }

    mod action_menu_title {
        use super::*;

        #[test]
        fn returns_context_display_name() {
            let menu = ActionMenu::new(
                Context::WorkingFiles,
                vec![],
                test_repo_path(),
            );
            assert_eq!(menu.title(), "Working Files");
        }

        #[test]
        fn returns_staged_files_for_staged_context() {
            let menu = ActionMenu::new(
                Context::StagedFiles,
                vec![],
                test_repo_path(),
            );
            assert_eq!(menu.title(), "Staged Files");
        }

        #[test]
        fn returns_history_for_history_context() {
            let menu = ActionMenu::new(
                Context::HistoryCommits,
                vec![],
                test_repo_path(),
            );
            assert_eq!(menu.title(), "History");
        }
    }

    mod action_menu_items {
        use super::*;

        #[test]
        fn converts_actions_to_menu_items() {
            let actions = vec![
                create_app_action("s", "Stage", AppAction::ToggleStage),
                create_app_action("d", "Diff", AppAction::ShowDiff),
            ];
            let menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let items = menu.items();

            assert_eq!(items.len(), 2);
            assert_eq!(items[0].label, "Stage");
            assert_eq!(items[1].label, "Diff");
        }

        #[test]
        fn includes_key_hints() {
            let actions = vec![create_app_action("s", "Stage", AppAction::ToggleStage)];
            let menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let items = menu.items();

            assert_eq!(items[0].key_hint, Some("s".to_string()));
        }

        #[test]
        fn cli_actions_have_command_prefix() {
            let actions = vec![create_cli_action("!", "Run command", "echo test")];
            let menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let items = menu.items();

            assert!(items[0].label.starts_with("⌘ "));
        }

        #[test]
        fn alias_actions_have_alias_prefix() {
            let actions = vec![create_alias_action("co", "checkout")];
            let menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let items = menu.items();

            assert!(items[0].label.starts_with("⎇ "));
        }

        #[test]
        fn app_actions_have_no_prefix() {
            let actions = vec![create_app_action("s", "Stage", AppAction::ToggleStage)];
            let menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let items = menu.items();

            assert!(!items[0].label.starts_with("⌘ "));
            assert!(!items[0].label.starts_with("⎇ "));
        }
    }

    mod action_menu_selection {
        use super::*;

        #[test]
        fn selected_returns_current_index() {
            let menu = ActionMenu::new(
                Context::WorkingFiles,
                vec![],
                test_repo_path(),
            );
            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn set_selected_updates_selection() {
            let actions = vec![
                create_app_action("a", "Action A", AppAction::ToggleStage),
                create_app_action("b", "Action B", AppAction::ShowDiff),
            ];
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            menu.set_selected(1);

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn set_selected_ignores_out_of_bounds() {
            let actions = vec![create_app_action("a", "Action", AppAction::ToggleStage)];
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            menu.set_selected(5);

            assert_eq!(menu.selected(), 0);
        }
    }

    mod action_menu_handle_key {
        use super::*;

        #[test]
        fn up_navigates_to_previous() {
            let actions = vec![
                create_app_action("a", "Action A", AppAction::ToggleStage),
                create_app_action("b", "Action B", AppAction::ShowDiff),
            ];
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );
            menu.set_selected(1);

            let result = menu.handle_key(KeyEvent::from(KeyCode::Up));

            assert!(matches!(result, MenuResult::Continue));
            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn k_navigates_to_previous() {
            let actions = vec![
                create_app_action("a", "Action A", AppAction::ToggleStage),
                create_app_action("b", "Action B", AppAction::ShowDiff),
            ];
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );
            menu.set_selected(1);

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('k')));

            assert!(matches!(result, MenuResult::Continue));
            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn down_navigates_to_next() {
            let actions = vec![
                create_app_action("a", "Action A", AppAction::ToggleStage),
                create_app_action("b", "Action B", AppAction::ShowDiff),
            ];
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Down));

            assert!(matches!(result, MenuResult::Continue));
            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn j_navigates_to_next() {
            let actions = vec![
                create_app_action("a", "Action A", AppAction::ToggleStage),
                create_app_action("b", "Action B", AppAction::ShowDiff),
            ];
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('j')));

            assert!(matches!(result, MenuResult::Continue));
            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn esc_closes_menu() {
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                vec![],
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Esc));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn q_closes_menu() {
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                vec![],
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('q')));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn m_closes_menu() {
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                vec![],
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('m')));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn enter_on_empty_continues() {
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                vec![],
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            assert!(matches!(result, MenuResult::Continue));
        }

        #[test]
        fn enter_on_app_action_executes_app_action() {
            let actions = vec![create_app_action("s", "Stage", AppAction::ToggleStage)];
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            match result {
                MenuResult::Execute(MenuAction::App(action)) => {
                    assert_eq!(action, AppAction::ToggleStage);
                }
                _ => panic!("Expected Execute(App(ToggleStage))"),
            }
        }

        #[test]
        fn enter_on_cli_action_executes_command() {
            let actions = vec![create_cli_action("!", "Run", "status")];
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            match result {
                MenuResult::Execute(MenuAction::Command(request)) => {
                    assert!(request.display_name.contains("status"));
                }
                _ => panic!("Expected Execute(Command)"),
            }
        }

        #[test]
        fn enter_on_invalid_cli_closes() {
            // Empty command should fail to parse
            let actions = vec![create_cli_action("!", "Empty", "")];
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn enter_on_alias_action_executes_alias_command() {
            let actions = vec![create_alias_action("co", "checkout")];
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                actions,
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            match result {
                MenuResult::Execute(MenuAction::Command(request)) => {
                    // display_name is "git <alias_name>", args contain the actual command
                    assert!(request.display_name.contains("co"));
                    assert!(request
                        .args
                        .contains(&"checkout".to_string()));
                }
                _ => panic!("Expected Execute(Command)"),
            }
        }

        #[test]
        fn unhandled_key_continues() {
            let mut menu = ActionMenu::new(
                Context::WorkingFiles,
                vec![],
                test_repo_path(),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('x')));

            assert!(matches!(result, MenuResult::Continue));
        }
    }

    mod action_menu_context_name {
        use super::*;

        #[test]
        fn returns_context_display_name() {
            let menu = ActionMenu::new(
                Context::BranchHeader,
                vec![],
                test_repo_path(),
            );

            assert_eq!(menu.context_name(), "Branch");
        }
    }
}
