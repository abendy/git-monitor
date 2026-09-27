//! Selection identity.
//!
//! The cursor is stored as a global row index, which drifts when a refresh
//! adds or removes rows above it. `SelectionKey` names the selected item so a
//! refresh can find the same item again at its new row.

use std::path::PathBuf;

use super::App;
use crate::section::SectionId;

/// Stable identity of a selectable row
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SelectionKey {
    /// Command input line
    Command,
    /// File in the Staged section
    Staged(PathBuf),
    /// File in the Working section
    Working(PathBuf),
    /// History section header
    HistoryHeader,
    /// Commit in the History section
    HistoryCommit(String),
    /// Branch header in the Branches section
    BranchHeader(String),
    /// Commit under the expanded branch
    BranchCommit(String),
}

impl App {
    /// Identity of the currently selected row, if it has one
    pub(super) fn selection_key(&self) -> Option<SelectionKey> {
        let selected = self.selected?;
        let counts = self.section_item_counts();
        let lookup = self
            .section_registry
            .lookup_index(selected, &counts)?;
        let local = lookup.local_index;

        match lookup.section_id {
            SectionId::Command => Some(SelectionKey::Command),
            SectionId::Staged => self
                .snapshot
                .status
                .staged_changes()
                .get(local)
                .map(|f| SelectionKey::Staged(f.path.clone())),
            SectionId::Working => self
                .snapshot
                .status
                .working_changes()
                .get(local)
                .map(|f| SelectionKey::Working(f.path.clone())),
            SectionId::History if local == 0 => Some(SelectionKey::HistoryHeader),
            SectionId::History => self
                .snapshot
                .history
                .get(local - 1)
                .and_then(|cmd| cmd.sha.clone())
                .map(SelectionKey::HistoryCommit),
            SectionId::Branches => self.selection_key_for_branch_local(local),
        }
    }

    /// Current global row of the item named by `key`, if it is still listed
    pub(super) fn index_for_key(&self, key: &SelectionKey) -> Option<usize> {
        let counts = self.section_item_counts();
        let start = |id| {
            self.section_registry
                .section_start_index(id, &counts)
        };

        match key {
            SelectionKey::Command => start(SectionId::Command),
            SelectionKey::Staged(path) => {
                let local = self
                    .snapshot
                    .status
                    .staged_changes()
                    .iter()
                    .position(|f| &f.path == path)?;
                Some(start(SectionId::Staged)? + local)
            }
            SelectionKey::Working(path) => {
                let local = self
                    .snapshot
                    .status
                    .working_changes()
                    .iter()
                    .position(|f| &f.path == path)?;
                Some(start(SectionId::Working)? + local)
            }
            SelectionKey::HistoryHeader => start(SectionId::History),
            SelectionKey::HistoryCommit(sha) => {
                if self.history_collapsed {
                    return None;
                }
                let local = self
                    .snapshot
                    .history
                    .iter()
                    .position(|cmd| cmd.sha.as_ref() == Some(sha))?;
                Some(start(SectionId::History)? + 1 + local)
            }
            SelectionKey::BranchHeader(_) | SelectionKey::BranchCommit(_) => {
                let branches_start = start(SectionId::Branches)?;
                (0..counts.branches)
                    .find(|&local| {
                        self.selection_key_for_branch_local(local)
                            .as_ref()
                            == Some(key)
                    })
                    .map(|local| branches_start + local)
            }
        }
    }

    /// Identity of a row in the Branches section by its local index
    fn selection_key_for_branch_local(&self, local: usize) -> Option<SelectionKey> {
        if let Some(branch) = self.branch_header_for_local_index(local) {
            return Some(SelectionKey::BranchHeader(
                branch.name.clone(),
            ));
        }
        self.branch_commit_index_for_local_index(local)
            .and_then(|idx| self.expanded_branch_commits().get(idx))
            .and_then(|cmd| cmd.sha.clone())
            .map(SelectionKey::BranchCommit)
    }

    /// Move the cursor back onto `key` after the rows changed.
    ///
    /// Falls back to clamping the old row when the item is gone.
    pub(super) fn restore_selection(&mut self, key: Option<&SelectionKey>) {
        if let Some(idx) = key.and_then(|key| self.index_for_key(key)) {
            self.selected = Some(idx);
        } else {
            self.clamp_selection();
        }
    }
}
