# ADR-004: Command Output and Feedback System

## Status
Implemented

**Implementation Date:** 2026-01-17

Superseded in part: toasts render in the footer, not the header. The inline ✓/✗ status line was not built; inline output uses white/red text and a "more lines" hint.

## Context (Historical)
*This section describes the state that motivated this ADR.*

Command output was previously displayed through multiple mechanisms with inconsistent behavior:

### Previous Output Display Paths

1. **Inline Command Section** (`ui.rs`)
   - Shows first 4 lines of `command_output`
   - Color-coded (white for success, red for failure)
   - "o to expand" hint when output exceeds 4 lines
   - Hidden during alias mode

2. **Full-Screen Popup** (`ui.rs`)
   - Scrollable view of complete output
   - Triggered by:
     - User pressing `o` on command section
     - Auto-open when output > 5 lines (`AUTO_POPUP_LINE_THRESHOLD`)
     - Always on `execute_push()` failure

3. **Error Field** (`app.rs`)
   - `self.error: Option<String>` for transient messages
   - Displayed in header, auto-clears
   - Used for non-command feedback (e.g., "Copied: abc123")

### Problems

1. **Inconsistent auto-popup**: `execute_push()` always opens on failure; `execute_command()` respects threshold
2. **No success indication for short output**: A successful 2-line command shows inline but no explicit "success" signal
3. **Output persistence unclear**: `command_output` persists until next command, which can be confusing
4. **Inline display not scrollable**: Users can only see first 4 lines without opening popup
5. **No unified feedback abstraction**: Different code paths implement their own feedback logic

## Decision
Implement a unified feedback system with clear ownership and consistent behavior.

### 1. Feedback Types
One enum covers command output, transient toasts, and errors. Toasts have levels (info, success, warning). See `Feedback` in `src/feedback/mod.rs` and `Toast`/`ToastLevel` in `src/feedback/toast.rs`.

### 2. Feedback Display Policy
Define when each feedback type triggers each display mechanism:

| Feedback Type | Inline Section | Popup | Toast (Header) |
|--------------|----------------|-------|----------------|
| CommandOutput (success, short) | Yes | No | Optional "Done" |
| CommandOutput (success, long) | Yes | Auto | No |
| CommandOutput (failure) | Yes | Auto | No |
| Toast | No | No | Yes (auto-dismiss) |
| Error | No | Optional | Yes (persist) |

"Short" = output lines <= `AUTO_POPUP_LINE_THRESHOLD` (5)
"Long" = output lines > threshold

### 3. Unified Feedback Entry Point
One method receives all feedback and applies the source-aware popup rule: menu and alias browser always pop up; keyboard and palette pop up on failure or long output. See `FeedbackManager::show()` in `src/feedback/mod.rs`.

### 4. Toast System Enhancement
Add auto-dismissing toasts (3 seconds) to the header for lightweight feedback, cleared on tick. See `FeedbackManager::tick()` in `src/feedback/mod.rs`.

### 5. Inline Section Enhancement
Show a clear success or failure marker in the inline command section, with a line count and an `o` expand hint.

## Rationale

1. **Predictable behavior**: Users know what to expect based on command source
2. **Appropriate feedback level**: Quick keyboard commands get lightweight feedback; deliberate menu selections get full popup
3. **Clear success indication**: Toast or inline indicator confirms completion
4. **Unified ownership**: All feedback flows through `FeedbackManager::show()`
5. **Extensibility**: Toast system enables future notifications (e.g., file watcher events)

## Trade-offs

- **More visual elements**: Toast in header adds UI complexity
- **Policy may not match all preferences**: Some users might want all popups, others none
- **Breaking UX change**: Existing users may notice different behavior

## Future Considerations

1. **User-configurable policy**: Settings for `feedback_policy: AlwaysPopup | Default | Minimal`
2. **Sound/system notifications**: For background operations
3. **Output history panel**: Scrollable log of recent command outputs

## Alternatives Considered

1. **Always popup**: Show popup for every command. Rejected as too intrusive for quick operations.

2. **Never auto-popup**: Only show popup on `o` press. Rejected because failures need immediate visibility.

3. **Modal notification**: Full-screen "Command completed" dialog. Rejected as too disruptive.

## Consequences

- Feedback behavior becomes source-aware and predictable
- Toasts carry transient messages; a persistent `error` field remains for errors
- Popup auto-opens consistently based on documented policy
- Users can predict feedback based on how they triggered the command
