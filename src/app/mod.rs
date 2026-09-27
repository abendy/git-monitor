use std::path::PathBuf;

use crate::actions::ActionRegistry;
use crate::command::{CommandExecutor, CommandHistory, ExternalCommand};
use crate::config::GitConfig;
use crate::feedback::FeedbackManager;
pub use crate::git::HistoryMode;
use crate::git::{CommitDetail, GitRepo, RepoSnapshot};
use crate::input::Keymap;
use crate::menu::MenuStack;
use crate::section::{
    BranchesSection, CommandSection, FileListSection, HistorySection, SectionRegistry,
};
use crate::watcher::RepoWatcher;

mod actions;
mod branches;
mod commands;
mod history;
mod init;
mod input;
mod navigation;
mod runtime;
mod sections;
mod selection;

/// Page size for history pagination
const PAGE_SIZE: usize = 50;

/// Where the cursor lands after turning a history page
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageLanding {
    /// Where it was the last time this page was shown, else the first commit
    Remembered,
    /// The first commit on the page
    First,
    /// The last commit on the page
    Last,
}

/// A held `J`/`K`: direction, time of the last repeat, and repeats so far
#[derive(Debug, Clone, Copy)]
pub(crate) struct SectionHold {
    down: bool,
    at: std::time::Instant,
    streak: u32,
}

/// Cursor spot remembered for one history page
#[derive(Debug, Clone)]
pub(crate) struct PageCursor {
    /// Selected commit
    sha: String,
    /// Body scroll offset at the time
    scroll: usize,
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
    /// Git data from the latest refresh (replaced whole, never edited in place)
    snapshot: RepoSnapshot,
    /// Git config with aliases
    pub config: GitConfig,
    /// File watcher
    #[allow(dead_code)]
    watcher: Option<RepoWatcher>,
    /// Whether the application is running
    pub running: bool,
    /// Selected index in main list (None = nothing focused, Some(0) = command, etc.)
    pub selected: Option<usize>,
    /// First visible row of the main panel, kept between frames for scrolling
    pub body_scroll: usize,
    /// Show help overlay
    pub show_help: bool,
    /// Current view mode
    pub view_mode: ViewMode,
    /// Current command input buffer
    pub command_input: String,
    /// Draft command to restore after browsing history
    command_draft: Option<String>,
    /// Command history (managed by `CommandHistory` module)
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
    /// Whether the History section (current branch) is collapsed
    pub history_collapsed: bool,
    /// Requested history page (0-indexed); synced to the page the snapshot loaded
    pub history_page: usize,
    /// Last `J`/`K` repeat, for speeding up a held key
    section_hold: Option<SectionHold>,
    /// Cursor spot per history page, so paging back returns to it
    history_page_cursors: std::collections::HashMap<(HistoryMode, usize), PageCursor>,
    /// Which non-current branch is expanded (showing its commits)
    pub expanded_branch: Option<String>,
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
    pub staged_section: FileListSection,
    /// Working files section
    pub working_section: FileListSection,
    /// History section (commits/reflog)
    pub history_section: HistorySection,
    /// Branches section
    pub branches_section: BranchesSection,
    /// Section registry for index calculations
    section_registry: SectionRegistry,
}

impl App {
    /// Git data from the latest refresh
    #[must_use]
    pub const fn snapshot(&self) -> &RepoSnapshot {
        &self.snapshot
    }
}
