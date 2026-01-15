//! Branches section.
//!
//! Displays non-current branches with expandable commit history.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::{
    actions::{ActionRegistry, AppAction, AppState, Context},
    git::{BranchInfo, CommitDetail, GitCommand},
};

use super::{Section, SectionAction, SectionId, SectionState};

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

    /// Render context hints for branch actions
    fn render_hints(&self) -> Line<'static> {
        let Some(ref registry) = self.data.action_registry else {
            return Line::from("");
        };
        let Some(ref state) = self.data.app_state else {
            return Line::from("");
        };

        let actions = registry.hint_actions_for_context(Context::BranchCommits, state);

        if actions.is_empty() {
            return Line::from("");
        }

        let mut spans = vec![Span::raw("  ")];
        let italic_gray = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC);

        for (i, action) in actions.iter().take(5).enumerate() {
            if i > 0 {
                spans.push(Span::styled(" · ", Style::default().fg(Color::DarkGray)));
            }
            spans.push(Span::styled(
                format!("{} ", action.key),
                Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::styled(action.label.clone(), italic_gray));
        }

        Line::from(spans)
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
        vec![Context::BranchCommits]
    }

    fn item_count(&self) -> usize {
        // Branches section items = expanded branch commits (branches themselves aren't selectable)
        if self.data.expanded_branch.is_some() {
            self.data.expanded_branch_commits.len()
        } else {
            0
        }
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
            format!("  {arrow} Branches ({})", other_branches.len()),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));

        // Hints (only when focused)
        if in_section {
            lines.push(self.render_hints());
        }

        // Branch entries - branch names shown, commits under expanded branch
        for branch in &other_branches {
            let is_expanded = self.data.expanded_branch.as_deref() == Some(&branch.name);

            // Branch name color: remote=red, local=white
            let branch_color = if branch.is_remote {
                Color::Red
            } else {
                Color::White
            };

            lines.push(Line::from(Span::styled(
                format!("  {}", branch.name),
                Style::default().fg(branch_color),
            )));

            // Expanded branch commits - rendering delegated to ui.rs for now
            // The section tracks the data, ui.rs handles the actual commit line rendering
            if is_expanded {
                // Placeholder - commit rendering happens in ui.rs
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
                Some(SectionAction::AppAction(AppAction::ExpandCommit))
            }
            KeyCode::Char('c') => {
                // Checkout commit
                Some(SectionAction::AppAction(AppAction::Checkout))
            }
            KeyCode::Char('y') => {
                // Copy short SHA
                Some(SectionAction::AppAction(AppAction::CopyShortSha))
            }
            _ => None,
        }
    }
}
