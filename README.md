# git-monitor

A real-time TUI for monitoring and interacting with git repositories.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Live file watching** - Auto-refreshes when files change
- **Unified changes view** - Staged and working files in one panel
- **Git history** - Browse commit log or reflog with pagination
- **Branch management** - View, expand, and checkout branches
- **Contextual actions** - Smart action menu shows relevant commands
- **Git alias integration** - Browse and execute aliases from gitconfig
- **Command mode** - Run git commands with history navigation
- **External tools** - Integrates with your pager and difftool

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
│                                                                │
│   ▸ Branches                                                   │
│     ⎇ main (current)                                           │
│     ⎇ feature/new-ui                                           │
├────────────────────────────────────────────────────────────────┤
│ j/k nav  m actions  : cmd  ? help  q quit                      │
└────────────────────────────────────────────────────────────────┘
```

## Quick Start

```bash
# Install
cargo install --path .

# Run in current directory
git-monitor

# Run in specific directory
git-monitor --path /path/to/repo
```

## Key Commands

| Key | Action |
|-----|--------|
| `j`/`k` | Navigate up/down |
| `s` | Stage/unstage file |
| `d` | Show diff |
| `m` | Open action menu |
| `:` | Command mode |
| `?` | Help overlay |
| `q` | Quit |

See [docs/KEYBINDINGS.md](docs/KEYBINDINGS.md) for complete reference.

## Requirements

- Rust 1.70+
- Git repository
- Terminal with 256-color support

## Documentation

- [Contributing](CONTRIBUTING.md) - Development setup and guidelines
- [Architecture](docs/ARCHITECTURE.md) - System design
- [Keybindings](docs/KEYBINDINGS.md) - Full keybinding reference
- [ADRs](docs/adr/) - Architecture decisions

## License

MIT
