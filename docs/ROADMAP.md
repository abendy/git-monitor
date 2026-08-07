# Product Roadmap

## North Star

`git-monitor` should be the quiet, need-to-know surface for repository work: fast to scan, mostly
passive, and ready for a human decision when automation reaches a meaningful gate. A developer
should be able to see local changes, agent activity, pull-request health, review state, and delivery
blockers without assembling that picture across terminals and browser tabs.

Interactions remain part of the product, but the dashboard does not become a command catalog.
Every action should arise from visible state and make its consequences clear—for example, merging
a reviewed and green pull request after confirmation.

## Current Baseline

Version 0.8 provides:

- local Git status, history, branches, diffs, staging, and selected Git actions;
- contextual menus, command execution, feedback overlays, and gitconfig aliases;
- debounced refresh for normal repositories and linked worktrees;
- strict format/lint/test gates and a reproducible release build.

It does not yet provide forge data, agent-session data, a provider abstraction, or a `jj` backend.
The section registry is implemented for built-in sections; it is not a dynamic plugin platform.

## Product Principles

1. **Passive first.** Refresh and summarize automatically; interrupt only for a real decision.
2. **Progressive disclosure.** Lead with blockers and transitions, then reveal detail on demand.
3. **Local truth stays useful offline.** Provider outages must not break repository monitoring.
4. **Explicit mutations.** External or destructive actions show scope, preconditions, and outcome.
5. **Provider-neutral core.** GitHub is the first adapter, not the shape of application state.
6. **Correlated work.** Worktree, branch, commit, agent session, and pull request should converge on
   one workflow identity where evidence permits it.
7. **Honest freshness.** The UI distinguishes current, refreshing, stale, partial, and failed data.

## Delivery Sequence

### 1. Repository State Spine

- Separate repository snapshots from rendering and cursor state.
- Give refreshes explicit reasons, timestamps, and failure state.
- Coalesce filesystem, timer, command, and provider updates into predictable state transitions.
- Establish stable repository/worktree identity for correlating later provider and agent events.
- Add the second VCS use case with a narrow `jj` read adapter before generalizing a backend trait.

**Exit:** Git and `jj` can populate a small shared repository summary without provider code or UI
code knowing how either command-line tool stores its state.

### 2. GitHub Read Adapter

- Discover the GitHub repository from remotes and make ambiguous remotes visible.
- Reuse authenticated `gh` state first; do not create a token store in the initial adapter.
- Fetch pull requests, reviews, checks, mergeability, and relevant issue links.
- Cache the last successful snapshot and expose rate-limit, auth, offline, and stale states.
- Render one compact workflow section ordered by attention, not separate API-shaped panels.

**Exit:** the dashboard can answer: What is open? What changed? What is blocked? Who or what is
waiting?

### 3. Agent Activity Correlation

- Define a small event/snapshot contract for agent runs, commits, pushes, and pull-request creation.
- Start with one concrete local source and keep adapters isolated from core state.
- Correlate conservatively using repository, worktree, branch, commit, and PR identifiers.
- Show running, waiting-for-human, failed, superseded, and completed work without streaming logs into
  the main view.

**Exit:** agent work changes the dashboard state and surfaces a clear handoff when human input is
required.

### 4. Human Gates

- Open local or provider detail from the selected workflow item.
- Confirm and execute merge only when repository policy and provider state allow it.
- Add narrowly justified follow-ups such as rerunning checks or opening a review request.
- Record action progress and result in the same workflow item; never hide a failed mutation.

**Exit:** a developer can safely complete the common green-PR handoff without turning the TUI into
a full forge client.

### 5. Additional Providers

- Validate the provider boundary with a second forge such as GitLab or Forgejo.
- Keep provider-specific capabilities behind explicit capability flags.
- Revisit extension protocols only after two adapters expose real common and divergent needs.

## Near-Term UX Work

- Attention-ranked workflow summary and stale/error indicators.
- Better in-TUI diff navigation and search.
- Conflict and rebase state with safe continue/abort handoffs.
- Branch and pull-request relationships in history without a permanently dense graph.
- Accessible color plus text/icon signals for every important state.

## Deferred Until Proven

- A general workflow engine.
- External binary section protocols.
- User-authored dynamic layouts.
- A full issue tracker, log viewer, or replacement for provider web UIs.

When present, `.project/tasks/index.md` is the active local review queue for migration to Linear.
Its linked task files preserve useful detail, but their status labels and implementation claims may
be stale. Validate them against the code, and use this roadmap to decide sequencing; do not close a
task merely because part of its design has landed.
