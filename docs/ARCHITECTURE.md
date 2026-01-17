# Git Monitor TUI - Architecture Document

## Overview

A terminal-based user interface for monitoring git repository activity in real-time. The application watches file changes, tracks staging operations, and displays git command activity from the reflog or commit log.

## Core Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Event Loop                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │   Input     │  │   Timer     │  │   File      │              │
│  │   Events    │  │   Ticks     │  │   Events    │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
│         │                │                │                      │
│         └────────────────┼────────────────┘                      │
│                          ▼                                       │
│                   ┌──────────────┐                               │
│                   │  App State   │                               │
│                   └──────┬───────┘                               │
│                          ▼                                       │
│                   ┌──────────────┐                               │
│                   │   Renderer   │                               │
│                   └──────────────┘                               │
└─────────────────────────────────────────────────────────────────┘
```

## Module Structure

| Module | File | Purpose |
|--------|------|---------|
| `main` | `src/main.rs` | CLI entry point, argument parsing (clap) |
| `app` | `src/app.rs` | Central state container, event handling, keybindings |
| `ui` | `src/ui.rs` | UI rendering (header, body, footer, popup overlays) |
| `git` | `src/git.rs` | Git operations wrapper around libgit2 |
| `tui` | `src/tui.rs` | Terminal setup/teardown (crossterm + ratatui) |
| `event` | `src/event.rs` | Event types and handler thread |
| `watcher` | `src/watcher.rs` | File system watching with debouncing (notify) |
| `config` | `src/config.rs` | Git config and alias parsing |
| `actions` | `src/actions.rs` | Contextual action framework (see ADR-002) |
| `command/` | `src/command/` | Unified command execution framework (see ADR-003) |
| `feedback/` | `src/feedback/` | Feedback system for output display (see ADR-004) |
| `input/` | `src/input/` | Declarative keybindings (`Keymap`, `KeyBinding`) |
| `menu/` | `src/menu/` | Modular menu system with stack navigation (see ADR-005) |
| `section/` | `src/section/` | Section trait and implementations for UI regions |

## Component Design

### 1. Event System (`src/event.rs`)

```rust
pub enum Event {
    Tick,                  // Periodic timer tick
    Key(KeyEvent),         // Keyboard input
    Mouse(MouseEvent),     // Mouse input
    Resize(u16, u16),      // Terminal resize
    FileChanged,           // File system change detected
}
```

The `EventHandler` spawns a background thread that:
- Polls crossterm for terminal events
- Sends periodic tick events (configurable rate)
- Exposes a sender for external events (file watcher)

### 2. Git Operations (`src/git.rs`)

`GitRepo` wraps `git2::Repository` and provides:
- `status()` - Get complete status snapshot (`GitStatus`)
- `stage(path)` - Stage a file via index
- `unstage(path)` - Unstage a file via reset
- `diff_file(path, staged)` - Get diff for a file
- `reflog(limit)` - Parse recent activity from HEAD reflog

### 3. File Watcher (`src/watcher.rs`)

`RepoWatcher` uses `notify-debouncer-mini` for efficient file watching:
- 100ms debounce on all events
- Watches working directory recursively
- Watches specific `.git/` paths: `index`, `HEAD`, `refs/`, `logs/`

Watch events are categorized:
```rust
pub enum WatchEvent {
    WorkingDirectory(PathBuf),
    GitIndex,
    GitHead,
    GitRefs,
}
```

### 4. Git Config (`src/config.rs`)

`GitConfig` loads and parses git aliases from `.gitconfig`:
- Groups aliases by section (parsed from `# --- section ---` comments)
- Supports alias browser with section-grouped categories
- Falls back to a single "all" section if no section headers found

```rust
pub struct GitConfig {
    pub sections: Vec<AliasSection>,
}

pub struct AliasSection {
    pub name: String,
    pub aliases: Vec<Alias>,
}

pub struct Alias {
    pub name: String,
    pub command: String,
}
```

### 5. Application State (`src/app.rs`)

```rust
pub struct App {
    pub repo_path: PathBuf,
    repo: GitRepo,
    pub status: GitStatus,
    pub activity: Vec<GitCommand>,
    pub config: GitConfig,
    watcher: Option<RepoWatcher>,
    pub running: bool,
    pub selected: Option<usize>,      // Selection index (None = nothing focused)
    pub show_help: bool,
    pub view_mode: ViewMode,          // Current interaction mode
    pub error: Option<String>,
    // Command mode fields
    pub command_input: String,
    pub command_output: String,
    pub command_success: bool,
    pub command_history: Vec<String>,
    pub history_index: Option<usize>,
    // History mode (reflog vs commit log)
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
    pub history_page: usize,              // Pagination offset
    pub expanded_branch: Option<String>,
    pub expanded_branch_commits: Vec<GitCommand>,
    // External command handling
    pub pending_external: Option<ExternalCommand>,
    // Contextual actions
    pub action_registry: ActionRegistry,
    // Menu stack for modal dialogs (see ADR-005)
    pub menu_stack: MenuStack,
}
```

**View Modes**: Simplified to 2 variants (see ADR-005):
- `Normal` - Standard navigation
- `Command` - Typing git commands

Modal dialogs (action menu, alias browser, push confirmation) are managed by `MenuStack` instead of ViewMode variants.

The UI uses a single unified scrollable list with sections: Command → Staged → Working → History → Branches.

### 6. Menu System (`src/menu/`)

The menu module provides composable menus with stack-based navigation (see ADR-005):

```rust
pub trait Menu: Send {
    fn title(&self) -> &str;
    fn items(&self) -> Vec<MenuItem>;
    fn selected(&self) -> usize;
    fn set_selected(&mut self, idx: usize);
    fn handle_key(&mut self, key: KeyEvent) -> MenuResult;
    fn render(&self, frame: &mut Frame, area: Rect);
}

pub enum MenuResult {
    Continue,           // Keep menu open
    Close,              // Close this menu
    Execute(MenuAction), // Execute action and close
    Push(Box<dyn Menu>), // Push nested menu
    Pop,                // Return to previous menu
    CloseAll,           // Clear entire stack
}
```

**Menu Types**:
- `ActionMenu` - Context-filtered actions from registry
- `AliasSectionMenu` / `AliasItemsMenu` - Two-level alias browser
- `PushConfirmMenu` - Push with force/upstream options
- `SelectMenu` / `ConfirmMenu` - Reusable generic menus

### 7. Command Execution (`src/command/`)

Unified command execution framework (see ADR-003):

```rust
pub struct CommandRequest {
    pub program: String,
    pub args: Vec<String>,
    pub display_name: String,
    pub cwd: Option<PathBuf>,
    pub source: CommandSource,
    pub feedback: FeedbackPolicy,
    pub refresh_after: bool,
}

pub enum CommandSource {
    Keyboard,       // Direct keybinding
    Palette,        // : mode
    ActionMenu,     // m menu
    AliasBrowser,   // a mode
    Internal,       // Background operations
}
```

### 8. Feedback System (`src/feedback/`)

Manages all user feedback (see ADR-004):

```rust
pub enum Feedback {
    CommandOutput { command, result, source },
    Toast { message, level },
    Error { message },
    Diff { path, content, is_staged },
}

pub struct FeedbackManager {
    pub command_output: Option<CommandOutput>,
    pub popup: PopupState,
    pub toast: Option<Toast>,
    pub error: Option<String>,
}
```

Feedback behavior varies by `CommandSource`: menu/browser selections always show popup; keyboard shortcuts use inline display with auto-popup for failures or long output.

### 9. Input System (`src/input/`)

Declarative keybindings that map key combinations to actions based on context:

```rust
/// A key binding (key code + modifiers)
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

/// Declarative keymap that maps key bindings to actions
pub struct Keymap {
    global: HashMap<KeyBinding, AppAction>,
    contextual: HashMap<Context, HashMap<KeyBinding, AppAction>>,
}

impl Keymap {
    pub fn lookup(&self, key: KeyEvent, context: Context) -> Option<AppAction>;
}
```

The keymap is consulted in `App::handle_key()` after checking for menu stack and popup state. Context-specific bindings take priority over global bindings.

### 10. Section System (`src/section/`)

Each UI section implements the `Section` trait for consistent rendering and key handling:

```rust
pub trait Section: Send + Sync {
    fn id(&self) -> SectionId;
    fn item_count(&self) -> usize;
    fn render(&self, state: &SectionState) -> Vec<Line<'static>>;
    fn actions(&self, item_idx: usize) -> Vec<Action>;
    fn handle_key(&self, key: KeyEvent, item_idx: usize) -> Option<SectionAction>;
}
```

**Section Implementations**:
- `CommandSection` - Command input and output display
- `StagedSection` - Staged files list
- `WorkingSection` - Working directory changes
- `HistorySection` - Commit log / reflog (collapsible)
- `BranchesSection` - Local branches (expandable)

### 11. UI Rendering (`src/ui.rs`)

Layout structure (unified vertical list):
```
┌──────────────────────────────────────────────────────────────┐
│ git-monitor │ ⎇ branch ↑N ↓N │ repo-name │ ● watching       │  Header
├──────────────────────────────────────────────────────────────┤
│ ▸ : git status                                               │  Command
│   ✓ (no output)                                              │  Section
├──────────────────────────────────────────────────────────────┤
│ Staged (2)                                                   │  Staged
│   A new_file.rs                                              │  Section
│   M config.rs                                                │
├──────────────────────────────────────────────────────────────┤
│ Working Directory (3)                                        │  Working
│   M src/main.rs                                              │  Section
│   ? untracked.txt                                            │
├──────────────────────────────────────────────────────────────┤
│ ▶ History (main) ─ 2 hr ago                                  │  History
│   abc1234  Add feature X  (HEAD)                    2 hr ago │  (collapsible)
│   └─ M src/app.rs  +45 -12                                   │  expanded commit
│   └─ A src/new.rs  +100                                      │  with files
│   def5678  Fix bug                                  3 hr ago │
├──────────────────────────────────────────────────────────────┤
│ ─ Branches ──────────────────────────────────────────────────│
│   feature/new (5 ahead)                                      │  Branch list
│   └─ ghi9012  WIP on feature                        1 day ago│  (expandable)
│   bugfix/login (2 behind)                                    │
├──────────────────────────────────────────────────────────────┤
│ [:] cmd  [s] stage  [d] diff  [h] history  [?] help         │  Footer
└──────────────────────────────────────────────────────────────┘
```

**Popup System**: Full-screen overlays replace the body entirely (not layered):
- Help overlay - keybinding reference
- Command output popup - scrollable command results
- Diff popup - syntax-highlighted diff view
- Action menu - context-aware action list (see ADR-002)

```rust
pub enum PopupContent {
    None,
    CommandOutput { command, output, success },
    Diff { path, content, is_staged },
}
```

### 12. Contextual Actions (`src/actions.rs`)

The action framework provides context-aware keybinding hints and a discoverable action menu:

```rust
pub enum Context {
    Command, StagedFiles, WorkingFiles, HistoryHeader,
    HistoryCommits, CommitFiles, BranchCommits, Global,
}

pub struct ActionRegistry { actions: Vec<Action> }
```

Actions can be conditional (e.g., Push only available when `BranchAhead`). The registry is the single source of truth for keybindings, used by both inline hints and the action menu popup.

## Data Flow

1. **Initialization**
   ```
   main() → App::new() → GitRepo::open() → status() + reflog()
          → Tui::new() → EventHandler::new()
          → App::setup_watcher() → RepoWatcher::new()
   ```

2. **Event Loop**
   ```
   loop {
       tui.draw(|f| ui::render(f, app))
       match tui.events.next() {
           Key(key) → app.handle_key(key)
           Tick → app.on_tick()
           FileChanged → app.refresh_status()
       }
   }
   ```

3. **Key Handling Priority**
   - Menu stack active → delegate to current menu
   - Command mode → capture all input for command editing
   - Popup open → scroll/close popup
   - Help overlay → dismiss on any key
   - Otherwise → normal key handling

## Threading Model

```
Main Thread: Event loop + Rendering + Git operations
├── Event Thread: Polls crossterm, sends events via mpsc
└── Watcher Thread: Receives notify events, forwards to main via mpsc
```

All git operations run on the main thread to avoid libgit2 threading issues.
