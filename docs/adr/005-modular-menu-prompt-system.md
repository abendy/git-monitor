# ADR-005: Modular Menu and Prompt System

## Status
Implemented

**Implementation Date:** 2026-01-17

Superseded in part:
- `ViewMode` has two variants (`Normal`, `Command`); an open menu is tracked by `MenuStack`, not `ViewMode::Menu`.
- Menus return a `MenuAction` enum (command, app action, or callback) instead of `FnOnce(&mut App)` closures.
- Each menu handles its own keys and renders itself, usually through the shared `render_menu()` helper.
- The generic `FormPrompt` became a dedicated `PushConfirmMenu`.

## Context
The application has evolved several interactive overlays that serve different purposes but share common patterns:

### Current Interactive Overlays

1. **Action Menu** (`ViewMode::ActionMenu`)
   - Triggered by `m` key
   - Shows context-filtered actions from the registry
   - Navigable with j/k, selectable with Enter
   - Source: `src/actions/` registry

2. **Alias Browser** (`ViewMode::AliasSections`, `ViewMode::AliasItems`)
   - Triggered by `a` key from command section
   - Two-level hierarchy: sections → aliases
   - Navigable, selectable, executes git aliases
   - Source: `config.rs` gitconfig parsing

3. **Push Confirmation** (`ViewMode::Confirm(ConfirmAction::Push)`)
   - Triggered by `P` key (or action menu)
   - Shows branch/remote info with options
   - Has checkbox-style options (force, set-upstream)
   - Keyboard: y/n, f toggle, Enter confirm

4. **Help Overlay** (`show_help: bool`)
   - Triggered by `?` key
   - Static content, closes on any key
   - Not a ViewMode, separate flag

5. **Output Popup** (`PopupState`)
   - Triggered by `o` key or auto-open
   - Scrollable content viewer
   - Not a menu, but shares overlay behavior

### Problems

1. **No unified abstraction**: Each overlay is implemented independently with similar but subtly different behavior
2. **Inconsistent keyboard handling**: Escape closes some overlays but not others uniformly
3. **No composition**: Can't show a confirmation after action menu selection without state juggling
4. **ViewMode proliferation**: Adding new prompts requires new ViewMode variants
5. **No standard menu component**: Each menu reimplements navigation, selection, rendering

## Decision
Introduce a modular menu/prompt system with composable components.

### 1. Menu Abstraction
A `Menu` trait gives every overlay the same title, items, selection, and key handling. Items carry a label, description, key hint, enabled flag, and style (including checkboxes). Key handling returns a result that keeps the menu open, closes it, runs an action, or pushes/pops a nested menu. See `Menu`, `MenuItem`, `ItemStyle`, `MenuResult`, and `MenuAction` in `src/menu/mod.rs`.

### 2. Prompt Types
Standard prompts built on the menu abstraction: confirm (yes/no), select from a list, and a multi-option form (like push confirmation). See `ConfirmMenu` in `src/menu/confirm.rs`, `SelectMenu` in `src/menu/select.rs`, and `PushConfirmMenu` in `src/menu/push.rs`.

### 3. Menu Stack
Support nested menus without ViewMode explosion. See `MenuStack` in `src/menu/stack.rs`.

### 4. Unified Rendering
One render path for all menus. See `render_menu()` in `src/menu/mod.rs` and the overlay in `src/render/menu.rs`.

### 5. Simplified ViewMode
Reduce ViewMode to essential states: normal navigation, command input, and menu open.

## Migration Path

- **Action menu** becomes a select menu over context-filtered actions (`src/menu/action.rs`).
- **Alias browser** becomes nested select menus: sections, then aliases (`src/menu/alias.rs`).
- **Push confirmation** becomes a form with branch/remote info and force/upstream checkboxes (`src/menu/push.rs`).

## Rationale

1. **Consistency**: All menus behave identically for navigation and dismissal
2. **Composability**: Menus can be nested or chained without ViewMode changes
3. **Reusability**: Common patterns (confirm, select, form) are standardized
4. **Extensibility**: New menu types implement the trait without core changes
5. **Testability**: Menu logic is isolated from rendering

## Trade-offs

- **Significant refactor**: Existing ViewMode handling must be rewritten
- **Trait object overhead**: Dynamic dispatch has minor performance cost
- **Learning curve**: Contributors must understand the menu abstraction
- **Closure complexity**: `Box<dyn FnOnce>` patterns require careful lifetime handling

## Alternatives Considered

1. **Keep ViewMode variants**: Continue adding variants as needed. Rejected because it doesn't scale and leads to duplicated logic.

2. **Enum-based menus**: Use an enum of menu types instead of trait objects. Rejected because it doesn't allow external extension.

3. **ECS-style composition**: Components for menu state. Rejected as overkill for this application.

## Consequences

- ViewMode simplifies to 2 variants (see Status)
- Menus render through the menu stack overlay
- Keyboard handling in menu mode delegates to `MenuStack`
- New prompts/menus are easy to add without touching core state
- Action menu, alias browser, and confirmation dialogs share infrastructure

## Related ADRs

- ADR-002: Contextual Action Menu Framework (action registry, context detection)
- ADR-003: Unified Command Execution Framework (action execution path)
- ADR-004: Command Output and Feedback System (post-action feedback)
