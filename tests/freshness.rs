//! Integration tests: the header shows how fresh the data is, and failed refreshes go stale.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use git_monitor::freshness::{FreshnessState, RefreshReason};
use git_monitor::App;

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
        parent = Some(
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("Commit {i}"),
                &tree,
                &parents,
            )
            .expect("commit"),
        );
    }
    dir
}

/// The header's text row
fn header(app: &mut App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("terminal");
    terminal
        .draw(|frame| git_monitor::ui::render(frame, app))
        .expect("draw frame");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.width)
        .map(|x| buffer[(x, 1)].symbol())
        .collect()
}

#[test]
fn header_shows_age_and_trigger_after_a_load() {
    let dir = repo_with_commits(3);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");

    let row = header(&mut app);
    assert!(
        row.contains("● now (start)"),
        "fresh after startup: {row}"
    );

    app.refresh(RefreshReason::FileChange);
    let row = header(&mut app);
    assert!(
        row.contains("● now (files)"),
        "trigger shown: {row}"
    );
}

#[test]
fn failed_refresh_goes_stale_and_keeps_the_old_data() {
    let dir = repo_with_commits(3);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let commits_before = app.snapshot().history.len();

    // Hide the object store so reading history and status fails
    let objects = dir.path().join(".git/objects");
    let hidden = dir.path().join(".git/objects-hidden");
    std::fs::rename(&objects, &hidden).expect("hide objects");
    app.refresh(RefreshReason::FileChange);

    assert!(
        matches!(
            app.freshness()
                .state(std::time::Instant::now()),
            FreshnessState::Stale { .. }
        ),
        "stale after a failed refresh"
    );
    assert_eq!(
        app.snapshot().history.len(),
        commits_before,
        "old data still shown"
    );
    let row = header(&mut app);
    assert!(
        row.contains("⚠ refresh failed"),
        "header says so: {row}"
    );
    assert!(
        row.contains("e details"),
        "points to details: {row}"
    );

    // The error stays viewable even after other messages replace the footer error
    app.feedback.error = None;
    app.handle_key(KeyEvent::new(
        KeyCode::Char('e'),
        KeyModifiers::NONE,
    ));
    assert!(
        app.feedback.popup.is_open(),
        "e opens the refresh error"
    );
}

#[test]
fn next_good_refresh_is_current_again() {
    let dir = repo_with_commits(3);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");
    let objects = dir.path().join(".git/objects");
    let hidden = dir.path().join(".git/objects-hidden");

    std::fs::rename(&objects, &hidden).expect("hide objects");
    app.refresh(RefreshReason::FileChange);
    assert!(
        app.freshness().failure().is_some(),
        "refresh failed"
    );
    std::fs::rename(&hidden, &objects).expect("restore objects");
    app.refresh(RefreshReason::Manual);

    let row = header(&mut app);
    assert!(
        row.contains("● now (manual)"),
        "current again: {row}"
    );
    assert!(
        app.freshness().failure().is_none(),
        "failure cleared"
    );
}
