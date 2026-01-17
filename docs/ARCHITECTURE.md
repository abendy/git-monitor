# Git Monitor TUI - Architecture Document

## Overview

A terminal-based user interface for monitoring git repository activity in real-time. The application watches file changes, tracks staging operations, and displays git command activity from the reflog.

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
| `ui` | `src/ui.rs` | UI rendering (header, panels, footer, overlays) |
| `git` | `src/git.rs` | Git operations wrapper around libgit2 |
| `tui` | `src/tui.rs` | Terminal setup/teardown (crossterm + ratatui) |
| `event` | `src/event.rs` | Event types and handler thread |
| `watcher` | `src/watcher.rs` | File system watching with debouncing (notify) |

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

### 4. Application State (`src/app.rs`)

```rust
pub struct App {
    pub repo_path: PathBuf,
    repo: GitRepo,
    pub status: GitStatus,
    pub activity: Vec<GitCommand>,
    watcher: Option<RepoWatcher>,
    pub running: bool,
    pub active_panel: Panel,
    pub working_selected: usize,
    pub staged_selected: usize,
    pub show_help: bool,
    pub show_diff: bool,
    pub diff_content: String,
    pub diff_path: String,
    pub error: Option<String>,
}
```

Panel navigation cycles through `Working → Staged → Activity`.

### 5. UI Rendering (`src/ui.rs`)

Layout structure:
```
┌──────────────────────────────────────────────────────────────┐
│ git-monitor │ ⎇ branch ↑N ↓N │ repo-name │ ● watching       │  Header
├────────────────────────┬─────────────────────────────────────┤
│ Working Directory (N)  │ Staged (N)                          │  Body
│  ▸ M file.rs           │    A new.rs                         │  (60%)
│    ? untracked.txt     │                                     │
├────────────────────────┴─────────────────────────────────────┤
│ Recent Activity (N)                                          │  Activity
│  HH:MM:SS  ● commit: message                                 │  (40%)
│  HH:MM:SS  ⎇ checkout: from branch to branch                │
├──────────────────────────────────────────────────────────────┤
│ [Tab] switch  [j/k] nav  [s] stage  [d] diff  [?] help      │  Footer
└──────────────────────────────────────────────────────────────┘
```

Overlays render as full-screen replacements (not layered):
- Help overlay - keybinding reference
- Diff overlay - syntax-highlighted diff view

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
   - Help overlay open → dismiss on any key
   - Diff overlay open → handle q/Esc/d to close
   - Otherwise → normal key handling

## Threading Model

```
Main Thread: Event loop + Rendering + Git operations
├── Event Thread: Polls crossterm, sends events via mpsc
└── Watcher Thread: Receives notify events, forwards to main via mpsc
```

All git operations run on the main thread to avoid libgit2 threading issues.
