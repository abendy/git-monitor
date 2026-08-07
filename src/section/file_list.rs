//! Generic file list section for staged and working directory files.
//!
//! Unifies the staged and working sections into a single configurable type,
//! eliminating ~130 lines of duplicate code.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::{Section, SectionAction, SectionId, SectionState};
use crate::actions::{Action, ActionRegistry, AppAction, AppState, Context};
use crate::git::FileStatus;
use crate::render::file_list::{render_file_entry, FileEntryView, FileListStyle};

/// Selects which fields to extract from [`FileStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldSelector {
    /// Use `staged`, `staged_insertions`, `staged_deletions`
    Staged,
    /// Use `working`, `working_insertions`, `working_deletions`
    Working,
}

/// Configuration for a file list section.
#[derive(Debug, Clone)]
pub struct FileListConfig {
    /// Section identifier
    pub id: SectionId,
    /// Display name for the section header
    pub name: &'static str,
    /// Context for action hints
    pub context: Context,
    /// Color for the section header
    pub header_color: Color,
    /// Indicator prefix and its color (e.g., "● " green for staged)
    pub indicator: (&'static str, Color),
    /// Which fields to extract from `FileStatus`
    pub field_selector: FieldSelector,
    /// Keys that trigger the stage/unstage action
    pub stage_keys: &'static [char],
}

impl FileListConfig {
    /// Configuration for the staged files section.
    pub const STAGED: Self = Self {
        id: SectionId::Staged,
        name: "Staged",
        context: Context::StagedFiles,
        header_color: Color::Green,
        indicator: ("● ", Color::Green),
        field_selector: FieldSelector::Staged,
        stage_keys: &['u', 's'],
    };

    /// Configuration for the working directory files section.
    pub const WORKING: Self = Self {
        id: SectionId::Working,
        name: "Working",
        context: Context::WorkingFiles,
        header_color: Color::Yellow,
        indicator: ("○ ", Color::Yellow),
        field_selector: FieldSelector::Working,
        stage_keys: &['s'],
    };
}

/// Shared data for file list sections.
#[derive(Debug, Clone, Default)]
pub struct FileListData {
    /// List of files to display
    pub files: Vec<FileStatus>,
    /// Whether command mode is active (suppresses selection highlight)
    pub command_mode_active: bool,
    /// Action registry for hints
    pub action_registry: Option<ActionRegistry>,
    /// App state for action conditions
    pub app_state: Option<AppState>,
}

/// A file list section (staged or working directory).
///
/// This is a unified implementation that handles both staged and working
/// directory file lists through configuration rather than separate types.
pub struct FileListSection {
    config: FileListConfig,
    data: FileListData,
}

impl FileListSection {
    /// Create a staged files section.
    #[must_use]
    pub fn staged() -> Self {
        Self {
            config: FileListConfig::STAGED,
            data: FileListData::default(),
        }
    }

    /// Create a working directory files section.
    #[must_use]
    pub fn working() -> Self {
        Self {
            config: FileListConfig::WORKING,
            data: FileListData::default(),
        }
    }

    /// Update section data from app state.
    pub fn update(&mut self, data: FileListData) {
        self.data = data;
    }

    /// Render context hints for file actions.
    fn render_hints(&self) -> Line<'static> {
        let Some(ref registry) = self.data.action_registry else {
            return Line::from("");
        };
        let Some(ref state) = self.data.app_state else {
            return Line::from("");
        };

        let actions = registry.hint_actions_for_context(self.config.context, state);

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

impl Default for FileListSection {
    fn default() -> Self {
        Self::staged()
    }
}

impl Section for FileListSection {
    fn id(&self) -> SectionId {
        self.config.id
    }

    fn name(&self) -> &'static str {
        self.config.name
    }

    fn contexts(&self) -> Vec<Context> {
        vec![self.config.context]
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
            format!(
                "  {arrow} {} ({file_count})",
                self.config.name
            ),
            Style::default()
                .fg(self.config.header_color)
                .add_modifier(Modifier::BOLD),
        )));

        // Hints (only when focused)
        if in_section {
            lines.push(self.render_hints());
        }

        // File entries using shared renderer
        let style = FileListStyle {
            indicator: Some(self.config.indicator),
            ..Default::default()
        };

        for (i, file) in self.data.files.iter().enumerate() {
            let selected = state.local_selection == Some(i)
                && state.is_focused
                && !self.data.command_mode_active;

            let path = file.path.to_string_lossy();
            let entry = match self.config.field_selector {
                FieldSelector::Staged => FileEntryView {
                    path: &path,
                    status: file.staged,
                    insertions: file.staged_insertions,
                    deletions: file.staged_deletions,
                },
                FieldSelector::Working => FileEntryView {
                    path: &path,
                    status: file.working,
                    insertions: file.working_insertions,
                    deletions: file.working_deletions,
                },
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
            KeyCode::Char(c) if self.config.stage_keys.contains(&c) => Some(
                SectionAction::AppAction(AppAction::ToggleStage),
            ),
            KeyCode::Char('d') => {
                // Show inline diff
                Some(SectionAction::AppAction(
                    AppAction::ShowDiff,
                ))
            }
            _ => None,
        }
    }
}
