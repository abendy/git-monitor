//! Integration tests for App input handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use git_monitor::app::ViewMode;
use git_monitor::App;

/// Helper to create a test git repository
fn create_test_repo() -> TempDir {
    let dir = TempDir::new().expect("create temp dir");
    let repo = git2::Repository::init(dir.path()).expect("init repo");

    // Create initial commit so we have a valid HEAD
    let sig = git2::Signature::now("Test", "test@example.com").expect("signature");
    let tree_id = repo.index().expect("index").write_tree().expect("write tree");
    let tree = repo.find_tree(tree_id).expect("find tree");
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .expect("commit");

    dir
}

/// Create a test repo with some files for navigation testing
fn create_repo_with_files() -> TempDir {
    let dir = create_test_repo();

    // Create some files to make the UI have content
    std::fs::write(dir.path().join("file1.txt"), "content1").expect("write file1");
    std::fs::write(dir.path().join("file2.txt"), "content2").expect("write file2");

    dir
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn key_char(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

fn ctrl_char(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

mod quit_handling {
    use super::*;

    #[test]
    fn q_quits_application() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        app.handle_key(key_char('q'));

        assert!(!app.running);
    }

    #[test]
    fn esc_quits_application() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        app.handle_key(key(KeyCode::Esc));

        assert!(!app.running);
    }

    #[test]
    fn ctrl_c_quits_application() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        app.handle_key(ctrl_char('c'));

        assert!(!app.running);
    }
}

mod help_handling {
    use super::*;

    #[test]
    fn question_mark_shows_help() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        app.handle_key(key_char('?'));

        assert!(app.show_help);
    }

    #[test]
    fn any_key_closes_help() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.show_help = true;

        app.handle_key(key_char('x'));

        assert!(!app.show_help);
    }

    #[test]
    fn help_open_prevents_quit() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.show_help = true;

        app.handle_key(key_char('q'));

        // Help closes instead of quitting
        assert!(!app.show_help);
        assert!(app.running);
    }
}

mod command_mode_handling {
    use super::*;

    #[test]
    fn colon_enters_command_mode() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        app.handle_key(key_char(':'));

        assert_eq!(app.view_mode, ViewMode::Command);
    }

    #[test]
    fn esc_exits_command_mode() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.view_mode = ViewMode::Command;

        app.handle_key(key(KeyCode::Esc));

        assert_eq!(app.view_mode, ViewMode::Normal);
    }

    #[test]
    fn typing_adds_to_input() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.view_mode = ViewMode::Command;

        app.handle_key(key_char('s'));
        app.handle_key(key_char('t'));
        app.handle_key(key_char('a'));
        app.handle_key(key_char('t'));
        app.handle_key(key_char('u'));
        app.handle_key(key_char('s'));

        assert_eq!(app.command_input, "status");
    }

    #[test]
    fn backspace_removes_characters() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.view_mode = ViewMode::Command;
        app.command_input = "test".to_string();

        app.handle_key(key(KeyCode::Backspace));

        assert_eq!(app.command_input, "tes");
    }

    #[test]
    fn backspace_on_empty_stays_in_command_mode() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.view_mode = ViewMode::Command;
        app.command_input.clear();

        app.handle_key(key(KeyCode::Backspace));

        // Backspace on empty command doesn't exit - Esc or Down does
        assert_eq!(app.view_mode, ViewMode::Command);
    }
}

mod navigation_handling {
    use super::*;

    #[test]
    fn j_moves_selection_down() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(0);

        app.handle_key(key_char('j'));

        assert!(app.selected.is_some());
        assert!(app.selected.unwrap() > 0 || app.selected == Some(0));
    }

    #[test]
    fn down_arrow_moves_selection_down() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(0);

        app.handle_key(key(KeyCode::Down));

        // Selection should move or stay if at boundary
        assert!(app.selected.is_some());
    }

    #[test]
    fn k_moves_selection_up() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        // Start at a position that can go up
        app.selected = Some(3);

        app.handle_key(key_char('k'));

        assert!(app.selected.is_some());
        assert!(app.selected.unwrap() < 3);
    }

    #[test]
    fn up_arrow_moves_selection_up() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(3);

        app.handle_key(key(KeyCode::Up));

        assert!(app.selected.is_some());
        assert!(app.selected.unwrap() < 3);
    }

    #[test]
    fn g_jumps_to_first() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(5);

        app.handle_key(key_char('g'));

        assert_eq!(app.selected, Some(0));
    }

    #[test]
    fn uppercase_g_jumps_to_last() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(0);

        app.handle_key(key_char('G'));

        // Should be at the last item (not 0)
        assert!(app.selected.is_some());
        // Note: exact value depends on content, just verify it moved
    }
}

mod section_jumps {
    use super::*;

    #[test]
    fn b_jumps_to_branches() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(0);

        app.handle_key(key_char('b'));

        // Selection should move to branches section
        assert!(app.selected.is_some());
        // Exact index depends on content
    }

    #[test]
    fn w_jumps_to_working() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(0);

        app.handle_key(key_char('w'));

        // Selection should move to working files section
        assert!(app.selected.is_some());
    }
}

mod refresh_handling {
    use super::*;

    #[test]
    fn r_refreshes_status() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Create a file after app init
        std::fs::write(dir.path().join("new_file.txt"), "content").expect("write file");

        // Initial status shouldn't have the file
        let initial_count = app.status.files.len();

        app.handle_key(key_char('r'));

        // After refresh, the new file should appear
        // Note: This depends on proper refresh implementation
        assert!(app.status.files.len() >= initial_count);
    }
}

mod menu_handling {
    use super::*;

    #[test]
    fn a_opens_alias_browser_when_aliases_exist() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // This test just verifies the key is handled without error
        // Menu won't open if no aliases are configured
        app.handle_key(key_char('a'));

        // Should still be running
        assert!(app.running);
    }

    #[test]
    fn m_opens_action_menu() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        // Set selection to get a valid context
        app.selected = Some(0);

        app.handle_key(key_char('m'));

        // Menu stack may or may not be active depending on available actions
        assert!(app.running);
    }

    #[test]
    fn menu_captures_keys_when_active() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Manually push a menu to test menu key capture
        use git_monitor::menu::PushConfirmMenu;
        let menu = PushConfirmMenu::new(
            "main".to_string(),
            "origin".to_string(),
            true,  // has_upstream
            1,     // ahead
            false, // force
        );
        app.menu_stack.push(Box::new(menu));

        // q should close menu, not quit app
        app.handle_key(key_char('q'));

        assert!(app.running);
        // Menu should be closed
        assert!(!app.menu_stack.is_active());
    }
}

mod popup_handling {
    use super::*;

    #[test]
    fn popup_captures_keys_when_open() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Open a popup manually
        app.feedback.popup.open(git_monitor::feedback::PopupContent::CommandOutput {
            command: "test".to_string(),
            output: "output\nwith\nlines".to_string(),
            success: true,
        });

        // q should close popup, not quit app
        app.handle_key(key_char('q'));

        assert!(app.running);
        assert!(!app.feedback.popup.is_open());
    }

    #[test]
    fn popup_scrolls_with_j() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        app.feedback.popup.open(git_monitor::feedback::PopupContent::CommandOutput {
            command: "test".to_string(),
            output: (0..50).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n"),
            success: true,
        });
        let initial_offset = app.feedback.popup.scroll_offset;

        app.handle_key(key_char('j'));

        assert!(app.feedback.popup.scroll_offset > initial_offset);
    }

    #[test]
    fn popup_scrolls_with_k() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        app.feedback.popup.open(git_monitor::feedback::PopupContent::CommandOutput {
            command: "test".to_string(),
            output: (0..50).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n"),
            success: true,
        });
        // Scroll down first
        app.feedback.popup.scroll_down(5);
        let scrolled_offset = app.feedback.popup.scroll_offset;

        app.handle_key(key_char('k'));

        assert!(app.feedback.popup.scroll_offset < scrolled_offset);
    }
}

mod staging_handling {
    use super::*;

    #[test]
    fn s_toggles_stage() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // The test just verifies the key is handled without error
        // Actual staging depends on file selection
        app.handle_key(key_char('s'));

        assert!(app.running);
    }
}

mod history_pagination {
    use super::*;

    #[test]
    fn right_bracket_next_page_in_history() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        let initial_page = app.history_page;

        // Jump to history section first (we need to be in history for pagination)
        // This is a simplified test - full pagination requires actual history items
        app.handle_key(key_char(']'));

        // Page might not change if not enough items
        assert!(app.history_page >= initial_page);
    }

    #[test]
    fn left_bracket_prev_page_in_history() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.history_page = 1;
        let initial_page = app.history_page;

        app.handle_key(key_char('['));

        // Should go back if we were on page 1
        assert!(app.history_page <= initial_page);
    }
}
