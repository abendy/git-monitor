//! Shared commit rendering helpers for History and Branches sections.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::actions::{ActionRegistry, ActionType, AppAction, AppState, Context};
use crate::git::{format_relative_time, CommandType, CommitDetail, GitCommand, RefDecoration};
use crate::render::file_list::{render_file_entry, FileEntryView, FileListStyle};
use crate::section::SectionLines;

/// Below this panel width the commit graph moves left to leave room for text
const NARROW_WIDTH: u16 = 80;

/// Message columns to keep before extra refs fold into "+N"
const MIN_MESSAGE_WIDTH: usize = 20;

#[allow(clippy::too_many_lines)] // Render functions are naturally verbose
pub fn render_commit_line(
    cmd: &GitCommand,
    selected: bool,
    is_last: bool,
    render_width: u16,
) -> Line<'static> {
    let selection_prefix = if selected { "▸" } else { " " };
    let local_indent = if render_width < NARROW_WIDTH {
        " "
    } else {
        "   "
    };
    let (indent, graph_char) = if cmd.is_remote_only {
        ("", "├—")
    } else if is_last {
        (local_indent, "└─")
    } else {
        (local_indent, "├─")
    };

    let time_str = format_relative_time(cmd.timestamp);
    let icon = cmd.command_type.icon();
    let color = commit_command_color(cmd.command_type);
    let sha_str = cmd.sha.as_deref().unwrap_or("-------");

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

    // Panel borders take one column on each side
    let prefix_width: usize = spans.iter().map(Span::width).sum();
    let available = usize::from(render_width).saturating_sub(2 + prefix_width);

    // Show every ref when the message keeps enough room; otherwise fold extras into "+N"
    let mut decoration_spans = format_decorations(&cmd.decorations, false);
    let full_width = spans_width(&decoration_spans);
    if full_width > 0 && available.saturating_sub(full_width + 1) < MIN_MESSAGE_WIDTH {
        decoration_spans = format_decorations(&cmd.decorations, true);
    }
    let decoration_width = spans_width(&decoration_spans);
    let max_msg_len = if decoration_width == 0 {
        available
    } else {
        available.saturating_sub(decoration_width + 1)
    };
    let message = truncate_message(&cmd.message, max_msg_len);

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

#[allow(clippy::too_many_lines)] // Render functions are naturally verbose
pub fn render_commit_detail(
    detail: &CommitDetail,
    expanded_file_idx: Option<usize>,
    action_registry: Option<&ActionRegistry>,
    app_state: Option<&AppState>,
) -> SectionLines {
    let mut lines = SectionLines::default();

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

            lines.push_marked(
                render_file_entry(&entry, &file_style, is_file_selected),
                is_file_selected,
            );
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
        CommandType::Commit | CommandType::Clone | CommandType::Init => Color::Green,
        CommandType::Checkout | CommandType::Branch => Color::Cyan,
        CommandType::Merge | CommandType::CherryPick => Color::Magenta,
        CommandType::Rebase | CommandType::Stash => Color::Yellow,
        CommandType::Pull | CommandType::Push | CommandType::Fetch => Color::Blue,
        CommandType::Reset | CommandType::Revert => Color::Red,
        CommandType::Other => Color::DarkGray,
    }
}

/// Render refs as `(HEAD → branch, other, ...)`.
///
/// With `collapse`, only the leading ref stays (`HEAD → branch` when HEAD is
/// here) and the rest become a `+N` count.
fn format_decorations(decorations: &[RefDecoration], collapse: bool) -> Vec<Span<'static>> {
    let gray = Style::default().fg(Color::DarkGray);
    let has_head = decorations
        .iter()
        .any(|dec| matches!(dec, RefDecoration::Head));
    let head_branch = if has_head {
        decorations
            .iter()
            .find_map(|dec| match dec {
                RefDecoration::LocalBranch(name) => Some(name.as_str()),
                _ => None,
            })
    } else {
        None
    };

    let mut entries: Vec<Vec<Span<'static>>> = Vec::new();
    if has_head {
        let head_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        entries.push(head_branch.map_or_else(
            || vec![Span::styled("HEAD", head_style)],
            |branch| {
                vec![
                    Span::styled("HEAD → ", head_style),
                    Span::styled(
                        branch.to_string(),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]
            },
        ));
    }

    for dec in decorations {
        match dec {
            RefDecoration::Head => {}
            RefDecoration::LocalBranch(name) if head_branch == Some(name.as_str()) => {}
            RefDecoration::LocalBranch(name) => entries.push(vec![Span::styled(
                name.clone(),
                Style::default().fg(Color::Green),
            )]),
            RefDecoration::RemoteBranch(name) => entries.push(vec![Span::styled(
                name.clone(),
                Style::default().fg(Color::Red),
            )]),
            RefDecoration::Tag(name) => entries.push(vec![
                Span::styled("tag: ", gray),
                Span::styled(
                    name.clone(),
                    Style::default().fg(Color::Yellow),
                ),
            ]),
        }
    }

    if entries.is_empty() {
        return Vec::new();
    }

    let hidden = if collapse { entries.len() - 1 } else { 0 };
    let mut spans = vec![Span::styled("(", gray)];
    for (i, entry) in entries
        .into_iter()
        .take(if collapse { 1 } else { usize::MAX })
        .enumerate()
    {
        if i > 0 {
            spans.push(Span::styled(", ", gray));
        }
        spans.extend(entry);
    }
    if hidden > 0 {
        spans.push(Span::styled(
            format!(" +{hidden}"),
            gray,
        ));
    }
    spans.push(Span::styled(")", gray));
    spans
}

/// Display width of a run of spans
fn spans_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(Span::width).sum()
}

/// Display width of one character
fn char_width(c: char) -> usize {
    let mut buf = [0u8; 4];
    Span::raw(&*c.encode_utf8(&mut buf)).width()
}

/// Fit `message` into `max_width` display columns, ending in "..." when cut
fn truncate_message(message: &str, max_width: usize) -> String {
    if message
        .chars()
        .map(char_width)
        .sum::<usize>()
        <= max_width
    {
        return message.to_string();
    }
    if max_width <= 3 {
        return String::new();
    }

    let budget = max_width - 3;
    let mut used = 0;
    let mut out: String = message
        .chars()
        .take_while(|&c| {
            used += char_width(c);
            used <= budget
        })
        .collect();
    out.push_str("...");
    out
}

fn editor_available() -> bool {
    std::env::var("EDITOR")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use chrono::Local;

    use super::*;

    fn commit(message: &str, decorations: Vec<RefDecoration>) -> GitCommand {
        GitCommand {
            timestamp: Local::now(),
            command_type: CommandType::Commit,
            message: message.to_string(),
            sha: Some("e29d83a".to_string()),
            decorations,
            is_remote_only: false,
        }
    }

    fn head_on_develop() -> Vec<RefDecoration> {
        vec![
            RefDecoration::Head,
            RefDecoration::LocalBranch("develop".to_string()),
            RefDecoration::RemoteBranch("origin/develop".to_string()),
        ]
    }

    fn text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    const MESSAGE: &str = "docs: record the repository snapshot design";

    #[test]
    fn narrow_line_keeps_message_and_folds_extra_refs() {
        let line = render_commit_line(
            &commit(MESSAGE, head_on_develop()),
            false,
            false,
            66,
        );
        let text = text(&line);

        assert!(
            text.contains("docs: record"),
            "message shown: {text}"
        );
        assert!(
            text.contains("(HEAD → develop +1)"),
            "refs folded: {text}"
        );
        assert!(
            !text.contains("origin/develop"),
            "remote hidden: {text}"
        );
        assert!(
            line.width() <= 64,
            "fits inside borders: {text}"
        );
    }

    #[test]
    fn wide_line_shows_every_ref() {
        let line = render_commit_line(
            &commit(MESSAGE, head_on_develop()),
            false,
            false,
            140,
        );
        let text = text(&line);

        assert!(
            text.contains(MESSAGE),
            "full message: {text}"
        );
        assert!(
            text.contains("(HEAD → develop, origin/develop)"),
            "all refs: {text}"
        );
    }

    #[test]
    fn commit_without_head_keeps_its_first_ref() {
        let refs = vec![
            RefDecoration::RemoteBranch("origin/feature".to_string()),
            RefDecoration::Tag("v1.0.0".to_string()),
        ];
        let text = text(&render_commit_line(
            &commit(MESSAGE, refs),
            false,
            false,
            60,
        ));

        assert!(
            text.contains("(origin/feature +1)"),
            "first ref kept: {text}"
        );
    }

    #[test]
    fn narrow_panel_moves_graph_two_columns_left() {
        let narrow = text(&render_commit_line(
            &commit(MESSAGE, Vec::new()),
            false,
            false,
            66,
        ));
        let wide = text(&render_commit_line(
            &commit(MESSAGE, Vec::new()),
            false,
            false,
            100,
        ));

        assert_eq!(
            narrow.find('├'),
            Some(3),
            "narrow: {narrow}"
        );
        assert_eq!(wide.find('├'), Some(5), "wide: {wide}");
    }

    #[test]
    fn truncation_respects_multibyte_characters() {
        let line = render_commit_line(
            &commit(
                "fix: gérer les accents — ça marche 🎉 vraiment",
                Vec::new(),
            ),
            false,
            false,
            40,
        );

        assert!(
            text(&line).ends_with("..."),
            "{}",
            text(&line)
        );
        assert!(
            line.width() <= 38,
            "fits: {}",
            text(&line)
        );
    }
}
