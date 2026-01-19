//! Push confirmation menu.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{Menu, MenuAction, MenuItem, MenuResult};
use crate::command::{CommandRequest, CommandSource};

/// Menu for confirming git push operation
pub struct PushConfirmMenu {
    branch: String,
    remote: String,
    has_upstream: bool,
    ahead: usize,
    force: bool,
    /// Focus: 0 = confirm button, 1 = force checkbox
    focus: usize,
}

impl PushConfirmMenu {
    /// Create a new push confirmation menu
    pub const fn new(
        branch: String,
        remote: String,
        has_upstream: bool,
        ahead: usize,
        force: bool,
    ) -> Self {
        Self {
            branch,
            remote,
            has_upstream,
            ahead,
            force,
            focus: 0,
        }
    }

    /// Build the git push command
    fn build_command(&self) -> CommandRequest {
        let mut args = vec!["push".to_string()];

        if self.force {
            args.push("--force-with-lease".to_string());
        }

        if !self.has_upstream {
            args.push("-u".to_string());
            args.push(self.remote.clone());
            args.push(self.branch.clone());
        }

        let display = if self.force {
            format!(
                "git push --force-with-lease {}",
                self.remote
            )
        } else if !self.has_upstream {
            format!(
                "git push -u {} {}",
                self.remote, self.branch
            )
        } else {
            "git push".to_string()
        };

        CommandRequest::git(args)
            .with_source(CommandSource::Keyboard)
            .with_display_name(display)
    }
}

impl Menu for PushConfirmMenu {
    fn title(&self) -> &'static str {
        "Push Confirmation"
    }

    fn items(&self) -> Vec<MenuItem> {
        // Custom rendering, so return empty
        Vec::new()
    }

    fn selected(&self) -> usize {
        self.focus
    }

    fn set_selected(&mut self, idx: usize) {
        if idx <= 1 {
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
                if self.focus < 1 {
                    self.focus += 1;
                }
                MenuResult::Continue
            }

            // Toggle force push (when focused or via shortcut)
            KeyCode::Char(' ') => {
                if self.focus == 1 {
                    self.force = !self.force;
                }
                MenuResult::Continue
            }
            KeyCode::Char('f' | 'F') => {
                self.force = !self.force;
                MenuResult::Continue
            }

            // Confirm
            KeyCode::Enter => {
                if self.focus == 0 {
                    // Confirm button - execute push
                    let request = self.build_command();
                    MenuResult::Execute(MenuAction::Command(request))
                } else {
                    // Toggle checkbox
                    self.force = !self.force;
                    MenuResult::Continue
                }
            }
            KeyCode::Char('y') => {
                let request = self.build_command();
                MenuResult::Execute(MenuAction::Command(request))
            }

            // Cancel
            KeyCode::Esc | KeyCode::Char('n' | 'q') => MenuResult::CloseAll,

            _ => MenuResult::Continue,
        }
    }

    #[allow(clippy::too_many_lines)] // Render methods are naturally verbose
    fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        frame.render_widget(Clear, area);

        let mut lines = Vec::new();

        // Message
        let message = if self.has_upstream {
            format!(
                "Push {} to {}?",
                self.branch, self.remote
            )
        } else {
            format!(
                "Push {} to {} and set upstream?",
                self.branch, self.remote
            )
        };
        lines.push(Line::from(Span::styled(
            message,
            Style::default().fg(Color::White),
        )));
        lines.push(Line::from(""));

        // Info lines
        lines.push(Line::from(vec![
            Span::styled(
                "Branch: ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                &self.branch,
                Style::default().fg(Color::Cyan),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled(
                "Remote: ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                &self.remote,
                Style::default().fg(Color::Cyan),
            ),
        ]));
        if self.ahead > 0 {
            lines.push(Line::from(vec![
                Span::styled(
                    "Commits: ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{} ahead", self.ahead),
                    Style::default().fg(Color::Green),
                ),
            ]));
        }
        lines.push(Line::from(""));

        // Force push checkbox
        let force_focused = self.focus == 1;
        let check = if self.force { "☑" } else { "☐" };
        let prefix = if force_focused { "▸ " } else { "  " };

        lines.push(Line::from(vec![
            Span::raw(prefix),
            Span::styled(
                format!("{check} "),
                if force_focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                },
            ),
            Span::styled(
                "Force push (--force-with-lease)",
                if force_focused {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            ),
            Span::styled(
                " (f)",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::from(""));

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
                .title(" Push Confirmation ")
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

    mod push_confirm_menu {
        use super::*;

        fn create_menu() -> PushConfirmMenu {
            PushConfirmMenu::new(
                "main".to_string(),
                "origin".to_string(),
                true,  // has_upstream
                3,     // ahead
                false, // force
            )
        }

        #[test]
        fn new_creates_menu() {
            let menu = create_menu();

            assert_eq!(menu.title(), "Push Confirmation");
            assert_eq!(menu.selected(), 0); // Focus on confirm button
        }

        #[test]
        fn new_without_upstream() {
            let menu = PushConfirmMenu::new(
                "feature".to_string(),
                "origin".to_string(),
                false, // no upstream
                0,
                false,
            );

            assert!(!menu.has_upstream);
        }

        #[test]
        fn set_selected_updates_focus() {
            let mut menu = create_menu();

            menu.set_selected(1);

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn set_selected_ignores_out_of_bounds() {
            let mut menu = create_menu();

            menu.set_selected(5);

            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn handle_key_f_toggles_force() {
            let mut menu = create_menu();
            assert!(!menu.force);

            menu.handle_key(KeyEvent::from(KeyCode::Char('f')));

            assert!(menu.force);
        }

        #[test]
        fn handle_key_f_toggles_force_off() {
            let mut menu = PushConfirmMenu::new(
                "main".to_string(),
                "origin".to_string(),
                true,
                0,
                true, // force enabled
            );

            menu.handle_key(KeyEvent::from(KeyCode::Char('f')));

            assert!(!menu.force);
        }

        #[test]
        fn handle_key_space_toggles_when_on_checkbox() {
            let mut menu = create_menu();
            menu.set_selected(1); // Focus on checkbox

            menu.handle_key(KeyEvent::from(KeyCode::Char(' ')));

            assert!(menu.force);
        }

        #[test]
        fn handle_key_space_does_nothing_when_on_button() {
            let mut menu = create_menu();
            // Focus is on button (0)

            menu.handle_key(KeyEvent::from(KeyCode::Char(' ')));

            assert!(!menu.force);
        }

        #[test]
        fn handle_key_y_executes() {
            let mut menu = create_menu();

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('y')));

            assert!(matches!(result, MenuResult::Execute(MenuAction::Command(_))));
        }

        #[test]
        fn handle_key_enter_on_button_executes() {
            let mut menu = create_menu();
            // Focus is on button (0)

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            assert!(matches!(result, MenuResult::Execute(MenuAction::Command(_))));
        }

        #[test]
        fn handle_key_enter_on_checkbox_toggles() {
            let mut menu = create_menu();
            menu.set_selected(1); // Focus on checkbox

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            assert!(matches!(result, MenuResult::Continue));
            assert!(menu.force);
        }

        #[test]
        fn handle_key_n_closes_all() {
            let mut menu = create_menu();

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('n')));

            assert!(matches!(result, MenuResult::CloseAll));
        }

        #[test]
        fn handle_key_esc_closes_all() {
            let mut menu = create_menu();

            let result = menu.handle_key(KeyEvent::from(KeyCode::Esc));

            assert!(matches!(result, MenuResult::CloseAll));
        }

        #[test]
        fn handle_key_q_closes_all() {
            let mut menu = create_menu();

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('q')));

            assert!(matches!(result, MenuResult::CloseAll));
        }

        #[test]
        fn handle_key_down_navigates() {
            let mut menu = create_menu();

            menu.handle_key(KeyEvent::from(KeyCode::Down));

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn handle_key_up_navigates() {
            let mut menu = create_menu();
            menu.set_selected(1);

            menu.handle_key(KeyEvent::from(KeyCode::Up));

            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn handle_key_j_navigates_down() {
            let mut menu = create_menu();

            menu.handle_key(KeyEvent::from(KeyCode::Char('j')));

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn handle_key_k_navigates_up() {
            let mut menu = create_menu();
            menu.set_selected(1);

            menu.handle_key(KeyEvent::from(KeyCode::Char('k')));

            assert_eq!(menu.selected(), 0);
        }
    }

    mod build_command {
        use super::*;

        #[test]
        fn simple_push_with_upstream() {
            let menu = PushConfirmMenu::new(
                "main".to_string(),
                "origin".to_string(),
                true,  // has_upstream
                0,
                false, // no force
            );

            let cmd = menu.build_command();

            assert_eq!(cmd.display_name, "git push");
        }

        #[test]
        fn push_without_upstream_sets_upstream() {
            let menu = PushConfirmMenu::new(
                "feature".to_string(),
                "origin".to_string(),
                false, // no upstream
                0,
                false,
            );

            let cmd = menu.build_command();

            assert_eq!(cmd.display_name, "git push -u origin feature");
        }

        #[test]
        fn force_push_uses_force_with_lease() {
            let menu = PushConfirmMenu::new(
                "main".to_string(),
                "origin".to_string(),
                true,
                0,
                true, // force
            );

            let cmd = menu.build_command();

            assert_eq!(cmd.display_name, "git push --force-with-lease origin");
        }
    }
}
