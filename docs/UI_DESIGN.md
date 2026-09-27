# Git Monitor - UI Design

For keys, see [KEYBINDINGS.md](KEYBINDINGS.md).

## Layout

The screen stacks three blocks:

- **Header**: 3 rows.
- **Body**: the rest of the screen, at least 10 rows.
- **Footer**: 3 rows.

A popup (diff, command output, or error details) replaces the body, and the footer switches to
popup hints. The help overlay and menus draw on top of the whole screen.

### Main View

```text
┌ git-monitor ─────────────────────────────────────────────────────────────────────┐
│ ⎇ develop ↑1 │ my-project │ ● watching                                           │
└──────────────────────────────────────────────────────────────────────────────────┘
┌ Repository ──────────────────────────────────────────────────────────────────────┐
│  : command  a aliases                                                            │
│                                                                                  │
│                                                                                  │
│  ▸ Staged (1)                                                                    │
│  ● A src/new_file.rs  +12                                                        │
│                                                                                  │
│  ▸ Working (2)                                                                   │
│  ○ M src/main.rs  +5/-2                                                          │
│  ○ ? untracked.txt                                                               │
│                                                                                  │
│  ▾ History (3)                                                                   │
│  h log/reflog · Space expand · [ prev page · ] next page · y copy short          │
│  develop ↑1 → origin/develop  · P push  f fetch                                  │
│▸    ├─ e29d83a 14 min ago  ● Keep commit messages visible (HEAD → develop)       │
│     ├─ 9bfa421 2 hr ago  ● Include expanded-branch commits (origin/develop)      │
│     └─ 234a04a 3 hr ago  ● Load repository data as one snapshot                  │
│  [ prev · page ]  1/1 of 3 items                                                 │
│                                                                                  │
│  ▸ Branches (1)                                                                  │
│  feature/new-ui                                                                  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

The body is one list with five sections: the command row, Staged, Working, History, and Branches.
One cursor moves through all of them. These rows are selectable:

- the command row
- each staged and working file
- the History header and each history commit
- each branch, and each commit of the expanded branch

The Staged, Working, and Branches headers are not selectable. Inside an expanded commit, `j` / `k`
move through its files.

A blank line follows the command row, and another separates each section, so two blank lines sit
below the command row.

Staged, Working, and Branches hide when empty. History always shows its header, even with no
commits. The body has a "No changes or activity" fallback line, but History always counts its
header, so the line never shows.

The body scrolls to keep the cursor in view. It keeps 5 rows of margin above and below the cursor,
or a third of the visible rows on short terminals. It never scrolls past the last line.

## Components

### Header

```text
 ⎇ develop ↑1↓2 │ my-project │ ● watching
```

- The block title "git-monitor" sits on the top border in bold cyan. The border is dark gray.
- `⎇` is cyan. The branch name is bold green, or "(no branch)" when there is none.
- `↑N` (green) and `↓N` (red) show only when nonzero.
- The repository folder name is white.
- "● watching" is green.

### Command Row

| State | Looks like |
|-------|------------|
| Idle | `  : command  a aliases` |
| Selected | `▸ : type command   a aliases` |
| Typing | `▸ : git status_` |

- `:` is bold cyan when selected or typing. The `a` key is cyan. Other hint text is dark gray.
- The cursor `_` is cyan.
- After a command runs, up to 4 output lines show as `  > line`, white on success and red on
  failure. Longer output ends with `    ... (N more lines) o to expand`.
- Output hides while a menu is open.
- The prompt runs any command.

### File Sections

```text
  ▾ Working (2)
  s stage/unstage · d diff · Enter diff · Space pager · M difftool
▸ ○ M src/main.rs  +5/-2
  ○ ? untracked.txt
```

- The header reads `▾ Staged (N)` or `▾ Working (N)` while the cursor is in the section, and `▸`
  otherwise. Staged is bold green. Working is bold yellow.
- While the cursor is in the section, a hint line lists up to five actions. Keys are cyan. Labels
  are dark gray italic.
- Staged files start with a green `●`. Working files start with a yellow `○`.
- The selected row gets a `▸` prefix and bold text.
- Line counts show as `+N` (green) and `-N` (red).

| Status | Character | Color |
|--------|-----------|-------|
| Modified | M | Yellow |
| Added | A | Green |
| Deleted | D | Red |
| Renamed | R | Cyan |
| Untracked | ? | Dark Gray |
| Conflicted | U | Magenta |

### History Section

The header reads `▾ History (N)` when expanded and `▸ History (N)` when collapsed. It reads
"Reflog" in reflog mode. It is bold cyan, and N counts the entries on the current page. The header
is selectable. History starts expanded.

Expanding History collapses any expanded branch. Expanding a branch collapses History.

Below an expanded header:

1. A hint line, shown only when the cursor is on a commit.
2. The branch line: `  develop ↑1 → origin/develop  · P push  f fetch`. The branch name is cyan
   and the upstream is dark gray. `P push` shows when ahead, `p pull` when behind, and `f fetch`
   always.
3. One line per commit.
4. The page line: `  [ prev · page ]  1/4 of 180 items`, in dark gray. A page holds 50 local entries.

#### Commit Lines

History and expanded branches share this format:

```text
     ├─ e29d83a 14 min ago  ● Keep commit messages visible (HEAD → develop +1)
     └─ 234a04a 3 hr ago  ● Load repository data as one snapshot
```

- The graph is dark gray. `├─` marks each commit and `└─` the last one.
- Below 80 columns the graph sits 2 columns further left.
- Upstream commits missing locally have no indent, a red `├—` graph, a red SHA, and a dark gray
  message.
- The SHA is yellow. The time is dark gray, in the form "just now", "14 min ago", "2 hr ago",
  "3 days ago", "2 wk ago", "5 mo ago", or "1 yr ago".
- The icon takes its type's color (see below). The message is white.
- `fixup!`, `squash!`, and `amend!` prefixes are bold magenta. `WIP` and `wip:` prefixes are bold
  yellow.
- The selected commit is bold.

Refs follow the message in dark gray parentheses. `HEAD →` is bold cyan, the HEAD branch is bold
green, other local branches are green, remote branches are red, and tags read `tag: name` in
yellow. When all refs would leave the message under 20 columns, only the first ref stays and the
rest fold into `+N`. Messages that still do not fit end in `...`.

In reflog mode each line carries the reflog message, such as `⎇ checkout: moving from main to
feature`.

#### Expanded Commits

`Space` on a commit adds its details below it:

```text
    Author:    Ada Lovelace <ada@example.com>
    Date:      2026-09-27 14:32:01 -0400
    Commit:    e29d83a4b1c9f07d2e6a8b35c0f41d9e7a2b6c13

    Keep commit messages visible in narrow panels

    2 file(s) changed  +45 / -12
  ▸ M src/app.rs  +40/-12
    A src/new.rs  +5
```

- The author name is green and the email cyan. The date and full SHA are yellow.
- A Committer line shows when it differs from the author. A magenta GPG line shows when the commit
  has a signature status.
- A hint line for commit files appears above the files while one is selected.

#### Command Icons

| Command | Icon | Color |
|---------|------|-------|
| Commit | ● | Green |
| Checkout | ⎇ | Cyan |
| Merge | ⑂ | Magenta |
| Rebase | ↺ | Yellow |
| Pull | ↓ | Blue |
| Push | ↑ | Blue |
| Fetch | ⟳ | Blue |
| Reset | ↩ | Red |
| CherryPick | ❋ | Magenta |
| Revert | ⊗ | Red |
| Stash | □ | Yellow |
| Branch | ⌥ | Cyan |
| Clone | ⊕ | Green |
| Init | ★ | Green |
| Other | • | Dark Gray |

Commit log entries always use the Commit icon. The reflog parser does not detect Push, Fetch, or
Stash yet, so those entries show as Other.

### Branches Section

```text
  ▾ Branches (2)
  Enter checkout · Space expand · e edit
▸ bugfix/login
     ├─ 3b011dc 2 days ago  ● Add parser (bugfix/login)
     └─ f489741 3 days ago  ● Initial commit
  feature/new-ui
```

- The header reads `▾ Branches (N)` while the cursor is in the section, and `▸` otherwise. It is
  bold cyan and not selectable.
- The list shows local branches other than the current one. Names are white, and bold when
  selected.
- An expanded branch lists its commits below it in the shared commit line format.

### Footer

The footer shows one of these, in priority order:

1. **Command prompt**: `Enter execute  Esc cancel  Up/Down history`.
2. **Toast**: a short message. Info is blue, success green, warning yellow, and error red.
3. **Error**: the error message in red.
4. **Hints**: `j/k nav  g/G top/btm  m menu  e edit │ : cmd  w files  h history  b branches │ ? help  q quit`.

The hints never change with the cursor. Context hints render inside the sections instead.
`e edit` shows only when `$EDITOR` is set. The full hint line needs about 116 columns, and
narrower terminals cut off its right end.

Hints are centered. Keys sit on dark gray bold chips. The border is dark gray.

While a popup is open, the footer reads `j/k scroll  g/G top/bottom  Ctrl+d/u page  q close`.

### Popups

A popup fills the body area with a cyan border and a bold cyan title:

- ` Diff: path ` for file diffs. Untracked files show their content as added lines under
  "(new file)".
- ` Output: command ` for command output. Text is white on success and red on failure.
- ` Error: title ` for error details. The message is red and the context dark gray.

When content overflows, the bottom border shows the scroll position as `[N/M]` in dark gray.

| Diff line | Color |
|-----------|-------|
| Addition (`+`) | Green |
| Deletion (`-`) | Red |
| Hunk header (`@@`) | Cyan |
| File header (`diff`, `index`) | Yellow |
| Other, including `+++` and `---` | White |

### Help Overlay

`?` opens a centered box, 60% wide and 70% tall, with a cyan border and the title " Help ". It
lists these groups: Navigation, Staged/Working Files, History, Commit Files, Branches,
Command & Aliases, and General. It ends with "Press any key to close" in dark gray italic.

The action registry builds most lines, so `P` and `p` appear only when ahead or behind. The box
does not scroll, so short terminals cut off the lower groups.

### Menus

Menus draw as centered overlays 50% wide with a cyan border. The menu stack manages them (see
[ADR-005](adr/005-modular-menu-prompt-system.md)).

- **Action menu** (`m`): titled " {Context} Actions ", such as " Working Files Actions ". Each
  item shows its key. Alias items start with `⎇`. It ends with `j/k navigate  Enter execute  m/Esc
  close`. See [ADR-002](adr/002-contextual-action-menu-framework.md).
- **Alias browser** (`a`): an "Alias Sections" list, then the aliases in the chosen section.
- **Push confirmation** (`P`): asks "Push {branch} to {remote}?", adding "and set upstream"
  when the branch has none. It offers a force checkbox.

## Colors

The UI uses only the 16 named terminal colors.

| Element | Color |
|---------|-------|
| Block titles | Bold cyan |
| Header and footer borders | Dark gray |
| Body, popup, help, and menu borders | Cyan |
| Selection | `▸` prefix and bold text |
| Hint keys | Cyan |
| Hint labels | Dark gray italic |
| SHA | Yellow |
| Times and graph | Dark gray |
| Errors | Red |
