//! Staged files section.
//!
//! Displays files staged for commit with contextual actions.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::{
    actions::{Action, ActionRegistry, AppState, Context},
    git::{FileState, FileStatus},
};

use super::{Section, SectionAction, SectionId, SectionState};

/// Get color for file state
const fn state_color(state: FileState) -> Color {
    match state {
        FileState::Modified => Color::Yellow,
        FileState::Added => Color::Green,
        FileState::Deleted => Color::Red,
        FileState::Renamed => Color::Cyan,
        FileState::Untracked => Color::DarkGray,
        FileState::Conflicted => Color::Magenta,
        FileState::Unmodified | FileState::Ignored => Color::White,
    }
}

/// Data needed by the staged section for rendering
#[derive(Debug, Clone, Default)]
pub struct StagedSectionData {
    /// List of staged files
    pub files: Vec<FileStatus>,
    /// Whether command mode is active (suppresses selection highlight)
    pub command_mode_active: bool,
    /// Action registry for hints
    pub action_registry: Option<ActionRegistry>,
    /// App state for action conditions
    pub app_state: Option<AppState>,
}

/// Staged files section
pub struct StagedSection {
    data: StagedSectionData,
}

impl StagedSection {
    /// Create a new staged section
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: StagedSectionData::default(),
        }
    }

    /// Update section data from app state
    pub fn update(&mut self, data: StagedSectionData) {
        self.data = data;
    }

    /// Render context hints for file actions
    fn render_hints(&self) -> Line<'static> {
        let Some(ref registry) = self.data.action_registry else {
            return Line::from("");
        };
        let Some(ref state) = self.data.app_state else {
            return Line::from("");
        };

        let actions = registry.hint_actions_for_context(Context::StagedFiles, state);

        if actions.is_empty() {
            return Line::from("");
        }

        let mut spans = vec![Span::raw("  ")];
        let italic_gray = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);

        for (i, action) in actions.iter().take(5).enumerate() {
            if i > 0 {
                spans.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            }
            spans.push(Span::styled(
                format!("{} ", action.key),
                Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::styled(action.label.clone(), italic_gray));
        }

        Line::from(spans)
    }
}

impl Default for StagedSection {
    fn default() -> Self {
        Self::new()
    }
}

impl Section for StagedSection {
    fn id(&self) -> SectionId {
        SectionId::Staged
    }

    fn name(&self) -> &str {
        "Staged"
    }

    fn contexts(&self) -> Vec<Context> {
        vec![Context::StagedFiles]
    }

    fn item_count(&self) -> usize {
        self.data.files.len()
    }

    fn render(&self, state: &SectionState) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        if self.data.files.is_empty() {
            return lines;
        }

        let file_count = self.data.files.len();
        let in_section = state.is_focused && !self.data.command_mode_active;
        let arrow = if in_section { "▾" } else { "▸" };

        // Section header
        lines.push(Line::from(Span::styled(
            format!("  {arrow} Staged ({file_count})"),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));

        // Hints (only when focused)
        if in_section {
            lines.push(self.render_hints());
        }

        // File entries
        for (i, file) in self.data.files.iter().enumerate() {
            let selected =
                state.local_selection == Some(i) && state.is_focused && !self.data.command_mode_active;
            let prefix = if selected { "▸ " } else { "  " };
            let status_char = file.staged.as_char();
            let color = state_color(file.staged);
            let path = file.path.to_string_lossy();

            let style = if selected {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled("● ", Style::default().fg(Color::Green)),
                Span::styled(format!("{status_char} "), style),
                Span::styled(path.to_string(), style),
            ]));
        }

        lines
    }

    fn actions(&self, _item_idx: usize) -> Vec<Action> {
        // Actions are provided by ActionRegistry
        Vec::new()
    }

    fn handle_key(&self, key: KeyEvent, item_idx: usize) -> Option<SectionAction> {
        // Most actions handled at app level via ActionRegistry
        // Section handles direct shortcuts for common actions
        if item_idx >= self.data.files.len() {
            return None;
        }

        match key.code {
            KeyCode::Char('u') | KeyCode::Char('s') => {
                // Toggle stage (unstage for staged files)
                Some(SectionAction::AppAction(
                    crate::actions::AppAction::ToggleStage,
                ))
            }
            KeyCode::Char('d') => {
                // Show inline diff
                Some(SectionAction::AppAction(
                    crate::actions::AppAction::ShowDiff,
                ))
            }
            _ => None,
        }
    }
}
