# Rust Testing

## Required Commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --locked
```

Run the focused test while iterating, then the complete suite before committing. Build the release
binary when dependency, linking, feature, or packaging behavior changes.

## Test Placement

- Put focused behavior tests in an inline `#[cfg(test)]` module.
- Put public, multi-module workflows in `tests/`.
- Name tests for observable behavior, not implementation steps.
- Test success, empty/boundary state, and the failure that matters to the user.

## Hermetic Repositories

- Use `tempfile` and initialize Git repositories with `git2`.
- Configure test identities only in the temporary repository.
- Never write user history, global Git config, clipboard contents, credentials, or real repository
  state from a test.
- Inject a temporary persistence path or disable persistence in unit tests.
- Cover normal repositories and linked worktrees whenever administrative paths are involved.

## Assertions

Tests may use `expect` and `unwrap` for setup and invariants; messages should name the operation
that failed. Prefer `expect_err`, `matches!`, and equality assertions when they communicate the
contract more clearly.

## Event Tests

Prefer channels with bounded `recv_timeout` waits. A short initialization delay may be necessary
for platform watchers, but do not turn fixed sleeps into the assertion.

macOS FSEvents can be blocked by a filesystem sandbox. If only watcher event tests fail in that
environment, rerun the unchanged test command in host context. Initialization and categorization
tests should still run in the sandbox.

## Behavior Verification

A green compile does not prove a UI or repository-state change. Verify the smallest public behavior
affected, then run the unchanged full path. For interaction changes, test the state transition and
the displayed feedback or follow-up action where practical.
