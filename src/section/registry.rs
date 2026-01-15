//! Section registry for managing UI sections.
//!
//! The registry provides:
//! - Ordered collection of sections
//! - Global index to section/local index mapping
//! - Context resolution based on selection

use super::{Section, SectionId, SectionState};
use crate::actions::Context;

/// Result of looking up a global index
#[derive(Debug, Clone, Copy)]
pub struct IndexLookup {
    /// The section containing this index
    pub section_id: SectionId,
    /// The local index within the section
    pub local_index: usize,
}

/// Registry for managing sections and index calculations.
///
/// The registry maintains the ordering of sections and provides
/// methods to map between global indices and section-local indices.
#[derive(Debug, Clone)]
pub struct SectionRegistry {
    /// Ordered list of section IDs
    sections: Vec<SectionId>,
}

impl Default for SectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SectionRegistry {
    /// Create a new registry with the standard section order
    #[must_use]
    pub fn new() -> Self {
        Self {
            sections: vec![
                SectionId::Command,
                SectionId::Staged,
                SectionId::Working,
                SectionId::History,
                SectionId::Branches,
            ],
        }
    }

    /// Get the ordered list of section IDs
    #[must_use]
    pub fn sections(&self) -> &[SectionId] {
        &self.sections
    }

    /// Look up which section contains a global index and the local index within it.
    ///
    /// Returns `None` if the index is out of bounds.
    #[must_use]
    pub fn lookup_index(&self, global_index: usize, item_counts: &SectionItemCounts) -> Option<IndexLookup> {
        let mut offset = 0;

        for &section_id in &self.sections {
            let count = item_counts.get(section_id);
            if count == 0 {
                continue;
            }

            if global_index < offset + count {
                return Some(IndexLookup {
                    section_id,
                    local_index: global_index - offset,
                });
            }
            offset += count;
        }

        None
    }

    /// Get the context for a global index.
    ///
    /// Returns `Context::Global` if the index is out of bounds.
    #[must_use]
    pub fn context_for_index(&self, global_index: usize, item_counts: &SectionItemCounts) -> Context {
        self.lookup_index(global_index, item_counts)
            .map(|lookup| section_to_context(lookup.section_id, lookup.local_index))
            .unwrap_or(Context::Global)
    }

    /// Calculate the total number of selectable items across all sections.
    #[must_use]
    pub fn total_items(&self, item_counts: &SectionItemCounts) -> usize {
        self.sections
            .iter()
            .map(|&id| item_counts.get(id))
            .sum()
    }

    /// Get the global index for the start of a section.
    ///
    /// Returns `None` if the section has no items.
    #[must_use]
    pub fn section_start_index(&self, section_id: SectionId, item_counts: &SectionItemCounts) -> Option<usize> {
        let count = item_counts.get(section_id);
        if count == 0 {
            return None;
        }

        let mut offset = 0;
        for &id in &self.sections {
            if id == section_id {
                return Some(offset);
            }
            offset += item_counts.get(id);
        }
        None
    }

    /// Build section states for rendering based on global selection.
    ///
    /// Returns a list of (SectionId, SectionState) for sections that have items.
    #[must_use]
    pub fn build_section_states(
        &self,
        global_selection: Option<usize>,
        item_counts: &SectionItemCounts,
    ) -> Vec<(SectionId, SectionState)> {
        let lookup = global_selection.and_then(|idx| self.lookup_index(idx, item_counts));

        let mut states = Vec::new();
        for &section_id in &self.sections {
            let count = item_counts.get(section_id);
            if count == 0 {
                continue;
            }

            let (is_focused, local_selection) = match lookup {
                Some(ref l) if l.section_id == section_id => (true, Some(l.local_index)),
                _ => (false, None),
            };

            states.push((
                section_id,
                SectionState {
                    is_focused,
                    local_selection,
                    global_selection,
                },
            ));
        }

        states
    }
}

/// Item counts for each section.
///
/// This is passed to registry methods so they can calculate indices
/// without needing direct access to the sections.
#[derive(Debug, Clone, Default)]
pub struct SectionItemCounts {
    pub command: usize,
    pub staged: usize,
    pub working: usize,
    pub history: usize,
    pub branches: usize,
}

impl SectionItemCounts {
    /// Get the item count for a section
    #[must_use]
    pub fn get(&self, section_id: SectionId) -> usize {
        match section_id {
            SectionId::Command => self.command,
            SectionId::Staged => self.staged,
            SectionId::Working => self.working,
            SectionId::History => self.history,
            SectionId::Branches => self.branches,
        }
    }

    /// Create counts from section implementations
    pub fn from_sections(
        command: &impl Section,
        staged: &impl Section,
        working: &impl Section,
        history: &impl Section,
        branches: &impl Section,
    ) -> Self {
        Self {
            command: command.item_count(),
            staged: staged.item_count(),
            working: working.item_count(),
            history: history.item_count(),
            branches: branches.item_count(),
        }
    }
}

/// Map section ID and local index to a context.
fn section_to_context(section_id: SectionId, local_index: usize) -> Context {
    match section_id {
        SectionId::Command => Context::Command,
        SectionId::Staged => Context::StagedFiles,
        SectionId::Working => Context::WorkingFiles,
        SectionId::History => {
            // Index 0 is the header, rest are commits
            if local_index == 0 {
                Context::HistoryHeader
            } else {
                Context::HistoryCommits
            }
        }
        SectionId::Branches => Context::BranchCommits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_counts() -> SectionItemCounts {
        SectionItemCounts {
            command: 1,
            staged: 3,
            working: 2,
            history: 5, // 1 header + 4 commits
            branches: 2,
        }
    }

    #[test]
    fn lookup_command_section() {
        let registry = SectionRegistry::new();
        let counts = test_counts();

        let lookup = registry.lookup_index(0, &counts).unwrap();
        assert_eq!(lookup.section_id, SectionId::Command);
        assert_eq!(lookup.local_index, 0);
    }

    #[test]
    fn lookup_staged_section() {
        let registry = SectionRegistry::new();
        let counts = test_counts();

        // Staged starts at index 1 (after command)
        let lookup = registry.lookup_index(1, &counts).unwrap();
        assert_eq!(lookup.section_id, SectionId::Staged);
        assert_eq!(lookup.local_index, 0);

        let lookup = registry.lookup_index(3, &counts).unwrap();
        assert_eq!(lookup.section_id, SectionId::Staged);
        assert_eq!(lookup.local_index, 2);
    }

    #[test]
    fn lookup_working_section() {
        let registry = SectionRegistry::new();
        let counts = test_counts();

        // Working starts at index 4 (after command + staged)
        let lookup = registry.lookup_index(4, &counts).unwrap();
        assert_eq!(lookup.section_id, SectionId::Working);
        assert_eq!(lookup.local_index, 0);
    }

    #[test]
    fn lookup_history_section() {
        let registry = SectionRegistry::new();
        let counts = test_counts();

        // History starts at index 6 (after command + staged + working)
        let lookup = registry.lookup_index(6, &counts).unwrap();
        assert_eq!(lookup.section_id, SectionId::History);
        assert_eq!(lookup.local_index, 0); // Header

        let lookup = registry.lookup_index(7, &counts).unwrap();
        assert_eq!(lookup.section_id, SectionId::History);
        assert_eq!(lookup.local_index, 1); // First commit
    }

    #[test]
    fn lookup_out_of_bounds() {
        let registry = SectionRegistry::new();
        let counts = test_counts();

        // Total items = 1 + 3 + 2 + 5 + 2 = 13
        assert!(registry.lookup_index(13, &counts).is_none());
        assert!(registry.lookup_index(100, &counts).is_none());
    }

    #[test]
    fn context_for_history_header() {
        let registry = SectionRegistry::new();
        let counts = test_counts();

        // History header is at index 6
        let ctx = registry.context_for_index(6, &counts);
        assert_eq!(ctx, Context::HistoryHeader);
    }

    #[test]
    fn context_for_history_commit() {
        let registry = SectionRegistry::new();
        let counts = test_counts();

        // First commit is at index 7
        let ctx = registry.context_for_index(7, &counts);
        assert_eq!(ctx, Context::HistoryCommits);
    }

    #[test]
    fn total_items() {
        let registry = SectionRegistry::new();
        let counts = test_counts();

        assert_eq!(registry.total_items(&counts), 13);
    }

    #[test]
    fn section_start_index() {
        let registry = SectionRegistry::new();
        let counts = test_counts();

        assert_eq!(registry.section_start_index(SectionId::Command, &counts), Some(0));
        assert_eq!(registry.section_start_index(SectionId::Staged, &counts), Some(1));
        assert_eq!(registry.section_start_index(SectionId::Working, &counts), Some(4));
        assert_eq!(registry.section_start_index(SectionId::History, &counts), Some(6));
        assert_eq!(registry.section_start_index(SectionId::Branches, &counts), Some(11));
    }

    #[test]
    fn empty_section_skipped() {
        let registry = SectionRegistry::new();
        let counts = SectionItemCounts {
            command: 1,
            staged: 0, // Empty
            working: 2,
            history: 3,
            branches: 0, // Empty
        };

        // Working should start at index 1 (directly after command, since staged is empty)
        let lookup = registry.lookup_index(1, &counts).unwrap();
        assert_eq!(lookup.section_id, SectionId::Working);
        assert_eq!(lookup.local_index, 0);

        // History should start at index 3
        let lookup = registry.lookup_index(3, &counts).unwrap();
        assert_eq!(lookup.section_id, SectionId::History);
        assert_eq!(lookup.local_index, 0);

        // Total should be 6 (1 + 0 + 2 + 3 + 0)
        assert_eq!(registry.total_items(&counts), 6);
    }
}
