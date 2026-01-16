use std::path::PathBuf;

use crate::actions::ActionRegistry;
use crate::command::{CommandExecutor, CommandHistory, ExternalCommand};
use crate::config::GitConfig;
use crate::feedback::FeedbackManager;
use crate::git::{BranchInfo, CommitDetail, GitCommand, GitRepo, GitStatus};
use crate::input::Keymap;
use crate::menu::MenuStack;
use crate::section::{
    BranchesSection, CommandSection, HistorySection, SectionRegistry, StagedSection,
    WorkingSection,
};
use crate::watcher::RepoWatcher;

mod actions;
mod branches;
mod commands;
mod history;
mod init;
mod input;
mod key_handlers;
mod navigation;
mod runtime;
mod sections;

/// Page size for history pagination
const PAGE_SIZE: usize = 50;

/// History display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryMode {
    /// Show reflog (git actions)
    Reflog,
    /// Show commit log
    #[default]
    CommitLog,
}

/// View mode for the application body
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Normal navigation mode (default)
    #[default]
    Normal,
    /// Command input mode (typing git commands)
    Command,
}

/// Application state
pub struct App {
    /// Path to the repository
    pub repo_path: PathBuf,
    /// Git repository handle
    repo: GitRepo,
    /// Current git status
    pub status: GitStatus,
    /// Recent git activity
    pub activity: Vec<GitCommand>,
    /// Git config with aliases
    pub config: GitConfig,
    /// File watcher
    #[allow(dead_code)]
    watcher: Option<RepoWatcher>,
    /// Whether the application is running
    pub running: bool,
    /// Selected index in main list (None = nothing focused, Some(0) = command, etc.)
    pub selected: Option<usize>,
    /// Show help overlay
    pub show_help: bool,
    /// Current view mode
    pub view_mode: ViewMode,
    /// Current command input buffer
    pub command_input: String,
    /// Command history (managed by CommandHistory module)
    pub command_history: CommandHistory,
    /// Current history display mode (reflog vs commit log)
    pub history_mode: HistoryMode,
    /// Feedback manager (handles output, popups, toasts, errors)
    pub feedback: FeedbackManager,
    /// SHA of currently expanded commit in history (None = collapsed)
    pub expanded_commit: Option<String>,
    /// Cached detail for expanded commit
    pub expanded_detail: Option<CommitDetail>,
    /// Selected file index within expanded commit (None = on commit header)
    pub expanded_file_idx: Option<usize>,
    /// List of local branches
    pub branches: Vec<BranchInfo>,
    /// Whether the History section (current branch) is collapsed
    pub history_collapsed: bool,
    /// Current page offset for history pagination (0-indexed)
    pub history_page: usize,
    /// Total history items for current mode
    pub history_total_items: usize,
    /// Total pages for history pagination
    pub history_total_pages: usize,
    /// Which non-current branch is expanded (showing its commits)
    pub expanded_branch: Option<String>,
    /// Cached commits for the expanded branch
    pub expanded_branch_commits: Vec<GitCommand>,
    /// Pending external command (requires TUI suspension)
    pub pending_external: Option<ExternalCommand>,
    /// Action registry for contextual actions
    pub action_registry: ActionRegistry,
    /// Command executor for running commands
    executor: CommandExecutor,
    /// Menu stack for modal dialogs
    pub menu_stack: MenuStack,
    /// Declarative keymap for action lookup
    keymap: Keymap,
    // ─────────────────────────────────────────────────────────────────────────
    // Section instances (for modular architecture)
    // ─────────────────────────────────────────────────────────────────────────
    /// Command input section
    pub command_section: CommandSection,
    /// Staged files section
    pub staged_section: StagedSection,
    /// Working files section
    pub working_section: WorkingSection,
    /// History section (commits/reflog)
    pub history_section: HistorySection,
    /// Branches section
    pub branches_section: BranchesSection,
    /// Section registry for index calculations
    section_registry: SectionRegistry,
}
