//! Command input section.
//!
//! Handles the command palette UI and keyboard input for git commands.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::{Section, SectionAction, SectionId, SectionState};
use crate::actions::{Action, Context};
use crate::feedback::CommandOutput;

/// Data needed by the command section for rendering
#[derive(Debug, Clone, Default)]
pub struct CommandSectionData {
    /// Current command input buffer
    pub input: String,
    /// Whether command mode is active (typing)
    pub is_active: bool,
    /// Command output to display (if any)
    pub output: Option<CommandOutput>,
    /// Whether menu is showing (suppresses output display)
    pub menu_active: bool,
}

/// Command input section at the top of the UI
pub struct CommandSection {
    data: CommandSectionData,
}

impl CommandSection {
    /// Create a new command section
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: CommandSectionData::default(),
        }
    }

    /// Update section data from app state
    pub fn update(&mut self, data: CommandSectionData) {
        self.data = data;
    }

    /// Render the command line itself
    fn render_command_line(&self, state: &SectionState) -> Line<'static> {
        let selected = state.is_focused && state.local_selection == Some(0);
        let prefix = if selected { "▸ " } else { "  " };

        let bold_cyan = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let italic_gray = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);

        if self.data.is_active {
            // Active command input with cursor
            Line::from(vec![
                Span::raw(prefix),
                Span::styled(": ", bold_cyan),
                Span::styled(
                    self.data.input.clone(),
                    Style::default().fg(Color::White),
                ),
                Span::styled("_", Style::default().fg(Color::Cyan)), // Cursor
            ])
        } else if selected {
            // Selected but not active - show hint
            Line::from(vec![
                Span::raw(prefix),
                Span::styled(": ", bold_cyan),
                Span::styled("type command   ", italic_gray),
                Span::styled("a ", Style::default().fg(Color::Cyan)),
                Span::styled("aliases", italic_gray),
            ])
        } else {
            // Not selected
            Line::from(vec![
                Span::raw(prefix),
                Span::styled(
                    ": ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    "command  ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("a ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "aliases",
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        }
    }

    /// Render command output lines (if any)
    fn render_output_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        if self.data.menu_active {
            return lines;
        }

        if let Some(ref output) = self.data.output {
            let output_color = if output.success {
                Color::White
            } else {
                Color::Red
            };

            for line in output.output.lines().take(4) {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "> ",
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        line.to_string(),
                        Style::default().fg(output_color),
                    ),
                ]));
            }

            let line_count = output.output.lines().count();
            if line_count > 4 {
                let italic_gray = Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC);
                let bold_cyan = Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD);
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(
                            "    ... ({} more lines) ",
                            line_count - 4
                        ),
                        italic_gray,
                    ),
                    Span::styled("o", bold_cyan),
                    Span::styled(" to expand", italic_gray),
                ]));
            }
        }

        lines
    }
}

impl Default for CommandSection {
    fn default() -> Self {
        Self::new()
    }
}

impl Section for CommandSection {
    fn id(&self) -> SectionId {
        SectionId::Command
    }

    fn name(&self) -> &str {
        "Command"
    }

    fn contexts(&self) -> Vec<Context> {
        vec![Context::Command]
    }

    fn item_count(&self) -> usize {
        // Command line is always 1 selectable item
        1
    }

    fn render(&self, state: &SectionState) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Main command line
        lines.push(self.render_command_line(state));

        // Output lines (non-selectable)
        lines.extend(self.render_output_lines());

        // Spacer after command section
        lines.push(Line::from(""));

        lines
    }

    fn actions(&self, _item_idx: usize) -> Vec<Action> {
        // Command section actions are handled via ActionRegistry
        Vec::new()
    }

    fn handle_key(&self, key: KeyEvent, _item_idx: usize) -> Option<SectionAction> {
        // Most key handling is done at the app level for command mode
        // Section only handles section-specific shortcuts
        match key.code {
            KeyCode::Char('a') if !self.data.is_active => {
                // Open alias browser - delegate to app
                Some(SectionAction::AppAction(
                    crate::actions::AppAction::BrowseAliases,
                ))
            }
            _ => None,
        }
    }
}
