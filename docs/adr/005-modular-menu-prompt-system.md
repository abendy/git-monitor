# ADR-005: Modular Menu and Prompt System

## Status
Implemented

**Implementation Date:** 2026-01-17

**What's Done:**
- `Menu` trait with title, items, selected, handle_key, render methods
- `MenuItem` with label, description, key_hint, enabled, style
- `ItemStyle` enum (Normal, Disabled, Separator, Header, Checkbox)
- `MenuResult` enum (Continue, Close, Execute, Push, Pop, CloseAll)
- `MenuAction` for command execution or app actions
- `MenuStack` for nested menu management
- `ActionMenu` - context-filtered action display
- `AliasSectionMenu` / `AliasItemsMenu` - two-level alias browser
- `PushConfirmMenu` - push with force/upstream checkboxes
- `SelectMenu` / `ConfirmMenu` - reusable generic menus
- `render_menu()` helper for consistent rendering
- ViewMode simplified to Normal/Command only
- All modal dialogs migrated to MenuStack

## Context
The application has evolved several interactive overlays that serve different purposes but share common patterns:

### Current Interactive Overlays

1. **Action Menu** (`ViewMode::ActionMenu`)
   - Triggered by `m` key
   - Shows context-filtered actions from the registry
   - Navigable with j/k, selectable with Enter
   - Source: `actions.rs` registry

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
```rust
/// A menu that can be rendered and interacted with
pub trait Menu {
    /// Title for the menu overlay
    fn title(&self) -> &str;

    /// Items to display (label, optional description, optional key hint)
    fn items(&self) -> Vec<MenuItem>;

    /// Current selection index
    fn selected(&self) -> usize;

    /// Handle navigation (returns true if handled)
    fn navigate(&mut self, direction: NavDirection) -> bool;

    /// Handle selection (returns action to take)
    fn select(&mut self) -> MenuResult;

    /// Handle cancel (Escape)
    fn cancel(&mut self) -> MenuResult;
}

pub struct MenuItem {
    pub label: String,
    pub description: Option<String>,
    pub key_hint: Option<String>,
    pub enabled: bool,
    pub style: MenuItemStyle,
}

pub enum MenuItemStyle {
    Normal,
    Highlighted,
    Disabled,
    Separator,
    Checkbox { checked: bool },
}

pub enum MenuResult {
    /// Keep menu open, no action
    None,
    /// Close menu, no action
    Close,
    /// Close menu and execute action
    Action(Box<dyn FnOnce(&mut App)>),
    /// Replace with different menu
    PushMenu(Box<dyn Menu>),
    /// Return to previous menu
    PopMenu,
}
```

### 2. Prompt Types
Standard prompts built on the menu abstraction:

```rust
/// Simple confirmation (yes/no)
pub struct ConfirmPrompt {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub cancel_label: String,
    pub on_confirm: Box<dyn FnOnce(&mut App)>,
}

/// Selection from list
pub struct SelectPrompt<T> {
    pub title: String,
    pub items: Vec<(String, T)>,
    pub selected: usize,
    pub on_select: Box<dyn FnOnce(&mut App, T)>,
}

/// Multi-option form (like push confirmation)
pub struct FormPrompt {
    pub title: String,
    pub fields: Vec<FormField>,
    pub on_submit: Box<dyn FnOnce(&mut App, FormData)>,
}

pub enum FormField {
    Checkbox { label: String, key: String, checked: bool },
    Info { label: String, value: String },
    Separator,
}
```

### 3. Menu Stack
Support nested menus without ViewMode explosion:

```rust
pub struct MenuStack {
    stack: Vec<Box<dyn Menu>>,
}

impl MenuStack {
    pub fn push(&mut self, menu: Box<dyn Menu>);
    pub fn pop(&mut self) -> Option<Box<dyn Menu>>;
    pub fn current(&self) -> Option<&dyn Menu>;
    pub fn current_mut(&mut self) -> Option<&mut dyn Menu>;
    pub fn is_empty(&self) -> bool;
    pub fn clear(&mut self);
}
```

### 4. Unified Rendering
Single render function for all menus:

```rust
fn render_menu_overlay(frame: &mut Frame, menu: &dyn Menu, area: Rect) {
    // Clear area
    // Render title
    // Render items with selection highlight
    // Render footer with context-appropriate hints
}
```

### 5. Simplified ViewMode
Reduce ViewMode to essential states:

```rust
pub enum ViewMode {
    /// Normal navigation
    Normal,
    /// Typing in command input
    Command,
    /// A menu or prompt is open (handled by MenuStack)
    Menu,
}
```

## Migration Path

### Action Menu → SelectPrompt
```rust
// Before: ViewMode::ActionMenu { selected, actions, origin_context }
// After:
let menu = SelectPrompt {
    title: format!("{} Actions", context.display_name()),
    items: actions.iter().map(|a| (a.label.clone(), a.clone())).collect(),
    selected: 0,
    on_select: Box::new(|app, action| app.execute_action(action)),
};
app.menu_stack.push(Box::new(menu));
```

### Alias Browser → Nested SelectPrompt
```rust
// Section selection
let sections_menu = SelectPrompt {
    title: "Aliases",
    items: config.sections.iter().map(|s| (s.name.clone(), s.clone())).collect(),
    on_select: Box::new(|app, section| {
        // Push nested menu
        let aliases_menu = SelectPrompt {
            title: section.name.clone(),
            items: section.aliases.iter().map(|a| (a.name.clone(), a.clone())).collect(),
            on_select: Box::new(|app, alias| app.run_alias(alias)),
        };
        app.menu_stack.push(Box::new(aliases_menu));
    }),
};
```

### Push Confirmation → FormPrompt
```rust
let push_form = FormPrompt {
    title: "Push to Remote",
    fields: vec![
        FormField::Info { label: "Branch", value: branch.clone() },
        FormField::Info { label: "Remote", value: remote.clone() },
        FormField::Separator,
        FormField::Checkbox { label: "Force push", key: "force", checked: false },
        FormField::Checkbox { label: "Set upstream", key: "upstream", checked: !has_upstream },
    ],
    on_submit: Box::new(|app, data| {
        let force = data.get_bool("force");
        let upstream = data.get_bool("upstream");
        app.execute_push(force, upstream);
    }),
};
```

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

- ViewMode simplifies to 3 variants
- All interactive overlays render through `render_menu_overlay()`
- Keyboard handling in menu mode delegates to `MenuStack`
- New prompts/menus are easy to add without touching core state
- Action menu, alias browser, and confirmation dialogs share infrastructure

## Implementation Plan

1. Define `Menu` trait and `MenuItem` types
2. Implement `MenuStack` with push/pop/render
3. Create `SelectPrompt`, `ConfirmPrompt`, `FormPrompt` implementations
4. Add `ViewMode::Menu` and integrate with event loop
5. Migrate ActionMenu to SelectPrompt
6. Migrate Push confirmation to FormPrompt
7. Migrate Alias browser to nested SelectPrompts
8. Remove obsolete ViewMode variants
9. Update rendering to use unified `render_menu_overlay()`

## Related ADRs

- ADR-002: Contextual Action Menu Framework (action registry, context detection)
- ADR-003: Unified Command Execution Framework (action execution path)
- ADR-004: Command Output and Feedback System (post-action feedback)
