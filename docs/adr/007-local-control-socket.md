# ADR-007: Local Control Socket for Agents

## Status
Proposed

**Date:** 2026-09-27

**Tracking:** LON-153 (blocked by LON-134)

## Context

Agents working beside git-monitor can only see it through the terminal: `tmux capture-pane`
returns cropped, truncated text, and the agent has to guess which item the cursor is on. Driving
it means sending raw keystrokes. Neither gives structured state, and neither lets an agent refer
to "the commit you're looking at" reliably.

ADR-006 made this practical: Git data is one `RepoSnapshot`, and the selection has a stable
identity (`SelectionKey`). Both can be answered without reading the screen.

Two existing constraints apply:

- `.claude/rust-dependencies.md` defers a serialization framework and any plugin protocol until
  a concrete provider or backend path needs them.
- LON-139 rules out a generic external-binary/IPC *section platform* (outside programs
  contributing UI sections).

## Decision

1. **Unix domain socket per running instance.** The app listens on a socket in the worktree's
   git-dir (`<git-dir>/git-monitor.sock`), created with owner-only permissions and removed on
   exit. If the path exceeds the platform socket-path limit, fall back to a per-user runtime
   directory keyed by a hash of the git-dir.
2. **Line-delimited JSON requests and responses.** One request per line, one response per line,
   each with a request id. Versioned from the first message (`{"v":1,...}`).
3. **Main loop stays synchronous.** A listener thread parses requests and forwards them as a new
   `Event::Control` on the existing `EventHandler` channel (the same path the file watcher uses),
   with a reply channel. The main loop answers from `App` state; no async runtime.
4. **Capabilities in tiers:**
   - **Read:** snapshot, current selection (`SelectionKey`), open popup/diff, view mode.
   - **Navigate:** select an item by key, expand/collapse, open a diff, switch history mode.
     These change the view, never the repository.
   - **Mutate (later):** stage, commit, push, and similar. A request only *proposes* the action;
     the app shows its normal confirmation in the TUI and the human decides. The socket never
     bypasses `CommandRequest`/`CommandExecutor` or the confirmation rules in AGENTS.md.
5. **Not a section platform.** Clients query and steer the app; they cannot add sections, render
   UI, or register actions. LON-139's non-goal stands.
6. **Sequenced after LON-134.** The GitHub read adapter introduces `serde` for `gh` JSON; the
   socket reuses it rather than being the reason it is added.

## Rationale

- Structured state beats screen scraping: no truncation, no guessing the cursor.
- A shared selection identity lets a human and an agent talk about the same item.
- Routing through the event channel keeps one owner of `App` state and the synchronous design.
- Tiered capabilities keep the passive-first product intent: reading and navigating are safe;
  anything consequential stays human-gated.

## Trade-offs

- **New attack surface.** Any process running as the user can connect. Owner-only permissions
  limit it to the same user, who can already run `git` directly; mutations still need a human
  confirmation in the TUI.
- **Unix only.** Windows would need named pipes. Acceptable while the tool targets macOS/Linux.
- **One more thing to keep stable.** The protocol is versioned from day one so it can change
  without breaking clients silently.
- **Stale sockets.** A crash can leave a socket file behind; startup must detect a dead socket
  (connect fails) and replace it, and must not steal a live one from another instance.

## Alternatives Considered

1. **`git-monitor snapshot --json` (headless).** Simple, but only sees Git data, not what the
   human is looking at, and cannot navigate. Could still be added as a thin client of the same
   request handlers.
2. **State file written on change.** Read-only and polling-based; no navigation, no replies.
3. **Keystrokes via `tmux send-keys`.** Works today with no code, but is blind, layout-dependent,
   and breaks when bindings change.
4. **HTTP server on localhost.** Needs an HTTP stack (explicitly deferred) and is reachable by
   anything that can open a local port, including browsers.

## Consequences

- `Event` gains a `Control` variant carrying a request and a reply sender.
- A `control` module owns the listener, protocol types, and request dispatch; `App` exposes
  handlers that read snapshot/selection state and perform navigation.
- README and ARCHITECTURE document the socket location, protocol version, and capability tiers
  once implemented; this ADR moves to Accepted when the read tier ships.
