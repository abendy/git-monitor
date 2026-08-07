# Repository Guidance

## Product Intent

`git-monitor` is a passive-first terminal dashboard for repository and delivery state. It should
surface what a developer needs to know while agents and humans edit, commit, push, review, and
merge work. Mutating actions are useful, but they must be explicit, contextual, and human-gated
when they are consequential.

The current implementation is a local Git TUI. GitHub, other forge providers, agent activity, and
Jujutsu (`jj`) are roadmap work, not existing capabilities. Keep user-facing claims honest about
that boundary.

## Read Order

1. `README.md` for the current feature set and basic usage.
2. `docs/ROADMAP.md` for product direction and sequencing.
3. `docs/ARCHITECTURE.md` for the implemented system.
4. `docs/adr/` for decisions that still constrain the code.
5. `.agents/README.md` for focused Rust, test, dependency, package, and security guidance.

`.project/` is a local planning archive. It contains useful history and abandoned ideas, but it is
not authoritative. Promote durable decisions to tracked docs or an ADR.

## Required Checks

The repository selects stable Rust through `rust-toolchain.toml` and declares Rust 1.97 as its
minimum supported version.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --release --locked
```

macOS filesystem watcher tests require real FSEvents access. If they fail only in a filesystem
sandbox, rerun the same tests in host context before diagnosing the watcher itself.

## Architecture Boundaries

- `App` owns mutable UI state and runs a synchronous render/event loop.
- `EventHandler` handles terminal input and ticks; `RepoWatcher` handles filesystem changes.
- `GitRepo` is the local Git boundary. Use its discovered worktree, git-dir, and common-dir paths;
  never assume `.git` is a directory.
- `CommandRequest` and `CommandExecutor` are the single command-execution path.
- `MenuStack`, `FeedbackManager`, and the section registry own modal UI, feedback, and list-index
  behavior respectively. Do not bypass them with parallel state systems.
- Rendering consumes state. Network calls, Git commands, and filesystem discovery do not belong in
  render functions.

Provider and VCS integrations should produce provider-neutral state before the UI consumes it.
Do not build a generic plugin or workflow engine until a second concrete adapter or workflow proves
the abstraction.

## Rust Standards

- Keep `unsafe` forbidden.
- Production code must not use `unwrap`, `expect`, `panic`, `todo`, or `unimplemented`.
- Test code may fail fast with `unwrap`/`expect` when the message identifies the failed setup step.
- Use `anyhow::Context` at I/O, Git, process, and parsing boundaries.
- Preserve the synchronous design unless a measured requirement justifies an async runtime.
- Add dependencies only for a concrete use, use minimal features, update `Cargo.lock`, and run the
  dependency audit.
- Keep changes formatter-clean; do not hand-format around `rustfmt.toml`.

## Test Standards

- Put focused behavior tests next to implementation and end-to-end crate behavior in `tests/`.
- Use `tempfile` and temporary Git repositories. Tests must not write to the user's home directory,
  global Git configuration, clipboard, network accounts, or real repositories.
- Test repository discovery against normal repositories and linked worktrees when paths matter.
- Prefer deterministic channels and timeouts over unconditional sleeps for event-driven assertions.
- A passing build is not enough: exercise the behavior changed and then run the full suite.

## Security and Interaction

- The command palette currently accepts Git commands only. Do not loosen that boundary implicitly.
- Construct processes with program/argument APIs; never interpolate user data into a shell command.
- Require confirmation for force push, reset, branch deletion, merge, and other destructive or
  externally visible actions.
- Never persist provider tokens in repository files or application logs. Prefer an installed
  provider CLI or OS credential storage when provider work begins.
- Keep passive refresh failures visible without making the dashboard unusable.

## Documentation and Git

- Update `README.md` for user-visible behavior, `docs/ARCHITECTURE.md` for implemented structure,
  and `docs/ROADMAP.md` for sequencing changes.
- Create or supersede an ADR when changing a durable architectural tradeoff.
- Use focused Conventional Commits and stage files by explicit name.
- Commit working snapshots and push completed work to the current upstream unless the user asks
  otherwise.
