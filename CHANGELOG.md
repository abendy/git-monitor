# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
