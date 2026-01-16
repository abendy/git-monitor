//! Popup rendering for full-screen command output and diffs.
//!
//! When a popup is active, it replaces the body content entirely
//! rather than overlaying it, to avoid rendering artifacts.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;
use crate::feedback::PopupContent;
use crate::tui::Frame;

/// Render the full-screen popup (replaces body when active)
pub fn render(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    // Clear the area first to remove any artifacts from previous frame
    frame.render_widget(Clear, area);

    let title = app.feedback.popup.content.title();
    let total_lines = app.feedback.popup.content.line_count();
    let visible_height = area.height.saturating_sub(2) as usize; // Account for borders

    // Store visible height for scroll calculations in key handler
    app.feedback.popup.visible_height = visible_height;

    // Clamp scroll offset to valid range
    let max_offset = total_lines.saturating_sub(visible_height);
    if app.feedback.popup.scroll_offset > max_offset {
        app.feedback.popup.scroll_offset = max_offset;
    }

    // Build scroll indicator
    let scroll_info = if total_lines > visible_height {
        format!(
            " [{}/{}] ",
            app.feedback.popup.scroll_offset + 1,
            max_offset + 1
        )
    } else {
        String::new()
    };

    // Get lines for display based on content type
    let lines: Vec<Line<'_>> = match &app.feedback.popup.content {
        PopupContent::None => vec![],
        PopupContent::CommandOutput {
            output, success, ..
        } => output
            .lines()
            .skip(app.feedback.popup.scroll_offset)
            .take(visible_height)
            .map(|line| style_output_line(line, *success))
            .collect(),
        PopupContent::Diff { content, .. } => content
            .lines()
            .skip(app.feedback.popup.scroll_offset)
            .take(visible_height)
            .map(style_diff_line)
            .collect(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan).bg(Color::Reset))
        .style(Style::default().bg(Color::Reset)) // Fill block background
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .title_bottom(Line::from(Span::styled(
            scroll_info,
            Style::default().fg(Color::DarkGray),
        )));

    let popup = Paragraph::new(lines)
        .style(Style::default().bg(Color::Reset)) // Fill content background
        .block(block);

    frame.render_widget(popup, area);
}

/// Style a command output line with tab replacement
fn style_output_line(line: &str, success: bool) -> Line<'static> {
    let color = if success { Color::White } else { Color::Red };
    let line = line.replace('\t', "    ");
    Line::from(Span::styled(
        format!(" {line}"),
        Style::default()
            .fg(color)
            .bg(Color::Reset),
    ))
}

/// Style a diff line with syntax highlighting and tab replacement
fn style_diff_line(line: &str) -> Line<'static> {
    let line = line.replace('\t', "    ");
    let style = if line.starts_with('+') && !line.starts_with("+++") {
        Style::default()
            .fg(Color::Green)
            .bg(Color::Reset)
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default()
            .fg(Color::Red)
            .bg(Color::Reset)
    } else if line.starts_with("@@") {
        Style::default()
            .fg(Color::Cyan)
            .bg(Color::Reset)
    } else if line.starts_with("diff") || line.starts_with("index") {
        Style::default()
            .fg(Color::Yellow)
            .bg(Color::Reset)
    } else {
        Style::default()
            .fg(Color::White)
            .bg(Color::Reset)
    };
    Line::from(Span::styled(format!(" {line}"), style))
}
