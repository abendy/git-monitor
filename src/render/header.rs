//! Header rendering for the top status bar.
//!
//! Displays branch name, ahead/behind status, repository path, and data freshness.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use std::time::Instant;

use crate::app::App;
use crate::freshness::{format_age, FreshnessState};
use crate::tui::Frame;

/// Render the header bar
pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let branch_name = app
        .snapshot()
        .status
        .branch
        .as_deref()
        .unwrap_or("(no branch)");

    let mut header_spans = vec![
        Span::styled(" ⎇ ", Style::default().fg(Color::Cyan)),
        Span::styled(
            branch_name,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    // Ahead/behind indicators
    if app.snapshot().status.ahead > 0 || app.snapshot().status.behind > 0 {
        header_spans.push(Span::raw(" "));
        if app.snapshot().status.ahead > 0 {
            header_spans.push(Span::styled(
                format!("↑{}", app.snapshot().status.ahead),
                Style::default().fg(Color::Green),
            ));
        }
        if app.snapshot().status.behind > 0 {
            header_spans.push(Span::styled(
                format!("↓{}", app.snapshot().status.behind),
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
    header_spans.push(Span::styled(
        path_str,
        Style::default().fg(Color::White),
    ));
    header_spans.push(Span::raw(" │ "));
    header_spans.extend(freshness_spans(
        &app.freshness().state(Instant::now()),
    ));

    let header_text = Line::from(header_spans);

    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" git-monitor ")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(header, area);
}

/// How fresh the data is, in text and color: `● 12s ago (files)` or `⚠ stale 3m · e details`
fn freshness_spans(state: &FreshnessState<'_>) -> Vec<Span<'static>> {
    match state {
        FreshnessState::Unknown => vec![Span::styled(
            "○ loading",
            Style::default().fg(Color::DarkGray),
        )],
        FreshnessState::Current { age, reason } => {
            let (age, reason) = (*age, *reason);
            let age = format_age(age);
            let when = if age == "now" {
                age
            } else {
                format!("{age} ago")
            };
            vec![Span::styled(
                format!("● {when} ({})", reason.label()),
                Style::default().fg(Color::Green),
            )]
        }
        FreshnessState::Stale { age, .. } => {
            let label = match age.map(format_age).as_deref() {
                None => "⚠ no data".to_string(),
                Some("now") => "⚠ refresh failed".to_string(),
                Some(age) => format!("⚠ stale {age}"),
            };
            vec![
                Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " · e details",
                    Style::default().fg(Color::DarkGray),
                ),
            ]
        }
    }
}
