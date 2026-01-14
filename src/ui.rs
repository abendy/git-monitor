use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::{
    app::{App, HistoryMode, PopupContent},
    git::{format_relative_time, CommandType, FileState, RefDecoration},
    tui::Frame,
};

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
    let cmd_selected = app.selected == Some(0) && !app.show_aliases;
    let cmd_prefix = if cmd_selected { "▸ " } else { "  " };

    if app.command_mode {
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

    // Show alias categories when in alias browsing mode
    if app.show_aliases && !app.show_section_aliases {
        let section_count = app.config.sections.len();
        items.push(ListItem::new(Line::from(Span::styled(
            format!("── Aliases ({section_count} categories) ──"),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ))));

        for (i, section) in app.config.sections.iter().enumerate() {
            let selected = i == app.alias_section_selected;
            let prefix = if selected { "▸ " } else { "  " };
            let alias_count = section.aliases.len();

            let style = if selected {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            items.push(ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled(&section.name, style),
                Span::styled(format!(" ({alias_count})"), Style::default().fg(Color::DarkGray)),
            ])));
        }

        items.push(ListItem::new(Line::from(Span::styled(
            "  Enter select  Esc back",
            Style::default().fg(Color::DarkGray).italic(),
        ))));
    }

    // Show aliases within a section
    if app.show_aliases && app.show_section_aliases {
        if let Some(section) = app.config.sections.get(app.alias_section_selected) {
            items.push(ListItem::new(Line::from(Span::styled(
                format!("── {} ──", section.name),
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ))));

            for (i, alias) in section.aliases.iter().enumerate() {
                let selected = i == app.alias_selected;
                let prefix = if selected { "▸ " } else { "  " };

                let style = if selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                // Truncate command if too long
                let max_cmd_len = area.width.saturating_sub(20) as usize;
                let cmd_display = if alias.command.len() > max_cmd_len {
                    format!("{}...", &alias.command[..max_cmd_len.saturating_sub(3)])
                } else {
                    alias.command.clone()
                };

                items.push(ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(
                        format!("{:<12}", alias.name),
                        style,
                    ),
                    Span::styled(cmd_display, Style::default().fg(Color::DarkGray)),
                ])));
            }

            items.push(ListItem::new(Line::from(Span::styled(
                "  Enter run  Esc back",
                Style::default().fg(Color::DarkGray).italic(),
            ))));
        }
    }

    // Command output (if any, and not in alias mode)
    if !app.command_output.is_empty() && !app.show_aliases {
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
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  Staged ({staged_len})"),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ))));

        for (i, file) in staged_changes.iter().enumerate() {
            // Index 0 is command, so files start at index 1
            let global_idx = 1 + i;
            // Don't show selection when command mode is active (focus is on input)
            let selected = Some(global_idx) == app.selected && !app.command_mode;
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

        items.push(ListItem::new(Line::from(Span::styled(
            format!("  Working ({working_len})"),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))));

        for (i, file) in working_changes.iter().enumerate() {
            // Index 0 is command, staged starts at 1, working starts at 1 + staged_len
            let global_idx = 1 + staged_len + i;
            // Don't show selection when command mode is active (focus is on input)
            let selected = Some(global_idx) == app.selected && !app.command_mode;
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
        let header_selected = Some(current_idx) == app.selected && !app.command_mode;
        let collapse_indicator = if app.history_collapsed { "▸" } else { "▾" };
        let header_prefix = if header_selected { "▸ " } else { "  " };

        let header_style = if header_selected {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        };

        items.push(ListItem::new(Line::from(vec![
            Span::raw(header_prefix),
            Span::styled(format!("{collapse_indicator} "), header_style),
            Span::styled(format!("{history_label} ({activity_len})"), header_style),
        ])));
        current_idx += 1;

        // Only show commits if not collapsed
        if !app.history_collapsed {
            // Hint for switching modes and actions
            items.push(ListItem::new(Line::from(Span::styled(
                "  h toggle reflog/history · space expand · c copy hash",
                Style::default().fg(Color::DarkGray).italic(),
            ))));

            let activity_count = app.activity.len();
            for (i, cmd) in app.activity.iter().enumerate() {
                let selected = Some(current_idx) == app.selected && !app.command_mode;
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

        items.push(ListItem::new(Line::from(Span::styled(
            format!("  Branches ({})", other_branches.len()),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ))));

        // Hint for actions
        items.push(ListItem::new(Line::from(Span::styled(
            "  space expand · c copy hash",
            Style::default().fg(Color::DarkGray).italic(),
        ))));

        for branch in other_branches.iter() {
            let is_expanded = app.expanded_branch.as_deref() == Some(&branch.name);
            let header_selected = Some(current_idx) == app.selected && !app.command_mode;
            let collapse_indicator = if is_expanded { "▾" } else { "▸" };
            let prefix = if header_selected { "▸ " } else { "  " };

            // Branch name styling
            let name_style = if header_selected {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            items.push(ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled(format!("{collapse_indicator} "), name_style),
                Span::styled(branch.name.clone(), name_style),
            ])));
            current_idx += 1;

            // If this branch is expanded, show its commits
            if is_expanded {
                let branch_commit_count = app.expanded_branch_commits.len();
                for (i, cmd) in app.expanded_branch_commits.iter().enumerate() {
                    let selected = Some(current_idx) == app.selected && !app.command_mode;
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
    let (indent, graph_char) = if use_tree_style {
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

    let mut spans = vec![
        Span::raw(format!("{selection_prefix} {indent}")),
        Span::styled(format!("{graph_char} "), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{sha_str} "), style.fg(Color::Yellow)),
        Span::styled(format!("{time_str}  "), style.fg(Color::DarkGray)),
        Span::styled(format!("{icon} "), style.fg(color)),
        Span::styled(message, style.fg(Color::White)),
    ];

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
    let content = if app.command_mode {
        Line::from(vec![
            Span::styled(" Enter ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" execute  "),
            Span::styled(" Esc ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" cancel  "),
            Span::styled(" Up/Down ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" history "),
        ])
    } else if let Some(ref error) = app.error {
        Line::from(Span::styled(
            error.as_str(),
            Style::default().fg(Color::Red),
        ))
    } else {
        // Determine what section we're in for contextual hints
        let staged_len = app.status.staged_changes().len();
        let working_len = app.status.working_changes().len();
        let activity_len = app.activity.len();
        let files_total = staged_len + working_len;

        let selected = app.selected.unwrap_or(usize::MAX);
        let on_command = selected == 0;
        let on_file = selected >= 1 && selected <= files_total;
        let history_start = 1 + files_total;
        let history_end = history_start + activity_len;
        let on_history = selected >= history_start && selected < history_end;
        let branches_start = history_end;
        let on_branches = selected >= branches_start
            && selected < branches_start + app.branches.len();

        let mut hints = vec![
            Span::styled(" j/k ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" nav  "),
        ];

        // File-specific hints
        if on_file {
            hints.extend([
                Span::styled(" s ", Style::default().bg(Color::DarkGray).bold()),
                Span::raw(" stage  "),
                Span::styled(" d ", Style::default().bg(Color::DarkGray).bold()),
                Span::raw(" diff  "),
            ]);
        }

        // Command section hints
        if on_command && !app.command_output.is_empty() {
            hints.extend([
                Span::styled(" o ", Style::default().bg(Color::DarkGray).bold()),
                Span::raw(" output  "),
            ]);
        }

        // History section hints
        if on_history {
            hints.extend([
                Span::styled(" h ", Style::default().bg(Color::DarkGray).bold()),
                Span::raw(" log/reflog  "),
            ]);
        }

        // Branch-specific hints
        if on_branches {
            hints.extend([
                Span::styled(" Enter ", Style::default().bg(Color::DarkGray).bold()),
                Span::raw(" checkout  "),
            ]);
        }

        // Universal hints
        hints.extend([
            Span::styled(" g/G ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" top/btm  "),
            Span::styled(" b ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" branches  "),
            Span::styled(" : ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" cmd  "),
            Span::styled(" ? ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" help  "),
            Span::styled(" q ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" quit "),
        ]);

        Line::from(hints)
    };

    let footer = Paragraph::new(content).alignment(Alignment::Center).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer, area);
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
        Line::from("  b                  Jump to branches"),
        Line::from(""),
        Line::from(Span::styled(
            "Branches",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  Enter              Checkout branch"),
        Line::from(""),
        Line::from(Span::styled(
            "File Actions",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  s                  Stage/unstage file"),
        Line::from("  d / Enter          Show diff"),
        Line::from("  y                  Copy commit sha"),
        Line::from("  r                  Refresh status"),
        Line::from("  h                  Toggle history/reflog"),
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
