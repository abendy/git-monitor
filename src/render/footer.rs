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
#[allow(clippy::too_many_lines)] // Render functions are naturally verbose
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
        Line::from(vec![
            Span::styled(
                error.as_str(),
                Style::default().fg(Color::Red),
            ),
            Span::raw("  "),
            Span::styled(
                " e ",
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " details",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        // Keys shown elsewhere (`:` on the command row, `w`/`h`/`b` on section
        // headers) stay out of the footer
        let mut navigate = vec![
            Hint::new("j/k", "nav", 2),
            Hint::new("g/G", "top/btm", 1),
            Hint::new("m", "menu", Hint::ALWAYS),
        ];
        if editor_available() {
            navigate.push(Hint::new("e", "edit", 3));
        }

        let groups = vec![
            navigate,
            vec![
                Hint::new("?", "help", Hint::ALWAYS),
                Hint::new("q", "quit", 4),
            ],
        ];
        fit_hints(groups, inner_width(area))
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
    let content = fit_hints(
        vec![vec![
            Hint::new("j/k", "scroll", 3),
            Hint::new("g/G", "top/bottom", 1),
            Hint::new("Ctrl+d/u", "page", 2),
            Hint::new("q", "close", Hint::ALWAYS),
        ]],
        inner_width(area),
    );

    let footer = Paragraph::new(content)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(footer, area);
}

/// A key hint shown in the footer
struct Hint {
    key: &'static str,
    label: &'static str,
    /// Drop order when space runs out: lower ranks drop first
    rank: u8,
}

impl Hint {
    /// Rank for hints that are never dropped
    const ALWAYS: u8 = u8::MAX;

    const fn new(key: &'static str, label: &'static str, rank: u8) -> Self {
        Self { key, label, rank }
    }

    fn width(&self) -> usize {
        Span::raw(self.key).width() + Span::raw(self.label).width() + 4
    }
}

/// Footer text width inside the border
fn inner_width(area: Rect) -> usize {
    usize::from(area.width.saturating_sub(2))
}

/// Width of hint groups laid out as `key label key label │ key label`
fn groups_width(groups: &[Vec<Hint>]) -> usize {
    let visible: Vec<&Vec<Hint>> = groups
        .iter()
        .filter(|group| !group.is_empty())
        .collect();
    let hints: usize = visible
        .iter()
        .map(|group| {
            group
                .iter()
                .map(Hint::width)
                .sum::<usize>()
                + group.len().saturating_sub(1)
        })
        .sum();
    hints + 3 * visible.len().saturating_sub(1)
}

/// Drop the lowest-ranked hints until the groups fit `width`, then build the line
fn fit_hints(mut groups: Vec<Vec<Hint>>, width: usize) -> Line<'static> {
    while groups_width(&groups) > width {
        let lowest = groups
            .iter()
            .enumerate()
            .flat_map(|(g, group)| {
                group
                    .iter()
                    .enumerate()
                    .map(move |(h, hint)| (hint.rank, g, h))
            })
            .filter(|(rank, _, _)| *rank != Hint::ALWAYS)
            .min();
        let Some((_, g, h)) = lowest else {
            break;
        };
        groups[g].remove(h);
    }

    let key_style = Style::default()
        .bg(Color::DarkGray)
        .add_modifier(Modifier::BOLD);
    let mut spans = Vec::new();
    for group in groups
        .iter()
        .filter(|group| !group.is_empty())
    {
        if !spans.is_empty() {
            spans.push(Span::styled(
                " │ ",
                Style::default().fg(Color::DarkGray),
            ));
        }
        for (i, hint) in group.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                format!(" {} ", hint.key),
                key_style,
            ));
            spans.push(Span::raw(format!(" {} ", hint.label)));
        }
    }
    Line::from(spans)
}

fn editor_available() -> bool {
    std::env::var("EDITOR")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normal_groups() -> Vec<Vec<Hint>> {
        vec![
            vec![
                Hint::new("j/k", "nav", 4),
                Hint::new("g/G", "top/btm", 1),
                Hint::new("m", "menu", 7),
                Hint::new("e", "edit", 6),
            ],
            vec![
                Hint::new(":", "cmd", 8),
                Hint::new("w", "files", 3),
                Hint::new("h", "history", 5),
                Hint::new("b", "branches", 2),
            ],
            vec![
                Hint::new("?", "help", Hint::ALWAYS),
                Hint::new("q", "quit", 9),
            ],
        ]
    }

    fn text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn wide_footer_keeps_every_hint() {
        let line = fit_hints(normal_groups(), 200);
        let text = text(&line);

        for label in [
            "nav", "top/btm", "menu", "edit", "cmd", "files", "history", "branches", "help", "quit",
        ] {
            assert!(
                text.contains(label),
                "{label} missing: {text}"
            );
        }
        assert_eq!(
            line.width(),
            groups_width(&normal_groups())
        );
    }

    #[test]
    fn sidebar_footer_fits_and_keeps_help() {
        let line = fit_hints(normal_groups(), 66);
        let text = text(&line);
        println!("66 cols: {text}");

        assert!(line.width() <= 66, "fits: {text}");
        assert!(
            text.contains(" ? ") && text.contains("help"),
            "help kept: {text}"
        );
        assert!(
            !text.contains("top/btm"),
            "lowest rank dropped first: {text}"
        );
    }

    #[test]
    fn help_survives_any_width() {
        let text = text(&fit_hints(normal_groups(), 5));

        assert!(
            text.contains("help"),
            "help kept: {text}"
        );
        assert!(
            !text.contains("quit"),
            "everything else dropped: {text}"
        );
    }
}
