//! Rendering module for git-monitor TUI.
//!
//! This module coordinates all rendering through sub-modules:
//! - `header`: Top bar with branch, status, and repo info
//! - `footer`: Bottom bar with hints and status (pending)
//! - `body`: Main content area with sections (pending)
//! - `popup`: Full-screen popup for command output and diffs (pending)
//!
//! Migration is incremental: components are moved from ui.rs as they're refactored.

pub mod footer;
pub mod header;
pub mod popup;

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Calculate a centered rectangle within the given area
///
/// Used for popup dialogs and overlays.
#[allow(dead_code)]
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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
