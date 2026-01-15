//! History section (commits/reflog).
//!
//! Displays commit history or reflog entries with contextual actions.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::{
    actions::{ActionRegistry, AppAction, AppState, Context},
    app::HistoryMode,
    git::{BranchInfo, CommitDetail, GitCommand, GitStatus},
};

use super::{Section, SectionAction, SectionId, SectionState};

/// Data needed by the history section for rendering
#[derive(Debug, Clone, Default)]
pub struct HistorySectionData {
    /// Commit history entries
    pub activity: Vec<GitCommand>,
    /// Current history mode (reflog or commit log)
    pub history_mode: HistoryMode,
    /// Whether section is collapsed
    pub is_collapsed: bool,
    /// Current page number
    pub page: usize,
    /// Current branch info (for header display)
    pub current_branch: Option<BranchInfo>,
    /// Repository status (ahead/behind/upstream)
    pub status: GitStatus,
    /// Currently expanded commit SHA (if any)
    pub expanded_commit: Option<String>,
    /// Detail for expanded commit
    pub expanded_detail: Option<CommitDetail>,
    /// Whether command mode is active (suppresses selection highlight)
    pub command_mode_active: bool,
    /// Action registry for hints
    pub action_registry: Option<ActionRegistry>,
    /// App state for action conditions
    pub app_state: Option<AppState>,
}

/// History section (commits/reflog)
pub struct HistorySection {
    data: HistorySectionData,
}

impl HistorySection {
    /// Create a new history section
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: HistorySectionData::default(),
        }
    }

    /// Update section data from app state
    pub fn update(&mut self, data: HistorySectionData) {
        self.data = data;
    }

    /// Get the history mode label
    fn mode_label(&self) -> &'static str {
        match self.data.history_mode {
            HistoryMode::Reflog => "Reflog",
            HistoryMode::CommitLog => "History",
        }
    }

    /// Render context hints for history actions
    fn render_hints(&self) -> Line<'static> {
        let Some(ref registry) = self.data.action_registry else {
            return Line::from("");
        };
        let Some(ref state) = self.data.app_state else {
            return Line::from("");
        };

        let actions = registry.hint_actions_for_context(Context::HistoryCommits, state);

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

    /// Render the current branch info line
    fn render_branch_info(&self) -> Option<Line<'static>> {
        let branch = self.data.current_branch.as_ref()?;

        let mut spans = vec![Span::styled(
            format!("  {}", branch.name),
            Style::default().fg(Color::Cyan),
        )];

        // Add ahead/behind indicators
        if self.data.status.ahead > 0 || self.data.status.behind > 0 {
            spans.push(Span::raw(" "));
            if self.data.status.ahead > 0 {
                spans.push(Span::styled(
                    format!("↑{}", self.data.status.ahead),
                    Style::default().fg(Color::Green),
                ));
            }
            if self.data.status.behind > 0 {
                spans.push(Span::styled(
                    format!("↓{}", self.data.status.behind),
                    Style::default().fg(Color::Red),
                ));
            }
        }

        // Show upstream branch name
        if let Some(upstream) = &self.data.status.upstream {
            spans.push(Span::styled(
                format!(" → {upstream}"),
                Style::default().fg(Color::DarkGray),
            ));
        }

        Some(Line::from(spans))
    }
}

impl Default for HistorySection {
    fn default() -> Self {
        Self::new()
    }
}

impl Section for HistorySection {
    fn id(&self) -> SectionId {
        SectionId::History
    }

    fn name(&self) -> &str {
        self.mode_label()
    }

    fn contexts(&self) -> Vec<Context> {
        vec![Context::HistoryHeader, Context::HistoryCommits]
    }

    fn item_count(&self) -> usize {
        if self.data.is_collapsed {
            // Header only when collapsed
            1
        } else {
            // Header + commits
            1 + self.data.activity.len()
        }
    }

    fn is_collapsible(&self) -> bool {
        true
    }

    fn is_collapsed(&self) -> bool {
        self.data.is_collapsed
    }

    fn render(&self, state: &SectionState) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Skip if no activity and collapsed
        if self.data.activity.is_empty() && self.data.is_collapsed {
            return lines;
        }

        let activity_len = self.data.activity.len();
        let in_section = state.is_focused && !self.data.command_mode_active;
        let collapse_indicator = if self.data.is_collapsed { "▸" } else { "▾" };

        // Header is always at local index 0
        let header_selected = state.local_selection == Some(0) && in_section;
        let header_prefix = if header_selected { "▸ " } else { "  " };

        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);

        let mut header_spans = vec![
            Span::raw(header_prefix),
            Span::styled(format!("{collapse_indicator} "), header_style),
            Span::styled(format!("{} ({activity_len})", self.mode_label()), header_style),
        ];

        // Page indicator
        if self.data.page > 0 {
            header_spans.push(Span::styled(
                format!(" [page {}]", self.data.page + 1),
                Style::default().fg(Color::DarkGray),
            ));
        }

        lines.push(Line::from(header_spans));

        // If collapsed, stop here
        if self.data.is_collapsed {
            return lines;
        }

        // Hints (only when focused and not on header)
        if in_section && state.local_selection.is_some_and(|s| s > 0) {
            lines.push(self.render_hints());
        }

        // Current branch info
        if let Some(branch_line) = self.render_branch_info() {
            lines.push(branch_line);
        }

        // Commit entries - rendering delegated to ui.rs for now
        // The section tracks the data, ui.rs handles the actual commit line rendering
        // This is a placeholder that will be replaced when render/ module migration happens

        lines
    }

    fn actions(&self, item_idx: usize) -> Vec<crate::actions::Action> {
        // Actions are provided by ActionRegistry based on context
        let _ = item_idx;
        Vec::new()
    }

    fn handle_key(&self, key: KeyEvent, item_idx: usize) -> Option<SectionAction> {
        // Header (index 0) has different actions than commits
        if item_idx == 0 {
            match key.code {
                KeyCode::Char('h') => {
                    // Toggle history mode (log/reflog)
                    Some(SectionAction::AppAction(AppAction::ToggleHistoryMode))
                }
                // Enter/Space to toggle collapse handled imperatively in app.rs
                _ => None,
            }
        } else {
            // Commit actions
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Expand/collapse commit detail
                    Some(SectionAction::AppAction(AppAction::ExpandCommit))
                }
                KeyCode::Char('c') => {
                    // Checkout commit
                    Some(SectionAction::AppAction(AppAction::Checkout))
                }
                KeyCode::Char('y') => {
                    // Copy short SHA
                    Some(SectionAction::AppAction(AppAction::CopyShortSha))
                }
                KeyCode::Char('R') => {
                    // Interactive rebase
                    Some(SectionAction::AppAction(AppAction::InteractiveRebase))
                }
                _ => None,
            }
        }
    }
}
