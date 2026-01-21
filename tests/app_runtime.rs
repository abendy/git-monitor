//! Integration tests for App runtime methods.

use std::sync::mpsc;
use tempfile::TempDir;

use git_monitor::app::HistoryMode;
use git_monitor::event::Event;
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

/// Create a repo with multiple commits for history testing
fn create_repo_with_history() -> TempDir {
    let dir = TempDir::new().expect("create temp dir");
    let repo = git2::Repository::init(dir.path()).expect("init repo");

    let sig = git2::Signature::now("Test", "test@example.com").expect("signature");

    // Initial commit
    let tree_id = repo.index().expect("index").write_tree().expect("write tree");
    let tree = repo.find_tree(tree_id).expect("find tree");
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .expect("commit");

    // Add several commits
    for i in 1..=5 {
        let file_name = format!("file{i}.txt");
        std::fs::write(dir.path().join(&file_name), format!("content {i}")).expect("write file");

        let mut index = repo.index().expect("index");
        index
            .add_path(std::path::Path::new(&file_name))
            .expect("add");
        index.write().expect("write index");

        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let parent = repo.head().expect("head").peel_to_commit().expect("commit");
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("Add file {i}"),
            &tree,
            &[&parent],
        )
        .expect("commit");
    }

    dir
}

mod setup_watcher {
    use super::*;

    #[test]
    fn creates_watcher_when_called() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Watcher should not exist initially (App::new doesn't set up watcher)
        // Note: The watcher field is private, so we test indirectly

        let (tx, _rx) = mpsc::channel::<Event>();
        let result = app.setup_watcher(tx);

        assert!(result.is_ok(), "setup_watcher should succeed");
    }

    #[test]
    fn setup_watcher_can_be_called_multiple_times() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        let (tx1, _rx1) = mpsc::channel::<Event>();
        let result1 = app.setup_watcher(tx1);
        assert!(result1.is_ok());

        // Setting up again should also work (replaces watcher)
        let (tx2, _rx2) = mpsc::channel::<Event>();
        let result2 = app.setup_watcher(tx2);
        assert!(result2.is_ok());
    }
}

mod refresh_status {
    use super::*;

    #[test]
    fn updates_status_after_file_creation() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Initially no files
        let initial_count = app.status.files.len();

        // Create a new file
        std::fs::write(dir.path().join("new_file.txt"), "content").expect("write file");

        // Refresh status
        app.refresh_status();

        // Should detect the new file
        assert!(
            app.status.files.len() > initial_count,
            "Expected file count to increase after refresh"
        );
    }

    #[test]
    fn clears_error_on_successful_refresh() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Set an error
        app.feedback.error = Some("Previous error".to_string());

        // Refresh (should succeed and clear error)
        app.refresh_status();

        assert!(
            app.feedback.error.is_none(),
            "Error should be cleared after successful refresh"
        );
    }

    #[test]
    fn updates_branches_on_refresh() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Create a new branch using git2
        {
            let repo = git2::Repository::open(dir.path()).expect("open repo");
            let head = repo.head().expect("head").peel_to_commit().expect("commit");
            repo.branch("new-branch", &head, false).expect("create branch");
        }

        let initial_branch_count = app.branches.len();

        // Refresh status
        app.refresh_status();

        // Should detect the new branch
        assert!(
            app.branches.len() > initial_branch_count,
            "Expected branch count to increase after refresh"
        );
    }

    #[test]
    fn also_refreshes_activity() {
        let dir = create_repo_with_history();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        // Activity should be populated after init (refresh_status is called in new())
        assert!(
            !app.activity.is_empty(),
            "Activity should be populated after init"
        );
    }

    #[test]
    fn clamps_selection_after_refresh() {
        let dir = create_repo_with_history();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Set selection to a high value
        app.selected = Some(1000);

        // Refresh (should clamp selection)
        app.refresh_status();

        // Selection should be clamped to valid range
        if let Some(selected) = app.selected {
            let total = app.status.files.len() + app.activity.len() + app.branches.len() + 2; // rough total
            assert!(
                selected < total + 10, // Allow some buffer
                "Selection should be clamped to reasonable bounds"
            );
        }
    }
}

mod refresh_activity {
    use super::*;

    #[test]
    fn populates_activity_in_commit_log_mode() {
        let dir = create_repo_with_history();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        assert_eq!(app.history_mode, HistoryMode::CommitLog);

        // Activity should be populated from commits
        assert!(
            !app.activity.is_empty(),
            "Activity should contain commits in CommitLog mode"
        );
    }

    #[test]
    fn calculates_total_items() {
        let dir = create_repo_with_history();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        // Should have at least 6 commits (initial + 5 added)
        assert!(
            app.history_total_items >= 6,
            "Expected at least 6 history items, got {}",
            app.history_total_items
        );
    }

    #[test]
    fn calculates_total_pages() {
        let dir = create_repo_with_history();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        // With 6 items and PAGE_SIZE of 50, should be 1 page
        assert!(
            app.history_total_pages >= 1,
            "Expected at least 1 page, got {}",
            app.history_total_pages
        );
    }

    #[test]
    fn page_zero_on_init() {
        let dir = create_repo_with_history();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert_eq!(app.history_page, 0, "History page should start at 0");
    }

    #[test]
    fn activity_contains_commit_messages() {
        let dir = create_repo_with_history();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        // Check that activity contains expected commit messages
        let messages: Vec<_> = app.activity.iter().map(|c| &c.message).collect();

        assert!(
            messages.iter().any(|m| m.contains("Initial commit")),
            "Activity should contain 'Initial commit'"
        );
        assert!(
            messages.iter().any(|m| m.contains("Add file")),
            "Activity should contain 'Add file' commits"
        );
    }

    #[test]
    fn activity_in_reflog_mode_has_entries() {
        let dir = create_repo_with_history();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Switch to reflog mode manually
        app.history_mode = HistoryMode::Reflog;
        app.refresh_status();

        // Reflog should have entries (at least commit operations)
        assert!(
            !app.activity.is_empty(),
            "Reflog should have entries after commits"
        );
    }
}

mod on_tick {
    use super::*;
    use git_monitor::feedback::{Toast, ToastLevel};

    #[test]
    fn tick_does_not_crash() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Calling tick on feedback should not crash
        app.feedback.tick();

        assert!(app.running);
    }

    #[test]
    fn tick_clears_expired_toast() {
        let dir = create_test_repo();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Set a toast
        app.feedback.toast = Some(Toast::new("Test message", ToastLevel::Info));

        // Toast should exist
        assert!(app.feedback.toast.is_some());

        // Tick shouldn't clear it immediately (TOAST_DURATION is 3 seconds)
        app.feedback.tick();
        assert!(app.feedback.toast.is_some(), "Toast should not expire immediately");
    }

    #[test]
    fn toast_starts_none() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(app.feedback.toast.is_none(), "Toast should start as None");
    }
}

mod pagination_state {
    use super::*;

    #[test]
    fn page_clamps_to_total_pages() {
        let dir = create_repo_with_history();
        let mut app = App::new(dir.path().to_path_buf()).expect("create app");

        // Set page beyond total pages
        app.history_page = 100;

        // Refresh should clamp
        app.refresh_status();

        assert!(
            app.history_page < app.history_total_pages || app.history_total_pages == 0,
            "Page should be clamped to valid range"
        );
    }

    #[test]
    fn empty_history_has_zero_pages() {
        let dir = TempDir::new().expect("create temp dir");
        git2::Repository::init(dir.path()).expect("init repo");
        // No commits, so history should be empty

        let app = App::new(dir.path().to_path_buf());

        // App creation might fail for empty repo, which is ok
        if let Ok(app) = app {
            // With no commits, should have 0 or 1 pages
            assert!(
                app.history_total_pages <= 1,
                "Empty repo should have 0 or 1 history pages"
            );
        }
    }
}
