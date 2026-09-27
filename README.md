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
- **Command prompt** - run any command from `:`
- **Push confirmation** - optional `--force-with-lease`; sets upstream when missing
- **Destructive-action confirmation** - menu and key actions that reset, rebase, delete a branch,
  force-push, clean, or drop stashes show the exact command first; typed commands run as typed
- **Commit expansion** - file-level navigation within commits

## Direction

- GitHub pull-request, review, and check state
- Agent/worktree/branch/commit correlation
- A `jj` read backend alongside Git
- Need-to-know workflow summaries and explicit human gates such as merge

See the [product roadmap](docs/ROADMAP.md) for sequencing and non-goals.

## Screenshot

```text
┌ git-monitor ─────────────────────────────────────────────────────────────────────┐
│ ⎇ develop ↑1 │ my-project │ ● watching                                           │
└──────────────────────────────────────────────────────────────────────────────────┘
┌ Repository ──────────────────────────────────────────────────────────────────────┐
│  : command  a aliases                                                            │
│                                                                                  │
│                                                                                  │
│  ▸ Staged (1)                                                                    │
│  ● A src/new_file.rs  +12                                                        │
│                                                                                  │
│  ▸ Working (2)                                                                   │
│  ○ M src/main.rs  +5/-2                                                          │
│  ○ ? untracked.txt                                                               │
│                                                                                  │
│  ▾ History (3)                                                                   │
│  h log/reflog · Space expand · [ prev page · ] next page · y copy short          │
│  develop ↑1 → origin/develop  · P push  f fetch                                  │
│▸    ├─ e29d83a 14 min ago  ● Keep commit messages visible (HEAD → develop)       │
│     ├─ 9bfa421 2 hr ago  ● Include expanded-branch commits (origin/develop)      │
│     └─ 234a04a 3 hr ago  ● Load repository data as one snapshot                  │
│  [ prev · page ]  1/1 of 3 items                                                 │
│                                                                                  │
│  ▸ Branches (1)                                                                  │
│  feature/new-ui                                                                  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

In normal mode the footer shows `j/k nav  g/G top/btm  m menu  e edit │ ? help  q quit`.
`e edit` appears only when `$EDITOR` is set. Keys shown elsewhere stay out of the footer: `:` is on
the command row, and each section header shows its jump key (`w`, `h`, `b`) while the cursor is in
another section. In narrow panels the least-used hints drop first; `m menu` and `? help` always stay.

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
- Terminal with 16-color support

## Development Health

Source checkouts include a readiness check for the host toolchain, locked dependencies, and debug
build:

```bash
./doctor.sh

# Run the same formatting, lint, test, and release-build gates used before publishing
./doctor.sh --full
```

## Documentation

- [Roadmap](docs/ROADMAP.md) - Product direction and delivery sequence
- [Keybindings](docs/KEYBINDINGS.md) - Full keybinding reference
- [ADRs](docs/adr/) - Architecture decisions

## License

MIT
