//! UI rendering coordinator.
//!
//! This module serves as the entry point for all UI rendering.
//! Component rendering is delegated to the render/ submodules.

use ratatui::layout::{Constraint, Direction, Layout};

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
        crate::render::help::render(frame, area);
    }

    // Menu stack overlay (modular menu system)
    if app.menu_stack.is_active() {
        crate::render::menu::render(frame, app, area);
    }
}
