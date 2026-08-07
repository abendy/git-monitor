# Rust Style

## Baseline

- Stable Rust selected by `rust-toolchain.toml`; minimum supported version is in `Cargo.toml`.
- Rust 2021 edition, 100-column formatter configuration, and no nightly-only rustfmt options.
- `unsafe_code = "forbid"`; Clippy pedantic and nursery lints are enforced as errors in CI.
- Let rustfmt decide layout. Keep formatter-only changes isolated when they would obscure behavior.

## Errors and Invariants

- Use `anyhow::Result` and add `anyhow::Context` at filesystem, Git, process, parsing, and
  terminal boundaries.
- Production code must not use `unwrap`, `expect`, `panic`, `todo`, or `unimplemented`.
- Do not silently discard errors unless shutdown or best-effort behavior is both expected and
  documented. Log passive watcher/refresh failures with useful context.
- Model UI and workflow states with enums instead of coupled booleans when more than two valid
  states exist.

## Ownership and APIs

- Borrow `&Path`, `&str`, and slices when ownership is unnecessary.
- Return owned snapshots from boundaries when the caller must retain them across refreshes.
- Keep public types reachable through intentional re-exports; do not expose a public field whose
  type cannot be named by callers.
- Use `Path` operations rather than string matching. Git worktrees make `.git` a pointer file,
  so obtain administrative paths from `GitRepo`.

## Modules

- Add behavior to the existing owner: `git/` for local Git, `command/` for processes,
  `feedback/` for user-visible results, `menu/` for modal interaction, `section/` for list
  composition, and `render/` for drawing.
- Keep render functions side-effect free.
- Extend `CommandRequest`, `MenuStack`, `FeedbackManager`, or `SectionRegistry` instead of
  introducing parallel execution, modal, feedback, or index state.
- Prefer a small concrete implementation over an abstraction with only one consumer.

## Concurrency

The application currently uses standard threads and channels around a synchronous UI loop. Do not
add Tokio or another async runtime for hypothetical provider work. First establish a concrete
latency/concurrency requirement and an ownership model that keeps rendering responsive.
