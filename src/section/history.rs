//! History section (commits/reflog).
//!
//! Displays commit history or reflog entries with contextual actions.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};

use super::{Section, SectionAction, SectionId, SectionKeybinding, SectionState};
use crate::actions::{ActionRegistry, ActionType, AppAction, AppState, Context};
use crate::app::HistoryMode;
use crate::git::{
    BranchInfo, CommitDetail, GitCommand, GitStatus,
};
use crate::input::KeyBinding;
use crate::section::{
    render_commit_detail, render_commit_line, render_context_hint,
};

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
    /// Index of selected file in expanded commit detail
    pub expanded_file_idx: Option<usize>,
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

        render_context_hint(registry, Context::HistoryCommits, state)
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

    /// Render the branch info line with remote action hints
    fn render_branch_info_with_hints(&self) -> Option<Line<'static>> {
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

        // Dynamic hints from action registry for remote actions
        if let (Some(ref registry), Some(ref state)) = (
            &self.data.action_registry,
            &self.data.app_state,
        ) {
            let remote_actions: Vec<_> = registry
                .actions_for_context(Context::Global, state)
                .into_iter()
                .filter(|a| {
                    matches!(
                        a.action_type,
                        ActionType::App(AppAction::Push | AppAction::Pull | AppAction::Fetch)
                    )
                })
                .collect();

            if !remote_actions.is_empty() {
                spans.push(Span::styled(
                    "  · ",
                    Style::default().fg(Color::DarkGray),
                ));
                for (i, action) in remote_actions.iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::styled("  ", Style::default()));
                    }
                    spans.push(Span::styled(
                        format!("{} ", action.key),
                        Style::default().fg(Color::Cyan),
                    ));
                    spans.push(Span::styled(
                        action.label.clone(),
                        Style::default()
                            .fg(Color::DarkGray)
                            .italic(),
                    ));
                }
            }
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
            Span::styled(
                format!("{collapse_indicator} "),
                header_style,
            ),
            Span::styled(
                format!("{} ({activity_len})", self.mode_label()),
                header_style,
            ),
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
        if in_section
            && state
                .local_selection
                .is_some_and(|s| s > 0)
        {
            lines.push(self.render_hints());
        }

        // Current branch info with remote action hints
        if let Some(branch_line) = self.render_branch_info_with_hints() {
            lines.push(branch_line);
        }

        // Commit lines (local indices 1..=activity_len map to commits 0..activity_len)
        for (i, cmd) in self.data.activity.iter().enumerate() {
            // Local index for this commit is i + 1 (header is at 0)
            let selected = state.local_selection == Some(i + 1) && in_section;
            let is_last = i == activity_len - 1;
            lines.push(render_commit_line(
                cmd,
                selected,
                is_last,
                state.render_width,
            ));

            // If this commit is expanded, render detail lines
            if self.data.expanded_commit.as_ref() == cmd.sha.as_ref() {
                if let Some(ref detail) = self.data.expanded_detail {
                    lines.extend(render_commit_detail(
                        detail,
                        self.data.expanded_file_idx,
                        self.data.action_registry.as_ref(),
                        self.data.app_state.as_ref(),
                    ));
                }
            }
        }

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
                    Some(SectionAction::AppAction(
                        AppAction::ToggleHistoryMode,
                    ))
                }
                // Enter/Space to toggle collapse handled imperatively in app.rs
                _ => None,
            }
        } else {
            // Commit actions
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    // Expand/collapse commit detail
                    Some(SectionAction::AppAction(
                        AppAction::ExpandCommit,
                    ))
                }
                KeyCode::Char('c') => {
                    // Checkout commit
                    Some(SectionAction::AppAction(
                        AppAction::Checkout,
                    ))
                }
                KeyCode::Char('y') => {
                    // Copy short SHA
                    Some(SectionAction::AppAction(
                        AppAction::CopyShortSha,
                    ))
                }
                KeyCode::Char('R') => {
                    // Interactive rebase
                    Some(SectionAction::AppAction(
                        AppAction::InteractiveRebase,
                    ))
                }
                _ => None,
            }
        }
    }

    fn keybindings(&self) -> Vec<SectionKeybinding> {
        vec![
            SectionKeybinding::new(
                KeyBinding::char('h'),
                "Toggle",
                "Toggle between commit log and reflog",
            ),
            SectionKeybinding::new(
                KeyBinding::char(' '),
                "Expand",
                "Expand/collapse commit details",
            ),
            SectionKeybinding::new(
                KeyBinding::char('y'),
                "Copy SHA",
                "Copy short commit SHA",
            ),
            SectionKeybinding::new(
                KeyBinding::char('c'),
                "Copy SHA",
                "Copy commit SHA",
            ),
            SectionKeybinding::new(
                KeyBinding::char('['),
                "Prev Page",
                "Previous page of history",
            ),
            SectionKeybinding::new(
                KeyBinding::char(']'),
                "Next Page",
                "Next page of history",
            ),
            SectionKeybinding::new(
                KeyBinding::char('R'),
                "Rebase",
                "Interactive rebase onto commit",
            ),
        ]
    }
}
