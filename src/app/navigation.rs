use std::time::{Duration, Instant};

use super::{App, PageLanding, SectionHold};
use crate::section::SectionId;

impl App {
    /// Clamp selection to valid bounds
    ///
    /// Called after item counts change (file changes, commits, etc.) to ensure
    /// the selection index remains valid.
    pub(super) fn clamp_selection(&mut self) {
        if let Some(idx) = self.selected {
            let total = self.total_count();
            if total == 0 {
                self.selected = None;
            } else if idx >= total {
                self.selected = Some(total - 1);
            }
        }
    }

    /// Jump to working area (first file in staged or working changes)
    pub(super) fn jump_to_working(&mut self) {
        let files_total = self
            .snapshot
            .status
            .staged_changes()
            .len()
            + self
                .snapshot
                .status
                .working_changes()
                .len();
        if files_total == 0 {
            return;
        }

        // Select first file (index 1, after any spacer)
        self.selected = Some(1);
        self.close_expanded_commit();
    }

    /// Jump to history and expand it, landing on most recent commit
    pub(super) fn jump_to_history(&mut self) {
        if self.snapshot.history.is_empty() {
            return;
        }

        // Expand history (collapses any expanded branch)
        self.expand_history();

        let staged_len = self
            .snapshot
            .status
            .staged_changes()
            .len();
        let working_len = self
            .snapshot
            .status
            .working_changes()
            .len();
        let files_total = staged_len + working_len;

        // Select first commit (skip header)
        let history_header_idx = 1 + files_total;
        self.selected = Some(history_header_idx + 1);
        self.close_expanded_commit();
    }

    /// Jump to first branch and expand it, landing on first commit
    pub(super) fn jump_to_branches(&mut self) {
        let other_branches = self.other_branches();
        if other_branches.is_empty() {
            return;
        }

        // Get first branch name and expand it
        let first_branch_name = other_branches[0].name.clone();
        self.expand_branch(&first_branch_name);

        // Select first commit in the branch
        let branches_start = self.branches_start_index();
        self.selected = Some(branches_start);
        self.close_expanded_commit();
    }

    /// Select next item with accordion auto-expand/collapse
    pub(super) fn select_next(&mut self) {
        let len = self.total_count();
        if len == 0 {
            return;
        }

        match self.selected {
            None => {
                self.selected = Some(0);
            }
            Some(idx) => {
                self.close_expanded_commit();

                // Are we on the last history commit about to move to branches?
                let on_last_history_commit = !self.history_collapsed
                    && !self.snapshot.history.is_empty()
                    && idx == self.history_start_index() + self.snapshot.history.len();

                if on_last_history_commit {
                    // Moving from last history commit to first branch's first commit
                    let other_branches = self.other_branches();
                    if let Some(first_branch) = other_branches.first() {
                        let branch_name = first_branch.name.clone();
                        self.expand_branch(&branch_name);
                        self.selected = Some(self.branches_start_index());
                        return;
                    }
                }

                // Don't go past the last item if no branches to expand
                if idx >= len - 1 {
                    return;
                }

                // Normal navigation
                let new_idx = idx + 1;
                self.selected = Some(new_idx);
            }
        }
    }

    /// Select previous item with accordion auto-expand/collapse
    pub(super) fn select_prev(&mut self) {
        match self.selected {
            Some(idx) if idx > 0 => {
                self.close_expanded_commit();

                if self.history_collapsed
                    && !self.snapshot.history.is_empty()
                    && idx == self.branches_start_index()
                {
                    self.expand_history();
                    let history_header_idx = self.history_start_index();
                    self.selected = Some(history_header_idx + self.snapshot.history.len());
                    return;
                }

                // Normal navigation
                self.selected = Some(idx - 1);
            }
            _ => {}
        }
    }

    /// Select first item
    pub(super) fn select_first(&mut self) {
        self.selected = Some(0);
        self.close_expanded_commit();
    }

    /// Select last item
    pub(super) fn select_last(&mut self) {
        let len = self.total_count();
        if len > 0 {
            self.selected = Some(len - 1);
            self.close_expanded_commit();
        }
    }

    pub(super) fn select_default_section(&mut self) {
        let counts = self.section_item_counts();

        if counts.working > 0 {
            if let Some(start) = self
                .section_registry
                .section_start_index(SectionId::Working, &counts)
            {
                self.selected = Some(start);
                return;
            }
        }

        if counts.staged > 0 {
            if let Some(start) = self
                .section_registry
                .section_start_index(SectionId::Staged, &counts)
            {
                self.selected = Some(start);
                return;
            }
        }

        self.selected = Some(0);
    }

    /// Move within the current section for a held `J`/`K`, speeding up while the key repeats.
    pub(super) fn held_section_move(&mut self, down: bool) {
        let now = Instant::now();
        let streak = hold_streak(self.section_hold, down, now);
        self.section_hold = Some(SectionHold {
            down,
            at: now,
            streak,
        });

        for _ in 0..hold_step(streak) {
            if !self.move_within_section(down) {
                break;
            }
        }
    }

    /// Move one row within the current section. Returns whether the cursor moved.
    ///
    /// Never leaves the section. On History commits it turns the page at the edges.
    pub(super) fn move_within_section(&mut self, down: bool) -> bool {
        let Some(index) = self.selected else {
            self.selected = Some(0);
            return true;
        };

        // The command row is a launch point, not a list: step off it like `j`
        if index == 0 && down {
            self.select_next();
            return self.selected != Some(0);
        }

        if self.is_in_history() {
            let first = self.history_start_index() + 1;
            let last = first
                + self
                    .snapshot
                    .history
                    .len()
                    .saturating_sub(1);
            if down && index == last {
                return self.turn_history_page(true, PageLanding::First);
            }
            if !down && index == first && self.history_page > 0 {
                return self.turn_history_page(false, PageLanding::Last);
            }
        }

        let Some(target) = (if down {
            index.checked_add(1)
        } else {
            index.checked_sub(1)
        }) else {
            return false;
        };
        let counts = self.section_item_counts();
        let section_of = |i| {
            self.section_registry
                .lookup_index(i, &counts)
                .map(|lookup| lookup.section_id)
        };
        if section_of(index).is_none() || section_of(index) != section_of(target) {
            return false;
        }
        self.close_expanded_commit();
        self.selected = Some(target);
        true
    }
}

/// Key repeats closer together than this count as one held key
const HOLD_GAP: Duration = Duration::from_millis(150);

/// How many repeats in a row a held `J`/`K` has sent
pub(super) fn hold_streak(previous: Option<SectionHold>, down: bool, now: Instant) -> u32 {
    match previous {
        Some(hold) if hold.down == down && now.saturating_duration_since(hold.at) <= HOLD_GAP => {
            hold.streak.saturating_add(1)
        }
        _ => 0,
    }
}

/// Rows to move for a repeat: 1 at first, then doubling every 10 repeats up to 8
pub(super) const fn hold_step(streak: u32) -> usize {
    match streak {
        0..=9 => 1,
        10..=19 => 2,
        20..=29 => 4,
        _ => 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn hold(down: bool, at: Instant, streak: u32) -> SectionHold {
        SectionHold { down, at, streak }
    }

    #[test]
    fn quick_repeat_in_the_same_direction_extends_the_streak() {
        let start = Instant::now();
        let next = start + Duration::from_millis(30);

        assert_eq!(
            hold_streak(Some(hold(true, start, 4)), true, next),
            5
        );
    }

    #[test]
    fn pause_or_direction_change_restarts_the_streak() {
        let start = Instant::now();

        assert_eq!(
            hold_streak(
                Some(hold(true, start, 4)),
                true,
                start + Duration::from_millis(400)
            ),
            0
        );
        assert_eq!(
            hold_streak(
                Some(hold(true, start, 4)),
                false,
                start + Duration::from_millis(30)
            ),
            0
        );
        assert_eq!(hold_streak(None, true, start), 0);
    }

    #[test]
    fn step_speeds_up_while_held() {
        assert_eq!(hold_step(0), 1);
        assert_eq!(hold_step(9), 1);
        assert_eq!(hold_step(10), 2);
        assert_eq!(hold_step(25), 4);
        assert_eq!(hold_step(500), 8);
    }
}
