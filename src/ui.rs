//! UI rendering coordinator.
//!
//! This module serves as the entry point for all UI rendering.
//! Component rendering is delegated to the render/ submodules.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{app::App, tui::Frame};

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

    crate::render::header::render(frame, app, layout[0]);

    // Conditional rendering: popup replaces body, not overlays it
    if app.feedback.popup.is_open() {
        crate::render::popup::render(frame, app, layout[1]);
        crate::render::footer::render_popup(frame, layout[2]);
    } else {
        crate::render::body::render(frame, app, layout[1]);
        crate::render::footer::render(frame, app, layout[2]);
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

    let help = Paragraph::new(help_text)
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .title(" Help ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
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
