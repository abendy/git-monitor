//! Integration tests: destructive commands from menus ask before running.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use git_monitor::App;

const RESET_ALIAS: &str = "reset --hard HEAD~1";

/// Repo with three empty commits and a local `undo` alias that resets
fn repo_with_reset_alias() -> TempDir {
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
    for i in 0..3 {
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

    // Write only to this temp repo's own config file
    let mut config = git2::Config::open(&repo.path().join("config")).expect("open repo config");
    config
        .set_str("alias.undo", RESET_ALIAS)
        .expect("set alias");

    dir
}

fn head(dir: &TempDir) -> git2::Oid {
    git2::Repository::open(dir.path())
        .expect("open repo")
        .head()
        .expect("head")
        .target()
        .expect("head target")
}

fn press(app: &mut App, code: KeyCode) {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn menu_title(app: &App) -> Option<String> {
    app.menu_stack
        .current()
        .map(|menu| menu.title().to_string())
}

/// Jump to the newest commit, open the action menu, and pick the reset alias
fn pick_reset_alias(app: &mut App) {
    press(app, KeyCode::Char('h'));
    press(app, KeyCode::Char('m'));

    let index = app
        .menu_stack
        .current()
        .expect("action menu open")
        .items()
        .iter()
        .position(|item| item.label.contains(RESET_ALIAS))
        .expect("reset alias offered on a commit");
    for _ in 0..index {
        press(app, KeyCode::Char('j'));
    }
    press(app, KeyCode::Enter);
}

fn screen(app: &mut App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 40)).expect("terminal");
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
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn reset_alias_asks_first_and_cancel_keeps_head() {
    let dir = repo_with_reset_alias();
    let before = head(&dir);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");

    pick_reset_alias(&mut app);

    assert_eq!(
        menu_title(&app).as_deref(),
        Some(" Confirm "),
        "confirm dialog shown"
    );
    assert_eq!(head(&dir), before, "nothing ran yet");

    let screen = screen(&mut app);
    assert!(
        screen.contains(&format!("git {RESET_ALIAS}")),
        "dialog shows the exact command:\n{screen}"
    );
    assert!(
        screen.contains("not used by this command"),
        "dialog warns the selected commit is not the target:\n{screen}"
    );

    press(&mut app, KeyCode::Esc);

    assert!(
        menu_title(&app).is_none(),
        "dialog closed"
    );
    assert_eq!(head(&dir), before, "cancel keeps HEAD");
}

#[test]
fn reset_alias_runs_after_confirming() {
    let dir = repo_with_reset_alias();
    let before = head(&dir);
    let mut app = App::new(dir.path().to_path_buf()).expect("create app");

    pick_reset_alias(&mut app);
    press(&mut app, KeyCode::Char('y'));

    let repo = git2::Repository::open(dir.path()).expect("open repo");
    let parent = repo
        .find_commit(before)
        .expect("old head")
        .parent_id(0)
        .expect("parent");
    assert_eq!(
        head(&dir),
        parent,
        "reset ran once confirmed"
    );
    assert!(
        menu_title(&app).is_none(),
        "dialog closed"
    );
}
