# Keybindings

Keys depend on the selected row. Press `?` in the app for the same list, or `m` for the actions on
the current row.

## Global

These work on every row unless a section below gives the key another meaning.

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` / `G` | First / last row |
| `w` | Jump to the first staged or working file |
| `h` | Jump to History, expand it, and select the newest commit |
| `b` | Expand the first branch and select it |
| `m` | Action menu for the selected row |
| `:` | Open the command prompt |
| `a` | Alias browser (when aliases exist) |
| `e` | Open the selected file in `$EDITOR`, or the repository when no file is selected; warns when `$EDITOR` is unset |
| `P` | Push (opens a confirmation menu) |
| `p` | Pull |
| `f` | Fetch |
| `r` | Refresh |
| `?` | Help |
| `q` / `Esc` / `Ctrl+c` | Quit |

`P` and `p` always run. Being ahead or behind only decides whether menus, help, and hints list
them.

## Command Row

The command row is the top row of the list.

| Key | Action |
|-----|--------|
| `Enter` | Open the command prompt |
| `↑` | Open the prompt with the last command (when history exists) |
| `o` | Show the last command output in a popup |
| Other characters | Open the prompt and type that character |

Global keys still work here. Other characters, including `j`, `k`, `g`, `G`, and `m`, start a
command instead. Use `↓` to leave the row.

## Staged and Working Files

| Key | Action |
|-----|--------|
| `s` | Stage or unstage |
| `d` / `Enter` | Diff popup |
| `Space` | Diff in pager |
| `M` | Diff in difftool |

## History

On the History header:

| Key | Action |
|-----|--------|
| `Space` | Collapse or expand History |
| `h` | Switch between commit log and reflog |
| `[` / `]` | Previous / next page |

On a commit:

| Key | Action |
|-----|--------|
| `Space` | Expand or collapse the commit |
| `h` | Switch between commit log and reflog |
| `[` / `]` | Previous / next page |
| `y` | Copy short SHA |
| `c` | Copy SHA (full when the commit is expanded, short otherwise) |
| `R` | Interactive rebase onto the commit |

## Commit Files

In an expanded commit, `j` / `k` move through its files.

| Key | Action |
|-----|--------|
| `Space` | Diff in pager |
| `d` | Diff popup |
| `M` | Diff in difftool |
| `e` | Open the file in `$EDITOR` |

## Branches

On a branch:

| Key | Action |
|-----|--------|
| `Enter` | Check out the branch |
| `Space` | Expand or collapse its commits |

On a commit of the expanded branch:

| Key | Action |
|-----|--------|
| `Space` | Expand or collapse the commit |
| `y` | Copy short SHA |
| `c` | Copy SHA (full when the commit is expanded, short otherwise) |

## Command Prompt

The prompt runs any command.

| Key | Action |
|-----|--------|
| Any character | Type |
| `Backspace` | Delete the last character |
| `Enter` | Run the command |
| `Esc` / `Ctrl+c` | Cancel |
| `↑` | Older command from history |
| `↓` | Newer command, then back to your draft; when not browsing history, leave the prompt and move down |

## Popups (Diff, Output)

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down one line |
| `k` / `↑` | Scroll up one line |
| `Ctrl+d` / `Ctrl+u` | Scroll down / up by the visible height minus 2 |
| `g` / `G` | Top / bottom |
| `q` / `Esc` | Close |

## Menus

| Menu | Keys |
|------|------|
| Action menu | `j` / `k` / arrows move, `Enter` runs, `m` / `q` / `Esc` close |
| Alias sections | `j` / `k` / arrows move, `Enter` opens, `a` / `q` / `Esc` close |
| Aliases in a section | `j` / `k` / arrows move, `Enter` runs, `q` / `Esc` go back, `a` closes |
| Push confirmation | `Enter` / `y` push, `f` toggles `--force-with-lease`, `j` / `k` / arrows move to the force checkbox, `Space` / `Enter` toggle it there, `n` / `q` / `Esc` cancel |

## Help

Any key closes the help overlay.
