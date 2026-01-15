//! Declarative keybinding definitions.
//!
//! Maps key combinations to actions based on context.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::actions::{AppAction, Context};

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
        let mut keymap = Self::new();
        keymap.add_default_bindings();
        keymap
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

    /// Add default keybindings
    fn add_default_bindings(&mut self) {
        // Global actions
        // Note: 'q' and Esc have context-dependent behavior (quit vs close popup),
        // so they're handled in imperative handlers, not here.
        self.bind_global(KeyBinding::char('?'), AppAction::ShowHelp);
        self.bind_global(KeyBinding::char('r'), AppAction::Refresh);
        self.bind_global(KeyBinding::char('P'), AppAction::Push);
        self.bind_global(KeyBinding::char('p'), AppAction::Pull);
        self.bind_global(KeyBinding::char('f'), AppAction::Fetch);

        // Jump shortcuts
        self.bind_global(KeyBinding::char('w'), AppAction::JumpToWorking);
        self.bind_global(KeyBinding::char('b'), AppAction::JumpToBranches);
        // Note: 'h' has complex behavior (jump vs toggle), handled imperatively

        // Command section
        self.bind(
            Context::Command,
            KeyBinding::char(':'),
            AppAction::EnterCommandMode,
        );
        self.bind(
            Context::Command,
            KeyBinding::char('a'),
            AppAction::BrowseAliases,
        );

        // File sections (staged and working)
        // Note: 'm' for action menu is handled separately (opens modal dialog)
        for context in [Context::StagedFiles, Context::WorkingFiles] {
            self.bind(context, KeyBinding::char('s'), AppAction::ToggleStage);
            self.bind(context, KeyBinding::char('d'), AppAction::ShowDiff);
            self.bind(context, KeyBinding::char(' '), AppAction::FilePagerDiff);
            self.bind(context, KeyBinding::char('M'), AppAction::FileDiffTool);
        }

        // History section
        self.bind(
            Context::HistoryHeader,
            KeyBinding::char('t'),
            AppAction::ToggleHistoryMode,
        );
        self.bind(
            Context::HistoryCommits,
            KeyBinding::key(KeyCode::Enter),
            AppAction::ExpandCommit,
        );
        self.bind(
            Context::HistoryCommits,
            KeyBinding::char('y'),
            AppAction::CopyShortSha,
        );
        self.bind(
            Context::HistoryCommits,
            KeyBinding::char('Y'),
            AppAction::CopyFullSha,
        );
        self.bind(
            Context::HistoryCommits,
            KeyBinding::char('R'),
            AppAction::InteractiveRebase,
        );
        // Pagination uses [ and ] per docs/KEYBINDINGS.md
        self.bind(
            Context::HistoryCommits,
            KeyBinding::char(']'),
            AppAction::NextPage,
        );
        self.bind(
            Context::HistoryCommits,
            KeyBinding::char('['),
            AppAction::PrevPage,
        );

        // Commit files
        self.bind(
            Context::CommitFiles,
            KeyBinding::char(' '),
            AppAction::PagerDiff,
        );
        self.bind(
            Context::CommitFiles,
            KeyBinding::char('d'),
            AppAction::InlineDiff,
        );
        self.bind(
            Context::CommitFiles,
            KeyBinding::char('M'),
            AppAction::DiffTool,
        );

        // Branch section
        self.bind(
            Context::BranchCommits,
            KeyBinding::key(KeyCode::Enter),
            AppAction::Checkout,
        );
        self.bind(
            Context::BranchCommits,
            KeyBinding::char('e'),
            AppAction::ExpandBranch,
        );
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
