//! Shared commit rendering helpers for History and Branches sections.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::actions::{ActionRegistry, ActionType, AppAction, AppState, Context};
use crate::git::{format_relative_time, CommandType, CommitDetail, GitCommand, RefDecoration};
use crate::render::file_list::{render_file_entry, FileEntryView, FileListStyle};

pub fn render_commit_line(
    cmd: &GitCommand,
    selected: bool,
    is_last: bool,
    render_width: u16,
) -> Line<'static> {
    let selection_prefix = if selected { "▸" } else { " " };
    let (indent, graph_char) = if cmd.is_remote_only {
        ("", "├—")
    } else if is_last {
        ("   ", "└─")
    } else {
        ("   ", "├─")
    };

    let time_str = format_relative_time(cmd.timestamp);
    let icon = cmd.command_type.icon();
    let color = commit_command_color(cmd.command_type);
    let sha_str = cmd.sha.as_deref().unwrap_or("-------");

    let decoration_spans = format_decorations(&cmd.decorations);
    let decoration_width: usize = decoration_spans
        .iter()
        .map(|s| s.content.len())
        .sum();

    let fixed_width = selection_prefix.len()
        + 1
        + indent.len()
        + graph_char.len()
        + 1
        + sha_str.len()
        + 1
        + time_str.len()
        + 2
        + icon.len()
        + 1
        + if decoration_spans.is_empty() {
            0
        } else {
            1 + decoration_width
        };
    let max_msg_len = render_width.saturating_sub(fixed_width as u16) as usize;
    let message = truncate_message(&cmd.message, max_msg_len);

    let style = if selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let (graph_color, sha_color, msg_color) = if cmd.is_remote_only {
        (Color::Red, Color::Red, Color::DarkGray)
    } else {
        (
            Color::DarkGray,
            Color::Yellow,
            Color::White,
        )
    };

    let mut spans = vec![
        Span::raw(format!("{selection_prefix} {indent}")),
        Span::styled(
            format!("{graph_char} "),
            Style::default().fg(graph_color),
        ),
        Span::styled(
            format!("{sha_str} "),
            style.fg(sha_color),
        ),
        Span::styled(
            format!("{time_str}  "),
            style.fg(Color::DarkGray),
        ),
        Span::styled(format!("{icon} "), style.fg(color)),
    ];

    let special_prefixes = ["fixup!", "squash!", "amend!"];
    let wip_prefixes = ["WIP", "wip:", "wip ", "WIP:", "WIP "];

    if let Some(prefix) = special_prefixes
        .iter()
        .find(|p| message.starts_with(*p))
    {
        spans.push(Span::styled(
            prefix.to_string(),
            style
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            message[prefix.len()..].to_string(),
            style.fg(msg_color),
        ));
    } else if let Some(prefix) = wip_prefixes
        .iter()
        .find(|p| message.starts_with(*p))
    {
        spans.push(Span::styled(
            prefix.to_string(),
            style
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            message[prefix.len()..].to_string(),
            style.fg(msg_color),
        ));
    } else {
        spans.push(Span::styled(
            message,
            style.fg(msg_color),
        ));
    }

    if !decoration_spans.is_empty() {
        spans.push(Span::raw(" "));
        spans.extend(decoration_spans);
    }

    Line::from(spans)
}

pub fn render_commit_detail(
    detail: &CommitDetail,
    expanded_file_idx: Option<usize>,
    action_registry: Option<&ActionRegistry>,
    app_state: Option<&AppState>,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    lines.push(Line::raw(""));

    lines.push(Line::from(vec![
        Span::raw("    Author:    "),
        Span::styled(
            detail.author_name.clone(),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" <"),
        Span::styled(
            detail.author_email.clone(),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(">"),
    ]));

    if detail.committer_name != detail.author_name || detail.committer_email != detail.author_email
    {
        lines.push(Line::from(vec![
            Span::raw("    Committer: "),
            Span::styled(
                detail.committer_name.clone(),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" <"),
            Span::styled(
                detail.committer_email.clone(),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(">"),
        ]));
    }

    lines.push(Line::from(vec![
        Span::raw("    Date:      "),
        Span::styled(
            detail
                .author_time
                .format("%Y-%m-%d %H:%M:%S %z")
                .to_string(),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw("    Commit:    "),
        Span::styled(
            detail.full_sha.clone(),
            Style::default().fg(Color::Yellow),
        ),
    ]));

    if let Some(gpg) = &detail.gpg_status {
        lines.push(Line::from(vec![
            Span::raw("    GPG:       "),
            Span::styled(
                gpg.clone(),
                Style::default().fg(Color::Magenta),
            ),
        ]));
    }

    lines.push(Line::raw(""));

    for msg_line in detail.message.lines() {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::raw(msg_line.to_string()),
        ]));
    }

    if !detail.files.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled(
                format!(
                    "    {} file(s) changed  ",
                    detail.files.len()
                ),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("+{}", detail.insertions),
                Style::default().fg(Color::Green),
            ),
            Span::styled(
                " / ",
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("-{}", detail.deletions),
                Style::default().fg(Color::Red),
            ),
        ]));

        if expanded_file_idx.is_some() {
            if let (Some(registry), Some(state)) = (action_registry, app_state) {
                let hint = render_context_hint(registry, Context::CommitFiles, state);
                let mut spans = vec![Span::raw("  ")];
                spans.extend(hint.spans);
                lines.push(Line::from(spans));
            }
        }

        // Commit files use extra indentation
        let file_style = FileListStyle {
            indicator: None,
            selected_prefix: "  ▸ ",
            unselected_prefix: "    ",
        };

        for (file_idx, file) in detail.files.iter().enumerate() {
            let is_file_selected = expanded_file_idx == Some(file_idx);

            let entry = FileEntryView {
                path: &file.path,
                status: file.status,
                insertions: file.insertions,
                deletions: file.deletions,
            };

            lines.push(render_file_entry(
                &entry,
                &file_style,
                is_file_selected,
            ));
        }
    }

    lines.push(Line::raw(""));

    lines
}

pub fn render_context_hint(
    registry: &ActionRegistry,
    context: Context,
    state: &AppState,
) -> Line<'static> {
    let mut actions = registry.hint_actions_for_context(context, state);

    if editor_available() {
        if let Some(action) = registry
            .actions_for_context(context, state)
            .into_iter()
            .find(|action| {
                matches!(
                    action.action_type,
                    ActionType::App(AppAction::OpenEditor)
                )
            })
        {
            if !actions
                .iter()
                .any(|a| a.action_type == action.action_type)
            {
                actions.push(action);
                actions.sort_by_key(|a| a.priority);
            }
        }
    }

    if actions.is_empty() {
        return Line::from("");
    }

    let mut spans = vec![Span::raw("  ")];

    for (i, action) in actions.iter().take(5).enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                " · ",
                Style::default().fg(Color::DarkGray),
            ));
        }
        spans.push(Span::styled(
            format!("{} ", action.key),
            Style::default().fg(Color::Cyan),
        ));
        spans.push(Span::styled(
            action.label.clone(),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));
    }

    Line::from(spans)
}

pub const fn commit_command_color(cmd: CommandType) -> Color {
    match cmd {
        CommandType::Commit => Color::Green,
        CommandType::Checkout => Color::Cyan,
        CommandType::Merge => Color::Magenta,
        CommandType::Rebase => Color::Yellow,
        CommandType::Pull => Color::Blue,
        CommandType::Push => Color::Blue,
        CommandType::Reset => Color::Red,
        CommandType::CherryPick => Color::Magenta,
        CommandType::Revert => Color::Red,
        CommandType::Branch => Color::Cyan,
        CommandType::Clone | CommandType::Init => Color::Green,
        CommandType::Fetch => Color::Blue,
        CommandType::Stash => Color::Yellow,
        CommandType::Other => Color::DarkGray,
    }
}

fn format_decorations(decorations: &[RefDecoration]) -> Vec<Span<'static>> {
    if decorations.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::new();
    spans.push(Span::styled(
        "(",
        Style::default().fg(Color::DarkGray),
    ));

    let mut first = true;
    let mut has_head = false;
    let mut head_branch: Option<&str> = None;

    for dec in decorations {
        if matches!(dec, RefDecoration::Head) {
            has_head = true;
        }
    }

    if has_head {
        for dec in decorations {
            if let RefDecoration::LocalBranch(name) = dec {
                head_branch = Some(name);
                break;
            }
        }
    }

    if has_head {
        if let Some(branch) = head_branch {
            spans.push(Span::styled(
                "HEAD → ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::styled(
                branch.to_string(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
            first = false;
        } else {
            spans.push(Span::styled(
                "HEAD",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            first = false;
        }
    }

    for dec in decorations {
        if matches!(dec, RefDecoration::Head) {
            continue;
        }
        if let RefDecoration::LocalBranch(name) = dec {
            if head_branch == Some(name) {
                continue;
            }
        }

        if !first {
            spans.push(Span::styled(
                ", ",
                Style::default().fg(Color::DarkGray),
            ));
        }
        first = false;

        match dec {
            RefDecoration::LocalBranch(name) => {
                spans.push(Span::styled(
                    name.clone(),
                    Style::default().fg(Color::Green),
                ));
            }
            RefDecoration::RemoteBranch(name) => {
                spans.push(Span::styled(
                    name.clone(),
                    Style::default().fg(Color::Red),
                ));
            }
            RefDecoration::Tag(name) => {
                spans.push(Span::styled(
                    "tag: ",
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(
                    name.clone(),
                    Style::default().fg(Color::Yellow),
                ));
            }
            RefDecoration::Head => {}
        }
    }

    spans.push(Span::styled(
        ")",
        Style::default().fg(Color::DarkGray),
    ));
    spans
}

fn truncate_message(message: &str, max_len: usize) -> String {
    if max_len <= 3 {
        return String::new();
    }

    if message.len() > max_len {
        format!(
            "{}...",
            &message[..max_len.saturating_sub(3)]
        )
    } else {
        message.to_string()
    }
}

fn editor_available() -> bool {
    std::env::var("EDITOR")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}
