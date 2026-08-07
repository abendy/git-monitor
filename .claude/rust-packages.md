# Crate and Package Structure

`git-monitor` is intentionally one Cargo package with a reusable library target and a thin binary
entry point. It is not a workspace.

```text
src/
├── lib.rs          public module surface
├── main.rs         CLI, logging, terminal lifecycle
├── app/            application state and transitions
├── git/            local Git boundary
├── command/        process requests and execution
├── input/          keymap and input dispatch
├── menu/           modal interaction
├── feedback/       errors, output, toasts, popups
├── section/        built-in dashboard sections
└── render/         side-effect-free drawing
```

## Package Rules

- Keep `main.rs` orchestration-only; reusable behavior belongs in the library.
- Re-export only types that form a deliberate public API.
- Do not split into a workspace until a second independently buildable package exists.
- Do not create a provider SDK crate before two adapters demonstrate a stable boundary.
- Keep platform-specific filesystem behavior behind the watcher boundary.

## Build and Install

```bash
cargo check --all-targets --all-features --locked
cargo build --release --locked
cargo install --locked --path .
```

The release artifact must be native to the target host and must not link to developer-specific
package-manager paths. Keep `Cargo.lock`, `rust-toolchain.toml`, Clippy configuration, rustfmt
configuration, and CI commands aligned.
