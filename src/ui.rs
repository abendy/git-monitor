use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::{
    actions::{ActionRegistry, ActionType, AppAction, AppState, Context},
    app::{App, HistoryMode, PopupContent, ViewMode, ConfirmAction},
    git::{format_relative_time, CommandType, FileState, RefDecoration},
    tui::Frame,
};

/// Render context-specific hints as a Line
fn render_context_hint(registry: &ActionRegistry, context: Context, state: &AppState) -> Line<'static> {
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

/// Main render function
pub fn render(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();

    // Main layout: header, body, footer
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Body
            Constraint::Length(3), // Footer
        ])
        .split(area);

    render_header(frame, app, layout[0]);

    // Conditional rendering: popup replaces body, not overlays it
    if app.popup.is_open() {
        render_popup(frame, app, layout[1]);
        render_popup_footer(frame, layout[2]);
    } else {
        render_body(frame, app, layout[1]);
        render_footer(frame, app, layout[2]);
    }

    // Help overlay (centered popup, separate from full-screen popup)
    if app.show_help {
        render_help(frame, area);
    }

    // Menu stack overlay (modular menu system)
    if app.menu_stack.is_active() {
        render_menu_stack(frame, app, area);
    }
}

/// Render the header bar
fn render_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let branch_name = app
        .status
        .branch
        .as_deref()
        .unwrap_or("(no branch)");

    let mut header_spans = vec![
        Span::styled(" ⎇ ", Style::default().fg(Color::Cyan)),
        Span::styled(branch_name, Style::default().fg(Color::Green).bold()),
    ];

    // Ahead/behind indicators
    if app.status.ahead > 0 || app.status.behind > 0 {
        header_spans.push(Span::raw(" "));
        if app.status.ahead > 0 {
            header_spans.push(Span::styled(
                format!("↑{}", app.status.ahead),
                Style::default().fg(Color::Green),
            ));
        }
        if app.status.behind > 0 {
            header_spans.push(Span::styled(
                format!("↓{}", app.status.behind),
                Style::default().fg(Color::Red),
            ));
        }
    }

    // Repo path
    let path_str = app
        .repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    header_spans.push(Span::raw(" │ "));
    header_spans.push(Span::styled(path_str, Style::default().fg(Color::White)));
    header_spans.push(Span::raw(" │ "));
    header_spans.push(Span::styled("● watching", Style::default().fg(Color::Green)));

    let header_text = Line::from(header_spans);

    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" git-monitor ")
            .title_style(Style::default().fg(Color::Cyan).bold()),
    );

    frame.render_widget(header, area);
}

/// Render the main body (single unified panel)
fn render_body(frame: &mut Frame<'_>, app: &App, area: Rect) {
    render_main_panel(frame, app, area);
}

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

/// Render the unified main panel (command + staged + working + activity)
fn render_main_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let staged_changes = app.status.staged_changes();
    let working_changes = app.status.working_changes();
    let staged_len = staged_changes.len();
    let working_len = working_changes.len();
    let files_total = staged_len + working_len;
    let activity_len = app.activity.len();

    let mut items: Vec<ListItem<'_>> = Vec::new();

    // Command section (always at top, index 0)
    let cmd_selected = app.selected == Some(0) && !app.menu_stack.is_active();
    let cmd_prefix = if cmd_selected { "▸ " } else { "  " };

    if app.is_command_mode() {
        // Active command input
        items.push(ListItem::new(Line::from(vec![
            Span::raw(cmd_prefix),
            Span::styled(": ", Style::default().fg(Color::Cyan).bold()),
            Span::styled(&app.command_input, Style::default().fg(Color::White)),
            Span::styled("_", Style::default().fg(Color::Cyan)), // Cursor
        ])));
    } else if cmd_selected {
        // Selected but not active - show hint
        items.push(ListItem::new(Line::from(vec![
            Span::raw(cmd_prefix),
            Span::styled(": ", Style::default().fg(Color::Cyan).bold()),
            Span::styled("type command   ", Style::default().fg(Color::DarkGray).italic()),
            Span::styled("a ", Style::default().fg(Color::Cyan)),
            Span::styled("aliases", Style::default().fg(Color::DarkGray).italic()),
        ])));
    } else {
        // Not selected or showing aliases
        items.push(ListItem::new(Line::from(vec![
            Span::raw(cmd_prefix),
            Span::styled(": ", Style::default().fg(Color::DarkGray)),
            Span::styled("command  ", Style::default().fg(Color::DarkGray)),
            Span::styled("a ", Style::default().fg(Color::Cyan)),
            Span::styled("aliases", Style::default().fg(Color::DarkGray)),
        ])));
    }

    // Command output (if any, and not when menu overlay is active)
    if !app.command_output.is_empty() && !app.menu_stack.is_active() {
        let output_color = if app.command_success {
            Color::White
        } else {
            Color::Red
        };

        for line in app.command_output.lines().take(4) {
            items.push(ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled("> ", Style::default().fg(Color::DarkGray)),
                Span::styled(line, Style::default().fg(output_color)),
            ])));
        }

        let line_count = app.command_output.lines().count();
        if line_count > 4 {
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("    ... ({} more lines) ", line_count - 4),
                    Style::default().fg(Color::DarkGray).italic(),
                ),
                Span::styled("o", Style::default().fg(Color::Cyan).bold()),
                Span::styled(" to expand", Style::default().fg(Color::DarkGray).italic()),
            ])));
        }
    }

    // Spacer after command section
    items.push(ListItem::new(Line::from("")));

    // Staged section
    if !staged_changes.is_empty() {
        // Check if selection is within staged section (indices 1 to staged_len)
        let in_staged = app
            .selected
            .is_some_and(|s| s >= 1 && s < 1 + staged_len && !app.is_command_mode());
        let arrow = if in_staged { "▾" } else { "▸" };
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  {arrow} Staged ({staged_len})"),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ))));

        // Hint for file actions (only show when in staged section)
        if in_staged {
            items.push(ListItem::new(render_context_hint(&app.action_registry, app.current_context(), &app.app_state())));
        }

        for (i, file) in staged_changes.iter().enumerate() {
            // Index 0 is command, so files start at index 1
            let global_idx = 1 + i;
            // Don't show selection when command mode is active (focus is on input)
            let selected = Some(global_idx) == app.selected && !app.is_command_mode();
            let prefix = if selected { "▸ " } else { "  " };
            let status_char = file.staged.as_char();
            let color = state_color(file.staged);
            let path = file.path.to_string_lossy();

            let style = if selected {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };

            items.push(ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled("● ", Style::default().fg(Color::Green)),
                Span::styled(format!("{status_char} "), style),
                Span::styled(path.to_string(), style),
            ])));
        }
    }

    // Working section
    if !working_changes.is_empty() {
        if !staged_changes.is_empty() {
            items.push(ListItem::new(Line::from("")));
        }

        // Check if selection is within working section
        let working_start = 1 + staged_len;
        let working_end = working_start + working_len;
        let in_working = app
            .selected
            .is_some_and(|s| s >= working_start && s < working_end && !app.is_command_mode());
        let arrow = if in_working { "▾" } else { "▸" };
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  {arrow} Working ({working_len})"),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))));

        // Hint for file actions (only show when in working section)
        if in_working {
            items.push(ListItem::new(render_context_hint(&app.action_registry, Context::WorkingFiles, &app.app_state())));
        }

        for (i, file) in working_changes.iter().enumerate() {
            // Index 0 is command, staged starts at 1, working starts at 1 + staged_len
            let global_idx = working_start + i;
            // Don't show selection when command mode is active (focus is on input)
            let selected = Some(global_idx) == app.selected && !app.is_command_mode();
            let prefix = if selected { "▸ " } else { "  " };
            let status_char = file.working.as_char();
            let color = state_color(file.working);
            let path = file.path.to_string_lossy();

            let style = if selected {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };

            items.push(ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled("○ ", Style::default().fg(Color::Yellow)),
                Span::styled(format!("{status_char} "), style),
                Span::styled(path.to_string(), style),
            ])));
        }
    }

    // Track current index for selection (after files)
    let mut current_idx = 1 + files_total;

    // History section (current branch commits) - always show header
    if !app.activity.is_empty() || !app.history_collapsed {
        if files_total > 0 {
            items.push(ListItem::new(Line::from("")));
        }

        let history_label = match app.history_mode {
            HistoryMode::Reflog => "Reflog",
            HistoryMode::CommitLog => "History",
        };

        // History header is selectable
        let header_selected = Some(current_idx) == app.selected && !app.is_command_mode();
        let collapse_indicator = if app.history_collapsed { "▸" } else { "▾" };
        let header_prefix = if header_selected { "▸ " } else { "  " };

        let header_style = if header_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        };

        // Build header with optional page indicator
        let mut header_spans = vec![
            Span::raw(header_prefix),
            Span::styled(format!("{collapse_indicator} "), header_style),
            Span::styled(format!("{history_label} ({activity_len})"), header_style),
        ];

        // Show page indicator if not on first page
        if app.history_page > 0 {
            header_spans.push(Span::styled(
                format!(" [page {}]", app.history_page + 1),
                Style::default().fg(Color::DarkGray),
            ));
        }

        items.push(ListItem::new(Line::from(header_spans)));
        current_idx += 1;

        // Only show commits if not collapsed
        if !app.history_collapsed {
            // Hint for history actions (only show when in history section)
            let in_history = matches!(app.current_context(), Context::HistoryCommits | Context::HistoryHeader);
            if in_history {
                items.push(ListItem::new(render_context_hint(&app.action_registry, Context::HistoryCommits, &app.app_state())));
            }

            // Show current branch name with upstream tracking info
            if let Some(current_branch) = app.branches.iter().find(|b| b.is_current) {
                let mut branch_spans = vec![Span::styled(
                    format!("  {}", current_branch.name),
                    Style::default().fg(Color::Cyan),
                )];

                // Add ahead/behind indicators if tracking upstream
                if app.status.ahead > 0 || app.status.behind > 0 {
                    branch_spans.push(Span::raw(" "));
                    if app.status.ahead > 0 {
                        branch_spans.push(Span::styled(
                            format!("↑{}", app.status.ahead),
                            Style::default().fg(Color::Green),
                        ));
                    }
                    if app.status.behind > 0 {
                        branch_spans.push(Span::styled(
                            format!("↓{}", app.status.behind),
                            Style::default().fg(Color::Red),
                        ));
                    }
                }

                // Show upstream branch name
                if let Some(upstream) = &app.status.upstream {
                    branch_spans.push(Span::styled(
                        format!(" → {}", upstream),
                        Style::default().fg(Color::DarkGray),
                    ));
                }

                // Dynamic hints from action registry for remote actions
                let state = app.app_state();
                let remote_actions: Vec<_> = app
                    .action_registry
                    .actions_for_context(Context::Global, &state)
                    .into_iter()
                    .filter(|a| matches!(
                        a.action_type,
                        ActionType::App(AppAction::Push | AppAction::Pull | AppAction::Fetch)
                    ))
                    .collect();

                if !remote_actions.is_empty() {
                    branch_spans.push(Span::styled("  · ", Style::default().fg(Color::DarkGray)));
                    for (i, action) in remote_actions.iter().enumerate() {
                        if i > 0 {
                            branch_spans.push(Span::styled("  ", Style::default()));
                        }
                        branch_spans.push(Span::styled(
                            format!("{} ", action.key),
                            Style::default().fg(Color::Cyan),
                        ));
                        branch_spans.push(Span::styled(
                            action.label.clone(),
                            Style::default().fg(Color::DarkGray).italic(),
                        ));
                    }
                }

                items.push(ListItem::new(Line::from(branch_spans)));
            }

            let activity_count = app.activity.len();
            for (i, cmd) in app.activity.iter().enumerate() {
                let selected = Some(current_idx) == app.selected && !app.is_command_mode();
                let is_last = i == activity_count - 1;
                items.push(render_commit_line(cmd, selected, is_last, area.width, false));
                current_idx += 1;

                // If this commit is expanded, render detail lines
                if app.expanded_commit.as_ref() == cmd.sha.as_ref() {
                    if let Some(detail) = &app.expanded_detail {
                        render_commit_detail(&mut items, app, detail);
                    }
                }
            }
        }
    }

    // Branches section (other branches, excluding current)
    let other_branches: Vec<_> = app.branches.iter().filter(|b| !b.is_current).collect();
    if !other_branches.is_empty() {
        // Add spacer if there's content above
        if files_total > 0 || !app.activity.is_empty() {
            items.push(ListItem::new(Line::from("")));
        }

        // Check if selection is within branches section
        let branches_start = current_idx;
        let in_branches = app
            .selected
            .is_some_and(|s| s >= branches_start && !app.is_command_mode());
        let arrow = if in_branches { "▾" } else { "▸" };
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  {arrow} Branches ({})", other_branches.len()),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ))));

        // Hint for branch actions (only show when in branches section)
        if in_branches {
            items.push(ListItem::new(render_context_hint(&app.action_registry, Context::BranchCommits, &app.app_state())));
        }

        for branch in other_branches.iter() {
            let is_expanded = app.expanded_branch.as_deref() == Some(&branch.name);

            // Branch names are not selectable - just labels
            // Remote branches shown in red, local in white
            let branch_color = if branch.is_remote {
                Color::Red
            } else {
                Color::White
            };
            items.push(ListItem::new(Line::from(Span::styled(
                format!("  {}", branch.name),
                Style::default().fg(branch_color),
            ))));

            // If this branch is expanded, show its commits
            if is_expanded {
                let branch_commit_count = app.expanded_branch_commits.len();
                for (i, cmd) in app.expanded_branch_commits.iter().enumerate() {
                    let selected = Some(current_idx) == app.selected && !app.is_command_mode();
                    let is_last = i == branch_commit_count - 1;
                    items.push(render_commit_line(cmd, selected, is_last, area.width, true));
                    current_idx += 1;

                    // If this commit is expanded, render detail lines
                    if app.expanded_commit.as_ref() == cmd.sha.as_ref() {
                        if let Some(detail) = &app.expanded_detail {
                            render_commit_detail(&mut items, app, detail);
                        }
                    }
                }
            }
        }
    }

    // Empty state for files (command section is always shown)
    if files_total == 0 && app.activity.is_empty() && app.branches.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  No changes or activity",
            Style::default().fg(Color::DarkGray).italic(),
        ))));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Repository ")
            .title_style(Style::default().fg(Color::Cyan).bold()),
    );

    frame.render_widget(list, area);
}

/// Render a single commit line (used for both History and branch commits)
/// `is_last` indicates if this is the last (oldest) commit in the section
/// `use_tree_style` uses tree connectors (├─/└─) for branch hierarchy vs simple (│/╵) for history
fn render_commit_line<'a>(
    cmd: &'a crate::git::GitCommand,
    selected: bool,
    is_last: bool,
    area_width: u16,
    use_tree_style: bool,
) -> ListItem<'a> {
    let selection_prefix = if selected { "▸" } else { " " };
    // Tree style: indent + tree chars to show nesting under branch header
    // Simple style: just vertical line for history
    // Remote-only commits use a branch-off indicator
    let (indent, graph_char) = if cmd.is_remote_only {
        ("", "├—")
    } else if use_tree_style {
        ("   ", if is_last { "└─" } else { "├─" })
    } else {
        ("", if is_last { "╵" } else { "│" })
    };

    let time_str = format_relative_time(cmd.timestamp);
    let icon = cmd.command_type.icon();
    let color = command_color(cmd.command_type);
    let sha_str = cmd.sha.as_deref().unwrap_or("-------");

    // Build decoration spans
    let decoration_spans = format_decorations(&cmd.decorations);
    let decoration_width: usize = decoration_spans.iter().map(|s| s.content.len()).sum();

    // Truncate message if too long (account for sha, time, decorations, graph, indent)
    // Tree style uses 3-char indent + 2-char graph (├─/└─), simple uses 1-char (│/╵)
    let extra_width = if use_tree_style { 4 } else { 0 };
    let base_width = 35 + decoration_width + extra_width;
    let max_msg_len = area_width.saturating_sub(base_width as u16) as usize;
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
        Span::raw(format!("{selection_prefix} {indent}")),
        Span::styled(format!("{graph_char} "), Style::default().fg(graph_color)),
        Span::styled(format!("{sha_str} "), style.fg(sha_color)),
        Span::styled(format!("{time_str}  "), style.fg(Color::DarkGray)),
        Span::styled(format!("{icon} "), style.fg(color)),
    ];

    // Check for special commit prefixes (fixup!, squash!, amend!, wip)
    // For remote-only commits, use dimmer colors throughout
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

    ListItem::new(Line::from(spans))
}

/// Render expanded commit detail (author, date, files, etc.)
fn render_commit_detail<'a>(
    items: &mut Vec<ListItem<'a>>,
    app: &'a App,
    detail: &'a crate::git::CommitDetail,
) {
    // Empty line
    items.push(ListItem::new(Line::raw("")));

    // Author
    items.push(ListItem::new(Line::from(vec![
        Span::raw("    Author:    "),
        Span::styled(&detail.author_name, Style::default().fg(Color::Green)),
        Span::raw(" <"),
        Span::styled(&detail.author_email, Style::default().fg(Color::Cyan)),
        Span::raw(">"),
    ])));

    // Committer (if different from author)
    if detail.committer_name != detail.author_name
        || detail.committer_email != detail.author_email
    {
        items.push(ListItem::new(Line::from(vec![
            Span::raw("    Committer: "),
            Span::styled(&detail.committer_name, Style::default().fg(Color::Green)),
            Span::raw(" <"),
            Span::styled(&detail.committer_email, Style::default().fg(Color::Cyan)),
            Span::raw(">"),
        ])));
    }

    // Date
    items.push(ListItem::new(Line::from(vec![
        Span::raw("    Date:      "),
        Span::styled(
            detail.author_time.format("%Y-%m-%d %H:%M:%S %z").to_string(),
            Style::default().fg(Color::Yellow),
        ),
    ])));

    // Full SHA
    items.push(ListItem::new(Line::from(vec![
        Span::raw("    Commit:    "),
        Span::styled(&detail.full_sha, Style::default().fg(Color::Yellow)),
    ])));

    // GPG info (if present)
    if let Some(gpg) = &detail.gpg_status {
        items.push(ListItem::new(Line::from(vec![
            Span::raw("    GPG:       "),
            Span::styled(gpg, Style::default().fg(Color::Magenta)),
        ])));
    }

    // Empty line before message
    items.push(ListItem::new(Line::raw("")));

    // Commit message (indented, may be multi-line)
    for msg_line in detail.message.lines() {
        items.push(ListItem::new(Line::from(vec![
            Span::raw("    "),
            Span::raw(msg_line.to_string()),
        ])));
    }

    // Files section with stats
    if !detail.files.is_empty() {
        items.push(ListItem::new(Line::raw("")));
        items.push(ListItem::new(Line::from(vec![
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
        ])));

        // Hint for commit file actions (only show when a file is selected)
        if matches!(app.current_context(), Context::CommitFiles) {
            // Use extra indent for commit files hints
            let hint = render_context_hint(&app.action_registry, Context::CommitFiles, &app.app_state());
            let mut spans = vec![Span::raw("  ")]; // Extra indent
            spans.extend(hint.spans);
            items.push(ListItem::new(Line::from(spans)));
        }

        for (file_idx, file) in detail.files.iter().enumerate() {
            let is_file_selected = app.expanded_file_idx == Some(file_idx);
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

            items.push(ListItem::new(Line::from(spans)));
        }
    }

    // Separator line
    items.push(ListItem::new(Line::raw("")));
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
            spans.push(Span::styled("HEAD → ", Style::default().fg(Color::Cyan).bold()));
            spans.push(Span::styled(branch.to_string(), Style::default().fg(Color::Green).bold()));
            first = false;
        } else {
            spans.push(Span::styled("HEAD", Style::default().fg(Color::Cyan).bold()));
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
                spans.push(Span::styled(name.clone(), Style::default().fg(Color::Green)));
            }
            RefDecoration::RemoteBranch(name) => {
                spans.push(Span::styled(name.clone(), Style::default().fg(Color::Red)));
            }
            RefDecoration::Tag(name) => {
                spans.push(Span::styled("tag: ", Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(name.clone(), Style::default().fg(Color::Yellow)));
            }
            RefDecoration::Head => {} // Already handled
        }
    }

    spans.push(Span::styled(")", Style::default().fg(Color::DarkGray)));
    spans
}

/// Render the footer with keybindings
fn render_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Show different hints based on mode
    let content = if app.is_command_mode() {
        Line::from(vec![
            Span::styled(" Enter ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" execute  "),
            Span::styled(" Esc ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" cancel  "),
            Span::styled(" Up/Down ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" history "),
        ])
    } else if let ViewMode::Confirm(action) = &app.view_mode {
        Line::from(render_confirm_footer_spans(action))
    } else if let Some(ref error) = app.error {
        Line::from(Span::styled(
            error.as_str(),
            Style::default().fg(Color::Red),
        ))
    } else {
        // Universal hints: nav top/btm m menu | : cmd  w files  h history  b branches | help quit
        Line::from(vec![
            Span::styled(" j/k ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" nav  "),
            Span::styled(" g/G ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" top/btm  "),
            Span::styled(" m ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" menu "),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(" : ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" cmd  "),
            Span::styled(" w ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" files  "),
            Span::styled(" h ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" history  "),
            Span::styled(" b ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" branches "),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled(" ? ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" help "),
            Span::styled(" q ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" quit "),
        ])
    };

    let footer = Paragraph::new(content).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer, area);
}

/// Build footer spans for confirmation prompts
fn render_confirm_footer_spans(action: &ConfirmAction) -> Vec<Span<'static>> {
    match action {
        ConfirmAction::Push { branch, remote, has_upstream, ahead, force } => {
            let mut spans: Vec<Span<'static>> = vec![];

            if !*has_upstream {
                spans.push(Span::raw("Set upstream and push "));
                spans.push(Span::styled(branch.clone(), Style::default().fg(Color::Yellow).bold()));
                spans.push(Span::raw(" → "));
                spans.push(Span::styled(format!("{remote}/{branch}"), Style::default().fg(Color::Cyan)));
                spans.push(Span::raw("? "));
            } else {
                let action_text = if *force { "Force push " } else { "Push " };
                spans.push(Span::raw(action_text));
                spans.push(Span::styled(branch.clone(), Style::default().fg(Color::Yellow).bold()));
                spans.push(Span::raw(" → "));
                spans.push(Span::styled(format!("{remote}/{branch}"), Style::default().fg(Color::Cyan)));
                if *ahead > 0 {
                    spans.push(Span::styled(format!(" ({}↑)", ahead), Style::default().fg(Color::Green)));
                }
                spans.push(Span::raw("? "));
            }

            if *force {
                spans.push(Span::styled("⚠ ", Style::default().fg(Color::Yellow)));
            }

            spans.push(Span::styled(" Enter ", Style::default().bg(Color::DarkGray).bold()));
            spans.push(Span::raw(" yes  "));
            spans.push(Span::styled(" f ", Style::default().bg(Color::DarkGray).bold()));
            spans.push(Span::raw(if *force { " normal  " } else { " force  " }));
            spans.push(Span::styled(" Esc ", Style::default().bg(Color::DarkGray).bold()));
            spans.push(Span::raw(" cancel "));
            spans
        }
    }
}

/// Render help overlay
fn render_help(frame: &mut Frame<'_>, area: Rect) {
    // Center the help box
    let help_area = centered_rect(60, 70, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Navigation",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / ↓              Move down"),
        Line::from("  k / ↑              Move up"),
        Line::from("  g                  Go to first"),
        Line::from("  G                  Go to last"),
        Line::from("  w                  Jump to working files"),
        Line::from("  b                  Jump to branches"),
        Line::from(""),
        Line::from(Span::styled(
            "Staged/Working Files",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  s                  Stage/unstage file"),
        Line::from("  d / Enter          Show diff"),
        Line::from("  P                  Push (when ahead)"),
        Line::from("  p                  Pull (when behind)"),
        Line::from("  f                  Fetch"),
        Line::from(""),
        Line::from(Span::styled(
            "History",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  h                  Toggle log/reflog"),
        Line::from("  Space              Expand commit details"),
        Line::from("  c                  Copy commit hash"),
        Line::from(""),
        Line::from(Span::styled(
            "Commit Files",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  Space              Diff with pager"),
        Line::from("  d                  Diff inline"),
        Line::from("  M                  Diff with difftool"),
        Line::from(""),
        Line::from(Span::styled(
            "Branches",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  Enter              Checkout branch"),
        Line::from("  Space              Expand branch commits"),
        Line::from("  c                  Copy commit hash"),
        Line::from(""),
        Line::from(Span::styled(
            "Command & Aliases",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  :                  Enter command mode"),
        Line::from("  a                  Browse aliases"),
        Line::from("  o                  Expand output popup"),
        Line::from("  Enter              Execute command"),
        Line::from("  Esc / Ctrl+C       Cancel"),
        Line::from("  Up / Down          Navigate history"),
        Line::from(""),
        Line::from(Span::styled(
            "General",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  m                  Action menu"),
        Line::from("  r                  Refresh status"),
        Line::from("  ?                  Toggle help"),
        Line::from("  q / Esc            Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray).italic(),
        )),
    ];

    let help = Paragraph::new(help_text).alignment(Alignment::Left).block(
        Block::default()
            .title(" Help ")
            .title_style(Style::default().fg(Color::Cyan).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(help, help_area);
}

/// Render the menu stack overlay
fn render_menu_stack(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if let Some(menu) = app.menu_stack.current() {
        // Dynamic sizing based on menu items (min 20%, max 60%)
        let item_count = menu.items().len().max(5); // Minimum for custom menus
        let height_percent = ((item_count + 4) * 3).min(60) as u16;
        let menu_area = centered_rect(50, height_percent.max(20), area);

        // Delegate rendering to the menu implementation
        menu.render(frame, menu_area);
    }
}

/// Style a command output line with tab replacement
fn style_output_line(line: &str, success: bool) -> Line<'static> {
    let color = if success { Color::White } else { Color::Red };
    let line = line.replace('\t', "    ");
    Line::from(Span::styled(
        format!(" {line}"),
        Style::default().fg(color).bg(Color::Reset),
    ))
}

/// Style a diff line with syntax highlighting and tab replacement
fn style_diff_line(line: &str) -> Line<'static> {
    let line = line.replace('\t', "    ");
    let style = if line.starts_with('+') && !line.starts_with("+++") {
        Style::default().fg(Color::Green).bg(Color::Reset)
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default().fg(Color::Red).bg(Color::Reset)
    } else if line.starts_with("@@") {
        Style::default().fg(Color::Cyan).bg(Color::Reset)
    } else if line.starts_with("diff") || line.starts_with("index") {
        Style::default().fg(Color::Yellow).bg(Color::Reset)
    } else {
        Style::default().fg(Color::White).bg(Color::Reset)
    };
    Line::from(Span::styled(format!(" {line}"), style))
}

/// Render the full-screen popup (replaces body when active)
fn render_popup(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    // Clear the area first to remove any artifacts from previous frame
    frame.render_widget(Clear, area);

    let title = app.popup.content.title();
    let total_lines = app.popup.content.line_count();
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

    // Store visible height for scroll calculations in key handler
    app.popup.visible_height = visible_height;

    // Clamp scroll offset to valid range
    let max_offset = total_lines.saturating_sub(visible_height);
    if app.popup.scroll_offset > max_offset {
        app.popup.scroll_offset = max_offset;
    }

    // Build scroll indicator
    let scroll_info = if total_lines > visible_height {
        format!(" [{}/{}] ", app.popup.scroll_offset + 1, max_offset + 1)
    } else {
        String::new()
    };

    // Get lines for display based on content type
    let lines: Vec<Line<'_>> = match &app.popup.content {
        PopupContent::None => vec![],
        PopupContent::CommandOutput { output, success, .. } => output
            .lines()
            .skip(app.popup.scroll_offset)
            .take(visible_height)
            .map(|line| style_output_line(line, *success))
            .collect(),
        PopupContent::Diff { content, .. } => content
            .lines()
            .skip(app.popup.scroll_offset)
            .take(visible_height)
            .map(style_diff_line)
            .collect(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan).bg(Color::Reset))
        .style(Style::default().bg(Color::Reset)) // Fill block background
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).bold())
        .title_bottom(Line::from(Span::styled(
            scroll_info,
            Style::default().fg(Color::DarkGray),
        )));

    let popup = Paragraph::new(lines)
        .style(Style::default().bg(Color::Reset)) // Fill content background
        .block(block);

    frame.render_widget(popup, area);
}

/// Render footer when popup is active
fn render_popup_footer(frame: &mut Frame<'_>, area: Rect) {
    let content = Line::from(vec![
        Span::styled(" j/k ", Style::default().bg(Color::DarkGray).bold()),
        Span::raw(" scroll  "),
        Span::styled(" g/G ", Style::default().bg(Color::DarkGray).bold()),
        Span::raw(" top/bottom  "),
        Span::styled(" Ctrl+d/u ", Style::default().bg(Color::DarkGray).bold()),
        Span::raw(" page  "),
        Span::styled(" q ", Style::default().bg(Color::DarkGray).bold()),
        Span::raw(" close "),
    ]);

    let footer = Paragraph::new(content).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer, area);
}

/// Create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
