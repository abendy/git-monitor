# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-01-17

### Added

- **Modular menu system** with trait-based abstraction and stack navigation (see ADR-005)
  - `Menu` trait for composable menu implementations
  - `MenuStack` for nested menu management (push/pop)
  - Unified rendering via `render_menu()` helper
- **Unified command execution framework** (see ADR-003)
  - `CommandRequest` type with source tracking and feedback policy
  - `CommandExecutor` for consistent command execution
  - `CommandHistory` for persistent history across sessions
- **Feedback system** for command output display (see ADR-004)
  - `FeedbackManager` centralizes all user feedback
  - Source-aware popup policy (menu selections → always popup)
  - Toast types defined for future transient notifications
- **Section abstraction** for UI regions
  - `Section` trait for render, actions, key handling
  - `SectionRegistry` for index management across sections

### Changed

- **ViewMode simplified** from 6 variants to 2 (Normal, Command)
- **Modal dialogs migrated to MenuStack**: action menu, alias browser, push confirmation
- All command execution now flows through unified `run_command()` path
- Documentation updated with new module structure and types

### Fixed

- Capture stderr output for successful git commands
- Make branch info hints dynamic based on repository state

## [0.4.0] - 2026-01-17

### Added

- Contextual action menu framework with `m` key to show available actions (see ADR-002)
- Conditional actions: `P` to push (when ahead), `p` to pull (when behind), `f` to fetch
- History pagination with `[`/`]` keys for navigating through commit history
- `w` key to jump to working files section
- Remote branch tracking and divergence visualization
- Dynamic section hints that only show relevant actions for current cursor position
- Push confirmation prompt with branch/remote info in footer

### Changed

- Footer hints are now context-aware and dynamically generated from ActionRegistry
- ViewMode extended with `ActionMenu` and `Confirm` variants
- Action registry is single source of truth for keybindings (used by hints and action menu)

## [0.3.0] - 2026-01-17

### Added

- Commit detail expansion in history view with file list and diff stats
- File navigation within expanded commits (`j`/`k` to move, `Space` for pager, `M` for difftool)
- External diff tools support respecting `core.pager` and `diff.tool` from gitconfig (see ADR-001)
- Branch browser section showing local branches with ahead/behind indicators
- Branch expansion to view recent commits on other branches
- Branch checkout with `c` key
- Accordion-style History/Branches sections with continuous scrolling
- Focus indicators on section headers
- Contextual footer hints based on current selection
- Persistent command history across sessions
- Highlight special commit prefixes (`fixup!`, `squash!`, `amend!`, etc.)
- Relative timestamps in history ("2 hr ago", "3 days ago")
- Branch graph visualization with tree connectors
- `g`/`G` shortcuts for top/bottom navigation
- `b` key to jump to branches section

### Changed

- View state consolidated into `ViewMode` enum (Normal, Command, AliasSections, AliasItems)
- Selection state changed to `Option<usize>` to support "nothing focused" state
- History section now collapsible (toggle with `h` key)
- Branch names displayed as non-selectable header labels
- Improved navigation: last history commit can continue to branches section

### Fixed

- Navigation from last history commit now continues to branches section
- `b` key now selects first commit in branches, not the header
- Expanded commit auto-closes when navigating away
- Clear selection highlight in command mode

## [0.2.0] - 2026-01-17

### Added

- Command mode with input history (`:`/`;` to enter, up/down to navigate history)
- Alias browser with section-grouped categories (`:` then browse aliases)
- History/reflog toggle with `h` key
- Branch and tag decorations in history view
- Commit SHA display in history with copy shortcut (`y` to copy)
- Reusable popup system for full-screen overlays
- Cross-platform clipboard support via arboard crate

### Changed

- Unified changes panel into single vertical scrollable list (Working + Staged + History)
- Command section integrated into main scrollable list
- Improved popup system architecture for better rendering

### Fixed

- Correct handling of invalid config values

## [0.1.0] - 2026-01-17

### Added

- Initial release of git-monitor TUI
- Real-time file watching with automatic status refresh
- Three-panel layout: Working Directory, Staged, and Activity Log
- Git status display with color-coded file states (Modified, Added, Deleted, Renamed, Untracked, Conflicted)
- Stage and unstage files with `s` key
- View file diffs with `d` key or Enter
- Activity log showing recent git commands from reflog
- Keyboard navigation with vim-style bindings (j/k, g/G)
- Panel switching with Tab/Shift+Tab
- Help overlay with `?` key
- Branch information with ahead/behind tracking
- CLI arguments for repository path (`--path`) and tick rate (`--tick-rate`)
- Cross-platform file watching via notify crate
- Debounced file events (100ms) for efficient updates
