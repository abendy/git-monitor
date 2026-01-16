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
    pub fn new(
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
    fn title(&self) -> &str {
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
            KeyCode::Char('f') | KeyCode::Char('F') => {
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
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => MenuResult::CloseAll,

            _ => MenuResult::Continue,
        }
    }

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
