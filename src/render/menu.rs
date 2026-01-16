//! Menu stack overlay rendering.
//!
//! Renders the current menu from the menu stack as a centered overlay.

use ratatui::layout::Rect;

use crate::app::App;
use crate::tui::Frame;

/// Render the menu stack overlay
pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    if let Some(menu) = app.menu_stack.current() {
        // Dynamic sizing based on menu items (min 20%, max 60%)
        let item_count = menu.items().len().max(5); // Minimum for custom menus
        let height_percent = ((item_count + 4) * 3).min(60) as u16;
        let menu_area = super::centered_rect(50, height_percent.max(20), area);

        // Delegate rendering to the menu implementation
        menu.render(frame, menu_area);
    }
}
