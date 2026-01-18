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

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed design documentation.

## Commit Guidelines

Use [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(<scope>): <description>

[optional body]

Task: .project/tasks/YYYY-MM-DD-description.md
```

**Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`

**Examples**:

```text
feat(menu): add keyboard shortcut hints
fix(git): handle detached HEAD state
docs: update keybindings reference
refactor(render): split body into submodules
```

## Task Tracking

Active work is tracked in `.project/tasks/`:

1. Create task file: `.project/tasks/YYYY-MM-DD-brief-description.md`
2. Update checklist as you progress
3. Mark completed when done

## Pull Requests

1. Create a feature branch from `develop`
2. Make focused, atomic commits
3. Ensure tests pass and clippy is clean
4. Update documentation if needed
5. Open PR against `develop`

## Documentation

- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** - System design, module structure
- **[KEYBINDINGS.md](docs/KEYBINDINGS.md)** - Complete keybinding reference
- **[AI_INTERNALS.md](.project/reference/AI_INTERNALS.md)** - Implementation patterns
- **[ADRs](docs/adr/)** - Architecture Decision Records
