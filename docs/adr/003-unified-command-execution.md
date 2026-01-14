# ADR-003: Unified Command Execution Framework

## Status
Proposed

## Context
The application currently has multiple paths for executing commands:

1. **`execute_push()`** (app.rs:1672-1783) - Dedicated push logic with:
   - Custom argument building
   - History recording (deduplicated)
   - Always opens popup on failure
   - Auto-popup on long output (>5 lines)

2. **`execute_command()`** (app.rs:2132-2206) - Generic command execution with:
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

### 1. Command Request Type
```rust
pub struct CommandRequest {
    /// Program to execute (e.g., "git", "cargo", "make")
    pub program: String,
    /// Arguments to pass to the program
    pub args: Vec<String>,
    /// Human-readable description for history/popup
    pub display_name: String,
    /// Working directory (None = use repo path)
    pub cwd: Option<PathBuf>,
    /// Source of the command for context-aware feedback
    pub source: CommandSource,
    /// Override default feedback behavior
    pub feedback: FeedbackPolicy,
    /// Whether to refresh git status after execution
    pub refresh_after: bool,
}

pub enum CommandSource {
    Keyboard,       // Direct keybinding
    CommandPalette, // : mode
    ActionMenu,     // m menu
    AliasBrowser,   // a mode
}

pub enum FeedbackPolicy {
    Default,              // Use source-based defaults
    AlwaysPopup,          // Always show popup
    InlineOnly,           // Never auto-popup
    Silent,               // No visible feedback (for background ops)
}
```

### 2. Command Result Type
```rust
pub struct CommandResult {
    pub success: bool,
    pub output: String,
    pub exit_code: Option<i32>,
    pub duration: Duration,
}
```

### 3. Single Execution Entry Point
```rust
fn execute_git_command(&mut self, request: CommandRequest) -> CommandResult {
    // 1. Execute command
    // 2. Capture output (prefer stderr for git)
    // 3. Record in history (always)
    // 4. Determine feedback based on policy and result
    // 5. Refresh status
    // 6. Return result for caller inspection
}
```

### 4. Feedback Determination
Default feedback behavior by source:
| Source | Success | Failure |
|--------|---------|---------|
| Keyboard | Inline (popup if >5 lines) | Always popup |
| CommandPalette | Inline (popup if >5 lines) | Always popup |
| ActionMenu | Always popup | Always popup |
| AliasBrowser | Always popup | Always popup |

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

- All command execution flows through `execute_git_command()`
- `execute_push()` becomes a thin wrapper that builds a `CommandRequest`
- `execute_pull()`, `execute_fetch()` become one-liners
- Action menu and alias browser use the same path with appropriate `CommandSource`
- Output display behavior becomes predictable and documented

## Implementation Plan

1. Define `CommandRequest`, `CommandResult`, `CommandSource`, `FeedbackPolicy` types
2. Implement `execute_git_command()` with unified logic
3. Refactor `execute_push()` to use new framework
4. Refactor `execute_command()` to use new framework
5. Update action menu and alias browser to specify source
6. Add configuration option for default feedback policy (future)
