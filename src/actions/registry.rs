//! Central registry of all actions.

use crossterm::event::KeyCode;

use crate::config::Alias;
use crate::input::KeyBinding;

use super::context::Context;
use super::state::{ActionCondition, AppState};
use super::types::{Action, AppAction};

/// Central registry of all actions
#[derive(Debug, Clone)]
pub struct ActionRegistry {
    actions: Vec<Action>,
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionRegistry {
    /// Build the action registry with all built-in actions
    #[must_use]
    #[allow(clippy::too_many_lines)] // Registry initialization is naturally verbose
    pub fn new() -> Self {
        let mut actions = Vec::new();

        // --- File actions (staged + working) ---
        let file_contexts = vec![Context::StagedFiles, Context::WorkingFiles];
        actions.push(Action::app(
            KeyBinding::char('s'),
            "stage/unstage",
            AppAction::ToggleStage,
            file_contexts.clone(),
            10,
        ));
        actions.push(Action::app(
            KeyBinding::char('d'),
            "diff",
            AppAction::ShowDiff,
            file_contexts.clone(),
            20,
        ));
        actions.push(Action::app(
            KeyBinding::key(KeyCode::Enter),
            "diff",
            AppAction::ShowDiff,
            file_contexts.clone(),
            21,
        ));
        actions.push(Action::app(
            KeyBinding::char(' '),
            "pager",
            AppAction::FilePagerDiff,
            file_contexts.clone(),
            30,
        ));
        actions.push(Action::app(
            KeyBinding::char('M'),
            "difftool",
            AppAction::FileDiffTool,
            file_contexts,
            40,
        ));

        // --- History actions ---
        actions.push(Action::app(
            KeyBinding::char('h'),
            "log/reflog",
            AppAction::ToggleHistoryMode,
            vec![Context::HistoryHeader, Context::HistoryCommits],
            10,
        ));
        actions.push(Action::app(
            KeyBinding::char(' '),
            "expand",
            AppAction::ExpandCommit,
            vec![Context::HistoryCommits, Context::BranchCommits],
            20,
        ));
        actions.push(Action::app(
            KeyBinding::char('['),
            "prev page",
            AppAction::PrevPage,
            vec![Context::HistoryHeader, Context::HistoryCommits],
            30,
        ));
        actions.push(Action::app(
            KeyBinding::char(']'),
            "next page",
            AppAction::NextPage,
            vec![Context::HistoryHeader, Context::HistoryCommits],
            31,
        ));
        actions.push(Action::app(
            KeyBinding::char('y'),
            "copy short",
            AppAction::CopyShortSha,
            vec![Context::HistoryCommits, Context::BranchCommits],
            40,
        ));
        actions.push(Action::app(
            KeyBinding::char('c'),
            "copy sha",
            AppAction::CopyFullSha,
            vec![Context::HistoryCommits, Context::BranchCommits],
            41,
        ));
        actions.push(Action::app(
            KeyBinding::char('R'),
            "rebase -i",
            AppAction::InteractiveRebase,
            vec![Context::HistoryCommits],
            50,
        ));

        // --- Commit file actions ---
        let commit_file_ctx = vec![Context::CommitFiles];
        actions.push(Action::app(
            KeyBinding::char(' '),
            "pager",
            AppAction::PagerDiff,
            commit_file_ctx.clone(),
            10,
        ));
        actions.push(Action::app(
            KeyBinding::char('d'),
            "inline diff",
            AppAction::InlineDiff,
            commit_file_ctx.clone(),
            20,
        ));
        actions.push(Action::app(
            KeyBinding::char('M'),
            "difftool",
            AppAction::DiffTool,
            commit_file_ctx,
            30,
        ));

        // --- Branch actions ---
        actions.push(Action::app(
            KeyBinding::key(KeyCode::Enter),
            "checkout",
            AppAction::Checkout,
            vec![Context::BranchHeader],
            10,
        ));
        actions.push(Action::app(
            KeyBinding::char(' '),
            "expand",
            AppAction::ExpandBranch,
            vec![Context::BranchHeader],
            20,
        ));

        // --- Global actions (available everywhere) ---
        // Push only shows when ahead of remote
        actions.push(Action::app_when(
            KeyBinding::char('P'),
            "push",
            AppAction::Push,
            vec![Context::Global],
            50,
            ActionCondition::BranchAhead,
        ));
        // Pull only shows when behind remote
        actions.push(Action::app_when(
            KeyBinding::char('p'),
            "pull",
            AppAction::Pull,
            vec![Context::Global],
            50,
            ActionCondition::BranchBehind,
        ));
        // Fetch always available (lowercase f)
        actions.push(Action::app(
            KeyBinding::char('f'),
            "fetch",
            AppAction::Fetch,
            vec![Context::Global],
            51,
        ));
        actions.push(Action::app(
            KeyBinding::char('r'),
            "refresh",
            AppAction::Refresh,
            vec![Context::Global],
            51,
        ));
        actions.push(Action::app(
            KeyBinding::char(':'),
            "command",
            AppAction::EnterCommandMode,
            vec![Context::Global],
            52,
        ));
        actions.push(Action::app(
            KeyBinding::char('a'),
            "aliases",
            AppAction::BrowseAliases,
            vec![Context::Global],
            53,
        ));
        actions.push(Action::app(
            KeyBinding::char('e'),
            "edit",
            AppAction::OpenEditor,
            vec![Context::Global],
            54,
        ));
        actions.push(Action::app(
            KeyBinding::char('?'),
            "help",
            AppAction::ShowHelp,
            vec![Context::Global],
            55,
        ));
        actions.push(Action::app(
            KeyBinding::char('q'),
            "quit",
            AppAction::Quit,
            vec![Context::Global],
            56,
        ));

        // --- Navigation (global) ---
        actions.push(Action::app(
            KeyBinding::char('w'),
            "working",
            AppAction::JumpToWorking,
            vec![Context::Global],
            60,
        ));
        actions.push(Action::app(
            KeyBinding::char('h'),
            "history",
            AppAction::JumpToHistory,
            vec![Context::Global],
            61,
        ));
        actions.push(Action::app(
            KeyBinding::char('b'),
            "branches",
            AppAction::JumpToBranches,
            vec![Context::Global],
            62,
        ));

        Self { actions }
    }

    #[must_use]
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    /// Get actions for a specific context (includes Global actions)
    /// Filters by both context and condition
    #[must_use]
    pub fn actions_for_context(&self, context: Context, state: &AppState) -> Vec<&Action> {
        let mut actions: Vec<_> = self
            .actions
            .iter()
            .filter(|a| {
                let context_matches =
                    a.contexts.contains(&context) || a.contexts.contains(&Context::Global);
                let condition_satisfied = state.satisfies(a.condition);
                context_matches && condition_satisfied
            })
            .collect();
        actions.sort_by_key(|a| a.priority);
        actions
    }

    /// Get inline hint actions for a context (limited set for display)
    /// Returns only context-specific actions, not global ones
    /// Filters by condition
    #[must_use]
    pub fn hint_actions_for_context(&self, context: Context, state: &AppState) -> Vec<&Action> {
        let mut actions: Vec<_> = self
            .actions
            .iter()
            .filter(|a| {
                // Only context-specific actions, not global
                let context_matches =
                    a.contexts.contains(&context) && !a.contexts.contains(&Context::Global);
                let condition_satisfied = state.satisfies(a.condition);
                context_matches && condition_satisfied && a.priority < 80
            })
            .collect();
        actions.sort_by_key(|a| a.priority);
        actions
    }

    /// Add alias-based actions based on command context inference
    pub fn add_alias_actions(&mut self, aliases: &[Alias]) {
        for alias in aliases {
            let contexts = Self::infer_alias_context(alias);
            if !contexts.is_empty() {
                self.actions.push(Action::from_alias(alias, contexts));
            }
        }
    }

    /// Infer which contexts an alias applies to based on its command
    fn infer_alias_context(alias: &Alias) -> Vec<Context> {
        let cmd = alias.command.to_lowercase();

        // Patterns for context inference
        if cmd.contains("stash") || cmd.contains("restore") {
            vec![Context::WorkingFiles, Context::StagedFiles]
        } else if cmd.contains("commit") || cmd.contains("amend") {
            vec![Context::StagedFiles]
        } else if cmd.contains("diff") && !cmd.contains("difftool") {
            vec![
                Context::StagedFiles,
                Context::WorkingFiles,
                Context::HistoryCommits,
            ]
        } else if cmd.contains("log") || cmd.contains("show") {
            vec![Context::HistoryCommits, Context::BranchCommits]
        } else if cmd.contains("branch") || cmd.contains("switch") {
            vec![Context::BranchHeader]
        } else if cmd.contains("checkout") && !cmd.contains("branch") {
            // Checkout can apply to files or branches
            vec![Context::WorkingFiles, Context::BranchHeader]
        } else if cmd.contains("push") || cmd.contains("pull") || cmd.contains("fetch") {
            vec![Context::Global]
        } else if cmd.contains("rebase") || cmd.contains("reset") {
            vec![Context::HistoryCommits]
        } else {
            // Default: show only in command section
            vec![Context::Command]
        }
    }
}
