//! Working directory files section.
//!
//! Displays unstaged changes in the working directory with contextual actions.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::{Section, SectionAction, SectionId, SectionState};
use crate::actions::{Action, ActionRegistry, AppState, Context};
use crate::git::FileStatus;
use crate::render::file_list::{render_file_entry, FileEntryView, FileListStyle};

/// Data needed by the working section for rendering
#[derive(Debug, Clone, Default)]
pub struct WorkingSectionData {
    /// List of working directory files with changes
    pub files: Vec<FileStatus>,
    /// Whether command mode is active (suppresses selection highlight)
    pub command_mode_active: bool,
    /// Action registry for hints
    pub action_registry: Option<ActionRegistry>,
    /// App state for action conditions
    pub app_state: Option<AppState>,
}

/// Working directory files section
pub struct WorkingSection {
    data: WorkingSectionData,
}

impl WorkingSection {
    /// Create a new working section
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: WorkingSectionData::default(),
        }
    }

    /// Update section data from app state
    pub fn update(&mut self, data: WorkingSectionData) {
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

        let actions = registry.hint_actions_for_context(Context::WorkingFiles, state);

        if actions.is_empty() {
            return Line::from("");
        }

        let mut spans = vec![Span::raw("  ")];
        let italic_gray = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);

        for (i, action) in actions.iter().take(5).enumerate() {
            if i > 0 {
                spans.push(Span::styled(
                    " · ",
                    Style::default().fg(Color::DarkGray),
                ));
            }
            spans.push(Span::styled(
                format!("{} ", action.key),
                Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::styled(
                action.label.clone(),
                italic_gray,
            ));
        }

        Line::from(spans)
    }
}

impl Default for WorkingSection {
    fn default() -> Self {
        Self::new()
    }
}

impl Section for WorkingSection {
    fn id(&self) -> SectionId {
        SectionId::Working
    }

    fn name(&self) -> &'static str {
        "Working"
    }

    fn contexts(&self) -> Vec<Context> {
        vec![Context::WorkingFiles]
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
            format!("  {arrow} Working ({file_count})"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        // Hints (only when focused)
        if in_section {
            lines.push(self.render_hints());
        }

        // File entries using shared renderer
        let style = FileListStyle {
            indicator: Some(("○ ", Color::Yellow)),
            ..Default::default()
        };

        for (i, file) in self.data.files.iter().enumerate() {
            let selected = state.local_selection == Some(i)
                && state.is_focused
                && !self.data.command_mode_active;

            let entry = FileEntryView {
                path: &file.path.to_string_lossy(),
                status: file.working,
                insertions: file.working_insertions,
                deletions: file.working_deletions,
            };

            lines.push(render_file_entry(
                &entry, &style, selected,
            ));
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
            KeyCode::Char('s') => {
                // Stage file
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
            // Note: discard (x) is handled at app level with confirmation
            _ => None,
        }
    }
}
