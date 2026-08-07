# git-monitor

A real-time TUI for monitoring and interacting with git repositories.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **File watching** - auto-refresh via notify, including linked worktrees
- **Unified view** - staged, working, history, branches
- **Action registry** - context detection and conditions
- **Menu system** - composable stack navigation for nested dialogs
- **Command execution** - unified path with feedback routing
- **Source-aware feedback** - keyboard vs menu vs alias triggers different behavior
- **Alias integration** - gitconfig aliases surfaced as contextual actions
- **Declarative keymap** - context-based lookup
- **External tools** - pager and difftool integration
- **Command palette** - run arbitrary git commands
- **Push confirmation** - menu with force and set-upstream options
- **Commit expansion** - file-level navigation within commits

## Direction

- GitHub pull-request, review, and check state
- Agent/worktree/branch/commit correlation
- A `jj` read backend alongside Git
- Need-to-know workflow summaries and explicit human gates such as merge

See the [product roadmap](docs/ROADMAP.md) for sequencing and non-goals.

## Screenshot

```text
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
cargo install --locked --path .

# Run in current directory
git-monitor

# Run in specific directory
git-monitor --path /path/to/repo
```

## Requirements

- Rust 1.97+
- Git repository
- Terminal with 256-color support

## Development Health

Source checkouts include a readiness check for the host toolchain, locked dependencies, and debug
build:

```bash
./doctor.sh

# Run the same formatting, lint, test, and release-build gates used before publishing
./doctor.sh --full
```

## Documentation

- [Contributing](CONTRIBUTING.md) - Development setup and guidelines
- [Architecture](docs/ARCHITECTURE.md) - System design
- [Roadmap](docs/ROADMAP.md) - Product direction and delivery sequence
- [Keybindings](docs/KEYBINDINGS.md) - Full keybinding reference
- [ADRs](docs/adr/) - Architecture decisions

## License

MIT
