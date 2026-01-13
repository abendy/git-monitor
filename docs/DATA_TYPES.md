# Git Monitor - Data Types Reference

## Core Types

### File Status

```rust
/// Status of a single file in the repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStatus {
    /// Path relative to repository root
    pub path: PathBuf,
    /// Status in working directory
    pub working_status: FileState,
    /// Status in staging area (index)
    pub index_status: FileState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileState {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Conflicted,
}

impl FileState {
    /// Character representation (like git status --short)
    pub fn as_char(&self) -> char {
        match self {
            Self::Unmodified => ' ',
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Copied => 'C',
            Self::Untracked => '?',
            Self::Ignored => '!',
            Self::Conflicted => 'U',
        }
    }

    /// Color for TUI display
    pub fn color(&self) -> Color {
        match self {
            Self::Modified => Color::Yellow,
            Self::Added => Color::Green,
            Self::Deleted => Color::Red,
            Self::Renamed => Color::Cyan,
            Self::Untracked => Color::Gray,
            Self::Conflicted => Color::Magenta,
            _ => Color::White,
        }
    }
}
```

### Git Status

```rust
/// Complete git status snapshot
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    /// Current branch name (None if detached HEAD)
    pub branch: Option<String>,
    /// Upstream branch name if tracking
    pub upstream: Option<String>,
    /// Commits ahead of upstream
    pub ahead: u32,
    /// Commits behind upstream
    pub behind: u32,
    /// Files with changes in working directory or index
    pub files: Vec<FileStatus>,
    /// Repository state (normal, merging, rebasing, etc.)
    pub state: RepoState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepoState {
    #[default]
    Normal,
    Merging,
    Rebasing,
    CherryPicking,
    Reverting,
    Bisecting,
}

impl GitStatus {
    /// Files changed in working directory (not staged)
    pub fn working_changes(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| f.working_status != FileState::Unmodified)
            .collect()
    }

    /// Files staged for commit
    pub fn staged_changes(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| f.index_status != FileState::Unmodified)
            .collect()
    }

    /// Untracked files
    pub fn untracked(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| f.working_status == FileState::Untracked)
            .collect()
    }
}
```

### Git Commands (Activity)

```rust
/// A detected git command from reflog or observation
#[derive(Debug, Clone)]
pub struct GitCommand {
    /// When the command was executed
    pub timestamp: DateTime<Local>,
    /// Type of command
    pub command_type: CommandType,
    /// Full command string if available
    pub command: String,
    /// Additional details (commit message, branch name, etc.)
    pub details: Option<String>,
    /// SHA involved (for commits, checkouts)
    pub sha: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    Commit,
    Checkout,
    Merge,
    Rebase,
    Pull,
    Push,
    Fetch,
    Reset,
    CherryPick,
    Revert,
    Stash,
    Branch,
    Tag,
    Clone,
    Init,
    Other,
}

impl CommandType {
    pub fn from_reflog_message(msg: &str) -> Self {
        let msg_lower = msg.to_lowercase();
        if msg_lower.starts_with("commit") {
            Self::Commit
        } else if msg_lower.starts_with("checkout") {
            Self::Checkout
        } else if msg_lower.starts_with("merge") {
            Self::Merge
        } else if msg_lower.starts_with("rebase") {
            Self::Rebase
        } else if msg_lower.starts_with("pull") {
            Self::Pull
        } else if msg_lower.starts_with("reset") {
            Self::Reset
        } else if msg_lower.starts_with("cherry-pick") {
            Self::CherryPick
        } else if msg_lower.starts_with("revert") {
            Self::Revert
        } else if msg_lower.starts_with("branch") {
            Self::Branch
        } else if msg_lower.starts_with("clone") {
            Self::Clone
        } else if msg_lower.starts_with("init") {
            Self::Init
        } else {
            Self::Other
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Commit => "●",
            Self::Checkout => "⎇",
            Self::Merge => "⑂",
            Self::Rebase => "↺",
            Self::Pull => "↓",
            Self::Push => "↑",
            Self::Fetch => "⟳",
            Self::Reset => "↩",
            Self::CherryPick => "🍒",
            Self::Revert => "⊗",
            Self::Stash => "📦",
            Self::Branch => "⌥",
            Self::Tag => "🏷",
            _ => "•",
        }
    }
}
```

### Application State

```rust
/// Main application state
pub struct App {
    /// Path to repository root
    pub repo_path: PathBuf,

    /// Current git status
    pub status: GitStatus,

    /// Recent git activity
    pub activity: VecDeque<GitCommand>,

    /// UI state
    pub ui: UiState,

    /// Is the application running
    pub running: bool,

    /// Last status refresh time
    pub last_refresh: Instant,

    /// Pending error message to display
    pub error: Option<String>,
}

/// UI-specific state
pub struct UiState {
    /// Currently focused panel
    pub active_panel: Panel,

    /// Selected index in working changes list
    pub working_selected: usize,

    /// Selected index in staged changes list
    pub staged_selected: usize,

    /// Scroll offset for activity log
    pub activity_scroll: usize,

    /// Whether help overlay is shown
    pub show_help: bool,

    /// Whether diff viewer is shown
    pub show_diff: bool,

    /// Content for diff viewer
    pub diff_content: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Panel {
    #[default]
    Working,
    Staged,
    Activity,
}

impl Panel {
    pub fn next(&self) -> Self {
        match self {
            Self::Working => Self::Staged,
            Self::Staged => Self::Activity,
            Self::Activity => Self::Working,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Working => Self::Activity,
            Self::Staged => Self::Working,
            Self::Activity => Self::Staged,
        }
    }
}
```

### Events

```rust
/// Events processed by the main loop
#[derive(Debug)]
pub enum AppEvent {
    /// Keyboard input
    Key(KeyEvent),

    /// Mouse input (if enabled)
    Mouse(MouseEvent),

    /// Terminal resized
    Resize(u16, u16),

    /// Timer tick for periodic updates
    Tick,

    /// File system change detected
    FileChanged(PathBuf),

    /// Git status was updated
    StatusUpdated(GitStatus),

    /// New git command detected
    CommandDetected(GitCommand),

    /// Error occurred
    Error(String),
}

/// Actions that can be performed
#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    Refresh,
    NextPanel,
    PrevPanel,
    SelectNext,
    SelectPrev,
    StageSelected,
    UnstageSelected,
    ShowDiff,
    HideDiff,
    ToggleHelp,
    ScrollUp,
    ScrollDown,
}
```

### Configuration

```rust
/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Repository path (defaults to current directory)
    pub repo_path: PathBuf,

    /// Tick rate for periodic updates (ms)
    pub tick_rate: u64,

    /// Debounce duration for file events (ms)
    pub debounce_ms: u64,

    /// Maximum activity entries to keep
    pub max_activity: usize,

    /// Enable mouse support
    pub mouse: bool,

    /// Color theme
    pub theme: Theme,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repo_path: PathBuf::from("."),
            tick_rate: 250,
            debounce_ms: 100,
            max_activity: 100,
            mouse: false,
            theme: Theme::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub modified: Color,
    pub added: Color,
    pub deleted: Color,
    pub untracked: Color,
    pub border: Color,
    pub highlight: Color,
}
```

## Mapping from git2 Types

```rust
impl From<git2::Status> for FileState {
    fn from(status: git2::Status) -> Self {
        if status.is_wt_new() {
            FileState::Untracked
        } else if status.is_wt_modified() {
            FileState::Modified
        } else if status.is_wt_deleted() {
            FileState::Deleted
        } else if status.is_wt_renamed() {
            FileState::Renamed
        } else if status.is_ignored() {
            FileState::Ignored
        } else if status.is_conflicted() {
            FileState::Conflicted
        } else {
            FileState::Unmodified
        }
    }
}

/// Extract index (staged) status from git2::Status
pub fn index_state_from_git2(status: git2::Status) -> FileState {
    if status.is_index_new() {
        FileState::Added
    } else if status.is_index_modified() {
        FileState::Modified
    } else if status.is_index_deleted() {
        FileState::Deleted
    } else if status.is_index_renamed() {
        FileState::Renamed
    } else {
        FileState::Unmodified
    }
}
```
