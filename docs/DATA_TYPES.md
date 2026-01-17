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

### Ref Decorations (`src/git.rs`)

```rust
/// A decoration (branch, tag, etc.) attached to a commit
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefDecoration {
    Head,                       // HEAD pointer
    LocalBranch(String),        // Local branch (e.g., main)
    RemoteBranch(String),       // Remote branch (e.g., origin/main)
    Tag(String),                // Tag
}
```

### Commit Detail (`src/git.rs`)

```rust
/// Detailed commit information for expanded view
#[derive(Debug, Clone)]
pub struct CommitDetail {
    /// Full 40-character SHA
    pub full_sha: String,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Author timestamp
    pub author_time: DateTime<Local>,
    /// Committer name
    pub committer_name: String,
    /// Committer email
    pub committer_email: String,
    /// Committer timestamp
    pub committer_time: DateTime<Local>,
    /// Full commit message (summary + body)
    pub message: String,
    /// GPG signature status (if signed)
    pub gpg_status: Option<String>,
    /// Files changed in this commit
    pub files: Vec<CommitFile>,
    /// Total lines added
    pub insertions: usize,
    /// Total lines deleted
    pub deletions: usize,
}

/// A file changed in a commit
#[derive(Debug, Clone)]
pub struct CommitFile {
    /// File path
    pub path: String,
    /// Change status
    pub status: FileState,
    /// Lines added in this file
    pub insertions: usize,
    /// Lines deleted in this file
    pub deletions: usize,
}
```

### Branch Info (`src/git.rs`)

```rust
/// Information about a local branch
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Whether this is the current (checked out) branch
    pub is_current: bool,
}
```

### Git Commands / Activity (`src/git.rs`)

```rust
/// A git command from the reflog or commit from log
#[derive(Debug, Clone)]
pub struct GitCommand {
    /// When the command was executed
    pub timestamp: DateTime<Local>,
    /// Type of command
    pub command_type: CommandType,
    /// Command message/description
    pub message: String,
    /// Short SHA if available
    pub sha: Option<String>,
    /// Decorations (branches, tags) pointing to this commit
    pub decorations: Vec<RefDecoration>,
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
    pub fn commit_log(&self, limit: usize) -> Result<Vec<GitCommand>>;
}
```

## Config Types

### Git Config (`src/config.rs`)

```rust
/// Git configuration with aliases grouped by section
#[derive(Debug, Clone, Default)]
pub struct GitConfig {
    pub sections: Vec<AliasSection>,
}

/// A section/category of aliases
#[derive(Debug, Clone)]
pub struct AliasSection {
    pub name: String,
    pub aliases: Vec<Alias>,
}

/// A single git alias
#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
    pub command: String,
}
```

Aliases are parsed from `~/.gitconfig`. Section headers are detected from comments like `# --- section ---`.

## UI Types

### View Mode (`src/app.rs`)

```rust
/// View mode for the application body
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Normal navigation mode (default)
    #[default]
    Normal,
    /// Command input mode (typing git commands)
    Command,
    /// Alias browser showing section list
    AliasSections {
        /// Currently selected section index
        selected: usize,
    },
    /// Alias browser showing aliases within a section
    AliasItems {
        /// Section being viewed
        section_idx: usize,
        /// Currently selected alias index within section
        selected: usize,
    },
}
```

### History Mode (`src/app.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryMode {
    Reflog,       // Show reflog (git actions)
    #[default]
    CommitLog,    // Show commit log
}
```

### External Command (`src/app.rs`)

```rust
/// External command requiring TUI suspension
#[derive(Debug, Clone)]
pub enum ExternalCommand {
    /// Show commit file diff with pager (uses core.pager from gitconfig)
    PagerDiff { commit_sha: String, file_path: String },
    /// Show commit file diff with difftool (uses diff.tool from gitconfig)
    DiffTool { commit_sha: String, file_path: String },
}
```

### Popup Content (`src/app.rs`)

```rust
#[derive(Debug, Clone, Default)]
pub enum PopupContent {
    #[default]
    None,
    CommandOutput {
        command: String,
        output: String,
        success: bool,
    },
    Diff {
        path: String,
        content: String,
        is_staged: bool,
    },
}

impl PopupContent {
    pub fn is_active(&self) -> bool;
    pub fn line_count(&self) -> usize;
    pub fn title(&self) -> String;
}
```

### Popup State (`src/app.rs`)

```rust
#[derive(Debug, Clone, Default)]
pub struct PopupState {
    pub content: PopupContent,
    pub scroll_offset: usize,
    pub visible_height: usize,
}

impl PopupState {
    pub fn open(&mut self, content: PopupContent);
    pub fn close(&mut self);
    pub fn is_open(&self) -> bool;
    pub fn scroll_down(&mut self, n: usize, max_lines: usize, visible_height: usize);
    pub fn scroll_up(&mut self, n: usize);
    pub fn scroll_to_top(&mut self);
    pub fn scroll_to_bottom(&mut self, max_lines: usize, visible_height: usize);
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
    /// Git config with aliases
    pub config: GitConfig,
    /// File watcher
    watcher: Option<RepoWatcher>,
    /// Whether the application is running
    pub running: bool,
    /// Selected index (None = nothing focused)
    pub selected: Option<usize>,
    /// Show help overlay
    pub show_help: bool,
    /// Current view mode
    pub view_mode: ViewMode,
    /// Error message to display
    pub error: Option<String>,
    // Command mode fields
    pub command_input: String,
    pub command_output: String,
    pub command_success: bool,
    pub command_history: Vec<String>,
    pub history_index: Option<usize>,
    // History display
    pub history_mode: HistoryMode,
    // Popup state
    pub popup: PopupState,
    // Commit expansion (history view)
    pub expanded_commit: Option<String>,
    pub expanded_detail: Option<CommitDetail>,
    pub expanded_file_idx: Option<usize>,
    // Branch browser
    pub branches: Vec<BranchInfo>,
    pub history_collapsed: bool,
    pub expanded_branch: Option<String>,
    pub expanded_branch_commits: Vec<GitCommand>,
    // External command handling
    pub pending_external: Option<ExternalCommand>,
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
