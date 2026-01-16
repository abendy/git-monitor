//! Help overlay rendering.
//!
//! Displays keybinding reference as a centered popup.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::Frame;

/// Render help overlay
pub fn render(frame: &mut Frame<'_>, area: Rect) {
    // Center the help box
    let help_area = super::centered_rect(60, 70, area);

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
        Line::from("  Space              Diff with pager"),
        Line::from("  d / Enter          Diff inline"),
        Line::from("  M                  Diff with difftool"),
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
            Style::default()
                .fg(Color::DarkGray)
                .italic(),
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
