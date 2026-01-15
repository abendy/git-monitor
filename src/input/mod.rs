//! Input handling system.
//!
//! Provides declarative keybinding definitions and centralized input processing.
//! The goal is to make key handling predictable and eventually configurable.

mod keymap;

pub use keymap::Keymap;
