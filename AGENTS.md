# Repository Guidance

This file is the canonical guidance for every coding agent working in this repository.

## Product Intent

`git-monitor` is a passive-first terminal dashboard for repository and delivery state. It surfaces
what a developer needs to know while agents and humans edit, commit, push, review, and merge work.

It is also where the maintainer reviews work. Reading diffs, spot-checking code, and reviewing pull
requests should be comfortable here, so an editor is only needed now and then. Mutating actions are
useful, but they must be explicit, contextual, and human-gated when they are consequential.

Today it is a local Git TUI. GitHub, other forge providers, and agent activity are roadmap work, not
existing capabilities. Keep user-facing claims honest about that boundary.

## Read Order

1. `README.md` for the current feature set and usage.
2. `docs/KEYBINDINGS.md` and `docs/UI_DESIGN.md` for interaction and layout.
3. `docs/ROADMAP.md` for direction and sequencing.
4. `docs/adr/` for decisions that still constrain the code.

Work is tracked in Linear (project `git-monitor`, team Lone Space), not in repository files.

## Required Checks

Stable Rust is selected by `rust-toolchain.toml`; the minimum supported version is in `Cargo.toml`.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --release --locked
```

Run the focused test while iterating, then the full suite before committing. macOS watcher tests
need real FSEvents access; if only they fail inside a sandbox, rerun them in host context before
debugging the watcher.

## Architecture Boundaries

- `App` owns mutable UI state and runs a synchronous render/event loop. Background threads (input,
  watcher) talk to it only through the `EventHandler` channel.
- Git data lives in `RepoSnapshot` (`src/git/snapshot.rs`). Load it only through
  `App::load_snapshot`, which swaps it in whole; never edit it in place. Cursor, scroll, expansion,
  and page stay on `App` (ADR-006).
- Track the cursor by `SelectionKey` (path, SHA, or branch), not by row index, across anything that
  can add or remove rows.
- `GitRepo` is the local Git boundary. Use its worktree, git-dir, and common-dir paths; never assume
  `.git` is a directory.
- `CommandRequest` and `CommandExecutor` are the single command-execution path.
- `MenuStack`, `FeedbackManager`, and `SectionRegistry` own modal UI, user-visible results, and
  list indexing. Extend them rather than adding parallel state.
- Add behavior to its owner: `git/` local Git, `command/` processes, `feedback/` results, `menu/`
  modals, `section/` list composition, `render/` drawing, `app/` state transitions and input.
- Rendering consumes state. Git commands, network calls, and filesystem discovery never run in
  render functions.
- One Cargo package with a library and a thin `main.rs`; not a workspace.

Provider and agent integrations produce provider-neutral state before the UI consumes it. Build an
abstraction only after a second concrete case needs it: no traits, plugin protocols, or workflow
engines with one consumer.

## Rust Standards

- `unsafe` stays forbidden. Clippy pedantic and nursery lints are on, and the required check treats
  warnings as errors.
- Production code never uses `unwrap`, `expect`, `panic`, `todo`, or `unimplemented`.
- Add `anyhow::Context` at I/O, Git, process, parsing, and terminal boundaries. Don't discard
  errors silently; passive refresh failures stay visible without breaking the dashboard.
- Use an enum, not coupled booleans, when a state has more than two valid values.
- Measure terminal text in display columns, not bytes, and cut strings on character boundaries.
- Keep the synchronous design unless a measured requirement justifies an async runtime.
- Let rustfmt decide layout.

## Dependencies

Add a dependency only for a concrete call site the standard library and existing crates can't
serve. Use minimal features, commit `Cargo.lock`, and run `cargo audit`. Don't add a serialization
framework, HTTP client, or async runtime until a concrete provider path needs it; GitHub work
starts with the authenticated `gh` CLI. `git2` vendors OpenSSL so release binaries don't depend on
Homebrew paths.

## Tests

- Focused tests go in an inline `#[cfg(test)]` module; cross-module behavior goes in `tests/`.
- Name tests for the behavior they prove. Cover the success path, the empty or boundary case, and
  the failure the user would hit.
- Use `tempfile` and `git2` repositories. Tests never touch the user's home, global Git config,
  clipboard, credentials, or real repositories.
- Cover linked worktrees whenever administrative paths are involved.
- Wait on channels with bounded timeouts, not fixed sleeps.
- For UI changes, render into a `TestBackend` and assert on what the user sees. A green build is
  not proof; exercise the changed behavior.
- Test setup may use `unwrap`/`expect` with a message naming the failed step.

## Security and Interaction

- The command prompt runs any line the user types, through their shell. A typed command is its own
  confirmation.
- Only text the user typed may reach a shell. Everything else (menus, aliases, agents, sockets,
  repository content) builds processes with program/argument APIs and never interpolates data into
  a shell command.
- Actions started by a key or menu that force-push, reset, rebase, delete a branch, merge, or are
  otherwise destructive or externally visible must confirm first, showing the target and command.
  Prefer `--force-with-lease` over `--force`.
- Never act on a different worktree or repository than the one shown.
- Treat remote text, URLs, check output, and agent metadata as untrusted. Strip control characters
  before rendering external text.
- Never store provider tokens in repository files, command history, logs, or snapshots. Prefer the
  provider's CLI or OS credential storage.
- Restore the terminal on every exit path, including errors and external-command handoffs.

## Documentation and Git

- Update `README.md` for user-visible behavior, `docs/KEYBINDINGS.md` for key changes, and
  `docs/ROADMAP.md` for sequencing. Don't paste code into docs; name the type and its path.
- Create or supersede an ADR when a durable architectural trade-off changes.
- Use focused Conventional Commits and stage files by name. Add `Refs LON-123` (or `Fixes LON-123`)
  to the body when the work belongs to a Linear issue.
- Commit working snapshots and push completed work to `develop` unless the user asks otherwise.
