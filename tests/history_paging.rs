//! Integration tests: history paging remembers the cursor, and J/K stay in a section.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use git_monitor::App;

/// Commits per history page (matches the app's page size)
const PAGE: usize = 50;

/// Repo with `count` empty commits
fn repo_with_commits(count: usize) -> TempDir {
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

fn press(app: &mut App, code: KeyCode, times: usize) {
    for _ in 0..times {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }
}

/// Screen row and text of the cursor line
fn cursor(terminal: &mut Terminal<TestBackend>, app: &mut App) -> (usize, String) {
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
        .enumerate()
        .find(|(_, row)| row.starts_with("│▸"))
        .expect("cursor on screen")
}

fn first_sha(app: &App) -> String {
    app.snapshot().history[0]
        .sha
        .clone()
        .expect("sha")
}

fn last_sha(app: &App) -> String {
    app.snapshot()
        .history
        .last()
        .and_then(|cmd| cmd.sha.clone())
        .expect("sha")
}

#[test]
fn paging_back_returns_to_the_same_commit_and_screen_row() {
    let dir = repo_with_commits(120);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");

    // Jump to the first commit, then walk to 5 from the bottom of the page
    press(&mut app, KeyCode::Char('h'), 1);
    press(&mut app, KeyCode::Char('j'), PAGE - 6);
    let before = cursor(&mut terminal, &mut app);

    press(&mut app, KeyCode::Char(']'), 1);
    cursor(&mut terminal, &mut app);
    assert_eq!(app.history_page, 1, "moved to page 2");

    press(&mut app, KeyCode::Char('['), 1);
    let after = cursor(&mut terminal, &mut app);

    assert_eq!(app.history_page, 0, "back on page 1");
    assert_eq!(
        after, before,
        "same commit at the same screen row"
    );
}

#[test]
fn unvisited_page_starts_on_its_first_commit() {
    let dir = repo_with_commits(120);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");

    press(&mut app, KeyCode::Char('h'), 1);
    press(&mut app, KeyCode::Char('j'), 10);
    press(&mut app, KeyCode::Char(']'), 1);

    let (_, line) = cursor(&mut terminal, &mut app);
    assert!(
        line.contains(&first_sha(&app)),
        "first commit: {line}"
    );
}

#[test]
fn shift_j_turns_the_page_instead_of_stopping() {
    let dir = repo_with_commits(120);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");

    // Walk with j so the J presses below are single taps, not a held key
    press(&mut app, KeyCode::Char('h'), 1);
    press(&mut app, KeyCode::Char('j'), PAGE - 1);
    let (_, line) = cursor(&mut terminal, &mut app);
    assert!(
        line.contains(&last_sha(&app)),
        "on the last commit: {line}"
    );

    press(&mut app, KeyCode::Char('J'), 1);
    let (_, line) = cursor(&mut terminal, &mut app);
    assert_eq!(app.history_page, 1, "turned to page 2");
    assert!(
        line.contains(&first_sha(&app)),
        "on page 2's first commit: {line}"
    );

    press(&mut app, KeyCode::Char('K'), 1);
    let (_, line) = cursor(&mut terminal, &mut app);
    assert_eq!(app.history_page, 0, "back to page 1");
    assert!(
        line.contains(&last_sha(&app)),
        "on page 1's last commit: {line}"
    );
}

#[test]
fn shift_j_stops_at_the_end_of_the_last_page() {
    let dir = repo_with_commits(3);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");

    press(&mut app, KeyCode::Char('h'), 1);
    press(&mut app, KeyCode::Char('J'), 10);

    let (_, line) = cursor(&mut terminal, &mut app);
    assert!(
        line.contains(&last_sha(&app)),
        "still on the last commit: {line}"
    );
}

#[test]
fn shift_j_never_leaves_the_files_section() {
    let dir = repo_with_commits(2);
    std::fs::write(dir.path().join("a.txt"), "a").expect("write a");
    std::fs::write(dir.path().join("b.txt"), "b").expect("write b");
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let mut terminal = Terminal::new(TestBackend::new(80, 30)).expect("terminal");

    // The app starts on the first changed file
    press(&mut app, KeyCode::Char('J'), 5);

    let (_, line) = cursor(&mut terminal, &mut app);
    assert!(
        line.contains("b.txt"),
        "stopped on the last file: {line}"
    );
}

#[test]
fn held_shift_j_speeds_up() {
    let dir = repo_with_commits(120);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");

    // 40 quick repeats: 10 rows at 1, 10 at 2, 10 at 4, then 8 per repeat
    press(&mut app, KeyCode::Char('h'), 1);
    press(&mut app, KeyCode::Char('J'), 40);

    assert!(
        app.history_page >= 2,
        "40 repeats covered more than two pages (page {})",
        app.history_page
    );
}
