# git-monitor

A real-time TUI for monitoring and interacting with git repositories.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Live file watching** - Auto-refreshes when files change in the working directory
- **Unified changes view** - Staged and working files in one panel with visual indicators
- **Git history** - Browse commit log or reflog with pagination
- **Branch management** - View, expand, and checkout branches
- **Contextual actions** - Smart action menu (`m`) shows relevant commands for current selection
- **Git alias integration** - Browse and execute aliases from your gitconfig
- **Command mode** - Run git commands directly with history navigation
- **External tool support** - Integrates with your configured pager and difftool
- **Conditional hints** - Push/pull/fetch shortcuts appear when relevant (ahead/behind)

## Screenshot

```
┌─ git-monitor ──────────────────────────────────────────────────┐
│ ⎇ main ↑2 ↓0 │ my-project │ ● watching                        │
├────────────────────────────────────────────────────────────────┤
│ ▸ : command   a aliases                                        │
│                                                                │
│   ▾ Staged (1)                                                 │
│     s unstage · d diff · P push                                │
│   ▸ ● A src/new_file.rs                                        │
│                                                                │
│   ▸ Working (3)                                                │
│     ○ M src/main.rs                                            │
│     ○ M src/lib.rs                                             │
│     ○ ? untracked.txt                                          │
│                                                                │
│   ▸ History (50+)                                              │
│     ● 14:32 abc1234 Add new feature (HEAD -> main)             │
│     ● 14:28 def5678 Fix bug in parser                          │
│     ● 14:15 ghi9012 Refactor utils                             │
│                                                                │
│   ▸ Branches                                                   │
│     ⎇ main (current)                                           │
│     ⎇ feature/new-ui                                           │
│     ⎇ origin/main                                              │
├────────────────────────────────────────────────────────────────┤
│ j/k nav  m actions  : cmd  ? help  q quit                      │
└────────────────────────────────────────────────────────────────┘
```

### Visual Indicators

| Symbol | Meaning |
|--------|---------|
| `●` (green) | Staged for commit |
| `○` (yellow) | Working directory changes |
| `?` | Untracked file |
| `↑N` | Commits ahead of remote |
| `↓N` | Commits behind remote |
| `▸` / `▾` | Collapsed / expanded section |

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/git-monitor
cd git-monitor

# Build and install
cargo install --path .
```

## Usage

```bash
# Run in current directory
git-monitor

# Run in specific directory
git-monitor --path /path/to/repo

# Custom refresh rate (default: 250ms)
git-monitor --tick-rate 100
```

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Go to first item |
| `G` | Go to last item |
| `w` | Jump to working files |
| `h` | Jump to history / toggle reflog |
| `b` | Jump to branches |

### File Actions (Staged/Working)

| Key | Action |
|-----|--------|
| `s` | Stage / unstage selected file |
| `d` / `Enter` | Show diff in popup |
| `Space` | Expand commit (when on history) |

### Remote Operations

| Key | Condition | Action |
|-----|-----------|--------|
| `P` | When ahead | Push to remote |
| `p` | When behind | Pull from remote |
| `f` | Always | Fetch from remote |

### History

| Key | Action |
|-----|--------|
| `h` | Toggle between commit log and reflog |
| `Space` | Expand/collapse commit to show files |
| `[` / `]` | Previous / next page |
| `y` | Copy short SHA to clipboard |
| `c` | Copy full SHA to clipboard |

### Commit Files (when expanded)

| Key | Action |
|-----|--------|
| `Space` | Open diff in external pager |
| `d` | Show inline diff popup |
| `M` | Open in external difftool |

### Branches

| Key | Action |
|-----|--------|
| `Space` | Expand/collapse branch commits |
| `Enter` | Checkout branch |

### Command Mode

| Key | Action |
|-----|--------|
| `:` | Enter command mode |
| `a` | Browse git aliases |
| `o` | Expand output in popup |
| `Enter` | Execute command |
| `Esc` | Cancel |
| `↑` / `↓` | Navigate command history |

### Popup Navigation

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll line by line |
| `g` / `G` | Jump to top / bottom |
| `Ctrl+d` / `Ctrl+u` | Page down / up |
| `q` / `Esc` | Close popup |

### General

| Key | Action |
|-----|--------|
| `m` | Open context action menu |
| `r` | Refresh status |
| `?` | Toggle help overlay |
| `q` / `Esc` | Quit (or close overlay) |

## Action Menu

Press `m` to open the contextual action menu. Available actions depend on your current selection:

- **On staged/working files**: Stage, unstage, show diff
- **On history commits**: Expand, copy SHA, pagination
- **On commit files**: Pager diff, inline diff, difftool
- **On branches**: Expand, checkout
- **Global**: Push, pull, fetch, refresh

The menu also includes git aliases from your `~/.gitconfig` that match the current context.

## Git Alias Integration

Press `a` when on the command section to browse aliases organized by category:
- Aliases are auto-categorized based on their commands
- Press `Enter` to execute the selected alias
- Aliases also appear in the contextual action menu (`m`)

## External Tool Integration

The application respects your git configuration:
- `core.pager` - Used for viewing commit file diffs (`Space` on commit files)
- `diff.tool` - Used for external diff viewing (`M` on commit files)

## Requirements

- Rust 1.70+
- Git repository
- Terminal with 256-color support

## Tech Stack

- [ratatui](https://github.com/ratatui-org/ratatui) - TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal backend
- [git2](https://github.com/rust-lang/git2-rs) - Git operations (libgit2)
- [notify](https://github.com/notify-rs/notify) - File watching
- [tokio](https://github.com/tokio-rs/tokio) - Async runtime
- [clap](https://github.com/clap-rs/clap) - CLI argument parsing

## Documentation

- [Architecture](docs/ARCHITECTURE.md) - System design and module structure
- [Keybindings](docs/KEYBINDINGS.md) - Complete keybinding reference
- [ADRs](docs/adr/) - Architecture Decision Records

## License

MIT
