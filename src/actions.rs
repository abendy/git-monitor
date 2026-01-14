//! Contextual action menu framework.
//!
//! Provides context-aware actions that change based on cursor position,
//! supporting app actions, CLI commands, and git aliases.

use crate::config::Alias;

/// Application context - represents where the user currently is
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Context {
    /// On command input section
    Command,
    /// Selection is in staged files
    StagedFiles,
    /// Selection is in working files
    WorkingFiles,
    /// On the History section header
    HistoryHeader,
    /// On a commit in history
    HistoryCommits,
    /// On a file within expanded commit
    CommitFiles,
    /// On a commit in expanded branch
    BranchCommits,
    /// Actions available everywhere
    Global,
}

impl Context {
    /// Display name for the context
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Command => "Command",
            Self::StagedFiles => "Staged Files",
            Self::WorkingFiles => "Working Files",
            Self::HistoryHeader => "History",
            Self::HistoryCommits => "History",
            Self::CommitFiles => "Commit Files",
            Self::BranchCommits => "Branch",
            Self::Global => "Global",
        }
    }
}

/// Type of action
#[derive(Debug, Clone)]
pub enum ActionType {
    /// Built-in application action (e.g., stage, diff)
    App(AppAction),
    /// Custom CLI command
    Cli(String),
    /// Git alias from gitconfig
    Alias(Alias),
}

/// Built-in app actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    // File actions
    /// Stage or unstage the selected file
    ToggleStage,
    /// Show diff for selected file
    ShowDiff,

    // History actions
    /// Toggle between log and reflog
    ToggleHistoryMode,
    /// Expand selected commit to show files
    ExpandCommit,
    /// Copy short SHA to clipboard
    CopyShortSha,
    /// Copy full SHA to clipboard
    CopyFullSha,
    /// Next page in history
    NextPage,
    /// Previous page in history
    PrevPage,

    // Commit file actions
    /// Open diff in pager
    PagerDiff,
    /// Show inline diff in popup
    InlineDiff,
    /// Open diff in external difftool
    DiffTool,

    // Branch actions
    /// Checkout the selected branch/commit
    Checkout,
    /// Expand branch to show commits
    ExpandBranch,

    // Global actions
    /// Push current branch
    Push,
    /// Refresh git status
    Refresh,
    /// Enter command mode
    EnterCommandMode,
    /// Browse git aliases
    BrowseAliases,
    /// Show help overlay
    ShowHelp,
    /// Quit the application
    Quit,

    // Navigation
    /// Jump to working files section
    JumpToWorking,
    /// Jump to history section
    JumpToHistory,
    /// Jump to branches section
    JumpToBranches,
}

/// A single action definition
#[derive(Debug, Clone)]
pub struct Action {
    /// Keyboard shortcut (e.g., "s", "Space", "Ctrl+d")
    pub key: String,
    /// Display label for the action
    pub label: String,
    /// Type of action
    pub action_type: ActionType,
    /// Contexts where this action is available
    pub contexts: Vec<Context>,
    /// Priority for display ordering (lower = higher priority)
    pub priority: u8,
}

impl Action {
    /// Create a new app action
    fn app(key: &str, label: &str, action: AppAction, contexts: Vec<Context>, priority: u8) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            action_type: ActionType::App(action),
            contexts,
            priority,
        }
    }

    /// Create a new alias action
    #[must_use]
    pub fn from_alias(alias: &Alias, contexts: Vec<Context>) -> Self {
        Self {
            key: format!(":{}", alias.name),
            label: alias.command.clone(),
            action_type: ActionType::Alias(alias.clone()),
            contexts,
            priority: 100, // Aliases show after built-in actions
        }
    }
}

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
    pub fn new() -> Self {
        let mut actions = Vec::new();

        // --- File actions (staged + working) ---
        let file_contexts = vec![Context::StagedFiles, Context::WorkingFiles];
        actions.push(Action::app(
            "s",
            "stage/unstage",
            AppAction::ToggleStage,
            file_contexts.clone(),
            10,
        ));
        actions.push(Action::app(
            "d",
            "diff",
            AppAction::ShowDiff,
            file_contexts,
            20,
        ));

        // --- History actions ---
        actions.push(Action::app(
            "h",
            "log/reflog",
            AppAction::ToggleHistoryMode,
            vec![Context::HistoryCommits],
            10,
        ));
        actions.push(Action::app(
            "Space",
            "expand",
            AppAction::ExpandCommit,
            vec![Context::HistoryCommits, Context::BranchCommits],
            20,
        ));
        actions.push(Action::app(
            "[",
            "prev page",
            AppAction::PrevPage,
            vec![Context::HistoryHeader, Context::HistoryCommits],
            30,
        ));
        actions.push(Action::app(
            "]",
            "next page",
            AppAction::NextPage,
            vec![Context::HistoryHeader, Context::HistoryCommits],
            31,
        ));
        actions.push(Action::app(
            "c",
            "copy sha",
            AppAction::CopyFullSha,
            vec![Context::HistoryCommits, Context::BranchCommits],
            40,
        ));

        // --- Commit file actions ---
        let commit_file_ctx = vec![Context::CommitFiles];
        actions.push(Action::app(
            "Space",
            "pager",
            AppAction::PagerDiff,
            commit_file_ctx.clone(),
            10,
        ));
        actions.push(Action::app(
            "d",
            "inline diff",
            AppAction::InlineDiff,
            commit_file_ctx.clone(),
            20,
        ));
        actions.push(Action::app("M", "difftool", AppAction::DiffTool, commit_file_ctx, 30));

        // --- Branch actions ---
        actions.push(Action::app(
            "Enter",
            "checkout",
            AppAction::Checkout,
            vec![Context::BranchCommits],
            10,
        ));

        // --- Global actions (available everywhere) ---
        actions.push(Action::app(
            "P",
            "push",
            AppAction::Push,
            vec![Context::Global],
            50,
        ));
        actions.push(Action::app(
            "r",
            "refresh",
            AppAction::Refresh,
            vec![Context::Global],
            51,
        ));
        actions.push(Action::app(
            ":",
            "command",
            AppAction::EnterCommandMode,
            vec![Context::Global],
            52,
        ));
        actions.push(Action::app(
            "a",
            "aliases",
            AppAction::BrowseAliases,
            vec![Context::Global],
            53,
        ));
        actions.push(Action::app(
            "?",
            "help",
            AppAction::ShowHelp,
            vec![Context::Global],
            54,
        ));
        actions.push(Action::app(
            "q",
            "quit",
            AppAction::Quit,
            vec![Context::Global],
            55,
        ));

        // --- Navigation (global) ---
        actions.push(Action::app(
            "w",
            "working",
            AppAction::JumpToWorking,
            vec![Context::Global],
            60,
        ));
        actions.push(Action::app(
            "h",
            "history",
            AppAction::JumpToHistory,
            vec![Context::Global],
            61,
        ));
        actions.push(Action::app(
            "b",
            "branches",
            AppAction::JumpToBranches,
            vec![Context::Global],
            62,
        ));

        Self { actions }
    }

    /// Get actions for a specific context (includes Global actions)
    #[must_use]
    pub fn actions_for_context(&self, context: Context) -> Vec<&Action> {
        let mut actions: Vec<_> = self
            .actions
            .iter()
            .filter(|a| a.contexts.contains(&context) || a.contexts.contains(&Context::Global))
            .collect();
        actions.sort_by_key(|a| a.priority);
        actions
    }

    /// Get inline hint actions for a context (limited set for display)
    /// Returns only context-specific actions, not global ones
    #[must_use]
    pub fn hint_actions_for_context(&self, context: Context) -> Vec<&Action> {
        let mut actions: Vec<_> = self
            .actions
            .iter()
            .filter(|a| {
                // Only context-specific actions, not global
                a.contexts.contains(&context)
                    && !a.contexts.contains(&Context::Global)
                    && a.priority < 80
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
            vec![Context::BranchCommits]
        } else if cmd.contains("checkout") && !cmd.contains("branch") {
            // Checkout can apply to files or branches
            vec![Context::WorkingFiles, Context::BranchCommits]
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
