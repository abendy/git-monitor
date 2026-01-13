use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{app::App, app::Panel, tui::Frame};

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
    render_footer(frame, layout[2]);

    // Help overlay
    if app.show_help {
        render_help(frame, area);
    }
}

/// Render the header bar
fn render_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let path_str = app
        .repo_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let header_text = Line::from(vec![
        Span::styled(" ⎇ ", Style::default().fg(Color::Cyan)),
        Span::styled("main", Style::default().fg(Color::Green).bold()),
        Span::raw(" │ "),
        Span::styled(path_str, Style::default().fg(Color::White)),
        Span::raw(" │ "),
        Span::styled("● watching", Style::default().fg(Color::Green)),
    ]);

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

/// Render the working directory panel
fn render_working_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Working;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let placeholder = vec![
        Line::from(Span::styled(
            "  M src/main.rs",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  ? untracked.txt",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let panel = Paragraph::new(placeholder).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Working Directory ")
            .title_style(if is_active {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            }),
    );

    frame.render_widget(panel, area);
}

/// Render the staged changes panel
fn render_staged_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Staged;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let placeholder = vec![Line::from(Span::styled(
        "  No staged changes",
        Style::default().fg(Color::DarkGray).italic(),
    ))];

    let panel = Paragraph::new(placeholder).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Staged ")
            .title_style(if is_active {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            }),
    );

    frame.render_widget(panel, area);
}

/// Render the activity log panel
fn render_activity_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let is_active = app.active_panel == Panel::Activity;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let placeholder = vec![
        Line::from(vec![
            Span::styled("  12:34:56  ", Style::default().fg(Color::DarkGray)),
            Span::styled("● ", Style::default().fg(Color::Green)),
            Span::raw("commit: Initial commit"),
        ]),
        Line::from(vec![
            Span::styled("  12:34:00  ", Style::default().fg(Color::DarkGray)),
            Span::styled("⎇ ", Style::default().fg(Color::Cyan)),
            Span::raw("checkout: main"),
        ]),
    ];

    let panel = Paragraph::new(placeholder).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Recent Activity ")
            .title_style(if is_active {
                Style::default().fg(Color::Cyan).bold()
            } else {
                Style::default().fg(Color::White)
            }),
    );

    frame.render_widget(panel, area);
}

/// Render the footer with keybindings
fn render_footer(frame: &mut Frame<'_>, area: Rect) {
    let keys = Line::from(vec![
        Span::styled(" Tab ", Style::default().bg(Color::DarkGray).bold()),
        Span::raw(" switch  "),
        Span::styled(" q ", Style::default().bg(Color::DarkGray).bold()),
        Span::raw(" quit  "),
        Span::styled(" ? ", Style::default().bg(Color::DarkGray).bold()),
        Span::raw(" help "),
    ]);

    let footer = Paragraph::new(keys)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));

    frame.render_widget(footer, area);
}

/// Render help overlay
fn render_help(frame: &mut Frame<'_>, area: Rect) {
    // Center the help box
    let help_area = centered_rect(60, 60, area);

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
        Line::from(""),
        Line::from(Span::styled(
            "Actions",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("  s                  Stage/unstage file"),
        Line::from("  d                  Show diff"),
        Line::from("  r                  Refresh"),
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

    let help = Paragraph::new(help_text)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .title(" Help ")
                .title_style(Style::default().fg(Color::Cyan).bold())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(help, help_area);
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
