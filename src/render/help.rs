//! Help overlay rendering.
//!
//! Displays keybinding reference as a centered popup.

use std::collections::HashSet;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::actions::{Action, ActionType, AppAction, Context};
use crate::app::App;
use crate::tui::Frame;

/// Render help overlay
pub fn render(frame: &mut Frame<'_>, area: Rect, app: &App) {
    // Center the help box
    let help_area = super::centered_rect(60, 70, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, help_area);

    let registry = &app.action_registry;
    let state = app.app_state();

    let global_actions = registry.actions_for_context(Context::Global, &state);

    let navigation_actions = filter_actions(
        &global_actions,
        &[
            AppAction::JumpToWorking,
            AppAction::JumpToHistory,
            AppAction::JumpToBranches,
        ],
    );

    let command_actions = filter_actions(
        &global_actions,
        &[AppAction::EnterCommandMode, AppAction::BrowseAliases],
    );

    let general_actions = filter_actions(
        &global_actions,
        &[
            AppAction::Push,
            AppAction::Pull,
            AppAction::Fetch,
            AppAction::Refresh,
            AppAction::ShowHelp,
            AppAction::Quit,
        ],
    );

    let history_actions =
        registry.hint_actions_for_context(Context::HistoryCommits, &state);
    let file_actions =
        registry.hint_actions_for_context(Context::StagedFiles, &state);
    let commit_file_actions =
        registry.hint_actions_for_context(Context::CommitFiles, &state);

    let branch_header_actions =
        registry.hint_actions_for_context(Context::BranchHeader, &state);
    let branch_commit_actions =
        registry.hint_actions_for_context(Context::BranchCommits, &state);
    let branch_actions = merge_actions(&[branch_header_actions, branch_commit_actions]);

    let mut help_text = Vec::new();

    push_section(
        &mut help_text,
        "Navigation",
        {
            let mut lines = vec![
                Line::from("  j / ↓              Move down"),
                Line::from("  k / ↑              Move up"),
                Line::from("  g                  Go to first"),
                Line::from("  G                  Go to last"),
            ];
            for action in navigation_actions {
                let label = format!("Jump to {}", action.label);
                lines.push(action_line(action, Some(&label)));
            }
            lines
        },
    );

    push_section(
        &mut help_text,
        "Staged/Working Files",
        action_lines(&file_actions, None),
    );

    push_section(
        &mut help_text,
        "History",
        action_lines(&history_actions, None),
    );

    push_section(
        &mut help_text,
        "Commit Files",
        action_lines(&commit_file_actions, None),
    );

    push_section(
        &mut help_text,
        "Branches",
        action_lines(&branch_actions, None),
    );

    push_section(
        &mut help_text,
        "Command & Aliases",
        {
            let mut lines = action_lines(&command_actions, None);
            lines.extend([
                Line::from("  o                  Expand output popup"),
                Line::from("  Enter              Execute command"),
                Line::from("  Esc / Ctrl+C       Cancel"),
                Line::from("  Up / Down          Navigate history"),
            ]);
            lines
        },
    );

    push_section(
        &mut help_text,
        "General",
        {
            let mut lines = vec![Line::from("  m                  Action menu")];
            lines.extend(action_lines(&general_actions, None));
            lines
        },
    );

    help_text.push(Line::from(""));
    help_text.push(Line::from(Span::styled(
        "Press any key to close",
        Style::default()
            .fg(Color::DarkGray)
            .italic(),
    )));

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

fn push_section(help_text: &mut Vec<Line<'static>>, title: &str, lines: Vec<Line<'static>>) {
    help_text.push(Line::from(""));
    help_text.push(section_title(title));
    help_text.extend(lines);
}

fn section_title(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        title.to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    ))
}

fn action_lines(
    actions: &[&Action],
    label_override: Option<&str>,
) -> Vec<Line<'static>> {
    actions
        .iter()
        .map(|action| action_line(action, label_override))
        .collect()
}

fn action_line(action: &Action, label_override: Option<&str>) -> Line<'static> {
    let label = label_override.unwrap_or(&action.label);
    Line::from(format!("  {:<6} {}", action.key, label))
}

fn filter_actions<'a>(
    actions: &'a [&'a Action],
    wanted: &[AppAction],
) -> Vec<&'a Action> {
    actions
        .iter()
        .copied()
        .filter(|action| match action.action_type {
            ActionType::App(app_action) => wanted.contains(&app_action),
            _ => false,
        })
        .collect()
}

fn merge_actions<'a>(groups: &[Vec<&'a Action>]) -> Vec<&'a Action> {
    let mut merged = Vec::new();
    let mut seen = HashSet::new();

    for actions in groups {
        for action in actions {
            let key = (action.key.clone(), action.label.clone());
            if seen.insert(key) {
                merged.push(*action);
            }
        }
    }

    merged.sort_by_key(|action| action.priority);
    merged
}
