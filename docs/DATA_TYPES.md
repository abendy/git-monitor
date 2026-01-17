# Git Monitor - Data Types Reference

## Core Types

### File Status (`src/git.rs`)

```rust
/// Status of a single file in the repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStatus {
    /// Path relative to repository root
    pub path: PathBuf,
    /// Status in working directory
    pub working: FileState,
    /// Status in staging area (index)
    pub staged: FileState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileState {
    #[default]
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Ignored,
    Conflicted,
}

impl FileState {
    /// Character representation (like git status --short)
    pub const fn as_char(self) -> char;

    /// Whether this state represents a change
    pub const fn is_changed(self) -> bool;
}
```

**Status Characters:**

| State | Char | Description |
|-------|------|-------------|
| Unmodified | ` ` | No changes |
| Modified | `M` | Content changed |
| Added | `A` | New file staged |
| Deleted | `D` | File deleted |
| Renamed | `R` | File renamed |
| Untracked | `?` | Not in git |
| Ignored | `!` | In .gitignore |
| Conflicted | `U` | Merge conflict |

### Repository State (`src/git.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepoState {
    #[default]
    Normal,
    Merge,
    Rebase,
    RebaseInteractive,
    CherryPick,
    Revert,
    Bisect,
}
```

Converted from `git2::RepositoryState`.

### Git Status (`src/git.rs`)

```rust
/// Complete git status snapshot
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    /// Current branch name (None if detached HEAD)
    pub branch: Option<String>,
    /// Upstream branch name if tracking
    pub upstream: Option<String>,
    /// Commits ahead of upstream
    pub ahead: usize,
    /// Commits behind upstream
    pub behind: usize,
    /// Files with changes
    pub files: Vec<FileStatus>,
    /// Repository state
    pub state: RepoState,
}

impl GitStatus {
    /// Files changed in working directory (not staged)
    pub fn working_changes(&self) -> Vec<&FileStatus>;

    /// Files staged for commit
    pub fn staged_changes(&self) -> Vec<&FileStatus>;
}
```

### Git Commands / Activity (`src/git.rs`)

```rust
/// A git command from the reflog
#[derive(Debug, Clone)]
pub struct GitCommand {
    /// When the command was executed
    pub timestamp: DateTime<Local>,
    /// Type of command
    pub command_type: CommandType,
    /// Command message/description from reflog
    pub message: String,
    /// Short SHA if available
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
    Reset,
    CherryPick,
    Revert,
    Branch,
    Clone,
    Init,
    Fetch,
    Stash,
    Other,
}

impl CommandType {
    /// Parse command type from reflog message
    pub fn from_message(msg: &str) -> Self;

    /// Icon for display in activity log
    pub const fn icon(self) -> &'static str;
}
```

**Command Icons:**

| Command | Icon |
|---------|------|
| Commit | ● |
| Checkout | ⎇ |
| Merge | ⑂ |
| Rebase | ↺ |
| Pull | ↓ |
| Push | ↑ |
| Fetch | ⟳ |
| Reset | ↩ |
| CherryPick | ❋ |
| Revert | ⊗ |
| Stash | □ |
| Branch | ⌥ |
| Clone | ⊕ |
| Init | ★ |
| Other | • |

### Git Repository Wrapper (`src/git.rs`)

```rust
/// Git repository wrapper around git2::Repository
pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    pub fn open(path: &Path) -> Result<Self>;
    pub fn workdir(&self) -> Option<&Path>;
    pub fn status(&self) -> Result<GitStatus>;
    pub fn stage(&self, path: &Path) -> Result<()>;
    pub fn unstage(&self, path: &Path) -> Result<()>;
    pub fn diff_file(&self, path: &Path, staged: bool) -> Result<String>;
    pub fn reflog(&self, limit: usize) -> Result<Vec<GitCommand>>;
}
```

## UI Types

### Panel (`src/app.rs`)

```rust
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    #[default]
    Working,
    Staged,
    Activity,
}

impl Panel {
    pub const fn next(self) -> Self;
    pub const fn prev(self) -> Self;
}
```

### Application State (`src/app.rs`)

```rust
pub struct App {
    /// Path to the repository
    pub repo_path: PathBuf,
    /// Git repository handle
    repo: GitRepo,
    /// Current git status
    pub status: GitStatus,
    /// Recent git activity (up to MAX_ACTIVITY = 50)
    pub activity: Vec<GitCommand>,
    /// File watcher
    watcher: Option<RepoWatcher>,
    /// Whether the application is running
    pub running: bool,
    /// Currently active panel
    pub active_panel: Panel,
    /// Selected index in working panel
    pub working_selected: usize,
    /// Selected index in staged panel
    pub staged_selected: usize,
    /// Show help overlay
    pub show_help: bool,
    /// Show diff overlay
    pub show_diff: bool,
    /// Current diff content
    pub diff_content: String,
    /// Diff file path (for title)
    pub diff_path: String,
    /// Error message to display
    pub error: Option<String>,
}
```

## Event Types

### Terminal Events (`src/event.rs`)

```rust
#[derive(Debug, Clone)]
pub enum Event {
    /// Terminal tick (for periodic updates)
    Tick,
    /// Keyboard input
    Key(KeyEvent),
    /// Mouse input
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// File system change detected
    FileChanged,
}
```

### Event Handler (`src/event.rs`)

```rust
pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self;
    pub fn sender(&self) -> mpsc::Sender<Event>;
    pub fn next(&self) -> Result<Event>;
}
```

### Watch Events (`src/watcher.rs`)

```rust
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// Working directory file changed
    WorkingDirectory(PathBuf),
    /// Git index (staging area) changed
    GitIndex,
    /// Git HEAD changed (branch switch, commit)
    GitHead,
    /// Git refs changed
    GitRefs,
}
```

## TUI Types

### Terminal Wrapper (`src/tui.rs`)

```rust
pub type Frame<'a> = ratatui::Frame<'a>;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    pub events: EventHandler,
}

impl Tui {
    pub fn new(tick_rate: u64) -> Result<Self>;
    pub fn enter(&mut self) -> Result<()>;
    pub fn exit(&mut self) -> Result<()>;
    pub fn draw<F>(&mut self, render: F) -> Result<()>
    where
        F: FnOnce(&mut Frame<'_>);
}
```

## Mapping from git2 Types

Working directory state extraction:
```rust
fn working_state_from_git2(status: Status) -> FileState {
    if status.is_wt_new() { FileState::Untracked }
    else if status.is_wt_modified() { FileState::Modified }
    else if status.is_wt_deleted() { FileState::Deleted }
    else if status.is_wt_renamed() { FileState::Renamed }
    else if status.is_conflicted() { FileState::Conflicted }
    else { FileState::Unmodified }
}
```

Staged (index) state extraction:
```rust
fn staged_state_from_git2(status: Status) -> FileState {
    if status.is_index_new() { FileState::Added }
    else if status.is_index_modified() { FileState::Modified }
    else if status.is_index_deleted() { FileState::Deleted }
    else if status.is_index_renamed() { FileState::Renamed }
    else { FileState::Unmodified }
}
```
