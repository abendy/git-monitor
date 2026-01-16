//! History section (commits/reflog).
//!
//! Displays commit history or reflog entries with contextual actions.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
};

use crate::{
    actions::{ActionRegistry, ActionType, AppAction, AppState, Context},
    app::HistoryMode,
    git::{
        format_relative_time, BranchInfo, CommandType, CommitDetail, FileState, GitCommand,
        GitStatus, RefDecoration,
    },
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
        if let (Some(ref registry), Some(ref state)) =
            (&self.data.action_registry, &self.data.app_state)
        {
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
                spans.push(Span::styled("  · ", Style::default().fg(Color::DarkGray)));
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
                        Style::default().fg(Color::DarkGray).italic(),
                    ));
                }
            }
        }

        Some(Line::from(spans))
    }

    /// Render a single commit line
    #[allow(clippy::unused_self)]
    fn render_commit_line(
        &self,
        cmd: &GitCommand,
        selected: bool,
        is_last: bool,
        render_width: u16,
    ) -> Line<'static> {
        let selection_prefix = if selected { "▸" } else { " " };
        // Simple style: just vertical line for history
        // Remote-only commits use a branch-off indicator
        let graph_char = if cmd.is_remote_only {
            "├—"
        } else if is_last {
            "╵"
        } else {
            "│"
        };

        let time_str = format_relative_time(cmd.timestamp);
        let icon = cmd.command_type.icon();
        let color = command_color(cmd.command_type);
        let sha_str = cmd.sha.as_deref().unwrap_or("-------");

        // Build decoration spans
        let decoration_spans = format_decorations(&cmd.decorations);
        let decoration_width: usize = decoration_spans.iter().map(|s| s.content.len()).sum();

        // Truncate message if too long
        let base_width = 35 + decoration_width;
        let max_msg_len = render_width.saturating_sub(base_width as u16) as usize;
        let message = if cmd.message.len() > max_msg_len && max_msg_len > 3 {
            format!("{}...", &cmd.message[..max_msg_len.saturating_sub(3)])
        } else if max_msg_len <= 3 {
            String::new()
        } else {
            cmd.message.clone()
        };

        let style = if selected {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        // Remote-only commits use dimmer colors and red graph indicator
        let (graph_color, sha_color, msg_color) = if cmd.is_remote_only {
            (Color::Red, Color::Red, Color::DarkGray)
        } else {
            (Color::DarkGray, Color::Yellow, Color::White)
        };

        let mut spans = vec![
            Span::raw(format!("{selection_prefix} ")),
            Span::styled(format!("{graph_char} "), Style::default().fg(graph_color)),
            Span::styled(format!("{sha_str} "), style.fg(sha_color)),
            Span::styled(format!("{time_str}  "), style.fg(Color::DarkGray)),
            Span::styled(format!("{icon} "), style.fg(color)),
        ];

        // Check for special commit prefixes (fixup!, squash!, amend!, wip)
        let special_prefixes = ["fixup!", "squash!", "amend!"];
        let wip_prefixes = ["wip:", "wip ", "WIP:", "WIP "];

        if let Some(prefix) = special_prefixes.iter().find(|p| message.starts_with(*p)) {
            spans.push(Span::styled(
                prefix.to_string(),
                style.fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                message[prefix.len()..].to_string(),
                style.fg(msg_color),
            ));
        } else if let Some(prefix) = wip_prefixes.iter().find(|p| message.starts_with(*p)) {
            spans.push(Span::styled(
                prefix.to_string(),
                style.fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                message[prefix.len()..].to_string(),
                style.fg(msg_color),
            ));
        } else {
            spans.push(Span::styled(message, style.fg(msg_color)));
        }

        // Add decorations if any
        if !decoration_spans.is_empty() {
            spans.push(Span::raw(" "));
            spans.extend(decoration_spans);
        }

        Line::from(spans)
    }

    /// Render expanded commit detail lines
    fn render_commit_detail(&self, detail: &CommitDetail) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Empty line
        lines.push(Line::raw(""));

        // Author
        lines.push(Line::from(vec![
            Span::raw("    Author:    "),
            Span::styled(
                detail.author_name.clone(),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" <"),
            Span::styled(
                detail.author_email.clone(),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(">"),
        ]));

        // Committer (if different from author)
        if detail.committer_name != detail.author_name
            || detail.committer_email != detail.author_email
        {
            lines.push(Line::from(vec![
                Span::raw("    Committer: "),
                Span::styled(
                    detail.committer_name.clone(),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(" <"),
                Span::styled(
                    detail.committer_email.clone(),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(">"),
            ]));
        }

        // Date
        lines.push(Line::from(vec![
            Span::raw("    Date:      "),
            Span::styled(
                detail
                    .author_time
                    .format("%Y-%m-%d %H:%M:%S %z")
                    .to_string(),
                Style::default().fg(Color::Yellow),
            ),
        ]));

        // Full SHA
        lines.push(Line::from(vec![
            Span::raw("    Commit:    "),
            Span::styled(detail.full_sha.clone(), Style::default().fg(Color::Yellow)),
        ]));

        // GPG info (if present)
        if let Some(gpg) = &detail.gpg_status {
            lines.push(Line::from(vec![
                Span::raw("    GPG:       "),
                Span::styled(gpg.clone(), Style::default().fg(Color::Magenta)),
            ]));
        }

        // Empty line before message
        lines.push(Line::raw(""));

        // Commit message (indented, may be multi-line)
        for msg_line in detail.message.lines() {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::raw(msg_line.to_string()),
            ]));
        }

        // Files section with stats
        if !detail.files.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    {} file(s) changed  ", detail.files.len()),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("+{}", detail.insertions),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(" / ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("-{}", detail.deletions),
                    Style::default().fg(Color::Red),
                ),
            ]));

            // Hint for commit file actions (only show when a file is selected)
            if self.data.expanded_file_idx.is_some() {
                if let (Some(ref registry), Some(ref state)) =
                    (&self.data.action_registry, &self.data.app_state)
                {
                    let hint = render_context_hint(registry, Context::CommitFiles, state);
                    let mut spans = vec![Span::raw("  ")]; // Extra indent
                    spans.extend(hint.spans);
                    lines.push(Line::from(spans));
                }
            }

            for (file_idx, file) in detail.files.iter().enumerate() {
                let is_file_selected = self.data.expanded_file_idx == Some(file_idx);
                let (status_char, color) = match file.status {
                    FileState::Added => ('A', Color::Green),
                    FileState::Modified => ('M', Color::Yellow),
                    FileState::Deleted => ('D', Color::Red),
                    FileState::Renamed => ('R', Color::Cyan),
                    _ => ('?', Color::White),
                };

                let file_prefix = if is_file_selected { "  ▸ " } else { "    " };
                let file_style = if is_file_selected {
                    Style::default().add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let mut spans = vec![
                    Span::styled(file_prefix.to_string(), file_style),
                    Span::styled(format!("{status_char}"), file_style.fg(color)),
                    Span::raw("  "),
                    Span::styled(file.path.clone(), file_style),
                ];

                // Add per-file stats if there are changes
                if file.insertions > 0 || file.deletions > 0 {
                    spans.push(Span::raw("  "));
                    if file.insertions > 0 {
                        spans.push(Span::styled(
                            format!("+{}", file.insertions),
                            file_style.fg(Color::Green),
                        ));
                    }
                    if file.insertions > 0 && file.deletions > 0 {
                        spans.push(Span::raw("/"));
                    }
                    if file.deletions > 0 {
                        spans.push(Span::styled(
                            format!("-{}", file.deletions),
                            file_style.fg(Color::Red),
                        ));
                    }
                }

                lines.push(Line::from(spans));
            }
        }

        // Separator line
        lines.push(Line::raw(""));

        lines
    }
}

/// Get color for command type
const fn command_color(cmd: CommandType) -> Color {
    match cmd {
        CommandType::Commit => Color::Green,
        CommandType::Checkout => Color::Cyan,
        CommandType::Merge => Color::Magenta,
        CommandType::Rebase => Color::Yellow,
        CommandType::Pull => Color::Blue,
        CommandType::Push => Color::Blue,
        CommandType::Reset => Color::Red,
        CommandType::CherryPick => Color::Magenta,
        CommandType::Revert => Color::Red,
        CommandType::Branch => Color::Cyan,
        CommandType::Clone | CommandType::Init => Color::Green,
        CommandType::Fetch => Color::Blue,
        CommandType::Stash => Color::Yellow,
        CommandType::Other => Color::DarkGray,
    }
}

/// Format decorations (branches, tags) into styled spans
fn format_decorations(decorations: &[RefDecoration]) -> Vec<Span<'static>> {
    if decorations.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::new();
    spans.push(Span::styled("(", Style::default().fg(Color::DarkGray)));

    let mut first = true;
    let mut has_head = false;
    let mut head_branch: Option<&str> = None;

    // Check for HEAD and find its branch
    for dec in decorations {
        if matches!(dec, RefDecoration::Head) {
            has_head = true;
        }
    }

    // Find local branch that HEAD points to
    if has_head {
        for dec in decorations {
            if let RefDecoration::LocalBranch(name) = dec {
                head_branch = Some(name);
                break;
            }
        }
    }

    // Format: HEAD → branch for the HEAD + branch combo
    if has_head {
        if let Some(branch) = head_branch {
            spans.push(Span::styled(
                "HEAD → ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                branch.to_string(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
            first = false;
        } else {
            spans.push(Span::styled(
                "HEAD",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            first = false;
        }
    }

    // Add remaining decorations
    for dec in decorations {
        // Skip HEAD (already handled) and the branch HEAD points to
        if matches!(dec, RefDecoration::Head) {
            continue;
        }
        if let RefDecoration::LocalBranch(name) = dec {
            if head_branch == Some(name) {
                continue;
            }
        }

        if !first {
            spans.push(Span::styled(", ", Style::default().fg(Color::DarkGray)));
        }
        first = false;

        match dec {
            RefDecoration::LocalBranch(name) => {
                spans.push(Span::styled(
                    name.clone(),
                    Style::default().fg(Color::Green),
                ));
            }
            RefDecoration::RemoteBranch(name) => {
                spans.push(Span::styled(name.clone(), Style::default().fg(Color::Red)));
            }
            RefDecoration::Tag(name) => {
                spans.push(Span::styled("tag: ", Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(
                    name.clone(),
                    Style::default().fg(Color::Yellow),
                ));
            }
            RefDecoration::Head => {} // Already handled
        }
    }

    spans.push(Span::styled(")", Style::default().fg(Color::DarkGray)));
    spans
}

/// Render context-specific hints as a Line
fn render_context_hint(
    registry: &ActionRegistry,
    context: Context,
    state: &AppState,
) -> Line<'static> {
    let actions = registry.hint_actions_for_context(context, state);

    if actions.is_empty() {
        return Line::from("");
    }

    let mut spans = vec![Span::raw("  ")];

    for (i, action) in actions.iter().take(5).enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(Span::styled(
            format!("{} ", action.key),
            Style::default().fg(Color::Cyan),
        ));
        spans.push(Span::styled(
            action.label.clone(),
            Style::default().fg(Color::DarkGray).italic(),
        ));
    }

    Line::from(spans)
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
        if in_section && state.local_selection.is_some_and(|s| s > 0) {
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
            lines.push(self.render_commit_line(cmd, selected, is_last, state.render_width));

            // If this commit is expanded, render detail lines
            if self.data.expanded_commit.as_ref() == cmd.sha.as_ref() {
                if let Some(ref detail) = self.data.expanded_detail {
                    lines.extend(self.render_commit_detail(detail));
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
