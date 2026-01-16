//! Branches section.
//!
//! Displays non-current branches with expandable commit history.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::{Section, SectionAction, SectionId, SectionKeybinding, SectionState};
use crate::actions::{ActionRegistry, AppAction, AppState, Context};
use crate::git::{BranchInfo, CommitDetail, GitCommand};
use crate::input::KeyBinding;
use crate::section::{
    render_commit_detail, render_commit_line, render_context_hint,
};

/// Data needed by the branches section for rendering
#[derive(Debug, Clone, Default)]
pub struct BranchesSectionData {
    /// All branches (will filter out current branch for display)
    pub branches: Vec<BranchInfo>,
    /// Currently expanded branch name (if any)
    pub expanded_branch: Option<String>,
    /// Commits for the expanded branch
    pub expanded_branch_commits: Vec<GitCommand>,
    /// Currently expanded commit SHA within branch (if any)
    pub expanded_commit: Option<String>,
    /// Detail for expanded commit
    pub expanded_detail: Option<CommitDetail>,
    /// Index of selected file in expanded commit detail
    pub expanded_file_idx: Option<usize>,
    /// Whether command mode is active (suppresses selection highlight)
    pub command_mode_active: bool,
    /// Action registry for hints
    pub action_registry: Option<ActionRegistry>,
    /// App state for action conditions
    pub app_state: Option<AppState>,
}

/// Branches section
pub struct BranchesSection {
    data: BranchesSectionData,
}

impl BranchesSection {
    /// Create a new branches section
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: BranchesSectionData::default(),
        }
    }

    /// Update section data from app state
    pub fn update(&mut self, data: BranchesSectionData) {
        self.data = data;
    }

    /// Get branches to display (excludes current branch)
    fn other_branches(&self) -> Vec<&BranchInfo> {
        self.data
            .branches
            .iter()
            .filter(|b| !b.is_current)
            .collect()
    }

    /// Find the index of the expanded branch within the other-branches list.
    fn expanded_branch_index(&self, branches: &[&BranchInfo]) -> Option<usize> {
        let expanded = self.data.expanded_branch.as_deref()?;
        branches.iter().position(|b| b.name == expanded)
    }

    /// Render context hints for branch actions
    fn render_hints(&self, context: Context) -> Line<'static> {
        let Some(ref registry) = self.data.action_registry else {
            return Line::from("");
        };
        let Some(ref state) = self.data.app_state else {
            return Line::from("");
        };

        render_context_hint(registry, context, state)
    }
}

impl Default for BranchesSection {
    fn default() -> Self {
        Self::new()
    }
}

impl Section for BranchesSection {
    fn id(&self) -> SectionId {
        SectionId::Branches
    }

    fn name(&self) -> &str {
        "Branches"
    }

    fn contexts(&self) -> Vec<Context> {
        vec![Context::BranchHeader, Context::BranchCommits]
    }

    fn item_count(&self) -> usize {
        // Branches section items = branch headers + expanded branch commits.
        let branches = self.other_branches();
        let header_count = branches.len();
        if header_count == 0 {
            return 0;
        }

        let commit_count = if self.expanded_branch_index(&branches).is_some() {
            self.data.expanded_branch_commits.len()
        } else {
            0
        };

        header_count + commit_count
    }

    fn render(&self, state: &SectionState) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        let other_branches = self.other_branches();
        if other_branches.is_empty() {
            return lines;
        }

        let in_section = state.is_focused && !self.data.command_mode_active;
        let arrow = if in_section { "▾" } else { "▸" };

        // Section header (not selectable)
        lines.push(Line::from(Span::styled(
            format!(
                "  {arrow} Branches ({})",
                other_branches.len()
            ),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        let expanded_idx = self.expanded_branch_index(&other_branches);
        let commit_len = if expanded_idx.is_some() {
            self.data.expanded_branch_commits.len()
        } else {
            0
        };
        let commit_start = expanded_idx.map(|idx| idx + 1);

        // Hints (only when focused)
        if in_section {
            let hint_context = state.local_selection.map_or(Context::BranchHeader, |local_index| {
                if let Some(expanded_idx) = expanded_idx {
                    let commit_start = expanded_idx + 1;
                    let commit_end = commit_start + commit_len;
                    if (commit_start..commit_end).contains(&local_index) {
                        Context::BranchCommits
                    } else {
                        Context::BranchHeader
                    }
                } else {
                    Context::BranchHeader
                }
            });
            lines.push(self.render_hints(hint_context));
        }

        // Branch entries - branch names shown, commits under expanded branch
        for (branch_idx, branch) in other_branches.iter().enumerate() {
            let header_local_index = if let Some(expanded_idx) = expanded_idx {
                if branch_idx > expanded_idx {
                    branch_idx + commit_len
                } else {
                    branch_idx
                }
            } else {
                branch_idx
            };
            let header_selected = state.local_selection == Some(header_local_index) && in_section;
            let prefix = if header_selected { "▸ " } else { "  " };
            let is_expanded = self.data.expanded_branch.as_deref() == Some(&branch.name);

            // Branch name color: remote=red, local=white
            let branch_color = if branch.is_remote {
                Color::Red
            } else {
                Color::White
            };

            let branch_style = if header_selected {
                Style::default()
                    .fg(branch_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(branch_color)
            };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(branch.name.clone(), branch_style),
            ]));

            // If this branch is expanded, show its commits
            if is_expanded {
                let Some(commit_start) = commit_start else {
                    continue;
                };
                for (i, cmd) in self
                    .data
                    .expanded_branch_commits
                    .iter()
                    .enumerate()
                {
                    let local_index = commit_start + i;
                    let selected = state.local_selection == Some(local_index) && in_section;
                    let is_last = i + 1 == commit_len;
                    lines.push(render_commit_line(
                        cmd,
                        selected,
                        is_last,
                        state.render_width,
                    ));

                    // If this commit is expanded, render detail lines
                    if self.data.expanded_commit.as_ref() == cmd.sha.as_ref() {
                        if let Some(ref detail) = self.data.expanded_detail {
                            lines.extend(render_commit_detail(
                                detail,
                                self.data.expanded_file_idx,
                                self.data.action_registry.as_ref(),
                                self.data.app_state.as_ref(),
                            ));
                        }
                    }
                }
            }
        }

        lines
    }

    fn actions(&self, item_idx: usize) -> Vec<crate::actions::Action> {
        // Actions are provided by ActionRegistry based on context
        let _ = item_idx;
        Vec::new()
    }

    fn handle_key(&self, key: KeyEvent, item_idx: usize) -> Option<SectionAction> {
        // item_idx maps to expanded_branch_commits
        if item_idx >= self.data.expanded_branch_commits.len() {
            return None;
        }

        match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Expand/collapse commit detail
                Some(SectionAction::AppAction(
                    AppAction::ExpandCommit,
                ))
            }
            KeyCode::Char('c') => {
                // Checkout commit
                Some(SectionAction::AppAction(
                    AppAction::Checkout,
                ))
            }
            KeyCode::Char('y') => {
                // Copy short SHA
                Some(SectionAction::AppAction(
                    AppAction::CopyShortSha,
                ))
            }
            _ => None,
        }
    }

    fn keybindings(&self) -> Vec<SectionKeybinding> {
        vec![
            SectionKeybinding::new(
                KeyBinding::char(' '),
                "Expand",
                "Expand/collapse branch commits",
            ),
            SectionKeybinding::new(
                KeyBinding::key(KeyCode::Enter),
                "Checkout",
                "Checkout branch or commit",
            ),
            SectionKeybinding::new(
                KeyBinding::char('y'),
                "Copy SHA",
                "Copy short commit SHA",
            ),
            SectionKeybinding::new(
                KeyBinding::char('c'),
                "Copy SHA",
                "Copy commit SHA",
            ),
        ]
    }
}
