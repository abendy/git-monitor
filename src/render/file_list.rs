//! Shared rendering utilities for file lists.
//!
//! Provides consistent display of file entries across working, staged, and commit file sections.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::git::FileState;

/// Map `FileState` to display color
pub const fn state_color(state: FileState) -> Color {
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

/// View struct for rendering a file entry
pub struct FileEntryView<'a> {
    /// File path to display
    pub path: &'a str,
    /// File status (M, A, D, etc.)
    pub status: FileState,
    /// Lines inserted
    pub insertions: usize,
    /// Lines deleted
    pub deletions: usize,
}

/// Style configuration for file list rendering
pub struct FileListStyle {
    /// Optional indicator prefix (e.g., "○ " for working, "● " for staged)
    pub indicator: Option<(&'static str, Color)>,
    /// Custom prefix for selected items (default: "▸ ")
    pub selected_prefix: &'static str,
    /// Custom prefix for unselected items (default: "  ")
    pub unselected_prefix: &'static str,
}

impl Default for FileListStyle {
    fn default() -> Self {
        Self {
            indicator: None,
            selected_prefix: "▸ ",
            unselected_prefix: "  ",
        }
    }
}

/// Render a single file entry line
///
/// Format: `[prefix][indicator][status_char] [path]  [+N/-M]`
///
/// Example output:
/// - Working:  `▸ ○ M src/file.rs  +5/-2`
/// - Staged:   `  ● A src/new.rs  +10`
/// - Commit:   `  M src/file.rs  +5/-2`
pub fn render_file_entry(
    entry: &FileEntryView<'_>,
    style: &FileListStyle,
    selected: bool,
) -> Line<'static> {
    let prefix = if selected {
        style.selected_prefix
    } else {
        style.unselected_prefix
    };
    let status_char = entry.status.as_char();
    let color = state_color(entry.status);

    let text_style = if selected {
        Style::default()
            .fg(color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(color)
    };

    let mut spans = vec![Span::raw(prefix.to_string())];

    // Add indicator if configured
    if let Some((indicator_str, indicator_color)) = style.indicator {
        spans.push(Span::styled(
            indicator_str.to_string(),
            Style::default().fg(indicator_color),
        ));
    }

    // Status character and path
    spans.push(Span::styled(
        format!("{status_char} "),
        text_style,
    ));
    spans.push(Span::styled(
        entry.path.to_string(),
        text_style,
    ));

    // Line change stats (only if there are changes)
    if entry.insertions > 0 || entry.deletions > 0 {
        spans.push(Span::raw("  "));
        if entry.insertions > 0 {
            spans.push(Span::styled(
                format!("+{}", entry.insertions),
                if selected {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                },
            ));
        }
        if entry.insertions > 0 && entry.deletions > 0 {
            spans.push(Span::raw("/"));
        }
        if entry.deletions > 0 {
            spans.push(Span::styled(
                format!("-{}", entry.deletions),
                if selected {
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Red)
                },
            ));
        }
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_color_returns_correct_colors() {
        assert_eq!(
            state_color(FileState::Modified),
            Color::Yellow
        );
        assert_eq!(
            state_color(FileState::Added),
            Color::Green
        );
        assert_eq!(
            state_color(FileState::Deleted),
            Color::Red
        );
        assert_eq!(
            state_color(FileState::Renamed),
            Color::Cyan
        );
        assert_eq!(
            state_color(FileState::Untracked),
            Color::DarkGray
        );
        assert_eq!(
            state_color(FileState::Conflicted),
            Color::Magenta
        );
        assert_eq!(
            state_color(FileState::Unmodified),
            Color::White
        );
        assert_eq!(
            state_color(FileState::Ignored),
            Color::White
        );
    }

    #[test]
    fn render_file_entry_basic() {
        let entry = FileEntryView {
            path: "src/main.rs",
            status: FileState::Modified,
            insertions: 0,
            deletions: 0,
        };
        let style = FileListStyle::default();
        let line = render_file_entry(&entry, &style, false);

        // Should produce something like "  M src/main.rs"
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn render_file_entry_with_stats() {
        let entry = FileEntryView {
            path: "src/main.rs",
            status: FileState::Modified,
            insertions: 5,
            deletions: 2,
        };
        let style = FileListStyle::default();
        let line = render_file_entry(&entry, &style, false);

        // Should include +5/-2
        let text: String = line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("+5"));
        assert!(text.contains("-2"));
    }

    #[test]
    fn render_file_entry_with_indicator() {
        let entry = FileEntryView {
            path: "src/main.rs",
            status: FileState::Modified,
            insertions: 0,
            deletions: 0,
        };
        let style = FileListStyle {
            indicator: Some(("○ ", Color::Yellow)),
            ..Default::default()
        };
        let line = render_file_entry(&entry, &style, false);

        let text: String = line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.contains("○"));
    }

    #[test]
    fn render_file_entry_selected() {
        let entry = FileEntryView {
            path: "src/main.rs",
            status: FileState::Modified,
            insertions: 0,
            deletions: 0,
        };
        let style = FileListStyle::default();
        let line = render_file_entry(&entry, &style, true);

        let text: String = line
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert!(text.starts_with("▸"));
    }
}
