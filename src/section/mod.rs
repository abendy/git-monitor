//! Section abstraction for UI regions.
//!
//! Each section (Command, Staged, Working, History, Branches) implements
//! the Section trait, providing its own rendering, actions, and key handling.

#![allow(dead_code)] // Module in progress - not yet fully integrated

mod branches;
mod commit;
mod command;
mod history;
mod registry;
mod staged;
mod working;

pub use branches::{BranchesSection, BranchesSectionData};
pub use commit::{
    render_commit_detail, render_commit_line, render_context_hint,
};
pub use command::{CommandSection, CommandSectionData};
use crossterm::event::KeyEvent;
pub use history::{HistorySection, HistorySectionData};
use ratatui::text::Line;
pub use registry::{SectionItemCounts, SectionRegistry};
pub use staged::{StagedSection, StagedSectionData};
pub use working::{WorkingSection, WorkingSectionData};

use crate::actions::{Action, Context};
use crate::input::KeyBinding;

/// Unique identifier for each section
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SectionId {
    /// Command input section
    Command,
    /// Staged files section
    Staged,
    /// Working files section
    Working,
    /// History section (commits/reflog)
    History,
    /// Branches section
    Branches,
}

impl SectionId {
    /// Get the display name for this section
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Command => "Command",
            Self::Staged => "Staged",
            Self::Working => "Working",
            Self::History => "History",
            Self::Branches => "Branches",
        }
    }
}

/// Action returned by section key handling
pub enum SectionAction {
    /// Execute a command
    Command(crate::command::CommandRequest),
    /// Show feedback
    Feedback(crate::feedback::Feedback),
    /// Open a menu
    OpenMenu(Box<dyn crate::menu::Menu>),
    /// Delegate to app-level action handler
    AppAction(crate::actions::AppAction),
    /// Navigate to a different index
    Navigate(NavigateAction),
    /// No action taken
    None,
}

impl std::fmt::Debug for SectionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(req) => f
                .debug_tuple("Command")
                .field(req)
                .finish(),
            Self::Feedback(_) => f
                .debug_tuple("Feedback")
                .field(&"...")
                .finish(),
            Self::OpenMenu(_) => f
                .debug_tuple("OpenMenu")
                .field(&"<menu>")
                .finish(),
            Self::AppAction(action) => f
                .debug_tuple("AppAction")
                .field(action)
                .finish(),
            Self::Navigate(nav) => f
                .debug_tuple("Navigate")
                .field(nav)
                .finish(),
            Self::None => write!(f, "None"),
        }
    }
}

/// Navigation actions within or between sections
#[derive(Debug, Clone, Copy)]
pub enum NavigateAction {
    /// Move to next item
    Next,
    /// Move to previous item
    Prev,
    /// Move to first item
    First,
    /// Move to last item
    Last,
    /// Jump to specific section
    JumpTo(SectionId),
}

/// Defines when a section should refresh its data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RefreshPolicy {
    /// Refresh when file system changes are detected (default for git sections)
    #[default]
    OnFileChange,
    /// Refresh at a fixed interval (for remote/API sections)
    Interval {
        /// Interval in seconds between refreshes
        seconds: u32,
    },
    /// Only refresh when user explicitly requests (e.g., 'r' key)
    Manual,
    /// Never refresh automatically (static content)
    Never,
}

/// A keybinding declared by a section.
///
/// Built-in sections use ActionRegistry for keybindings. This metadata is
/// reserved for external sections that need to self-describe bindings without
/// touching the core registry.
#[derive(Debug, Clone)]
pub struct SectionKeybinding {
    /// The key combination that triggers this action
    pub key: KeyBinding,
    /// Short label for the action (e.g., "Stage", "Diff")
    pub label: &'static str,
    /// Longer description for help display
    pub description: &'static str,
}

impl SectionKeybinding {
    /// Create a new section keybinding
    #[must_use]
    pub const fn new(key: KeyBinding, label: &'static str, description: &'static str) -> Self {
        Self {
            key,
            label,
            description,
        }
    }
}

/// State passed to sections for rendering and actions
#[derive(Debug, Clone)]
pub struct SectionState {
    /// Whether this section contains the current selection
    pub is_focused: bool,
    /// Index of selection within this section (if focused)
    pub local_selection: Option<usize>,
    /// Global selection index
    pub global_selection: Option<usize>,
    /// Available render width for message truncation
    pub render_width: u16,
}

/// A UI section that can be rendered and interacted with
pub trait Section: Send + Sync {
    /// Unique identifier for this section
    fn id(&self) -> SectionId;

    /// Display name for headers
    fn name(&self) -> &str {
        self.id().display_name()
    }

    /// Context type(s) this section provides
    fn contexts(&self) -> Vec<Context>;

    /// Number of selectable items in this section
    fn item_count(&self) -> usize;

    /// Whether this section is collapsible
    fn is_collapsible(&self) -> bool {
        false
    }

    /// Whether this section is currently collapsed
    fn is_collapsed(&self) -> bool {
        false
    }

    /// When this section should refresh its data
    ///
    /// Built-in git sections default to `OnFileChange`. External sections
    /// may use `Interval` for API polling or `Manual` for expensive operations.
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::OnFileChange
    }

    /// Keybindings available when this section is focused.
    ///
    /// Built-in sections should not implement this; ActionRegistry is the
    /// canonical source for built-in bindings. External sections can return
    /// metadata here for documentation.
    fn keybindings(&self) -> Vec<SectionKeybinding> {
        Vec::new()
    }

    /// Render this section to lines for display
    fn render(&self, state: &SectionState) -> Vec<Line<'static>>;

    /// Get actions available for selected item
    fn actions(&self, item_idx: usize) -> Vec<Action>;

    /// Handle a key event when this section is focused
    /// Returns Some(action) if handled, None to let app handle
    fn handle_key(&self, key: KeyEvent, item_idx: usize) -> Option<SectionAction>;
}
