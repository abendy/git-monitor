# ADR-006: Repository Snapshot Separate from UI State

## Status
Implemented

**Implementation Date:** 2026-09-27

**Tracking:** LON-123

## Context

`App` held Git data (status, history page, history totals, branches, expanded-branch commits) as
separate fields next to cursor, expansion, and popup state. Several functions wrote those fields
independently, and a failed read of one part was skipped while the others updated.

The cursor was a flat row index across all sections. When a refresh added or removed a file above
it, the same index pointed at a different item. `clamp_selection` only kept it in range. With
agents editing files while the dashboard is open, this moved the cursor silently, so the next
checkout or rebase could target a neighbouring commit.

The roadmap adds provider data (GitHub) and agent activity, each refreshing on its own schedule.
Each new source would multiply these drift and partial-update problems.

## Decision

1. **One snapshot for Git data.** `RepoSnapshot` (`src/git/snapshot.rs`) holds everything the
   view shows that comes from Git. `GitRepo::snapshot(&SnapshotRequest)` reads it in one pass and
   fails whole instead of returning partial data.
2. **One write path.** `App::load_snapshot` builds the request from view state (history mode,
   page, expanded branch) and swaps the result in. On failure the previous snapshot stays and the
   error is shown. The field is private; everything else reads `app.snapshot()`.
3. **UI state stays on `App`.** Cursor, scroll, expansion, collapsed sections, the requested
   page, and popups are not part of the snapshot, and the snapshot knows nothing about them.
4. **Cursor by identity.** `SelectionKey` names the selected row by path, SHA, or branch name.
   `refresh_status` records it before the swap and restores it after, falling back to clamping
   when the item is gone.
5. **Commit detail is a UI cache.** Detail for the expanded commit is keyed by SHA and never
   changes, so it stays outside the snapshot and is not re-read on refresh.

## Rationale

- A refresh can no longer mix data from two moments or leave one part stale.
- Rows moving above the cursor no longer change which item an action targets.
- Future sources (forge, agents) can follow the same pattern: a request derived from view state,
  a snapshot read whole, one swap point.

## Trade-offs

- **More work per load.** Expanding a branch or changing the history page now re-reads status
  too. Acceptable for the repositories in use; revisit with measurements if large repositories
  make expansion feel slow.
- **All-or-nothing failures.** A failing branch listing now blocks the status update it used to
  let through. The error is visible instead of silent.
- **Fallback is still positional.** When the selected item disappears (for example, a staged
  file), the cursor falls back to the clamped old row, matching previous behavior.

## Alternatives Considered

1. **Adjust the index by counting rows added above.** Rejected: fragile across collapse,
   expansion, and pagination, and it does not survive reordering.
2. **Keep separate fields, fix only the cursor.** Rejected: leaves partial updates and stale
   branch commits, and gives provider data no pattern to follow.
3. **Snapshot includes commit detail.** Rejected: detail is immutable per SHA, so re-reading it
   on every refresh is wasted work.

## Consequences

- `app.status`, `app.activity`, `app.branches`, and history totals moved to `app.snapshot()`.
- `HistoryMode` moved to `crate::git` and is re-exported from `crate::app`.
- Expanded-branch commits refresh with everything else, and a deleted branch collapses.
