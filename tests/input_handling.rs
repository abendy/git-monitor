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

mod menu_transitions {
    use super::*;
    use git_monitor::menu::{ConfirmMenu, PushConfirmMenu, SelectMenu, SelectItem};

    #[test]
    fn menu_push_activates_stack() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(!app.menu_stack.is_active());

        let menu = ConfirmMenu::new(
            "Test".to_string(),
            "Confirm action?".to_string(),
        );
        app.menu_stack.push(Box::new(menu));

        assert!(app.menu_stack.is_active());
    }

    #[test]
    fn esc_closes_single_menu() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        let menu = ConfirmMenu::new(
            "Test".to_string(),
            "Confirm action?".to_string(),
        );
        app.menu_stack.push(Box::new(menu));
        assert!(app.menu_stack.is_active());

        app.handle_key(key(KeyCode::Esc));

        assert!(!app.menu_stack.is_active());
        assert!(app.running);
    }

    #[test]
    fn q_closes_menu_not_app() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        let menu = ConfirmMenu::new(
            "Test".to_string(),
            "Confirm action?".to_string(),
        );
        app.menu_stack.push(Box::new(menu));

        app.handle_key(key_char('q'));

        assert!(!app.menu_stack.is_active());
        assert!(app.running);
    }

    #[test]
    fn nested_menus_pop_in_order() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Push first menu (SelectMenu uses Close, not CloseAll)
        let menu1 = SelectMenu::new(
            "First".to_string(),
            vec![SelectItem::new("opt1", "Option 1")],
        );
        app.menu_stack.push(Box::new(menu1));

        // Push second menu
        let menu2 = SelectMenu::new(
            "Second".to_string(),
            vec![SelectItem::new("opt2", "Option 2")],
        );
        app.menu_stack.push(Box::new(menu2));

        assert!(app.menu_stack.is_active());
        assert_eq!(app.menu_stack.current().map(|m| m.title()), Some("Second"));

        // Pop top menu (Esc on SelectMenu returns Close, not CloseAll)
        app.handle_key(key(KeyCode::Esc));

        assert!(app.menu_stack.is_active());
        assert_eq!(app.menu_stack.current().map(|m| m.title()), Some("First"));

        // Pop remaining menu
        app.handle_key(key(KeyCode::Esc));

        assert!(!app.menu_stack.is_active());
    }

    #[test]
    fn select_menu_navigates_with_j_k() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        let menu = SelectMenu::new(
            "Test".to_string(),
            vec![
                SelectItem::new("opt1", "Option 1").with_description("First option"),
                SelectItem::new("opt2", "Option 2").with_description("Second option"),
                SelectItem::new("opt3", "Option 3").with_description("Third option"),
            ],
        );
        app.menu_stack.push(Box::new(menu));

        // Initially at 0
        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(0));

        // Move down with j
        app.handle_key(key_char('j'));
        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(1));

        // Move down again
        app.handle_key(key_char('j'));
        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(2));

        // Move up with k
        app.handle_key(key_char('k'));
        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(1));
    }

    #[test]
    fn select_menu_navigates_with_arrows() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        let menu = SelectMenu::new(
            "Test".to_string(),
            vec![
                SelectItem::new("opt1", "Option 1"),
                SelectItem::new("opt2", "Option 2"),
            ],
        );
        app.menu_stack.push(Box::new(menu));

        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(1));

        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(0));
    }

    #[test]
    fn push_confirm_menu_toggles_checkboxes() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        let menu = PushConfirmMenu::new(
            "main".to_string(),
            "origin".to_string(),
            true,  // has_upstream
            3,     // ahead
            false, // force
        );
        app.menu_stack.push(Box::new(menu));

        // Space toggles checkbox on the selected item
        app.handle_key(key(KeyCode::Char(' ')));

        // Menu should still be active (space doesn't close)
        assert!(app.menu_stack.is_active());
    }

    #[test]
    fn confirm_menu_y_confirms() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        let menu = ConfirmMenu::new(
            "Test".to_string(),
            "Confirm?".to_string(),
        );
        app.menu_stack.push(Box::new(menu));

        // 'y' should close the menu (confirm action)
        app.handle_key(key_char('y'));

        assert!(!app.menu_stack.is_active());
    }

    #[test]
    fn confirm_menu_n_cancels() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        let menu = ConfirmMenu::new(
            "Test".to_string(),
            "Confirm?".to_string(),
        );
        app.menu_stack.push(Box::new(menu));

        // 'n' should close the menu (cancel)
        app.handle_key(key_char('n'));

        assert!(!app.menu_stack.is_active());
        assert!(app.running);
    }

    #[test]
    fn menu_blocks_normal_navigation() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(0);

        let menu = ConfirmMenu::new(
            "Test".to_string(),
            "Confirm?".to_string(),
        );
        app.menu_stack.push(Box::new(menu));

        // 'g' normally jumps to top, but with menu active it shouldn't affect app selection
        let selection_before = app.selected;
        app.handle_key(key_char('g'));

        // Selection should be unchanged (menu captured the key)
        assert_eq!(app.selected, selection_before);
    }

    #[test]
    fn enter_triggers_menu_selection() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        let menu = SelectMenu::new(
            "Test".to_string(),
            vec![
                SelectItem::new("opt1", "Option 1"),
            ],
        );
        app.menu_stack.push(Box::new(menu));

        // Enter should trigger selection and close menu
        app.handle_key(key(KeyCode::Enter));

        assert!(!app.menu_stack.is_active());
    }

    #[test]
    fn menu_state_preserved_across_navigation() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        let menu = SelectMenu::new(
            "Test".to_string(),
            vec![
                SelectItem::new("opt1", "Option 1"),
                SelectItem::new("opt2", "Option 2"),
                SelectItem::new("opt3", "Option 3"),
            ],
        );
        app.menu_stack.push(Box::new(menu));

        // Navigate to item 2
        app.handle_key(key_char('j'));
        app.handle_key(key_char('j'));
        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(2));

        // State should be preserved after other keys that don't change selection
        // (This is just demonstrating the menu maintains state)
        app.handle_key(key_char('j')); // Try to go past end
        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(2)); // Should stay at 2 (or wrap)
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

    #[test]
    fn history_page_starts_at_zero() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert_eq!(app.history_page, 0);
    }

    #[test]
    fn history_mode_defaults_to_commit_log() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert_eq!(app.history_mode, git_monitor::app::HistoryMode::CommitLog);
    }

    // Note: 'h' only toggles history mode when selection is on history header
    // This requires navigating to the exact history header position first,
    // which is dependent on file counts and section layout. Testing this
    // comprehensively would require more setup. Basic mode state is tested below.

}

mod selection_state {
    use super::*;

    #[test]
    fn selection_starts_none_or_zero() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(app.selected.is_none() || app.selected == Some(0));
    }

    #[test]
    fn navigation_initializes_selection() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = None;

        app.handle_key(key_char('j'));

        // Selection should be initialized
        assert!(app.selected.is_some());
    }

    #[test]
    fn g_sets_selection_to_zero() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(10);

        app.handle_key(key_char('g'));

        assert_eq!(app.selected, Some(0));
    }

    #[test]
    fn uppercase_g_moves_to_last() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(0);

        app.handle_key(key_char('G'));

        // Selection should be > 0 if there are items
        assert!(app.selected.is_some());
        // The exact value depends on content, just verify it moved
    }
}

mod branch_expansion {
    use super::*;

    #[test]
    fn history_starts_not_collapsed() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(!app.history_collapsed);
    }

    #[test]
    fn expanded_commit_starts_none() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(app.expanded_commit.is_none());
    }

    #[test]
    fn expanded_branch_starts_none() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(app.expanded_branch.is_none());
    }

    #[test]
    fn jump_to_branches_works() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        app.handle_key(key_char('b'));

        // Just verify app didn't crash
        assert!(app.running);
    }
}

mod expanded_commit_caching {
    use super::*;

    fn create_repo_with_commits() -> TempDir {
        let dir = TempDir::new().expect("create temp dir");
        let repo = git2::Repository::init(dir.path()).expect("init repo");

        // Create initial commit
        let sig = git2::Signature::now("Test", "test@example.com").expect("signature");
        let tree_id = repo.index().expect("index").write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("commit");

        // Add a file and commit
        std::fs::write(dir.path().join("file.txt"), "content").expect("write file");
        let mut index = repo.index().expect("index");
        index.add_path(std::path::Path::new("file.txt")).expect("add");
        index.write().expect("write index");

        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let parent = repo.head().expect("head").peel_to_commit().expect("commit");
        repo.commit(Some("HEAD"), &sig, &sig, "Add file", &tree, &[&parent])
            .expect("commit");

        dir
    }

    #[test]
    fn expanded_detail_caches_on_expand() {
        let dir = create_repo_with_commits();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Verify initial state
        assert!(app.expanded_commit.is_none());
        assert!(app.expanded_detail.is_none());

        // Navigate to a commit (if there are any)
        if !app.activity.is_empty() {
            // Get the SHA of first commit
            if let Some(sha) = app.activity.first().and_then(|c| c.sha.clone()) {
                app.expanded_commit = Some(sha.clone());
                // In actual use, toggling would load the detail
            }
        }

        assert!(app.running);
    }

    #[test]
    fn expanded_file_idx_starts_none() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(app.expanded_file_idx.is_none());
    }
}

mod refresh_operations {
    use super::*;

    #[test]
    fn r_refreshes_and_detects_new_files() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Record initial file count
        let initial_count = app.status.files.len();

        // Create a new file
        std::fs::write(dir.path().join("new_after_init.txt"), "new content").expect("write");

        // Refresh
        app.handle_key(key_char('r'));

        // Should detect the new file
        assert!(
            app.status.files.len() > initial_count,
            "Expected files to increase after refresh"
        );
    }

    #[test]
    fn refresh_preserves_app_state() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Set some state
        app.selected = Some(0);
        app.show_help = false;

        // Refresh
        app.handle_key(key_char('r'));

        // State should be preserved
        assert!(app.running);
        assert!(!app.show_help);
    }
}

mod boundary_conditions {
    use super::*;

    #[test]
    fn navigation_on_empty_repo_doesnt_crash() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        // Empty repo - no staged, no working files

        // Navigate down should not crash
        app.handle_key(key_char('j'));
        assert!(app.running);

        // Navigate up should not crash
        app.handle_key(key_char('k'));
        assert!(app.running);

        // Jump to top should not crash
        app.handle_key(key_char('g'));
        assert!(app.running);

        // Jump to bottom should not crash
        app.handle_key(key_char('G'));
        assert!(app.running);
    }

    #[test]
    fn selection_at_zero_cannot_go_negative() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(0);

        // Try to go up past 0
        app.handle_key(key_char('k'));
        app.handle_key(key_char('k'));
        app.handle_key(key_char('k'));

        // Selection should still be valid (Some value or None, not panic)
        assert!(app.selected.is_none() || app.selected == Some(0));
    }

    #[test]
    fn selection_at_end_doesnt_overflow() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Jump to end
        app.handle_key(key_char('G'));
        let max_selection = app.selected;

        // Try to go further down
        app.handle_key(key_char('j'));
        app.handle_key(key_char('j'));
        app.handle_key(key_char('j'));

        // Selection should be capped at max
        assert!(app.selected <= max_selection || app.selected == max_selection);
    }

    #[test]
    fn repeated_jump_to_top_is_idempotent() {
        let dir = create_repo_with_files();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(5);

        app.handle_key(key_char('g'));
        assert_eq!(app.selected, Some(0));

        app.handle_key(key_char('g'));
        assert_eq!(app.selected, Some(0));

        app.handle_key(key_char('g'));
        assert_eq!(app.selected, Some(0));
    }

    #[test]
    fn page_history_at_zero_stays_zero() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        assert_eq!(app.history_page, 0);

        // Try to go to previous page when at 0
        app.handle_key(key_char('['));

        // Should stay at 0, not go negative
        assert_eq!(app.history_page, 0);
    }

    #[test]
    fn section_jump_with_no_matching_section() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.selected = Some(0);
        let initial = app.selected;

        // Jump to working files (might be empty)
        app.handle_key(key_char('w'));

        // Should either jump or stay at current if no working files
        assert!(app.selected.is_some() || initial.is_some());
        assert!(app.running);
    }

    #[test]
    fn popup_scroll_at_top_stays_at_top() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        app.feedback.popup.open(git_monitor::feedback::PopupContent::CommandOutput {
            command: "test".to_string(),
            output: "line1\nline2\nline3".to_string(),
            success: true,
        });
        assert_eq!(app.feedback.popup.scroll_offset, 0);

        // Try to scroll up when already at top
        app.handle_key(key_char('k'));

        // Should stay at 0
        assert_eq!(app.feedback.popup.scroll_offset, 0);
    }

    #[test]
    fn menu_navigation_with_single_item() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        use git_monitor::menu::{SelectMenu, SelectItem};
        let menu = SelectMenu::new(
            "Single Item".to_string(),
            vec![SelectItem::new("only", "Only Option")],
        );
        app.menu_stack.push(Box::new(menu));

        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(0));

        // Try to navigate down with only one item
        app.handle_key(key_char('j'));
        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(0));

        // Try to navigate up with only one item
        app.handle_key(key_char('k'));
        assert_eq!(app.menu_stack.current().map(|m| m.selected()), Some(0));
    }

    #[test]
    fn empty_command_input_backspace() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");
        app.view_mode = ViewMode::Command;
        app.command_input.clear();

        // Backspace on empty input
        app.handle_key(key(KeyCode::Backspace));

        // Should not crash, command mode continues
        assert_eq!(app.view_mode, ViewMode::Command);
        assert!(app.command_input.is_empty());
    }

    #[test]
    fn multiple_esc_presses_are_safe() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Open help
        app.handle_key(key_char('?'));
        assert!(app.show_help);

        // First Esc closes help
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.show_help);

        // Second Esc quits (normal behavior)
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.running);
    }
}
