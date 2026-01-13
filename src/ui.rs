use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::{
    app::App,
    git::{CommandType, FileState},
    tui::Frame,
};

/// Main render function
pub fn render(frame: &mut Frame<'_>, app: &App) {
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
    render_body(frame, app, layout[1]);
    render_footer(frame, app, layout[2]);

    // Overlays
    if app.show_help {
        render_help(frame, area);
    }

    if app.show_diff {
        render_diff(frame, app, area);
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
    let cmd_selected = app.selected == 0;
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
        // Selected but not active
        items.push(ListItem::new(Line::from(vec![
            Span::raw(cmd_prefix),
            Span::styled(": ", Style::default().fg(Color::Cyan).bold()),
            Span::styled(
                "press Enter or : to run command",
                Style::default().fg(Color::DarkGray).italic(),
            ),
        ])));
    } else {
        // Not selected
        items.push(ListItem::new(Line::from(vec![
            Span::raw(cmd_prefix),
            Span::styled(": ", Style::default().fg(Color::DarkGray)),
            Span::styled("command", Style::default().fg(Color::DarkGray)),
        ])));
    }

    // Command output (if any)
    if !app.command_output.is_empty() {
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
            items.push(ListItem::new(Line::from(Span::styled(
                format!("    ... ({} more lines)", line_count - 4),
                Style::default().fg(Color::DarkGray).italic(),
            ))));
        }
    }

    // Spacer after command section
    items.push(ListItem::new(Line::from("")));

    // Staged section
    if !staged_changes.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            format!("── Staged ({staged_len}) ──"),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ))));

        for (i, file) in staged_changes.iter().enumerate() {
            // Index 0 is command, so files start at index 1
            let global_idx = 1 + i;
            let selected = global_idx == app.selected;
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
            format!("── Working ({working_len}) ──"),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ))));

        for (i, file) in working_changes.iter().enumerate() {
            // Index 0 is command, staged starts at 1, working starts at 1 + staged_len
            let global_idx = 1 + staged_len + i;
            let selected = global_idx == app.selected;
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

    // Activity section
    if !app.activity.is_empty() {
        if files_total > 0 {
            items.push(ListItem::new(Line::from("")));
        }

        items.push(ListItem::new(Line::from(Span::styled(
            format!("── History ({activity_len}) ──"),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ))));

        for (i, cmd) in app.activity.iter().enumerate() {
            // Index 0 is command, files start at 1, history starts at 1 + files_total
            let global_idx = 1 + files_total + i;
            let selected = global_idx == app.selected;
            let prefix = if selected { "▸ " } else { "  " };

            let time_str = cmd.timestamp.format("%H:%M:%S").to_string();
            let icon = cmd.command_type.icon();
            let color = command_color(cmd.command_type);

            // Truncate message if too long
            let max_msg_len = area.width.saturating_sub(20) as usize;
            let message = if cmd.message.len() > max_msg_len {
                format!("{}...", &cmd.message[..max_msg_len.saturating_sub(3)])
            } else {
                cmd.message.clone()
            };

            let style = if selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            items.push(ListItem::new(Line::from(vec![
                Span::raw(prefix),
                Span::styled(format!("{time_str}  "), style.fg(Color::DarkGray)),
                Span::styled(format!("{icon} "), style.fg(color)),
                Span::styled(message, style.fg(Color::White)),
            ])));
        }
    }

    // Empty state for files (command section is always shown)
    if files_total == 0 && app.activity.is_empty() {
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
        Line::from(vec![
            Span::styled(" j/k ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" nav  "),
            Span::styled(" s ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" stage  "),
            Span::styled(" d ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" diff  "),
            Span::styled(" : ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" cmd  "),
            Span::styled(" ? ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" help  "),
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
        Line::from(""),
        Line::from(Span::styled(
            "File Actions",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  s                  Stage/unstage file"),
        Line::from("  d / Enter          Show diff"),
        Line::from("  r                  Refresh status"),
        Line::from(""),
        Line::from(Span::styled(
            "Command Mode",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  :                  Enter command mode"),
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

/// Render diff overlay
fn render_diff(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Use most of the screen for diff
    let diff_area = centered_rect(90, 90, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, diff_area);

    // Parse diff content into styled lines
    let lines: Vec<Line<'_>> = app
        .diff_content
        .lines()
        .map(|line| {
            let (style, content) = if line.starts_with('+') && !line.starts_with("+++") {
                (Style::default().fg(Color::Green), line)
            } else if line.starts_with('-') && !line.starts_with("---") {
                (Style::default().fg(Color::Red), line)
            } else if line.starts_with("@@") {
                (Style::default().fg(Color::Cyan), line)
            } else if line.starts_with("diff") || line.starts_with("index") {
                (Style::default().fg(Color::Yellow), line)
            } else {
                (Style::default().fg(Color::White), line)
            };
            Line::from(Span::styled(format!(" {content}"), style))
        })
        .collect();

    let diff = Paragraph::new(lines)
        .block(
            Block::default()
                .title(format!(" Diff: {} ", app.diff_path))
                .title_style(Style::default().fg(Color::Cyan).bold())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(ratatui::widgets::Wrap { trim: false });

    frame.render_widget(diff, diff_area);

    // Footer hint
    let hint_area = Rect {
        x: diff_area.x,
        y: diff_area.y + diff_area.height - 1,
        width: diff_area.width,
        height: 1,
    };
    let hint = Paragraph::new(Line::from(vec![
        Span::styled(" q ", Style::default().bg(Color::DarkGray).bold()),
        Span::raw(" close "),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(hint, hint_area);
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
