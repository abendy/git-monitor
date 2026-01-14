# ADR-001: Press Enter Prompt After External Diff Commands

## Status
Accepted

## Context
When viewing commit file diffs, users can press `Space` to open the diff in an external pager (respecting `core.pager` from gitconfig) or `M` to open in an external difftool (respecting `diff.tool` from gitconfig).

The pager configuration may include flags like `-F` (quit if content fits on one screen) which causes the pager to exit immediately for small diffs. This results in the diff "flashing" on screen before the TUI resumes.

## Decision
After running an external diff command, display a "[Press Enter to continue]" prompt and wait for user input before resuming the TUI.

## Rationale
1. **Accommodates various pager configurations**: Users with `-F` flag or similar settings can still read the diff output before returning to the TUI
2. **Consistent experience**: The behavior is predictable regardless of diff size or pager settings
3. **Simple implementation**: Avoids complex heuristics like "only prompt if command exited within N seconds"
4. **User control**: The user decides when they're ready to return to the TUI

## Trade-offs
- Adds one extra keypress for users whose pager already waits for quit (e.g., `q` in less)
- Couples the behavior to the author's preferred workflow

## Alternatives Considered
1. **No prompt**: Trust the user's pager config entirely. Rejected because small diffs would flash by with `-F` flag.
2. **Heuristic prompt**: Only show prompt if command exits within ~1 second. Rejected as overly complex.
3. **Override pager flags**: Force specific less flags. Rejected as it ignores user's gitconfig preferences.

## Consequences
- Users see a consistent "[Press Enter to continue]" after every external diff
- The `d` key remains available for inline diff viewing without the prompt
- External difftool (`M` key) also shows the prompt, though GUI tools like meld manage their own window lifecycle
