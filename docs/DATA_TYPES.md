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

## Action Types (`src/actions.rs`)

### Context

```rust
/// Application context - represents where the user currently is
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Context {
    Command,         // On command input section
    StagedFiles,     // Selection is in staged files
    WorkingFiles,    // Selection is in working files
    HistoryHeader,   // On the History section header
    HistoryCommits,  // On a commit in history
    CommitFiles,     // On a file within expanded commit
    BranchCommits,   // On a commit in expanded branch
    Global,          // Actions available everywhere
}
```

### Action Types

```rust
/// Type of action
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionType {
    App(AppAction),   // Built-in application action
    Cli(String),      // Custom CLI command (future)
    Alias(Alias),     // Git alias from gitconfig
}

/// Built-in app actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    // File actions
    ToggleStage, ShowDiff, FilePagerDiff, FileDiffTool,
    // History actions
    ToggleHistoryMode, ExpandCommit, CopyShortSha, CopyFullSha, NextPage, PrevPage, InteractiveRebase,
    // Commit file actions
    PagerDiff, InlineDiff, DiffTool,
    // Branch actions
    Checkout, ExpandBranch,
    // Global actions
    Push, Pull, Fetch, Refresh, EnterCommandMode, BrowseAliases, ShowHelp, Quit,
    // Navigation
    JumpToWorking, JumpToHistory, JumpToBranches,
}
```

### Action Conditions

```rust
/// Conditions for when an action should be available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActionCondition {
    #[default]
    Always,              // Always available
    BranchAhead,         // Can push
    BranchBehind,        // Should pull
    HasUpstream,         // Has upstream configured
    NoUpstream,          // No upstream configured
    HasStagedChanges,    // Ready to commit
    HasWorkingChanges,   // Uncommitted changes
    HasUntrackedFiles,   // Untracked files exist
}

/// Snapshot of app state for evaluating conditions
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub ahead: usize,
    pub behind: usize,
    pub has_upstream: bool,
    pub staged_count: usize,
    pub working_count: usize,
    pub untracked_count: usize,
}
```

### Action Definition

```rust
/// A single action definition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub key: String,              // Keyboard shortcut
    pub label: String,            // Display label
    pub action_type: ActionType,  // Type of action
    pub contexts: Vec<Context>,   // Where available
    pub priority: u8,             // Display ordering
    pub condition: ActionCondition, // Availability condition
}
```

### Action Registry

```rust
/// Central registry of all actions
#[derive(Debug, Clone)]
pub struct ActionRegistry {
    actions: Vec<Action>,
}

impl ActionRegistry {
    pub fn new() -> Self;
    pub fn actions_for_context(&self, context: Context, state: &AppState) -> Vec<&Action>;
    pub fn hint_actions_for_context(&self, context: Context, state: &AppState) -> Vec<&Action>;
    pub fn add_alias_actions(&mut self, aliases: &[Alias]);
}
```

## Input Types (`src/input/`)

### KeyBinding

```rust
/// A key binding (key code + modifiers)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    pub const fn key(code: KeyCode) -> Self;
    pub const fn char(c: char) -> Self;
    pub const fn ctrl(code: KeyCode) -> Self;
    pub const fn ctrl_char(c: char) -> Self;
}
```

### Keymap

```rust
/// Declarative keymap that maps key bindings to actions
pub struct Keymap {
    global: HashMap<KeyBinding, AppAction>,
    contextual: HashMap<Context, HashMap<KeyBinding, AppAction>>,
}

impl Keymap {
    pub fn new() -> Self;
    pub fn with_defaults() -> Self;
    pub fn bind_global(&mut self, binding: KeyBinding, action: AppAction);
    pub fn bind(&mut self, context: Context, binding: KeyBinding, action: AppAction);
    pub fn lookup(&self, key: KeyEvent, context: Context) -> Option<AppAction>;
}
```

### InputResult

```rust
/// Result of processing a key event
pub enum InputResult {
    Action(AppAction),      // Execute an app-level action
    DelegateToMenu,         // Delegate to the active menu
    DelegateToPopup,        // Delegate to popup handler
    Unhandled,              // Key was not handled
}
```

## UI Types

### View Mode (`src/app.rs`)

```rust
/// View mode for the application body (simplified via ADR-005)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Normal navigation mode (default)
    #[default]
    Normal,
    /// Command input mode (typing git commands)
    Command,
}
```

Modal dialogs (action menu, alias browser, push confirmation) are now managed by `MenuStack` instead of ViewMode variants.

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
    pub history_page: usize,
    pub expanded_branch: Option<String>,
    pub expanded_branch_commits: Vec<GitCommand>,
    // External command handling
    pub pending_external: Option<ExternalCommand>,
    // Contextual actions
    pub action_registry: ActionRegistry,
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

## Menu Types (`src/menu/`)

### Menu Trait

```rust
/// A menu that can be rendered and interacted with
pub trait Menu: Send {
    fn title(&self) -> &str;
    fn items(&self) -> Vec<MenuItem>;
    fn selected(&self) -> usize;
    fn set_selected(&mut self, idx: usize);
    fn handle_key(&mut self, key: KeyEvent) -> MenuResult;
    fn render(&self, frame: &mut Frame, area: Rect);
}
```

### MenuItem

```rust
/// A menu item for display
pub struct MenuItem {
    pub label: String,
    pub description: Option<String>,
    pub key_hint: Option<String>,
    pub enabled: bool,
    pub style: ItemStyle,
}

pub enum ItemStyle {
    Normal,
    Disabled,
    Separator,
    Header,
    Checkbox { checked: bool },
}
```

### MenuResult

```rust
/// Result of menu interaction
pub enum MenuResult {
    Continue,                    // Keep menu open
    Close,                       // Close this menu
    Execute(MenuAction),         // Execute action and close
    Push(Box<dyn Menu>),         // Push nested menu
    Pop,                         // Return to previous menu
    CloseAll,                    // Clear entire stack
}

pub enum MenuAction {
    Command(CommandRequest),     // Execute a command
    App(AppAction),              // Trigger app action
    Custom(Box<dyn FnOnce() + Send>),
}
```

### MenuStack

```rust
/// Stack-based menu management
pub struct MenuStack {
    stack: Vec<Box<dyn Menu>>,
}

impl MenuStack {
    pub fn push(&mut self, menu: Box<dyn Menu>);
    pub fn pop(&mut self) -> Option<Box<dyn Menu>>;
    pub fn current(&self) -> Option<&dyn Menu>;
    pub fn current_mut(&mut self) -> Option<&mut Box<dyn Menu>>;
    pub fn is_empty(&self) -> bool;
    pub fn clear(&mut self);
}
```

## Command Types (`src/command/`)

### CommandRequest

```rust
/// A request to execute a command
pub struct CommandRequest {
    pub program: String,
    pub args: Vec<String>,
    pub display_name: String,
    pub cwd: Option<PathBuf>,
    pub source: CommandSource,
    pub feedback: FeedbackPolicy,
    pub refresh_after: bool,
}

impl CommandRequest {
    pub fn git(args: impl IntoIterator<Item = impl Into<String>>) -> Self;
    pub fn from_input(input: &str) -> Option<Self>;
    pub fn git_alias(name: &str, command: &str, repo_path: &Path) -> Self;
}
```

### CommandSource

```rust
/// Source of command execution - affects feedback behavior
pub enum CommandSource {
    Keyboard,       // Direct keybinding
    Palette,        // : mode
    ActionMenu,     // m menu
    AliasBrowser,   // a mode
    Internal,       // Background operations
}
```

### FeedbackPolicy

```rust
/// Policy for displaying command feedback
pub enum FeedbackPolicy {
    Default,        // Use source-based defaults
    AlwaysPopup,    // Always show popup
    InlineOnly,     // Never auto-popup
    Silent,         // No visible feedback
    External,       // External command (TUI suspension)
}
```

### CommandResult

```rust
/// Result of command execution
pub struct CommandResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}
```

## Feedback Types (`src/feedback/`)

### Feedback

```rust
/// Types of feedback that can be shown
pub enum Feedback {
    CommandOutput { command: String, result: CommandResult, source: CommandSource },
    Toast { message: String, level: ToastLevel },
    Error { message: String },
    Diff { path: String, content: String, is_staged: bool },
}
```

### FeedbackManager

```rust
/// Manages all feedback display state
pub struct FeedbackManager {
    pub command_output: Option<CommandOutput>,
    pub command_success: bool,
    pub popup: PopupState,
    pub toast: Option<Toast>,
    pub error: Option<String>,
}
```

### Toast

```rust
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub created: Instant,
}

pub enum ToastLevel {
    Info,     // Blue - informational
    Success,  // Green - completed
    Warning,  // Yellow - concerns
    Error,    // Red - failure
}
```

## Section Types (`src/section/`)

### SectionId

```rust
/// Unique identifier for each section
pub enum SectionId {
    Command,
    Staged,
    Working,
    History,
    Branches,
}
```

### Section Trait

```rust
/// A UI section that can be rendered and interacted with
pub trait Section: Send + Sync {
    fn id(&self) -> SectionId;
    fn name(&self) -> &str;
    fn contexts(&self) -> Vec<Context>;
    fn item_count(&self) -> usize;
    fn is_collapsible(&self) -> bool;
    fn is_collapsed(&self) -> bool;
    fn render(&self, state: &SectionState) -> Vec<Line<'static>>;
    fn actions(&self, item_idx: usize) -> Vec<Action>;
    fn handle_key(&self, key: KeyEvent, item_idx: usize) -> Option<SectionAction>;
}
```

### SectionState

```rust
/// State passed to sections for rendering and actions
pub struct SectionState {
    pub is_focused: bool,
    pub local_selection: Option<usize>,
    pub global_selection: Option<usize>,
    pub render_width: u16,
}
```

### SectionAction

```rust
/// Action returned by section key handling
pub enum SectionAction {
    Command(CommandRequest),
    Feedback(Feedback),
    OpenMenu(Box<dyn Menu>),
    AppAction(AppAction),
    Navigate(NavigateAction),
    None,
}
```

### NavigateAction

```rust
/// Navigation actions within or between sections
pub enum NavigateAction {
    Next,
    Prev,
    First,
    Last,
    JumpTo(SectionId),
}
```

### Section Implementations

Each section has a corresponding data struct for immutable state:

```rust
// Command section
pub struct CommandSectionData { pub command: String, pub output: String, pub success: bool }
pub struct CommandSection { data: CommandSectionData }

// Staged section
pub struct StagedSectionData { pub files: Vec<FileStatus> }
pub struct StagedSection { data: StagedSectionData }

// Working section
pub struct WorkingSectionData { pub files: Vec<FileStatus> }
pub struct WorkingSection { data: WorkingSectionData }

// History section
pub struct HistorySectionData { pub commits: Vec<GitCommand>, pub collapsed: bool, ... }
pub struct HistorySection { data: HistorySectionData }

// Branches section
pub struct BranchesSectionData { pub branches: Vec<BranchInfo>, ... }
pub struct BranchesSection { data: BranchesSectionData }
```

### SectionRegistry

Centralizes index calculations across all UI sections:

```rust
/// Result of looking up a global index
pub struct IndexLookup {
    pub section_id: SectionId,
    pub local_index: usize,
}

/// Registry for managing sections and index calculations
pub struct SectionRegistry {
    sections: Vec<SectionId>,
}

impl SectionRegistry {
    pub fn new() -> Self;
    pub fn sections(&self) -> &[SectionId];
    pub fn lookup_index(&self, global_index: usize, item_counts: &SectionItemCounts) -> Option<IndexLookup>;
    pub fn context_for_index(&self, global_index: usize, item_counts: &SectionItemCounts) -> Context;
    pub fn total_items(&self, item_counts: &SectionItemCounts) -> usize;
    pub fn section_start_index(&self, section_id: SectionId, item_counts: &SectionItemCounts) -> Option<usize>;
    pub fn build_section_states(&self, global_selection: Option<usize>, item_counts: &SectionItemCounts) -> Vec<(SectionId, SectionState)>;
}

/// Item counts for each section
pub struct SectionItemCounts {
    pub command: usize,
    pub staged: usize,
    pub working: usize,
    pub history: usize,
    pub branches: usize,
}

impl SectionItemCounts {
    pub fn get(&self, section_id: SectionId) -> usize;
    pub fn from_sections(command: &impl Section, staged: &impl Section, working: &impl Section, history: &impl Section, branches: &impl Section) -> Self;
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
