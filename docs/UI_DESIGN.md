# Git Monitor - UI Design

## Layout

### Main View

```
┌─────────────────────────────────────────────────────────────────────┐
│  git-monitor  │ ⎇ main ↑2 ↓0 │ repo-name │ ● watching             │  Header (3 lines)
├────────────────────────────┬────────────────────────────────────────┤
│ Working Directory (4)      │ Staged (2)                             │  Body (60%)
│ ───────────────────────    │ ────────────────────                   │
│ ▸ M src/main.rs            │   A src/new_file.rs                    │
│   M src/lib.rs             │   M src/config.rs                      │
│   ? untracked.txt          │                                        │
│   D old_file.rs            │                                        │
│                            │                                        │
├────────────────────────────┴────────────────────────────────────────┤
│ Recent Activity (12)                                                │  Activity (40%)
│ ────────────────────────────────────────────────────────────────────│
│   14:32:01  ● commit: Add feature X                                │
│   14:31:45  ⎇ checkout: moving from main to feature/new            │
│   14:30:22  ↓ pull: Fast-forward                                   │
├─────────────────────────────────────────────────────────────────────┤
│ [Tab] switch  [j/k] nav  [s] stage  [d] diff  [?] help  [q] quit  │  Footer (3 lines)
└─────────────────────────────────────────────────────────────────────┘
```

### Diff Overlay

Displays when pressing `d` or `Enter` on a selected file:

```
┌─────────────────────────────────────────────────────────────────────┐
│ Diff: src/main.rs                                                   │
├─────────────────────────────────────────────────────────────────────┤
│ @@ -10,6 +10,8 @@ fn main() {                                       │
│      let config = Config::load();                                   │
│      let app = App::new(config);                                    │
│ +    app.setup_watcher();                                           │
│ +    app.run();                                                     │
│      println!("Starting...");                                       │
│  }                                                                  │
│                                                                     │
│                           [q] close                                 │
└─────────────────────────────────────────────────────────────────────┘
```

Diff uses 90% of screen area. Lines are colored:
- Green (`+`) for additions
- Red (`-`) for deletions
- Cyan (`@@`) for hunk headers
- Yellow for diff/index headers

### Help Overlay

Displays when pressing `?`:

```
┌───────────────────────────────────────────────────────────┐
│                          Help                              │
├───────────────────────────────────────────────────────────┤
│                                                           │
│  Navigation                                               │
│  Tab / Shift+Tab    Switch panels                         │
│  j / ↓              Move down                             │
│  k / ↑              Move up                               │
│  g                  Go to first                           │
│  G                  Go to last                            │
│                                                           │
│  Actions                                                  │
│  s                  Stage/unstage file                    │
│  d                  Show diff                             │
│  r                  Refresh status                        │
│                                                           │
│  General                                                  │
│  ?                  Toggle help                           │
│  q / Esc            Quit                                  │
│                                                           │
│                  Press any key to close                   │
└───────────────────────────────────────────────────────────┘
```

Help overlay uses 60% width and 70% height, centered.

## Components

### Header Bar

```
 git-monitor  │ ⎇ main ↑2 ↓0 │ repo-name │ ● watching
└─────┬──────┘   └──┬───┘└─┬─┘  └───┬────┘   └───┬────┘
      │             │      │        │            │
   app title     branch  ahead/    repo       status
                       behind     name      indicator
```

**Elements**:
- **Title**: "git-monitor" in cyan
- **Branch**: Current branch with ⎇ icon (green)
- **Ahead/Behind**: ↑N (green) and ↓N (red) if tracking upstream
- **Repo Name**: Directory name from repo path
- **Status**: "● watching" in green

### File List Panels

Two panels side-by-side (50/50 split):

**Working Directory Panel**:
```
Working Directory (4)
  ▸ M src/main.rs        <- selected (bold, with ▸ indicator)
    M src/lib.rs
    ? untracked.txt
    D old_file.rs
```

**Staged Panel**:
```
Staged (2)
    A src/new_file.rs
    M src/config.rs
```

**Selection Indicator**: `▸` prefix for selected item
**Status Character**: Single character before path (M/A/D/R/?)
**Active Panel**: Cyan border, bold cyan title
**Inactive Panel**: Dark gray border, white title

### File Status Colors

| Status | Character | Color |
|--------|-----------|-------|
| Modified | M | Yellow |
| Added | A | Green |
| Deleted | D | Red |
| Renamed | R | Cyan |
| Untracked | ? | Dark Gray |
| Conflicted | U | Magenta |

### Activity Log

```
Recent Activity (12)
  14:32:01  ● commit: Add feature X
  14:31:45  ⎇ checkout: moving from main to feature/new
  14:30:22  ↓ pull: Fast-forward
```

**Format**: `  HH:MM:SS  icon message`
- Timestamp in dark gray
- Icon colored by command type
- Message in white (truncated with `...` if too long)

**Command Icons & Colors**:

| Command | Icon | Color |
|---------|------|-------|
| Commit | ● | Green |
| Checkout | ⎇ | Cyan |
| Merge | ⑂ | Magenta |
| Rebase | ↺ | Yellow |
| Pull | ↓ | Blue |
| Push | ↑ | Blue |
| Reset | ↩ | Red |
| CherryPick | ❋ | Magenta |
| Revert | ⊗ | Red |
| Branch | ⌥ | Cyan |
| Clone | ⊕ | Green |
| Init | ★ | Green |
| Other | • | Dark Gray |

### Footer

```
 [Tab] switch   [j/k] nav   [s] stage   [d] diff   [?] help   [q] quit
```

Keys shown with dark gray background, centered in footer area.

When an error occurs, the footer shows the error message in red instead of keybindings.

## Keybindings

### Global

| Key | Action |
|-----|--------|
| `q` | Quit application |
| `Esc` | Quit application |
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

### Actions

| Key | Action |
|-----|--------|
| `s` | Stage selected (in Working) / Unstage selected (in Staged) |
| `d` | Show diff for selected file |
| `Enter` | Show diff for selected file |

### Diff View

| Key | Action |
|-----|--------|
| `q` | Close diff view |
| `Esc` | Close diff view |
| `d` | Close diff view |

### Help Overlay

| Key | Action |
|-----|--------|
| Any key | Close help overlay |

## Color Scheme

### Element Colors

| Element | Color |
|---------|-------|
| App title | Cyan |
| Branch name | Green |
| Ahead count | Green |
| Behind count | Red |
| Repo name | White |
| Status indicator | Green |
| Active panel border | Cyan |
| Inactive panel border | Dark Gray |
| Active panel title | Cyan (bold) |
| Inactive panel title | White |
| Selection | Bold (same color as status) |
| Timestamp | Dark Gray |
| Error message | Red |
| Empty state text | Dark Gray (italic) |

### Diff Colors

| Line Type | Color |
|-----------|-------|
| Addition (`+`) | Green |
| Deletion (`-`) | Red |
| Hunk header (`@@`) | Cyan |
| File header (`diff`, `index`) | Yellow |
| Context | White |

## Layout Proportions

- **Header**: Fixed 3 lines
- **Body**: 60% of remaining space
  - Working panel: 50% width
  - Staged panel: 50% width
- **Activity**: 40% of remaining space
- **Footer**: Fixed 3 lines

## Empty States

When no items exist in a panel:
- Working Directory: "No changes" (dark gray, italic)
- Staged: "No staged changes" (dark gray, italic)
- Activity: "No activity yet" (dark gray, italic)
