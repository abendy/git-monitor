# Security and Safety

## Process Execution

- Build commands with `std::process::Command` program and argument fields.
- Never pass user, repository, branch, path, commit, or provider data through shell interpolation.
- Keep the command palette restricted to Git unless an explicit, reviewed allow-list is implemented.
- Preserve the unified `CommandRequest` → `CommandExecutor` path so cwd, feedback, and result
  handling stay consistent.
- External editors, pagers, diff tools, and provider CLIs must inherit only the environment they
  need and must restore the terminal on every exit path.

## Git Mutations

- Passive refresh is the default.
- Require clear confirmation for force push, destructive reset, branch deletion, merge, or any
  provider-visible mutation.
- Prefer `--force-with-lease` over force.
- Show the target repository, branch/PR, important preconditions, and final result.
- Never infer permission to modify a different worktree or repository.

## Paths and Repository Discovery

- Use `Path`/`PathBuf` and Git's discovered workdir, git-dir, and common-dir.
- Do not assume `.git` is a directory; linked worktrees use a pointer file.
- Keep test and temporary paths scoped to `tempfile` directories.
- Avoid following repository-controlled paths into credential files or unrelated user directories.

## Credentials and Providers

Provider adapters are future work. When added:

- Prefer an authenticated provider CLI or OS credential storage.
- Never store tokens in repository config, command history, logs, snapshots, or crash output.
- Redact authorization headers and sensitive query values.
- Make scopes and the active account/host visible before a write.
- Treat remote text, URLs, check output, and agent metadata as untrusted display data.
- Bound cache size and age, and distinguish stale cached data from live state.

## Logging and UI

- Logs must aid diagnosis without containing secrets or full sensitive command environments.
- Sanitize control characters from external text before rendering it in the terminal.
- Report passive failures without crashing the dashboard.
- Ensure terminal teardown runs after errors and external-command handoffs.
