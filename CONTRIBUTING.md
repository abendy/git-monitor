# Contributing to git-monitor

## Development Setup

### Prerequisites

- **Rust** 1.97+ (install via [rustup](https://rustup.rs/); the repository selects stable Rust)
- **Git** 2.x+
- Terminal with 256-color support

### Clone and Build

```bash
git clone https://github.com/abendy/git-monitor
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
# Run all targets with the locked dependency graph
cargo test --all-targets --all-features --locked

# Run tests with output
cargo test -- --nocapture
```

### Linting

The project uses strict lints configured in `Cargo.toml`:

```bash
# Check formatting
cargo fmt --all -- --check

# Run clippy across every target (pedantic + nursery lints enabled)
cargo clippy --all-targets --all-features -- -D warnings
```

Key lint rules:

- `unsafe_code = "forbid"` - No unsafe code allowed
- `unwrap_used = "warn"` - Prefer `?` with `anyhow::Context`
- `expect_used = "warn"` - Same as above
- `clippy::pedantic` and `clippy::nursery` enabled

### Code Style

- **Max line width**: 100 characters
- **Error handling**: Use `?` operator with `anyhow::Context`, avoid `.unwrap()`

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed design documentation.

## Commit Guidelines

Use [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(<scope>): <description>

[optional body]
```

**Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`

**Examples**:

```text
feat(menu): add keyboard shortcut hints
fix(git): handle detached HEAD state
docs: update keybindings reference
refactor(render): split body into submodules
```

## Pull Requests

1. Create a feature branch from `develop`
2. Make focused, atomic commits
3. Ensure tests pass and clippy is clean
4. Update documentation if needed
5. Open PR against `develop`

## Documentation

- **[ARCHITECTURE.md](docs/ARCHITECTURE.md)** - System design, module structure
- **[KEYBINDINGS.md](docs/KEYBINDINGS.md)** - Complete keybinding reference
- **[ADRs](docs/adr/)** - Architecture Decision Records
