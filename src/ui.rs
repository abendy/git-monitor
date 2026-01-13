use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::{
    app::{App, Panel},
    git::{CommandType, FileState},
    tui::Frame,
};

/// Main render function
pub fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();

    // Determine if we need command pane
    let show_command_pane = app.command_mode || !app.command_output.is_empty();

    // Main layout: header, body, [command], footer
    let layout = if show_command_pane {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(8),    // Body (reduced)
                Constraint::Length(6), // Command pane
                Constraint::Length(3), // Footer
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Body
                Constraint::Length(3), // Footer
            ])
            .split(area)
    };

    render_header(frame, app, layout[0]);
    render_body(frame, app, layout[1]);

    if show_command_pane {
        render_command_pane(frame, app, layout[2]);
        render_footer(frame, app, layout[3]);
    } else {
        render_footer(frame, app, layout[2]);
    }

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

/// Render the main body with panels
fn render_body(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Split body into upper (file lists) and lower (activity)
    let body_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60), // File panels
            Constraint::Percentage(40), // Activity log
        ])
        .split(area);

    // Split upper into working and staged panels
    let panels_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(body_layout[0]);

    render_working_panel(frame, app, panels_layout[0]);
    render_staged_panel(frame, app, panels_layout[1]);
    render_activity_panel(frame, app, body_layout[1]);
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

/// Render the working directory panel
fn render_working_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Working;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let working_changes = app.status.working_changes();
    let title = format!(" Working Directory ({}) ", working_changes.len());

    let items: Vec<ListItem<'_>> = if working_changes.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No changes",
            Style::default().fg(Color::DarkGray).italic(),
        )))]
    } else {
        working_changes
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let selected = is_active && i == app.working_selected;
                let prefix = if selected { "▸ " } else { "  " };
                let status_char = file.working.as_char();
                let color = state_color(file.working);
                let path = file.path.to_string_lossy();

                let style = if selected {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                };

                ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(format!("{status_char} "), style),
                    Span::styled(path.to_string(), style),
                ]))
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title)
            .title_style(if is_active {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            }),
    );

    frame.render_widget(list, area);
}

/// Render the staged changes panel
fn render_staged_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Staged;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let staged_changes = app.status.staged_changes();
    let title = format!(" Staged ({}) ", staged_changes.len());

    let items: Vec<ListItem<'_>> = if staged_changes.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No staged changes",
            Style::default().fg(Color::DarkGray).italic(),
        )))]
    } else {
        staged_changes
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let selected = is_active && i == app.staged_selected;
                let prefix = if selected { "▸ " } else { "  " };
                let status_char = file.staged.as_char();
                let color = state_color(file.staged);
                let path = file.path.to_string_lossy();

                let style = if selected {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                };

                ListItem::new(Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(format!("{status_char} "), style),
                    Span::styled(path.to_string(), style),
                ]))
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title)
            .title_style(if is_active {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            }),
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

/// Render the activity log panel
fn render_activity_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Activity;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Calculate how many items fit in the area (minus borders)
    let visible_height = area.height.saturating_sub(2) as usize;

    let items: Vec<ListItem<'_>> = if app.activity.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "  No activity yet",
            Style::default().fg(Color::DarkGray).italic(),
        )))]
    } else {
        app.activity
            .iter()
            .take(visible_height)
            .map(|cmd| {
                let time_str = cmd.timestamp.format("%H:%M:%S").to_string();
                let icon = cmd.command_type.icon();
                let color = command_color(cmd.command_type);

                // Truncate message if too long
                let max_msg_len = area.width.saturating_sub(16) as usize;
                let message = if cmd.message.len() > max_msg_len {
                    format!("{}...", &cmd.message[..max_msg_len.saturating_sub(3)])
                } else {
                    cmd.message.clone()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {time_str}  "),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(format!("{icon} "), Style::default().fg(color)),
                    Span::styled(message, Style::default().fg(Color::White)),
                ]))
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" Recent Activity ({}) ", app.activity.len()))
            .title_style(if is_active {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            }),
    );

    frame.render_widget(list, area);
}

/// Render the command input/output pane
fn render_command_pane(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let border_color = if app.command_mode {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    // Build content lines
    let mut lines: Vec<Line<'_>> = Vec::new();

    // Command input line
    if app.command_mode {
        lines.push(Line::from(vec![
            Span::styled(": ", Style::default().fg(Color::Cyan).bold()),
            Span::styled(&app.command_input, Style::default().fg(Color::White)),
            Span::styled("_", Style::default().fg(Color::Cyan)), // Cursor
        ]));
    } else if !app.command_output.is_empty() {
        // Show last command hint when not in command mode
        lines.push(Line::from(Span::styled(
            "  Press : to enter command mode",
            Style::default().fg(Color::DarkGray).italic(),
        )));
    }

    // Command output (up to 4 lines to fit in pane)
    if !app.command_output.is_empty() {
        let output_color = if app.command_success {
            Color::White
        } else {
            Color::Red
        };

        for line in app.command_output.lines().take(4) {
            lines.push(Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::DarkGray)),
                Span::styled(line, Style::default().fg(output_color)),
            ]));
        }

        // Show truncation indicator if output is longer
        let line_count = app.command_output.lines().count();
        if line_count > 4 {
            lines.push(Line::from(Span::styled(
                format!("  ... ({} more lines)", line_count - 4),
                Style::default().fg(Color::DarkGray).italic(),
            )));
        }
    }

    let command_block = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Command ")
            .title_style(if app.command_mode {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            }),
    );

    frame.render_widget(command_block, area);
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
            Span::styled(" Tab ", Style::default().bg(Color::DarkGray).bold()),
            Span::raw(" switch  "),
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
        Line::from("  Tab / Shift+Tab    Switch panels"),
        Line::from("  j / ↓              Move down"),
        Line::from("  k / ↑              Move up"),
        Line::from("  g                  Go to first"),
        Line::from("  G                  Go to last"),
        Line::from(""),
        Line::from(Span::styled(
            "Actions",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  s                  Stage/unstage file"),
        Line::from("  d                  Show diff"),
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
