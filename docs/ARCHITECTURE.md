# Git Monitor TUI - Architecture Document

## Overview

A terminal-based user interface for monitoring git repository activity in real-time. The application watches file changes, tracks staging operations, and displays git command activity.

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

## Component Design

### 1. Event System (`src/events/`)

```rust
enum AppEvent {
    // Terminal input
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),

    // Timer
    Tick,

    // File system
    FileChanged(PathBuf),
    GitIndexChanged,
    GitHeadChanged,

    // Git operations
    GitStatusUpdated(GitStatus),
    GitCommandDetected(GitCommand),
}
```

### 2. Git Monitor (`src/git/`)

**Status Tracker** - Polls/watches git status:
- Working directory changes (modified, untracked, deleted)
- Staged changes
- Branch information
- Remote tracking status

**Index Watcher** - Monitors `.git/` directory:
- `.git/index` - staging area changes
- `.git/HEAD` - branch switches
- `.git/FETCH_HEAD` - fetch operations
- `.git/logs/HEAD` - command history (reflog)
- `.git/refs/` - branch/tag updates

**Command Detector** - Infers git operations:
- Parse reflog for recent commands
- Detect operations via file change patterns
- Track operation duration where possible

### 3. File Watcher (`src/watcher/`)

Uses `notify` crate for cross-platform file watching:
- Debounce rapid changes
- Filter relevant paths (ignore `.git/objects/`)
- Batch updates for efficiency

### 4. UI Components (`src/ui/`)

```
┌──────────────────────────────────────────────────────────────┐
│ [branch: main] ↑2 ↓1  │  ~/projects/my-repo  │  watching... │
├────────────────────────┬─────────────────────────────────────┤
│ Working Directory      │ Staged Changes                      │
│ ─────────────────────  │ ─────────────────────               │
│  M src/main.rs         │  A src/new_file.rs                  │
│  M src/lib.rs          │  M src/config.rs                    │
│  ? untracked.txt       │                                     │
│  D old_file.rs         │                                     │
│                        │                                     │
├────────────────────────┴─────────────────────────────────────┤
│ Recent Activity                                               │
│ ─────────────────────────────────────────────────────────────│
│ 14:32:01  git commit -m "Add feature X"                      │
│ 14:31:45  git add src/new_file.rs                            │
│ 14:30:22  git pull origin main                               │
│ 14:28:10  git checkout -b feature/new-thing                  │
└──────────────────────────────────────────────────────────────┘
│ [q]uit  [r]efresh  [d]iff  [s]tage  [c]ommit  [?]help       │
└──────────────────────────────────────────────────────────────┘
```

### 5. App State (`src/app/`)

```rust
struct App {
    // Repository info
    repo_path: PathBuf,
    repo: Repository,  // git2::Repository

    // Git state
    current_branch: String,
    upstream_status: Option<UpstreamStatus>,
    working_changes: Vec<FileStatus>,
    staged_changes: Vec<FileStatus>,

    // Activity log
    recent_commands: VecDeque<GitCommand>,

    // UI state
    active_panel: Panel,
    selected_index: usize,
    scroll_offset: usize,

    // App state
    running: bool,
    last_update: Instant,
}
```

## Data Flow

1. **Initialization**
   - Detect git repository root
   - Initialize file watchers
   - Perform initial git status
   - Start event loop

2. **Event Processing**
   ```
   FileChanged → Debounce → GitStatusUpdated → Render
   KeyPress → Handle Input → Update State → Render
   Tick → Check for changes → Conditional Render
   ```

3. **Git Status Updates**
   - Triggered by file changes or timer
   - Runs `git status --porcelain=v2` or uses libgit2
   - Parses output into structured data
   - Updates app state
   - Triggers re-render

## Error Handling Strategy

- **Recoverable errors**: Log and continue (file access, git command failures)
- **Fatal errors**: Clean terminal state and exit with message
- **Git errors**: Display in status bar, allow retry

## Performance Considerations

1. **Debouncing**: 100ms debounce on file changes
2. **Throttling**: Max 10 status updates per second
3. **Lazy loading**: Don't compute diffs until requested
4. **Incremental updates**: Only re-render changed components
5. **Background operations**: Long operations on separate thread

## Threading Model

```
Main Thread: Event loop + Rendering
├── Watcher Thread: File system events (notify)
├── Git Thread: Status updates (when needed)
└── Timer Thread: Tick events (crossterm)
```

Using `tokio` for async or `std::sync::mpsc` for message passing.
