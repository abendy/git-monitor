# Git Monitor - UI Design

## Layout

### Main View (Unified Vertical List)

```
┌─────────────────────────────────────────────────────────────────────┐
│  git-monitor  │ ⎇ main ↑2 ↓0 │ repo-name │ ● watching             │  Header
├─────────────────────────────────────────────────────────────────────┤
│ ▸ : git status                                                      │  Command
│   ✓ (no output)                                                     │  Section
├─────────────────────────────────────────────────────────────────────┤
│ Staged (2)                                                          │  Staged
│   A src/new_file.rs                                                 │  Section
│   M src/config.rs                                                   │
├─────────────────────────────────────────────────────────────────────┤
│ Working Directory (4)                                               │  Working
│   M src/main.rs                                                     │  Section
│   M src/lib.rs                                                      │
│   ? untracked.txt                                                   │
│   D old_file.rs                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ Commit Log (50)                       [h to toggle to Reflog]       │  History
│   abc1234  14:32:01  Add feature X  (HEAD → main, origin/main)     │  Section
│   def5678  14:31:45  Fix critical bug                              │
│   ghi9012  14:30:22  Initial commit  (tag: v0.1.0)                 │
├─────────────────────────────────────────────────────────────────────┤
│ [:] cmd  [a] alias  [s] stage  [d] diff  [h] history  [?] help     │  Footer
└─────────────────────────────────────────────────────────────────────┘
```

**Unified List Navigation**: Single selection index spans all sections:
- Index 0: Command section
- Index 1..N: Staged files
- Index N+1..M: Working files
- Index M+1..end: History entries

### Popup System (Full-Screen Overlays)

Popups replace the body entirely when active:

**Diff Popup** (pressing `d` or `Enter` on a file):

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
│                    [j/k] scroll  [g/G] top/bottom  [q] close        │
└─────────────────────────────────────────────────────────────────────┘
```

**Command Output Popup** (pressing `o` on command section after execution):

```
┌─────────────────────────────────────────────────────────────────────┐
│ Output: git status                                                  │
├─────────────────────────────────────────────────────────────────────┤
│ On branch main                                                      │
│ Your branch is up to date with 'origin/main'.                      │
│                                                                     │
│ nothing to commit, working tree clean                               │
│                                                                     │
│                    [j/k] scroll  [g/G] top/bottom  [q] close        │
└─────────────────────────────────────────────────────────────────────┘
```

Popups support vim-style navigation: `j/k` scroll, `Ctrl+d/u` half-page, `g/G` top/bottom.

Diff uses full screen area. Lines are colored:
- Green (`+`) for additions
- Red (`-`) for deletions
- Cyan (`@@`) for hunk headers
- Yellow for diff/index headers

### Alias Browser

Displays when pressing `a` on command section (if aliases exist):

**Section List**:
```
┌───────────────────────────────────────────────────────────┐
│                      Alias Sections                        │
├───────────────────────────────────────────────────────────┤
│  ▸ fetch (5 aliases)                                      │
│    commit (8 aliases)                                     │
│    branch (4 aliases)                                     │
│    other (12 aliases)                                     │
│                                                           │
│           [Enter] open section  [q/Esc] close             │
└───────────────────────────────────────────────────────────┘
```

**Aliases within Section**:
```
┌───────────────────────────────────────────────────────────┐
│                      fetch aliases                         │
├───────────────────────────────────────────────────────────┤
│  ▸ fa       fetch --all                                   │
│    fp       fetch --prune                                 │
│    fu       fetch upstream                                │
│                                                           │
│       [Enter] run alias  [Esc] back  [q] close            │
└───────────────────────────────────────────────────────────┘
```

### Help Overlay

Displays when pressing `?`:

```
┌───────────────────────────────────────────────────────────┐
│                          Help                              │
├───────────────────────────────────────────────────────────┤
│                                                           │
│  Navigation                                               │
│  j / ↓              Move down                             │
│  k / ↑              Move up                               │
│  g                  Go to first                           │
│  G                  Go to last                            │
│                                                           │
│  Actions                                                  │
│  :                  Enter command mode                    │
│  a                  Open alias browser (on command)       │
│  s                  Stage/unstage file                    │
│  d                  Show diff                             │
│  h                  Jump to history / toggle mode         │
│  y                  Copy SHA (on history item)            │
│  o                  Open output popup (on command)        │
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

### Command Section

Always at the top of the list (index 0):
```
▸ : git status                    <- selected
  ✓ (no output)                   <- last command result
```

- Shows last executed command and its result
- `✓` in green for success, `✗` in red for failure
- Press `:` or `Enter` to enter command mode
- Press `a` to browse aliases
- Press `o` to view full output in popup

### File Sections (Staged + Working)

Vertical list below command section:
```
Staged (2)
  ▸ A src/new_file.rs        <- selected (bold, with ▸ indicator)
    M src/config.rs

Working Directory (4)
    M src/main.rs
    M src/lib.rs
    ? untracked.txt
    D old_file.rs
```

**Selection Indicator**: `▸` prefix for selected item
**Status Character**: Single character before path (M/A/D/R/?)

### File Status Colors

| Status | Character | Color |
|--------|-----------|-------|
| Modified | M | Yellow |
| Added | A | Green |
| Deleted | D | Red |
| Renamed | R | Cyan |
| Untracked | ? | Dark Gray |
| Conflicted | U | Magenta |

### History Section

Toggleable between Commit Log (default) and Reflog:

**Commit Log Mode** (default):
```
Commit Log (50)                       [h to toggle]
  abc1234  14:32:01  Add feature X  (HEAD → main)
  def5678  14:31:45  Fix critical bug  (origin/main)
  ghi9012  14:30:22  Initial commit  (tag: v0.1.0)
```

**Reflog Mode**:
```
Reflog (50)                           [h to toggle]
  14:32:01  ● commit: Add feature X
  14:31:45  ⎇ checkout: moving from main to feature/new
  14:30:22  ↓ pull: Fast-forward
```

**Format** (Commit Log): `  SHA  HH:MM:SS  message  (decorations)`
**Format** (Reflog): `  HH:MM:SS  icon message`
- SHA in dark gray (can copy with `y`)
- Timestamp in dark gray
- Icon colored by command type
- Decorations in parentheses (HEAD, branches, tags)

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
| `Esc` | Quit application (or close modal) |
| `Ctrl+c` | Quit application |
| `?` | Toggle help overlay |
| `r` | Force refresh status |

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next item |
| `k` / `↑` | Select previous item |
| `g` | Go to first item |
| `G` | Go to last item |
| `h` | Jump to history / toggle history mode |

### Actions

| Key | Action |
|-----|--------|
| `:` | Enter command mode |
| `Enter` | Enter command mode (on command) / Show diff (on file) |
| `a` | Open alias browser (on command section) |
| `o` | Open output popup (on command section) |
| `s` | Stage/Unstage selected file |
| `d` | Show diff for selected file |
| `y` | Copy SHA to clipboard (on history item) |

### Command Mode

| Key | Action |
|-----|--------|
| Any character | Type command |
| `Backspace` | Delete character |
| `Enter` | Execute command |
| `↑` | Previous history |
| `↓` | Next history |
| `Esc` / `Ctrl+c` | Cancel command mode |

### Popup (Diff/Output)

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down |
| `k` / `↑` | Scroll up |
| `Ctrl+d` | Scroll half page down |
| `Ctrl+u` | Scroll half page up |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `q` / `Esc` | Close popup |

### Alias Browser

| Key | Action |
|-----|--------|
| `j` / `↓` | Select next |
| `k` / `↑` | Select previous |
| `Enter` | Open section / Run alias |
| `Esc` | Back / Close |
| `q` | Close browser |

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
- **Body**: Unified vertical list (fills remaining space)
  - Command section: 2 lines
  - Staged section: dynamic
  - Working section: dynamic
  - History section: dynamic
- **Footer**: Fixed 3 lines

## Empty States

When no items exist in a section:
- Staged: "No staged changes" (dark gray, italic)
- Working Directory: "No changes" (dark gray, italic)
- History: "No activity yet" (dark gray, italic)
