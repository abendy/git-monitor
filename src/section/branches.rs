//! Branches section.
//!
//! Displays non-current branches with expandable commit history.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};

use super::{Section, SectionAction, SectionId, SectionKeybinding, SectionState};
use crate::actions::{ActionRegistry, AppAction, AppState, Context};
use crate::git::{
    format_relative_time, BranchInfo, CommandType, CommitDetail, FileState, GitCommand,
};
use crate::input::KeyBinding;

/// Data needed by the branches section for rendering
#[derive(Debug, Clone, Default)]
pub struct BranchesSectionData {
    /// All branches (will filter out current branch for display)
    pub branches: Vec<BranchInfo>,
    /// Currently expanded branch name (if any)
    pub expanded_branch: Option<String>,
    /// Commits for the expanded branch
    pub expanded_branch_commits: Vec<GitCommand>,
    /// Currently expanded commit SHA within branch (if any)
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

/// Branches section
pub struct BranchesSection {
    data: BranchesSectionData,
}

impl BranchesSection {
    /// Create a new branches section
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: BranchesSectionData::default(),
        }
    }

    /// Update section data from app state
    pub fn update(&mut self, data: BranchesSectionData) {
        self.data = data;
    }

    /// Get branches to display (excludes current branch)
    fn other_branches(&self) -> Vec<&BranchInfo> {
        self.data
            .branches
            .iter()
            .filter(|b| !b.is_current)
            .collect()
    }

    /// Find the index of the expanded branch within the other-branches list.
    fn expanded_branch_index(&self, branches: &[&BranchInfo]) -> Option<usize> {
        let expanded = self.data.expanded_branch.as_deref()?;
        branches.iter().position(|b| b.name == expanded)
    }

    /// Render context hints for branch actions
    fn render_hints(&self, context: Context) -> Line<'static> {
        let Some(ref registry) = self.data.action_registry else {
            return Line::from("");
        };
        let Some(ref state) = self.data.app_state else {
            return Line::from("");
        };

        let actions = registry.hint_actions_for_context(context, state);

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

    /// Render a single commit line with tree-style graph chars
    #[allow(clippy::unused_self)]
    fn render_commit_line(
        &self,
        cmd: &GitCommand,
        selected: bool,
        is_last: bool,
        render_width: u16,
    ) -> Line<'static> {
        let selection_prefix = if selected { "▸" } else { " " };
        // Tree style: indent + tree chars to show nesting under branch header
        // Remote-only commits use a branch-off indicator
        let (indent, graph_char) = if cmd.is_remote_only {
            ("", "├—")
        } else {
            ("   ", if is_last { "└─" } else { "├─" })
        };

        let time_str = format_relative_time(cmd.timestamp);
        let icon = cmd.command_type.icon();
        let color = command_color(cmd.command_type);
        let sha_str = cmd.sha.as_deref().unwrap_or("-------");

        // Truncate message if too long (account for sha, time, graph, indent)
        // Tree style uses 3-char indent + 2-char graph (├─/└─)
        let extra_width = 4;
        let base_width = 35 + extra_width;
        let max_msg_len = render_width.saturating_sub(base_width as u16) as usize;
        let message = if cmd.message.len() > max_msg_len && max_msg_len > 3 {
            format!(
                "{}...",
                &cmd.message[..max_msg_len.saturating_sub(3)]
            )
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
            (
                Color::DarkGray,
                Color::Yellow,
                Color::White,
            )
        };

        let mut spans = vec![
            Span::raw(format!("{selection_prefix} {indent}")),
            Span::styled(
                format!("{graph_char} "),
                Style::default().fg(graph_color),
            ),
            Span::styled(
                format!("{sha_str} "),
                style.fg(sha_color),
            ),
            Span::styled(
                format!("{time_str}  "),
                style.fg(Color::DarkGray),
            ),
            Span::styled(format!("{icon} "), style.fg(color)),
        ];

        // Check for special commit prefixes (fixup!, squash!, amend!, wip)
        let special_prefixes = ["fixup!", "squash!", "amend!"];
        let wip_prefixes = ["wip:", "wip ", "WIP:", "WIP "];

        if let Some(prefix) = special_prefixes
            .iter()
            .find(|p| message.starts_with(*p))
        {
            spans.push(Span::styled(
                prefix.to_string(),
                style
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                message[prefix.len()..].to_string(),
                style.fg(msg_color),
            ));
        } else if let Some(prefix) = wip_prefixes
            .iter()
            .find(|p| message.starts_with(*p))
        {
            spans.push(Span::styled(
                prefix.to_string(),
                style
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                message[prefix.len()..].to_string(),
                style.fg(msg_color),
            ));
        } else {
            spans.push(Span::styled(
                message,
                style.fg(msg_color),
            ));
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
            Span::styled(
                detail.full_sha.clone(),
                Style::default().fg(Color::Yellow),
            ),
        ]));

        // GPG info (if present)
        if let Some(gpg) = &detail.gpg_status {
            lines.push(Line::from(vec![
                Span::raw("    GPG:       "),
                Span::styled(
                    gpg.clone(),
                    Style::default().fg(Color::Magenta),
                ),
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
                    format!(
                        "    {} file(s) changed  ",
                        detail.files.len()
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("+{}", detail.insertions),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    " / ",
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("-{}", detail.deletions),
                    Style::default().fg(Color::Red),
                ),
            ]));

            // Hint for commit file actions (only show when a file is selected)
            if self.data.expanded_file_idx.is_some() {
                if let (Some(ref registry), Some(ref state)) = (
                    &self.data.action_registry,
                    &self.data.app_state,
                ) {
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
                    Span::styled(
                        format!("{status_char}"),
                        file_style.fg(color),
                    ),
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
            Style::default()
                .fg(Color::DarkGray)
                .italic(),
        ));
    }

    Line::from(spans)
}

impl Default for BranchesSection {
    fn default() -> Self {
        Self::new()
    }
}

impl Section for BranchesSection {
    fn id(&self) -> SectionId {
        SectionId::Branches
    }

    fn name(&self) -> &str {
        "Branches"
    }

    fn contexts(&self) -> Vec<Context> {
        vec![Context::BranchHeader, Context::BranchCommits]
    }

    fn item_count(&self) -> usize {
        // Branches section items = branch headers + expanded branch commits.
        let branches = self.other_branches();
        let header_count = branches.len();
        if header_count == 0 {
            return 0;
        }

        let commit_count = if self.expanded_branch_index(&branches).is_some() {
            self.data.expanded_branch_commits.len()
        } else {
            0
        };

        header_count + commit_count
    }

    fn render(&self, state: &SectionState) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        let other_branches = self.other_branches();
        if other_branches.is_empty() {
            return lines;
        }

        let in_section = state.is_focused && !self.data.command_mode_active;
        let arrow = if in_section { "▾" } else { "▸" };

        // Section header (not selectable)
        lines.push(Line::from(Span::styled(
            format!(
                "  {arrow} Branches ({})",
                other_branches.len()
            ),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));

        let expanded_idx = self.expanded_branch_index(&other_branches);
        let commit_len = if expanded_idx.is_some() {
            self.data.expanded_branch_commits.len()
        } else {
            0
        };
        let commit_start = expanded_idx.map(|idx| idx + 1);

        // Hints (only when focused)
        if in_section {
            let hint_context = state.local_selection.map_or(Context::BranchHeader, |local_index| {
                if let Some(expanded_idx) = expanded_idx {
                    let commit_start = expanded_idx + 1;
                    let commit_end = commit_start + commit_len;
                    if (commit_start..commit_end).contains(&local_index) {
                        Context::BranchCommits
                    } else {
                        Context::BranchHeader
                    }
                } else {
                    Context::BranchHeader
                }
            });
            lines.push(self.render_hints(hint_context));
        }

        // Branch entries - branch names shown, commits under expanded branch
        for (branch_idx, branch) in other_branches.iter().enumerate() {
            let header_local_index = if let Some(expanded_idx) = expanded_idx {
                if branch_idx > expanded_idx {
                    branch_idx + commit_len
                } else {
                    branch_idx
                }
            } else {
                branch_idx
            };
            let header_selected = state.local_selection == Some(header_local_index) && in_section;
            let prefix = if header_selected { "▸ " } else { "  " };
            let is_expanded = self.data.expanded_branch.as_deref() == Some(&branch.name);

            // Branch name color: remote=red, local=white
            let branch_color = if branch.is_remote {
                Color::Red
            } else {
                Color::White
            };

            let branch_style = if header_selected {
                Style::default()
                    .fg(branch_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(branch_color)
            };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(branch.name.clone(), branch_style),
            ]));

            // If this branch is expanded, show its commits
            if is_expanded {
                let Some(commit_start) = commit_start else {
                    continue;
                };
                for (i, cmd) in self
                    .data
                    .expanded_branch_commits
                    .iter()
                    .enumerate()
                {
                    let local_index = commit_start + i;
                    let selected = state.local_selection == Some(local_index) && in_section;
                    let is_last = i + 1 == commit_len;
                    lines.push(self.render_commit_line(
                        cmd,
                        selected,
                        is_last,
                        state.render_width,
                    ));

                    // If this commit is expanded, render detail lines
                    if self.data.expanded_commit.as_ref() == cmd.sha.as_ref() {
                        if let Some(ref detail) = self.data.expanded_detail {
                            lines.extend(self.render_commit_detail(detail));
                        }
                    }
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
        // item_idx maps to expanded_branch_commits
        if item_idx >= self.data.expanded_branch_commits.len() {
            return None;
        }

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
            _ => None,
        }
    }

    fn keybindings(&self) -> Vec<SectionKeybinding> {
        vec![
            SectionKeybinding::new(
                KeyBinding::char('e'),
                "Expand",
                "Expand/collapse branch commits",
            ),
            SectionKeybinding::new(
                KeyBinding::key(KeyCode::Enter),
                "Checkout",
                "Checkout branch or commit",
            ),
            SectionKeybinding::new(
                KeyBinding::char('y'),
                "Copy SHA",
                "Copy commit SHA",
            ),
        ]
    }
}
