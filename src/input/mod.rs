//! Input handling system.
//!
//! Provides declarative keybinding definitions and centralized input processing.
//! The goal is to make key handling predictable and eventually configurable.

mod keymap;

pub use keymap::{KeyBinding, Keymap};

use crate::actions::{AppAction, Context};

/// Result of processing a key event
#[derive(Debug, Clone)]
pub enum InputResult {
    /// Execute an app-level action
    Action(AppAction),
    /// Delegate to the active menu
    DelegateToMenu,
    /// Delegate to popup handler
    DelegateToPopup,
    /// Key was not handled
    Unhandled,
}
