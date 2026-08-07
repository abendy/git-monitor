# Rust Dependencies

## Current Direct Dependencies

| Crate | Purpose |
| --- | --- |
| `ratatui`, `crossterm` | Terminal UI and input |
| `git2` | Local Git repository operations |
| `notify`, `notify-debouncer-mini` | Debounced filesystem refresh |
| `anyhow` | Application error propagation and context |
| `arboard` | Clipboard integration |
| `dirs` | User history location |
| `clap` | CLI parsing |
| `chrono` | Git timestamp presentation |
| `tracing`, `tracing-subscriber` | Diagnostics |
| `tempfile` | Hermetic test repositories (development only) |

`git2` enables vendored OpenSSL so release binaries do not depend on a machine-specific Homebrew
OpenSSL path.

## Adding or Changing a Dependency

1. Identify the concrete call site and why the standard library or an existing crate is insufficient.
2. Check maintenance, license, Rust-version requirement, default features, and transitive cost.
3. Enable only features the implementation uses.
4. Update `Cargo.lock`; applications commit their lockfile.
5. Run format, Clippy, all tests, release build, and the security audit.
6. Remove superseded direct dependencies in the same focused change.

Do not add an async runtime, HTTP client, serialization framework, or plugin protocol before the
first concrete provider/backend path requires it. GitHub integration should initially prefer the
authenticated `gh` CLI, which avoids inventing credential storage and an HTTP stack.

## Routine Inspection

```bash
cargo tree
cargo tree -d
cargo audit
```

Treat an advisory by reachability and actual exposure, but record any accepted risk. Do not update a
major dependency as drive-by cleanup inside unrelated feature work.
