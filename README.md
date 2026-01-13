# git-monitor

A real-time TUI for monitoring git repository activity.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Live file watching** - Auto-refreshes when files change
- **Working directory** - See modified, untracked, and deleted files
- **Staged changes** - Track what's ready to commit
- **Activity log** - Recent git commands from reflog
- **Branch info** - Current branch with ahead/behind counts
- **Command mode** - Run git commands with history navigation

## Screenshot

```
┌─ git-monitor ──────────────────────────────────────────────────┐
│ ⎇ main ↑2 ↓0 │ my-project │ ● watching                        │
├────────────────────────────┬───────────────────────────────────┤
│ Working Directory (3)      │ Staged (1)                        │
│ ▸ M src/main.rs            │   A src/new_file.rs               │
│   M src/lib.rs             │                                   │
│   ? untracked.txt          │                                   │
├────────────────────────────┴───────────────────────────────────┤
│ Recent Activity (12)                                           │
│   14:32:01  ● commit: Add new feature                          │
│   14:28:45  ⎇ checkout: moving from main to feature            │
├─ Command ──────────────────────────────────────────────────────┤
│ : git log --oneline -3_                                        │
│ > abc1234 Add new feature                                      │
│ > def5678 Fix bug in parser                                    │
├────────────────────────────────────────────────────────────────┤
│ Tab switch  j/k nav  s stage  d diff  : cmd  ? help  q quit    │
└────────────────────────────────────────────────────────────────┘
```

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

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch panels |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Go to first item |
| `G` | Go to last item |
| `s` | Stage / Unstage selected file |
| `d` / `Enter` | Show diff for selected file |
| `r` | Refresh status |
| `:` | Enter command mode |
| `?` | Toggle help |
| `q` / `Esc` | Quit (or close overlay) |

### Command Mode

Press `:` to enter command mode and run git commands directly:

| Key | Action |
|-----|--------|
| `Enter` | Execute command |
| `Esc` / `Ctrl+C` | Cancel |
| `Up` / `Down` | Navigate command history |

Only `git` commands are allowed for safety. Examples:
- `git status`
- `git log --oneline -5`
- `git branch -a`

## Requirements

- Rust 1.70+
- Git repository

## Tech Stack

- [ratatui](https://github.com/ratatui-org/ratatui) - TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal backend
- [git2](https://github.com/rust-lang/git2-rs) - Git operations (libgit2)
- [notify](https://github.com/notify-rs/notify) - File watching

## License

MIT
