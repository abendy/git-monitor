# ADR-008: Repository and Worktree Identity by Location

## Status
Implemented

**Implementation Date:** 2026-09-27

**Tracking:** LON-133

## Context

Provider data (GitHub) and agent events need to attach to "this repository" and "this worktree",
and must keep attaching after a restart. Linked worktrees share one repository but are separate
places where separate agents work.

An identity can be based on where the repository lives, on its content (the root commit), or on
its remotes. Each answers a different question.

## Decision

1. **Repository identity (`RepoId`)** is the canonical path of the common Git directory. Linked
   worktrees share that directory, so they share the identity.
2. **Worktree identity (`WorktreeId`)** is the repository identity plus the canonical path of
   the worktree's root. A bare repository uses its Git directory as the root.
3. **Canonical** means symlinks resolved, so the same place always gives the same text.
4. `GitRepo::repo_id` and `GitRepo::worktree_id` are the only way to get an identity. `App`
   resolves it once at startup and exposes `App::worktree_id`. The types are opaque: callers
   compare and display them, and never parse them.

Defined behavior:

- Restarting, refreshing, or opening from a subdirectory or symlinked path gives the same
  identity.
- Moving or renaming the repository gives a new identity. It is a different location, and any
  agent session tied to the old path no longer points at it.
- Adding, changing, or removing remotes does not change identity. Which forge repository a
  checkout belongs to is a separate mapping (remote discovery, LON-124).

## Rationale

- Agents and humans work in directories, so location is what they share with the dashboard.
- Linked worktrees need distinct identities for correlation, and their paths already are.
- Keeping remotes out avoids identity churn when a remote is renamed or a fork is added.

## Trade-offs

- **Moves break continuity.** Anything keyed by the old identity is orphaned after a move.
  Acceptable: moves are rare, and a stale association is worse than none (roadmap principle 6).
- **Paths are not private.** Identities contain local paths. They must stay on this machine;
  anything sent elsewhere should hash them first.

## Alternatives Considered

1. **Root commit SHA.** Survives moves, but every clone and fork of a project shares it, so two
   checkouts on one machine could not be told apart.
2. **Normalized remote URL.** Changes when remotes change, and local-only repositories have none.
   Useful for forge mapping, not for identity.
3. **Random ID stored in the Git directory.** Survives moves, but writes into the user's
   repository and is copied by tools that copy `.git`.
