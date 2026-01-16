//! Declarative keybinding definitions.
//!
//! Maps key combinations to actions based on context.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::actions::{ActionRegistry, ActionType, AppAction, Context};

/// A key binding (key code + modifiers)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    /// The key code
    pub code: KeyCode,
    /// Required modifiers (Ctrl, Alt, Shift)
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    /// Create a new key binding
    #[must_use]
    pub const fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Create a key binding with no modifiers
    #[must_use]
    pub const fn key(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::NONE)
    }

    /// Create a key binding for a character
    #[must_use]
    pub const fn char(c: char) -> Self {
        Self::key(KeyCode::Char(c))
    }

    /// Create a key binding with Ctrl modifier
    #[must_use]
    #[allow(dead_code)] // Reserved for future Ctrl-key bindings
    pub const fn ctrl(code: KeyCode) -> Self {
        Self::new(code, KeyModifiers::CONTROL)
    }

    /// Create a key binding for Ctrl+char
    #[must_use]
    #[allow(dead_code)] // Reserved for future Ctrl-key bindings
    pub const fn ctrl_char(c: char) -> Self {
        Self::ctrl(KeyCode::Char(c))
    }

    #[must_use]
    pub fn label(&self) -> String {
        let mut parts = Vec::new();

        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("Ctrl");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("Alt");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("Shift");
        }

        let key = match self.code {
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::F(n) => format!("F{n}"),
            _ => format!("{:?}", self.code),
        };

        if parts.is_empty() {
            key
        } else {
            format!("{}+{}", parts.join("+"), key)
        }
    }
}

impl From<KeyEvent> for KeyBinding {
    fn from(event: KeyEvent) -> Self {
        Self {
            code: event.code,
            modifiers: event.modifiers,
        }
    }
}

/// Keymap for a specific context
type ContextMap = HashMap<KeyBinding, AppAction>;

/// Declarative keymap that maps key bindings to actions
#[derive(Debug, Clone, Default)]
pub struct Keymap {
    /// Global bindings (always checked)
    global: ContextMap,
    /// Context-specific bindings
    contextual: HashMap<Context, ContextMap>,
}

impl Keymap {
    /// Create a new empty keymap
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a keymap with default bindings
    #[must_use]
    pub fn with_defaults() -> Self {
        let registry = ActionRegistry::new();
        Self::from_registry(&registry)
    }

    /// Add a global binding
    pub fn bind_global(&mut self, binding: KeyBinding, action: AppAction) {
        self.global.insert(binding, action);
    }

    /// Add a contextual binding
    pub fn bind(&mut self, context: Context, binding: KeyBinding, action: AppAction) {
        self.contextual
            .entry(context)
            .or_default()
            .insert(binding, action);
    }

    /// Look up an action for a key event in a given context
    #[must_use]
    pub fn lookup(&self, key: KeyEvent, context: Context) -> Option<AppAction> {
        let binding = KeyBinding::from(key);

        // Check contextual bindings first
        if let Some(ctx_map) = self.contextual.get(&context) {
            if let Some(action) = ctx_map.get(&binding) {
                return Some(*action);
            }
        }

        // Fall back to global bindings
        self.global.get(&binding).copied()
    }

    /// Build keybindings from the action registry
    #[must_use]
    pub fn from_registry(registry: &ActionRegistry) -> Self {
        let mut keymap = Self::new();

        for action in registry.actions() {
            let ActionType::App(app_action) = action.action_type else {
                continue;
            };
            let Some(binding) = action.binding else {
                continue;
            };

            if action.contexts.contains(&Context::Global) {
                keymap.bind_global(binding, app_action);
            }

            for context in &action.contexts {
                if *context == Context::Global {
                    continue;
                }
                keymap.bind(*context, binding, app_action);
            }
        }

        keymap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_binding_from_event() {
        let event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let binding = KeyBinding::from(event);
        assert_eq!(binding.code, KeyCode::Char('q'));
        assert_eq!(binding.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn lookup_global_binding() {
        let keymap = Keymap::with_defaults();
        // '?' is globally bound to ShowHelp
        let event = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE);
        let action = keymap.lookup(event, Context::Global);
        assert_eq!(action, Some(AppAction::ShowHelp));

        // 'r' is globally bound to Refresh
        let event = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
        let action = keymap.lookup(event, Context::Global);
        assert_eq!(action, Some(AppAction::Refresh));
    }

    #[test]
    fn lookup_contextual_binding() {
        let keymap = Keymap::with_defaults();
        let event = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);

        // In staged files context, 's' should toggle stage
        let action = keymap.lookup(event, Context::StagedFiles);
        assert_eq!(action, Some(AppAction::ToggleStage));

        // In global context, 's' should not be bound
        let action = keymap.lookup(event, Context::Global);
        assert_eq!(action, None);
    }
}
