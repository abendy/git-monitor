//! Integration tests: the footer fits narrow panels and keeps its key hints.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use git_monitor::App;

/// Repo with one commit; optionally an untracked file and a second branch
fn repo(with_file: bool, with_branch: bool) -> TempDir {
    let dir = TempDir::new().expect("create temp dir");
    let repo = git2::Repository::init(dir.path()).expect("init repo");
    let sig = git2::Signature::now("Test", "test@example.com").expect("signature");
    let tree_id = repo
        .index()
        .expect("index")
        .write_tree()
        .expect("write tree");
    let tree = repo
        .find_tree(tree_id)
        .expect("find tree");
    let oid = repo
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        )
        .expect("commit");

    if with_branch {
        let commit = repo
            .find_commit(oid)
            .expect("find commit");
        repo.branch("feature", &commit, false)
            .expect("create branch");
    }
    if with_file {
        std::fs::write(dir.path().join("new.txt"), "new").expect("write file");
    }
    dir
}

fn screen(app: &mut App, width: u16) -> Vec<String> {
    let height = 30;
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    terminal
        .draw(|frame| git_monitor::ui::render(frame, app))
        .expect("draw frame");
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

fn footer_row(dir: &TempDir, width: u16) -> String {
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let rows = screen(&mut app, width);
    // Footer text sits one row above the bottom border
    rows[rows.len() - 2].clone()
}

/// The header row for a section, e.g. "History"
fn header<'a>(rows: &'a [String], name: &str) -> &'a str {
    rows.iter()
        .find(|row| row.contains(&format!("{name} (")))
        .unwrap_or_else(|| panic!("{name} header on screen"))
}

#[test]
fn sidebar_width_keeps_menu_and_help() {
    let row = footer_row(&repo(true, true), 68);

    assert!(
        row.contains("menu"),
        "menu visible: {row}"
    );
    assert!(
        row.contains("help"),
        "help visible: {row}"
    );
    assert!(
        row.trim_end().ends_with('│'),
        "nothing cut at the border: {row}"
    );
}

#[test]
fn footer_leaves_out_keys_shown_elsewhere() {
    let row = footer_row(&repo(true, true), 140);

    for label in ["cmd", "files", "history", "branches"] {
        assert!(
            !row.contains(label),
            "{label} not in footer: {row}"
        );
    }
    for label in ["nav", "top/btm", "menu", "help", "quit"] {
        assert!(
            row.contains(label),
            "{label} in footer: {row}"
        );
    }
}

#[test]
fn section_headers_show_jump_keys_when_the_cursor_is_elsewhere() {
    let dir = repo(true, true);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");

    // The app starts on the changed file, inside Working
    let rows = screen(&mut app, 80);
    assert!(
        !header(&rows, "Working").contains(")  w"),
        "no w while in Working"
    );
    assert!(
        header(&rows, "History").contains(")  h"),
        "h on History header"
    );
    assert!(
        header(&rows, "Branches").contains(")  b"),
        "b on Branches header"
    );

    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('h'),
        crossterm::event::KeyModifiers::NONE,
    ));
    let rows = screen(&mut app, 80);
    assert!(
        header(&rows, "Working").contains(")  w"),
        "w on Working header"
    );
    assert!(
        !header(&rows, "History").contains(")  h"),
        "no h while in History"
    );
}
