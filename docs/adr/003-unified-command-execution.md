# ADR-003: Unified Command Execution Framework

## Status
Implemented

**Implementation Date:** 2026-01-17

## Context
The application currently has multiple paths for executing commands:

1. **`execute_push()`** (then in `app.rs`) - Dedicated push logic with:
   - Custom argument building
   - History recording (deduplicated)
   - Always opens popup on failure
   - Auto-popup on long output (>5 lines)

2. **`execute_command()`** (then in `app.rs`) - Generic command execution with:
   - Whitespace-parsed command input
   - History recording
   - Threshold-based popup (>5 lines only)
   - No special failure handling

3. **Wrapper functions** - `execute_pull()` and `execute_fetch()` simply set `command_input` and call `execute_command()`

These paths are invoked from different triggers:
- Direct keyboard shortcuts (e.g., `P` for push, `f` for fetch)
- Command palette (`:` mode)
- Action menu (`m` key)
- Alias browser (`a` key)
- Conditional actions (context-dependent shortcuts)

### Problems

1. **Inconsistent feedback**: Push always shows popup on failure; generic commands only show popup if output exceeds threshold
2. **Lost context**: The source of command execution (keyboard, menu, alias) is lost, preventing context-aware feedback
3. **Duplicated logic**: Output capture, history recording, and status refresh are repeated across paths
4. **No unified result type**: Each path handles success/failure differently
5. **Unclear ownership**: It's not clear which component is responsible for feedback vs execution

## Decision
Introduce a unified command execution framework with these components:

1. **Command request** - One type describing what to run: program, args, display name, working directory, source, feedback policy, and whether to refresh status after. See `CommandRequest`, `CommandSource`, and `FeedbackPolicy` in `src/command/mod.rs`.
2. **Command result** - One type for success, output, exit code, and duration. See `CommandResult` in `src/command/executor.rs`.
3. **Single execution entry point** - Execute, capture output, record history, choose feedback, refresh status. See `App::run_command()` in `src/app/commands.rs`.
4. **Feedback determination** - Default feedback behavior by source:

| Source | Success | Failure |
|--------|---------|---------|
| Keyboard | Inline (popup if >5 lines) | Always popup |
| Command palette | Inline (popup if >5 lines) | Always popup |
| Action menu | Always popup | Always popup |
| Alias browser | Always popup | Always popup |

Rationale: Menu/browser actions are deliberate selections that deserve immediate feedback; keyboard shortcuts are often quick operations where inline suffices.

## Rationale

1. **Consistency**: All commands follow the same execution path with predictable behavior
2. **Flexibility**: FeedbackPolicy allows overrides for specific commands
3. **Traceability**: CommandSource enables analytics and context-aware UX
4. **Testability**: Single entry point is easier to test and mock
5. **Extensibility**: New sources or policies can be added without modifying core logic

## Trade-offs

- **Migration effort**: Existing `execute_push()` and `execute_command()` must be refactored
- **Complexity**: More types to manage, though they enable better organization
- **Breaking change**: Internal API changes, though external behavior improves

## Alternatives Considered

1. **Keep dual paths, add consistency patches**: Add popup-on-failure to `execute_command()`. Rejected because it doesn't address root cause and adds more special cases.

2. **Event-based execution**: Commands emit events, listeners handle feedback. Rejected as overengineered for current scope.

3. **Trait-based polymorphism**: Different command types implement a trait. Rejected because the variation is in policy, not behavior.

## Consequences

- All captured command execution flows through `App::run_command()`
- Push builds a `CommandRequest` from the push confirmation menu (`src/menu/push.rs`)
- `execute_pull()`, `execute_fetch()` become one-liners
- Action menu and alias browser use the same path with appropriate `CommandSource`
- Output display behavior becomes predictable and documented
