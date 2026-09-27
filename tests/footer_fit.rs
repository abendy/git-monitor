//! Integration test: the footer fits narrow panels.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use git_monitor::App;

fn footer_row(width: u16) -> String {
    let dir = TempDir::new().expect("create temp dir");
    git2::Repository::init(dir.path()).expect("init repo");
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
fn sidebar_width_footer_shows_help_and_quit() {
    let row = footer_row(68);

    assert!(
        row.contains("help"),
        "help visible: {row}"
    );
    assert!(
        row.contains("quit"),
        "quit visible: {row}"
    );
    assert!(
        row.trim_end().ends_with('│'),
        "nothing cut at the border: {row}"
    );
}

#[test]
fn wide_footer_shows_every_navigation_hint() {
    let row = footer_row(140);

    for label in [
        "nav", "top/btm", "menu", "cmd", "files", "history", "branches", "help", "quit",
    ] {
        assert!(
            row.contains(label),
            "{label} visible: {row}"
        );
    }
}
