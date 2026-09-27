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

    /// Move within the current section for a held `J`/`K`, speeding up the longer it is held.
    pub(super) fn held_section_move(&mut self, down: bool) {
        self.held_section_move_at(down, Instant::now());
    }

    /// `held_section_move` with an explicit time, so tests can simulate a held key
    pub(super) fn held_section_move_at(&mut self, down: bool, now: Instant) {
        let started = hold_start(self.section_hold, down, now);
        self.section_hold = Some(SectionHold {
            down,
            started,
            at: now,
        });

        for _ in 0..hold_step(now.saturating_duration_since(started)) {
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

/// Repeats closer together than this count as one held key. Generous enough for
/// key repeats that arrive unevenly over SSH and tmux.
const HOLD_GAP: Duration = Duration::from_millis(300);

/// When the current hold began: carried over from the last repeat if it is the same
/// direction and recent enough, otherwise now
pub(super) fn hold_start(previous: Option<SectionHold>, down: bool, now: Instant) -> Instant {
    match previous {
        Some(hold) if hold.down == down && now.saturating_duration_since(hold.at) <= HOLD_GAP => {
            hold.started
        }
        _ => now,
    }
}

/// Rows per repeat for how long the key has been held: 1, then 2, 4, and 8
pub(super) const fn hold_step(held: Duration) -> usize {
    match held.as_millis() {
        0..400 => 1,
        400..800 => 2,
        800..1200 => 4,
        _ => 8,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const fn hold(down: bool, started: Instant, at: Instant) -> SectionHold {
        SectionHold { down, started, at }
    }

    #[test]
    fn quick_repeat_in_the_same_direction_keeps_the_hold() {
        let start = Instant::now();
        let last = start + Duration::from_millis(500);
        let next = last + Duration::from_millis(250);

        assert_eq!(
            hold_start(
                Some(hold(true, start, last)),
                true,
                next
            ),
            start
        );
    }

    #[test]
    fn pause_or_direction_change_starts_a_new_hold() {
        let start = Instant::now();
        let last = start + Duration::from_millis(500);

        let after_pause = last + Duration::from_millis(400);
        assert_eq!(
            hold_start(
                Some(hold(true, start, last)),
                true,
                after_pause
            ),
            after_pause
        );

        let reversed = last + Duration::from_millis(50);
        assert_eq!(
            hold_start(
                Some(hold(true, start, last)),
                false,
                reversed
            ),
            reversed
        );
        assert_eq!(hold_start(None, true, start), start);
    }

    #[test]
    fn step_grows_with_hold_time() {
        assert_eq!(hold_step(Duration::ZERO), 1);
        assert_eq!(hold_step(Duration::from_millis(399)), 1);
        assert_eq!(hold_step(Duration::from_millis(400)), 2);
        assert_eq!(hold_step(Duration::from_millis(900)), 4);
        assert_eq!(hold_step(Duration::from_secs(5)), 8);
    }

    /// Repo with `count` empty commits
    fn repo_with_commits(count: usize) -> TempDir {
        let dir = TempDir::new().expect("create temp dir");
        let repo = git2::Repository::init(dir.path()).expect("init repo");
        let sig = git2::Signature::now("Test", "test@example.com").expect("signature");
        let tree_id = repo
            .index()
            .expect("index")
            .write_tree()
            .expect("write tree");
        let tree = repo
            .find_tree(tree_id)
            .expect("find tree");
        let mut parent: Option<git2::Oid> = None;
        for i in 0..count {
            let parent_commit = parent.map(|oid| {
                repo.find_commit(oid)
                    .expect("find parent")
            });
            let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();
            parent = Some(
                repo.commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("Commit {i}"),
                    &tree,
                    &parents,
                )
                .expect("commit"),
            );
        }
        dir
    }

    #[test]
    fn held_key_speeds_up_even_with_uneven_repeats() {
        let dir = repo_with_commits(120);
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.jump_to_history();

        // Two seconds of repeats arriving every 60-250 ms, like key repeat over SSH
        let start = Instant::now();
        let mut at = start;
        for gap in [60_u64, 250, 90, 200, 60, 250]
            .iter()
            .cycle()
            .take(14)
        {
            at += Duration::from_millis(*gap);
            app.held_section_move_at(true, at);
        }

        assert!(
            app.history_page >= 1,
            "held key crossed into page 2 (page {})",
            app.history_page
        );
    }

    #[test]
    fn separate_taps_move_one_row_each() {
        let dir = repo_with_commits(120);
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.jump_to_history();
        let first = app.selected.expect("on a commit");

        let start = Instant::now();
        for tap in 0..5 {
            app.held_section_move_at(
                true,
                start + Duration::from_millis(500 * tap),
            );
        }

        assert_eq!(app.selected, Some(first + 5));
    }
}
