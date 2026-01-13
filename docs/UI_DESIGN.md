# Git Monitor - UI Design

## Layout

### Main View

```
┌─────────────────────────────────────────────────────────────────────┐
│ ⎇ main ↑2 ↓0 │ ~/projects/git-monitor │ ● watching │ 14:32:05     │ <- Header
├────────────────────────────┬────────────────────────────────────────┤
│ Working Directory (4)      │ Staged (2)                             │ <- Panel Headers
│ ───────────────────────    │ ────────────────────                   │
│ > M src/main.rs            │   A src/new_file.rs                    │ <- File Lists
│   M src/lib.rs             │   M src/config.rs                      │
│   ? untracked.txt          │                                        │
│   D old_file.rs            │                                        │
│                            │                                        │
│                            │                                        │
├────────────────────────────┴────────────────────────────────────────┤
│ Recent Activity                                                      │ <- Activity Header
│ ────────────────────────────────────────────────────────────────────│
│ 14:32:01  ● commit: Add feature X                                   │ <- Activity Log
│ 14:31:45  + staged: src/new_file.rs                                 │
│ 14:30:22  ↓ pull: origin/main (fast-forward)                        │
│ 14:28:10  ⎇ checkout: feature/new-thing                             │
├─────────────────────────────────────────────────────────────────────┤
│ [Tab] switch │ [j/k] navigate │ [s] stage │ [d] diff │ [?] help    │ <- Footer
└─────────────────────────────────────────────────────────────────────┘
```

### Diff View (Overlay)

```
┌─────────────────────────────────────────────────────────────────────┐
│ Diff: src/main.rs                                           [Esc]   │
├─────────────────────────────────────────────────────────────────────┤
│ @@ -10,6 +10,8 @@ fn main() {                                       │
│      let config = Config::load();                                   │
│      let app = App::new(config);                                    │
│ +    app.setup_watcher();                                           │
│ +    app.run();                                                     │
│      println!("Starting...");                                       │
│  }                                                                  │
│                                                                     │
│                                                                     │
│ [j/k] scroll │ [q/Esc] close                                        │
└─────────────────────────────────────────────────────────────────────┘
```

### Help Overlay

```
┌───────────────────────────────────────────────────────────┐
│                     Git Monitor Help                       │
├───────────────────────────────────────────────────────────┤
│                                                           │
│  Navigation                                               │
│  ──────────                                               │
│  Tab / Shift+Tab    Switch panels                         │
│  j / ↓              Move down                             │
│  k / ↑              Move up                               │
│  g                  Go to top                             │
│  G                  Go to bottom                          │
│                                                           │
│  Actions                                                  │
│  ───────                                                  │
│  s                  Stage/unstage selected file           │
│  d                  Show diff for selected file           │
│  r                  Refresh status                        │
│  c                  Commit (if files staged)              │
│                                                           │
│  General                                                  │
│  ───────                                                  │
│  ?                  Toggle this help                      │
│  q / Ctrl+c         Quit                                  │
│                                                           │
│                    Press any key to close                 │
└───────────────────────────────────────────────────────────┘
```

## Components

### Header Bar

```
⎇ main ↑2 ↓0 │ ~/projects/git-monitor │ ● watching │ 14:32:05
└──┬───┘└─┬─┘   └─────────┬──────────┘   └────┬────┘  └───┬───┘
   │      │               │                   │          │
 branch  ahead/behind   repo path          status      time
```

**Elements**:
- **Branch**: Current branch with icon (⎇ for branch, `detached` for detached HEAD)
- **Ahead/Behind**: Commits ahead (↑) and behind (↓) upstream
- **Repo Path**: Shortened path to repository
- **Status**: Current watching status (● watching, ○ paused, ⟳ refreshing)
- **Time**: Current time, updates every second

### File List Panel

```
Working Directory (4)
─────────────────────
> M src/main.rs
  M src/lib.rs
  ? untracked.txt
  D old_file.rs
```

**Elements**:
- **Title**: Panel name with file count
- **Selection Indicator**: `>` for selected file
- **Status Character**: M/A/D/?/! etc.
- **File Path**: Relative path from repo root

**Colors**:
| Status | Character | Color |
|--------|-----------|-------|
| Modified | M | Yellow |
| Added | A | Green |
| Deleted | D | Red |
| Untracked | ? | Gray |
| Renamed | R | Cyan |
| Conflicted | U | Magenta |

### Activity Log

```
14:32:01  ● commit: Add feature X
14:31:45  + staged: src/new_file.rs
14:30:22  ↓ pull: origin/main (fast-forward)
```

**Format**: `HH:MM:SS  icon command: details`

**Icons by Command Type**:
| Command | Icon |
|---------|------|
| commit | ● |
| checkout | ⎇ |
| merge | ⑂ |
| pull | ↓ |
| push | ↑ |
| rebase | ↺ |
| reset | ↩ |
| stash | □ |
| stage | + |
| unstage | - |

### Footer

```
[Tab] switch │ [j/k] navigate │ [s] stage │ [d] diff │ [?] help
```

Context-sensitive hints for available actions.

## Keybindings

### Global

| Key | Action |
|-----|--------|
| `q` | Quit application |
| `Ctrl+c` | Quit application |
| `?` | Toggle help overlay |
| `r` | Force refresh status |
| `Tab` | Next panel |
| `Shift+Tab` | Previous panel |

### Navigation (in file lists)

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next item |
| `k` / `↑` | Select previous item |
| `g` | Go to first item |
| `G` | Go to last item |
| `Ctrl+d` | Page down |
| `Ctrl+u` | Page up |

### Actions

| Key | Action |
|-----|--------|
| `s` | Stage selected (in working) / Unstage selected (in staged) |
| `d` | Show diff for selected file |
| `Enter` | Show diff for selected file |
| `Esc` | Close overlay / Cancel |

### Diff View

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `q` / `Esc` | Close diff view |
| `g` | Go to top |
| `G` | Go to bottom |

## Color Scheme

### Default Theme

```rust
Theme {
    // File status colors
    modified: Color::Yellow,
    added: Color::Green,
    deleted: Color::Red,
    untracked: Color::DarkGray,
    renamed: Color::Cyan,
    conflicted: Color::Magenta,

    // UI elements
    border: Color::DarkGray,
    border_focused: Color::Blue,
    title: Color::White,
    title_focused: Color::Cyan,
    selection_bg: Color::DarkGray,
    selection_fg: Color::White,

    // Activity
    timestamp: Color::DarkGray,
    command: Color::White,
    details: Color::Gray,

    // Status bar
    status_ok: Color::Green,
    status_warning: Color::Yellow,
    status_error: Color::Red,
}
```

### NO_COLOR Support

When `NO_COLOR` environment variable is set:
- Use only default terminal colors
- Rely on bold/dim/underline for distinction
- Selection shown with inverse colors

## Responsive Layout

### Minimum Size: 60x15

Below minimum: Show warning message

### Small (60-80 columns)

- Single column layout
- File lists stacked vertically
- Activity log condensed

### Medium (80-120 columns)

- Two column layout (working | staged)
- Activity log full width below

### Large (120+ columns)

- Full three-panel layout
- More details visible
- Wider file paths

## Accessibility

1. **Screen reader support**: All UI elements have text alternatives
2. **High contrast**: Works with terminal high contrast themes
3. **No color-only information**: Status uses characters + colors
4. **Keyboard-only navigation**: Full functionality without mouse
5. **Configurable refresh rate**: Reduce flicker for sensitive users
