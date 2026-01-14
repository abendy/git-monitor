# ADR-002: Contextual Action Menu Framework

## Status
Accepted

## Context
The application has numerous keybindings that vary by context (staged files, working files, history, branches, commit files). Users need to discover available actions, and the UI needs a consistent way to show relevant hints without cluttering every section with static text.

Previously, hints were hardcoded strings scattered throughout `ui.rs`, always visible regardless of cursor position. This created visual noise and made it difficult to maintain consistency between hints and actual keybindings.

## Decision
Implement a centralized contextual action framework consisting of:

1. **Context enum** (`src/actions.rs`) - Formal definitions of UI sections (Command, StagedFiles, WorkingFiles, HistoryHeader, HistoryCommits, CommitFiles, BranchCommits, Global)

2. **Action registry** - Single source of truth for all actions, including:
   - Keyboard shortcut
   - Display label
   - Action type (App action, CLI command, git alias)
   - Applicable contexts
   - Display priority

3. **Dynamic hints** - Replace static hints with context-aware rendering that only shows hints for the active section

4. **Action menu popup** - A discoverable overlay (triggered by `m`) showing all available actions for the current context

## Rationale

1. **Single source of truth**: Actions defined once in the registry, used for both hints and the menu popup. No risk of hints diverging from actual keybindings.

2. **Reduced visual noise**: Hints only appear when relevant, keeping the UI cleaner when navigating between sections.

3. **Discoverability**: Users can press `m` anywhere to see all available actions, including git aliases that match the current context.

4. **Extensibility**: Adding new actions requires only updating the registry. The ActionType enum supports future CLI commands and already integrates git aliases.

5. **Context inference for aliases**: Git aliases are automatically categorized based on their command content (e.g., aliases containing "stash" appear in Working Files context).

## Trade-offs

- **Complexity**: More code than hardcoded strings, but better maintainability
- **Two ways to discover**: Both hints and menu popup exist; some redundancy but serves different user preferences (quick reference vs. full exploration)
- **Alias inference is heuristic**: May occasionally miscategorize aliases based on command keywords

## Alternatives Considered

1. **Static hints everywhere**: Keep existing hardcoded hints. Rejected because they don't adapt to context and create maintenance burden.

2. **vim-style which-key only**: Only show popup on keypress, no inline hints. Rejected because quick inline hints help power users.

3. **Command palette (fuzzy search)**: Search-based action discovery like VS Code. Rejected as overkill for current scope; could be added later.

4. **Separate hint system**: Maintain hints independently from action definitions. Rejected because it would drift out of sync.

## Consequences

- Press `m` to see context-aware action menu
- Inline hints only appear when cursor is in that section
- Adding new keybindings requires updating `ActionRegistry::new()` and the handler in `app.rs`
- Git aliases from user's gitconfig appear in relevant contexts automatically
- Help overlay (`?`) remains as comprehensive static reference

## Implementation

Key files:
- `src/actions.rs` - Context, Action, ActionType, AppAction, ActionRegistry
- `src/app.rs` - ViewMode::ActionMenu, current_context(), execute_app_action()
- `src/ui.rs` - render_action_menu(), render_context_hint()
