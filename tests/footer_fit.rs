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

fn footer_row(dir: &TempDir, width: u16) -> String {
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let height = 30;
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    terminal
        .draw(|frame| git_monitor::ui::render(frame, &mut app))
        .expect("draw frame");
    let buffer = terminal.backend().buffer();
    // Footer text sits one row above the bottom border
    (0..width)
        .map(|x| buffer[(x, height - 2)].symbol())
        .collect()
}

#[test]
fn sidebar_width_keeps_menu_help_and_jump_keys() {
    let row = footer_row(&repo(true, true), 68);

    for label in ["menu", "help", "files", "history", "branches"] {
        assert!(
            row.contains(label),
            "{label} visible: {row}"
        );
    }
    assert!(
        row.trim_end().ends_with('│'),
        "nothing cut at the border: {row}"
    );
}

#[test]
fn jump_keys_hide_when_there_is_nowhere_to_jump() {
    let row = footer_row(&repo(false, false), 140);

    assert!(
        row.contains("history"),
        "history always shown: {row}"
    );
    assert!(
        !row.contains("files"),
        "no files, no w: {row}"
    );
    assert!(
        !row.contains("branches"),
        "no other branches, no b: {row}"
    );
}

#[test]
fn wide_footer_shows_every_hint() {
    let row = footer_row(&repo(true, true), 140);

    for label in [
        "nav", "top/btm", "menu", "cmd", "files", "history", "branches", "help", "quit",
    ] {
        assert!(
            row.contains(label),
            "{label} visible: {row}"
        );
    }
}
