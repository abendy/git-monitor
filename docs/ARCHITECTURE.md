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
    pub selected: usize,              // Single unified selection index
    pub show_help: bool,
    pub error: Option<String>,
    // Command mode
    pub command_mode: bool,
    pub command_input: String,
    pub command_output: String,
    pub command_success: bool,
    pub command_history: Vec<String>,
    pub history_index: Option<usize>,
    // Alias browser
    pub show_aliases: bool,
    pub alias_section_selected: usize,
    pub show_section_aliases: bool,
    pub alias_selected: usize,
    // History mode (reflog vs commit log)
    pub history_mode: HistoryMode,
    // Popup state
    pub popup: PopupState,
}
```

The UI uses a single unified scrollable list with sections: Command → Staged → Working → History.

### 6. UI Rendering (`src/ui.rs`)

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
│ Commit Log (50)                                              │  History
│   abc1234  14:32:01  Add feature X  (HEAD, main)            │  Section
│   def5678  14:30:00  Fix bug                                │
├──────────────────────────────────────────────────────────────┤
│ [:] cmd  [s] stage  [d] diff  [h] history  [?] help         │  Footer
└──────────────────────────────────────────────────────────────┘
```

**Popup System**: Full-screen overlays replace the body entirely (not layered):
- Help overlay - keybinding reference
- Command output popup - scrollable command results
- Diff popup - syntax-highlighted diff view

```rust
pub enum PopupContent {
    None,
    CommandOutput { command, output, success },
    Diff { path, content, is_staged },
}
```

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
   - Command mode → capture all input for command editing
   - Alias browser → navigate sections/aliases
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
