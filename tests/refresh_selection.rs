//! Integration tests: a refresh keeps the cursor on the same item.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use git_monitor::App;

/// Create a repo with a linear history of empty commits
fn create_repo_with_commits(count: usize) -> TempDir {
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

    let mut parent: Option<git2::Oid> = None;
    for i in 0..count {
        let parent_commit = parent.map(|oid| {
            repo.find_commit(oid)
                .expect("find parent")
        });
        let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();
        let oid = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("Commit {i}"),
                &tree,
                &parents,
            )
            .expect("commit");
        parent = Some(oid);
    }

    dir
}

/// Text of the row holding the cursor marker
fn cursor_line(terminal: &mut Terminal<TestBackend>, app: &mut App) -> String {
    terminal
        .draw(|frame| git_monitor::ui::render(frame, app))
        .expect("draw frame");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .find(|row| row.starts_with("│▸"))
        .expect("cursor row on screen")
}

fn press_down(app: &mut App, times: usize) {
    for _ in 0..times {
        app.handle_key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        ));
    }
}

fn terminal() -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(100, 40)).expect("terminal")
}

#[test]
fn cursor_stays_on_commit_when_a_file_appears_above() {
    let dir = create_repo_with_commits(10);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let mut terminal = terminal();

    press_down(&mut app, 5);
    let before = cursor_line(&mut terminal, &mut app);
    assert!(
        before.contains("Commit"),
        "cursor on a commit: {before}"
    );

    std::fs::write(dir.path().join("new.txt"), "agent edit").expect("write file");
    app.refresh_status();

    let after = cursor_line(&mut terminal, &mut app);
    assert_eq!(
        after, before,
        "cursor moved after refresh"
    );
}

#[test]
fn cursor_stays_on_commit_when_a_file_disappears_above() {
    let dir = create_repo_with_commits(10);
    std::fs::write(dir.path().join("a.txt"), "a").expect("write a");
    std::fs::write(dir.path().join("b.txt"), "b").expect("write b");
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let mut terminal = terminal();

    press_down(&mut app, 7);
    let before = cursor_line(&mut terminal, &mut app);
    assert!(
        before.contains("Commit"),
        "cursor on a commit: {before}"
    );

    std::fs::remove_file(dir.path().join("a.txt")).expect("remove a");
    app.refresh_status();

    let after = cursor_line(&mut terminal, &mut app);
    assert_eq!(
        after, before,
        "cursor moved after refresh"
    );
}

#[test]
fn cursor_stays_on_file_when_another_file_sorts_above_it() {
    let dir = create_repo_with_commits(3);
    std::fs::write(dir.path().join("m.txt"), "m").expect("write m");
    std::fs::write(dir.path().join("z.txt"), "z").expect("write z");
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let mut terminal = terminal();

    let mut before = cursor_line(&mut terminal, &mut app);
    for _ in 0..5 {
        if before.contains("z.txt") {
            break;
        }
        press_down(&mut app, 1);
        before = cursor_line(&mut terminal, &mut app);
    }
    assert!(
        before.contains("z.txt"),
        "cursor on z.txt: {before}"
    );

    std::fs::write(dir.path().join("a.txt"), "a").expect("write a");
    app.refresh_status();

    let after = cursor_line(&mut terminal, &mut app);
    assert_eq!(
        after, before,
        "cursor moved after refresh"
    );
}
