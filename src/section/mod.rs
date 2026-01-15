//! Section abstraction for UI regions.
//!
//! Each section (Command, Staged, Working, History, Branches) implements
//! the Section trait, providing its own rendering, actions, and key handling.

#![allow(dead_code)] // Module in progress - not yet fully integrated

mod command;
mod staged;
mod working;

pub use command::{CommandSection, CommandSectionData};
pub use staged::{StagedSection, StagedSectionData};
pub use working::{WorkingSection, WorkingSectionData};

use crossterm::event::KeyEvent;
use ratatui::text::Line;

use crate::actions::{Action, Context};

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
            Self::Command(req) => f.debug_tuple("Command").field(req).finish(),
            Self::Feedback(_) => f.debug_tuple("Feedback").field(&"...").finish(),
            Self::OpenMenu(_) => f.debug_tuple("OpenMenu").field(&"<menu>").finish(),
            Self::AppAction(action) => f.debug_tuple("AppAction").field(action).finish(),
            Self::Navigate(nav) => f.debug_tuple("Navigate").field(nav).finish(),
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

/// State passed to sections for rendering and actions
#[derive(Debug, Clone)]
pub struct SectionState {
    /// Whether this section contains the current selection
    pub is_focused: bool,
    /// Index of selection within this section (if focused)
    pub local_selection: Option<usize>,
    /// Global selection index
    pub global_selection: Option<usize>,
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

    /// Render this section to lines for display
    fn render(&self, state: &SectionState) -> Vec<Line<'static>>;

    /// Get actions available for selected item
    fn actions(&self, item_idx: usize) -> Vec<Action>;

    /// Handle a key event when this section is focused
    /// Returns Some(action) if handled, None to let app handle
    fn handle_key(&self, key: KeyEvent, item_idx: usize) -> Option<SectionAction>;
}

/// Registry of all sections
pub struct SectionRegistry {
    sections: Vec<Box<dyn Section>>,
}

impl SectionRegistry {
    /// Create a new registry with the given sections
    #[must_use]
    pub fn new(sections: Vec<Box<dyn Section>>) -> Self {
        Self { sections }
    }

    /// Get an iterator over all sections
    pub fn iter(&self) -> impl Iterator<Item = &dyn Section> {
        self.sections.iter().map(|s| s.as_ref())
    }

    /// Get a section by ID
    #[must_use]
    pub fn get(&self, id: SectionId) -> Option<&dyn Section> {
        self.sections.iter().find(|s| s.id() == id).map(|s| s.as_ref())
    }

    /// Get mutable section by ID
    pub fn get_mut(&mut self, id: SectionId) -> Option<&mut Box<dyn Section>> {
        self.sections.iter_mut().find(|s| s.id() == id)
    }

    /// Calculate total item count across all sections
    #[must_use]
    pub fn total_items(&self) -> usize {
        self.sections.iter().map(|s| s.item_count()).sum()
    }

    /// Find which section contains a global index
    #[must_use]
    pub fn section_for_index(&self, global_idx: usize) -> Option<(SectionId, usize)> {
        let mut offset = 0;
        for section in &self.sections {
            let count = section.item_count();
            if global_idx < offset + count {
                return Some((section.id(), global_idx - offset));
            }
            offset += count;
        }
        None
    }

    /// Get the starting global index for a section
    #[must_use]
    pub fn section_start_index(&self, id: SectionId) -> Option<usize> {
        let mut offset = 0;
        for section in &self.sections {
            if section.id() == id {
                return Some(offset);
            }
            offset += section.item_count();
        }
        None
    }
}

impl Default for SectionRegistry {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl std::fmt::Debug for SectionRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SectionRegistry")
            .field("section_count", &self.sections.len())
            .field("total_items", &self.total_items())
            .finish()
    }
}
