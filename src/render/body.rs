//! Body rendering for the main content area.
//!
//! Renders the unified panel containing:
//! - Command input section
//! - Staged files section
//! - Working files section
//! - History section (commits/reflog)
//! - Branches section

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::{
    app::App,
    section::{Section, SectionState},
    tui::Frame,
};

/// Render the main body panel
pub fn render(frame: &mut Frame<'_>, app: &App, area: Rect) {
    render_main_panel(frame, app, area);
}

/// Render the unified main panel (command + staged + working + activity)
fn render_main_panel(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let staged_changes = app.status.staged_changes();
    let working_changes = app.status.working_changes();
    let staged_len = staged_changes.len();
    let working_len = working_changes.len();
    let files_total = staged_len + working_len;
    let activity_len = app.activity.len();

    let mut items: Vec<ListItem<'_>> = Vec::new();

    // Command section - delegate to CommandSection::render()
    // Command is always at index 0
    let cmd_selected = app.selected == Some(0) && !app.menu_stack.is_active();
    let command_state = SectionState {
        is_focused: cmd_selected,
        local_selection: if cmd_selected { Some(0) } else { None },
        global_selection: app.selected,
        render_width: area.width,
    };

    for line in app.command_section.render(&command_state) {
        items.push(ListItem::new(line));
    }

    // Staged section - delegate to StagedSection::render()
    if staged_len > 0 {
        // Build section state: staged items are at indices [1, 1+staged_len)
        let in_staged = app.selected.is_some_and(|s| s >= 1 && s < 1 + staged_len);
        let local_selection = if in_staged {
            app.selected.map(|s| s - 1) // Convert global to local (subtract command offset)
        } else {
            None
        };
        let staged_state = SectionState {
            is_focused: in_staged,
            local_selection,
            global_selection: app.selected,
            render_width: area.width,
        };

        for line in app.staged_section.render(&staged_state) {
            items.push(ListItem::new(line));
        }
    }

    // Working section - delegate to WorkingSection::render()
    if working_len > 0 {
        if staged_len > 0 {
            items.push(ListItem::new(Line::from("")));
        }

        // Build section state: working items are at indices [1+staged_len, 1+staged_len+working_len)
        let working_start = 1 + staged_len;
        let in_working = app
            .selected
            .is_some_and(|s| s >= working_start && s < working_start + working_len);
        let local_selection = if in_working {
            app.selected.map(|s| s - working_start) // Convert global to local
        } else {
            None
        };
        let working_state = SectionState {
            is_focused: in_working,
            local_selection,
            global_selection: app.selected,
            render_width: area.width,
        };

        for line in app.working_section.render(&working_state) {
            items.push(ListItem::new(line));
        }
    }

    // Track current index for selection (after files)
    let mut current_idx = 1 + files_total;

    // History section - delegate to HistorySection::render()
    if !app.activity.is_empty() || !app.history_collapsed {
        if files_total > 0 {
            items.push(ListItem::new(Line::from("")));
        }

        // Build section state for history
        // History section starts at index (1 + files_total)
        // Item count is 1 (header) + activity.len() when not collapsed
        let history_start = 1 + files_total;
        let history_item_count = if app.history_collapsed {
            1
        } else {
            1 + activity_len
        };
        let history_end = history_start + history_item_count;

        let in_history = app
            .selected
            .is_some_and(|s| s >= history_start && s < history_end && !app.is_command_mode());
        let local_selection = if in_history {
            app.selected.map(|s| s - history_start)
        } else {
            None
        };

        let history_state = SectionState {
            is_focused: in_history,
            local_selection,
            global_selection: app.selected,
            render_width: area.width,
        };

        for line in app.history_section.render(&history_state) {
            items.push(ListItem::new(line));
        }

        current_idx = history_end;
    }

    // Branches section - delegate to BranchesSection::render()
    let other_branches: Vec<_> = app.branches.iter().filter(|b| !b.is_current).collect();
    if !other_branches.is_empty() {
        // Add spacer if there's content above
        if files_total > 0 || !app.activity.is_empty() {
            items.push(ListItem::new(Line::from("")));
        }

        // Build section state for branches
        // Branches section selectable items are the expanded branch commits
        let branches_start = current_idx;
        let branches_item_count = app.branches_section.item_count();
        let branches_end = branches_start + branches_item_count;

        let in_branches = app
            .selected
            .is_some_and(|s| s >= branches_start && s < branches_end && !app.is_command_mode());
        let local_selection = if in_branches {
            app.selected.map(|s| s - branches_start)
        } else {
            None
        };

        let branches_state = SectionState {
            is_focused: in_branches,
            local_selection,
            global_selection: app.selected,
            render_width: area.width,
        };

        for line in app.branches_section.render(&branches_state) {
            items.push(ListItem::new(line));
        }
    }

    // Empty state for files (command section is always shown)
    if files_total == 0 && app.activity.is_empty() && app.branches.is_empty() {
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
