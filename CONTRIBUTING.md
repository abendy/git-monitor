# Contributing to git-monitor

## Development Setup

### Prerequisites

- **Rust** 1.70+ (install via [rustup](https://rustup.rs/) or Homebrew)
- **Git** 2.x+
- Terminal with 256-color support

### Clone and Build

```bash
git clone https://github.com/yourusername/git-monitor
cd git-monitor

# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

### Run

```bash
# Run in current directory
cargo run

# Run with a specific repo
cargo run -- --path /path/to/repo

# Custom tick rate (default: 250ms)
cargo run -- --tick-rate 100
```

## Development Workflow

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Linting

The project uses strict lints configured in `Cargo.toml`:

```bash
# Format code (nightly via rust-toolchain.toml)
cargo fmt

# Run clippy (pedantic + nursery lints enabled)
cargo clippy
```

Key lint rules:
- `unsafe_code = "forbid"` - No unsafe code allowed
- `unwrap_used = "warn"` - Prefer `?` with `anyhow::Context`
- `expect_used = "warn"` - Same as above
- `clippy::pedantic` and `clippy::nursery` enabled

### Code Style

- **Max line width**: 100 characters
- **Imports**: Grouped by std/external/crate, module granularity
- **Error handling**: Use `?` operator with `anyhow::Context`, avoid `.unwrap()`

See [`.claude/rust-style.md`](.claude/rust-style.md) for detailed guidelines.

## Architecture

The codebase follows an event-driven architecture:

```
src/
├── main.rs          # CLI entry, argument parsing
├── app.rs           # Central state, event handling
├── ui.rs            # Render coordinator
├── git.rs           # Git operations (libgit2)
├── tui.rs           # Terminal setup/teardown
├── event.rs         # Event types
├── watcher.rs       # File system watching
├── config.rs        # Git config parsing
├── actions.rs       # Action registry
├── command/         # Command execution framework
├── menu/            # Modal menu system
├── feedback/        # Output feedback management
├── section/         # UI sections and registry
├── input/           # Keymap and input handling
└── render/          # Split render modules
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed design documentation.

## Commit Guidelines

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

Task: .tasks/YYYY-MM-DD-description.md
```

**Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`

**Examples**:
```
feat(menu): add keyboard shortcut hints
fix(git): handle detached HEAD state
docs: update keybindings reference
refactor(render): split body into submodules
```

## Task Tracking

Active work is tracked in `.tasks/`:

1. Create task file: `.tasks/YYYY-MM-DD-brief-description.md`
2. Update checklist as you progress
3. Reference in commits: `Task: .tasks/...`
4. Mark completed when done

## Pull Requests

1. Create a feature branch from `develop`
2. Make focused, atomic commits
3. Ensure tests pass and clippy is clean
4. Update documentation if needed
5. Open PR against `develop`

## Documentation

- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** - System design, module structure
- **[KEYBINDINGS.md](docs/KEYBINDINGS.md)** - Complete keybinding reference
- **[AI_INTERNALS.md](docs/AI_INTERNALS.md)** - Implementation patterns
- **[ADRs](docs/adr/)** - Architecture Decision Records
