//! Action type definitions and builders.

use crate::config::Alias;
use crate::input::KeyBinding;

use super::context::Context;
use super::state::ActionCondition;

/// Type of action
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)] // Cli variant reserved for future extension
pub enum ActionType {
    /// Built-in application action (e.g., stage, diff)
    App(AppAction),
    /// Custom CLI command (for future use)
    Cli(String),
    /// Git alias from gitconfig
    Alias(Alias),
}

/// Built-in app actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Some variants reserved for future use
pub enum AppAction {
    // File actions
    /// Stage or unstage the selected file
    ToggleStage,
    /// Show diff for selected file
    ShowDiff,
    /// Open working/staged file diff in pager
    FilePagerDiff,
    /// Open working/staged file diff in external difftool
    FileDiffTool,
    /// Open selected file (or repo) in $EDITOR
    OpenEditor,

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
    /// Interactive rebase onto selected commit
    InteractiveRebase,

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

    // Remote actions
    /// Pull from remote
    Pull,
    /// Fetch from remote
    Fetch,
}

/// A single action definition
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_field_names)] // action_type is clear naming
pub struct Action {
    /// Keyboard shortcut (e.g., "s", "Space", "Ctrl+d")
    pub key: String,
    /// Structured binding for keymap lookup
    pub binding: Option<KeyBinding>,
    /// Display label for the action
    pub label: String,
    /// Type of action
    pub action_type: ActionType,
    /// Contexts where this action is available
    pub contexts: Vec<Context>,
    /// Priority for display ordering (lower = higher priority)
    pub priority: u8,
    /// Condition for when this action is available
    pub condition: ActionCondition,
}

impl Action {
    /// Create a new app action (always available)
    pub(super) fn app(
        binding: KeyBinding,
        label: &str,
        action: AppAction,
        contexts: Vec<Context>,
        priority: u8,
    ) -> Self {
        Self {
            key: binding.label(),
            binding: Some(binding),
            label: label.to_string(),
            action_type: ActionType::App(action),
            contexts,
            priority,
            condition: ActionCondition::Always,
        }
    }

    /// Create a new app action with a condition
    pub(super) fn app_when(
        binding: KeyBinding,
        label: &str,
        action: AppAction,
        contexts: Vec<Context>,
        priority: u8,
        condition: ActionCondition,
    ) -> Self {
        Self {
            key: binding.label(),
            binding: Some(binding),
            label: label.to_string(),
            action_type: ActionType::App(action),
            contexts,
            priority,
            condition,
        }
    }

    /// Create a new alias action
    #[must_use]
    pub fn from_alias(alias: &Alias, contexts: Vec<Context>) -> Self {
        Self {
            key: format!(":{}", alias.name),
            binding: None,
            label: alias.command.clone(),
            action_type: ActionType::Alias(alias.clone()),
            contexts,
            priority: 100, // Aliases show after built-in actions
            condition: ActionCondition::Always,
        }
    }
}
