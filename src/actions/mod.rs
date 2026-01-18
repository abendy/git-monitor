//! Contextual action menu framework.
//!
//! Provides context-aware actions that change based on cursor position,
//! supporting app actions, CLI commands, and git aliases.

mod context;
mod registry;
mod state;
mod types;

pub use context::Context;
pub use registry::ActionRegistry;
pub use state::AppState;
pub use types::{Action, ActionType, AppAction};
