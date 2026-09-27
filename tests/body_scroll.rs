//! Integration tests for main panel scrolling on short terminals.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use git_monitor::App;

/// Terminal height: 3 header + 3 footer + 2 border rows leave 12 body rows
const HEIGHT: u16 = 20;
const WIDTH: u16 = 80;

/// Create a repo with more commits than fit on a short terminal
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

/// Draw one frame and return the terminal rows as text
fn draw(terminal: &mut Terminal<TestBackend>, app: &mut App) -> Vec<String> {
    terminal
        .draw(|frame| git_monitor::ui::render(frame, app))
        .expect("draw frame");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

/// Row of the cursor marker inside the body panel
fn cursor_row(rows: &[String]) -> Option<usize> {
    rows.iter()
        .position(|row| row.starts_with("│▸"))
}

fn press(app: &mut App, code: KeyCode, times: usize) {
    for _ in 0..times {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }
}

#[test]
fn view_scrolls_before_cursor_reaches_bottom_edge() {
    let dir = create_repo_with_commits(30);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("terminal");

    // Walk well past the first screen of commits
    for step in 0..25 {
        press(&mut app, KeyCode::Down, 1);
        let rows = draw(&mut terminal, &mut app);
        let row = cursor_row(&rows).unwrap_or_else(|| {
            panic!(
                "cursor visible after {step} steps:\n{}",
                rows.join("\n")
            )
        });

        // Body rows span 4..=15; the last body row is 15. With a margin of 4,
        // the cursor never sits in the bottom four body rows while more
        // commits remain below it.
        assert!(
            row <= 11,
            "cursor at row {row} after {step} steps:\n{}",
            rows.join("\n")
        );
    }
    assert!(
        app.body_scroll > 0,
        "view scrolled down"
    );
}

#[test]
fn view_scrolls_back_up_with_cursor() {
    let dir = create_repo_with_commits(30);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).expect("terminal");

    press(&mut app, KeyCode::Down, 25);
    draw(&mut terminal, &mut app);
    assert!(
        app.body_scroll > 0,
        "view scrolled down"
    );

    // Walk back up one step at a time so the view follows
    for _ in 0..25 {
        press(&mut app, KeyCode::Up, 1);
        draw(&mut terminal, &mut app);
    }
    let rows = draw(&mut terminal, &mut app);
    assert_eq!(app.body_scroll, 0, "view back at top");
    assert!(
        cursor_row(&rows).is_some(),
        "cursor visible at top:\n{}",
        rows.join("\n")
    );
}
