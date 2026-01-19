//! Git Monitor - Real-time TUI for monitoring git repository activity.
//!
//! This library provides the core functionality for the git-monitor TUI application.

// Re-export modules for integration testing and library usage
pub mod actions;
pub mod app;
pub mod command;
pub mod config;
pub mod event;
pub mod feedback;
pub mod git;
pub mod input;
pub mod menu;
pub mod render;
pub mod section;
pub mod tui;
pub mod ui;
pub mod watcher;

// Re-export commonly used types at crate root
pub use app::App;
pub use command::{CommandExecutor, CommandRequest, CommandResult};
pub use git::GitRepo;
