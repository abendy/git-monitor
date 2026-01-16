//! Body rendering for the main content area.
//!
//! Renders the unified panel by iterating through the section registry
//! and composing each section's rendered output.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::{
    app::App,
    section::{Section, SectionId, SectionState},
    tui::Frame,
};

/// Render the main body panel
pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    render_main_panel(frame, app, area);
}

/// Render the unified main panel by iterating through sections
fn render_main_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let item_counts = app.section_item_counts();

    // Build section states from registry (handles index calculations)
    let section_states = app
        .section_registry()
        .build_section_states(app.selected, &item_counts);

    let mut items: Vec<ListItem<'_>> = Vec::new();
    let mut has_content_above = false;

    // Iterate through sections and render each
    for (section_id, mut state) in section_states {
        // Skip empty sections (except Command which always renders)
        let count = item_counts.get(section_id);
        if count == 0 && section_id != SectionId::Command {
            continue;
        }

        // Set render width for message truncation
        state.render_width = area.width;

        // If menu is active, unfocus all sections
        if app.menu_stack.is_active() {
            state.is_focused = false;
            state.local_selection = None;
        }

        // Add spacer between sections (except before Command)
        if has_content_above && section_id != SectionId::Command {
            items.push(ListItem::new(Line::from("")));
        }

        // Render the section
        let lines = render_section(app, section_id, &state);
        if !lines.is_empty() {
            has_content_above = true;
            for line in lines {
                items.push(ListItem::new(line));
            }
        }
    }

    // Empty state when no sections have content (besides command)
    let total_non_command =
        item_counts.staged + item_counts.working + item_counts.history + item_counts.branches;
    if total_non_command == 0 {
        items.push(ListItem::new(Line::from(Span::styled(
            "  No changes or activity",
            Style::default().fg(Color::DarkGray).italic(),
        ))));
    }

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Repository ")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(list, area);
}

/// Render a section by dispatching to the appropriate section's render method
fn render_section(app: &App, section_id: SectionId, state: &SectionState) -> Vec<Line<'static>> {
    match section_id {
        SectionId::Command => app.command_section.render(state),
        SectionId::Staged => app.staged_section.render(state),
        SectionId::Working => app.working_section.render(state),
        SectionId::History => app.history_section.render(state),
        SectionId::Branches => app.branches_section.render(state),
    }
}
