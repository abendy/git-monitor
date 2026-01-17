# ADR-004: Command Output and Feedback System

## Status
Implemented

**Implementation Date:** 2026-01-17

**What's Done:**
- `Feedback` enum with CommandOutput, Toast, Error, Diff variants
- `FeedbackManager` for centralized feedback state
- `PopupContent` and `PopupState` for scrollable overlays
- `Toast` and `ToastLevel` types defined
- Source-aware feedback policy (menu/browser → popup, keyboard → conditional)
- Integration with unified command execution framework
- Toast rendering in footer with level-based coloring
- Auto-dismiss tick handling for toast expiry

## Context (Historical)
*This section describes the pre-integration state that motivated this ADR. The system has since been unified as described in "What's Done" above.*

Command output was previously displayed through multiple mechanisms with inconsistent behavior:

### Previous Output Display Paths

1. **Inline Command Section** (ui.rs:270-296)
   - Shows first 4 lines of `command_output`
   - Color-coded (white for success, red for failure)
   - "o to expand" hint when output exceeds 4 lines
   - Hidden during alias mode

2. **Full-Screen Popup** (ui.rs:1224-1280)
   - Scrollable view of complete output
   - Triggered by:
     - User pressing `o` on command section
     - Auto-open when output > 5 lines (`AUTO_POPUP_LINE_THRESHOLD`)
     - Always on `execute_push()` failure

3. **Error Field** (app.rs)
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
```rust
pub enum Feedback {
    /// Command completed - show output
    CommandOutput {
        command: String,
        output: String,
        success: bool,
        source: CommandSource,
    },
    /// Transient notification (auto-dismisses)
    Toast {
        message: String,
        level: ToastLevel,
    },
    /// Error requiring acknowledgment
    Error {
        message: String,
        recoverable: bool,
    },
}

pub enum ToastLevel {
    Info,    // Blue - informational
    Success, // Green - operation completed
    Warning, // Yellow - completed with concerns
}
```

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
```rust
impl App {
    fn show_feedback(&mut self, feedback: Feedback) {
        match feedback {
            Feedback::CommandOutput { command, output, success, source } => {
                self.command_output = output.clone();
                self.command_success = success;

                let should_popup = match source {
                    // Menu/browser always popup for visibility
                    CommandSource::ActionMenu | CommandSource::AliasBrowser => true,
                    // Keyboard/palette: popup on failure or long output
                    _ => !success || output.lines().count() > AUTO_POPUP_LINE_THRESHOLD,
                };

                if should_popup {
                    self.popup.open(PopupContent::CommandOutput {
                        command,
                        output,
                        success,
                    });
                }
            }
            Feedback::Toast { message, level } => {
                self.toast = Some(Toast { message, level, created: Instant::now() });
            }
            Feedback::Error { message, recoverable } => {
                self.error = Some(message);
                if !recoverable {
                    // Could trigger error popup in future
                }
            }
        }
    }
}
```

### 4. Toast System Enhancement
Add auto-dismissing toasts to the header for lightweight feedback:

```rust
pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub created: Instant,
}

const TOAST_DURATION: Duration = Duration::from_secs(3);

impl App {
    fn tick(&mut self) {
        // Auto-dismiss expired toasts
        if let Some(toast) = &self.toast {
            if toast.created.elapsed() > TOAST_DURATION {
                self.toast = None;
            }
        }
    }
}
```

### 5. Inline Section Enhancement
Improve the command section to show clearer status:

```
Success case:
  : git fetch
  > Fetching origin...
  > Done (2 lines)  ✓

Failure case:
  : git push
  > error: failed to push
  > (1 more line)  ✗ o expand
```

## Rationale

1. **Predictable behavior**: Users know what to expect based on command source
2. **Appropriate feedback level**: Quick keyboard commands get lightweight feedback; deliberate menu selections get full popup
3. **Clear success indication**: Toast or inline indicator confirms completion
4. **Unified ownership**: All feedback flows through `show_feedback()`
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
- `error` field evolves into `toast` for transient messages
- Popup auto-opens consistently based on documented policy
- Users can predict feedback based on how they triggered the command

## Implementation Plan (Completed)

*All steps completed as of 2026-01-14. See "What's Done" section for details.*

1. ~~Add `Toast` struct and rendering in header~~
2. ~~Implement `show_feedback()` method~~
3. ~~Refactor `execute_git_command()` to use `show_feedback()`~~
4. ~~Update inline command section with success/failure indicators~~
5. ~~Add tick-based toast dismissal~~
6. Document feedback policy in help overlay (partial - covered in KEYBINDINGS.md)
