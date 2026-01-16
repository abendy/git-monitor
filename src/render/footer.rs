//! Footer rendering for the bottom status bar.
//!
//! Displays context-sensitive hints, toasts, errors, and command mode help.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::feedback::ToastLevel;
use crate::tui::Frame;

/// Render the footer with keybindings and status
pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    // Show different hints based on mode
    // Priority: command mode > toast (transient) > error (persistent) > normal hints
    let content = if app.is_command_mode() {
        Line::from(vec![
            Span::styled(
                " Enter ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" execute  "),
            Span::styled(
                " Esc ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" cancel  "),
            Span::styled(
                " Up/Down ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" history "),
        ])
    } else if let Some(ref toast) = app.feedback.toast {
        // Toast with level-based coloring
        let color = match toast.level {
            ToastLevel::Info => Color::Blue,
            ToastLevel::Success => Color::Green,
            ToastLevel::Warning => Color::Yellow,
            ToastLevel::Error => Color::Red,
        };
        Line::from(Span::styled(
            toast.message.as_str(),
            Style::default().fg(color),
        ))
    } else if let Some(ref error) = app.feedback.error {
        Line::from(Span::styled(
            error.as_str(),
            Style::default().fg(Color::Red),
        ))
    } else {
        // Universal hints: nav top/btm m menu | : cmd  w files  h history  b branches | help quit
        Line::from(vec![
            Span::styled(
                " j/k ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" nav  "),
            Span::styled(
                " g/G ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" top/btm  "),
            Span::styled(
                " m ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" menu "),
            Span::styled(
                " │ ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                " : ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" cmd  "),
            Span::styled(
                " w ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" files  "),
            Span::styled(
                " h ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" history  "),
            Span::styled(
                " b ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" branches "),
            Span::styled(
                " │ ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                " ? ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" help "),
            Span::styled(
                " q ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" quit "),
        ])
    };

    let footer = Paragraph::new(content)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(footer, area);
}

/// Render footer when popup is active (simplified hints for popup navigation)
pub fn render_popup(frame: &mut Frame<'_>, area: Rect) {
    let content = Line::from(vec![
        Span::styled(
            " j/k ",
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" scroll  "),
        Span::styled(
            " g/G ",
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" top/bottom  "),
        Span::styled(
            " Ctrl+d/u ",
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" page  "),
        Span::styled(
            " q ",
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" close "),
    ]);

    let footer = Paragraph::new(content)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(footer, area);
}
