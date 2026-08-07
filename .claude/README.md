# Agent Guidance Index

This directory is shared with tools that look for either `.claude/` or `.agents/`; the
repository tracks `.agents` as a compatibility symlink to this directory.

[`AGENTS.md`](../AGENTS.md) is canonical. The files here add focused guidance and must not
contradict it.

| File | Scope |
| --- | --- |
| [`rust-style.md`](rust-style.md) | Rust design, errors, modules, and concurrency |
| [`rust-testing.md`](rust-testing.md) | Unit/integration tests and host watcher verification |
| [`rust-dependencies.md`](rust-dependencies.md) | Direct dependency and audit policy |
| [`rust-packages.md`](rust-packages.md) | Crate layout, public API, and build artifacts |
| [`rust-security.md`](rust-security.md) | Command, Git mutation, path, and future provider safety |

`settings.local.json` is machine-owned and intentionally ignored. Durable project rules belong in
tracked Markdown, architecture docs, or ADRs.
