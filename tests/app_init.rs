//! Integration tests for App initialization and state management.

use std::path::PathBuf;

use git_monitor::App;
use tempfile::TempDir;

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

mod app_new {
    use super::*;

    #[test]
    fn creates_app_for_valid_repo() {
        let dir = create_test_repo();

        let result = App::new(dir.path().to_path_buf());

        assert!(result.is_ok());
    }

    #[test]
    fn fails_for_non_repo_directory() {
        let dir = TempDir::new().expect("create temp dir");
        // Don't init as git repo

        let result = App::new(dir.path().to_path_buf());

        assert!(result.is_err());
    }

    #[test]
    fn fails_for_nonexistent_path() {
        let path = PathBuf::from("/nonexistent/path/to/repo");

        let result = App::new(path);

        assert!(result.is_err());
    }

    #[test]
    fn discovers_repo_from_subdirectory() {
        let dir = create_test_repo();
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).expect("create subdir");

        let result = App::new(subdir);

        assert!(result.is_ok());
    }
}

mod app_state {
    use super::*;

    #[test]
    fn starts_running() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(app.running);
    }

    #[test]
    fn has_empty_command_input() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(app.command_input.is_empty());
    }

    #[test]
    fn starts_with_initial_selection() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        // selected can be None or Some(0) depending on initialization
        assert!(app.selected.is_none() || app.selected == Some(0));
    }

    #[test]
    fn popup_starts_closed() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(!app.feedback.popup.is_open());
    }

    #[test]
    fn menu_starts_inactive() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(!app.menu_stack.is_active());
    }

    #[test]
    fn help_starts_hidden() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(!app.show_help);
    }
}

mod app_git_status {
    use super::*;

    #[test]
    fn loads_branch_name() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        // Default branch should be master or main
        let branch = app.status.branch.as_deref();
        assert!(
            branch == Some("master") || branch == Some("main"),
            "Expected master or main, got: {branch:?}"
        );
    }

    #[test]
    fn clean_repo_has_no_changes() {
        let dir = create_test_repo();
        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(app.status.files.is_empty());
    }

    #[test]
    fn detects_untracked_file() {
        let dir = create_test_repo();

        // Create an untracked file
        std::fs::write(dir.path().join("untracked.txt"), "content").expect("write file");

        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(!app.status.files.is_empty());
        assert!(app
            .status
            .files
            .iter()
            .any(|f| f.path.to_string_lossy() == "untracked.txt"));
    }

    #[test]
    fn detects_modified_file() {
        let dir = create_test_repo();

        // Create and commit a file
        let file_path = dir.path().join("file.txt");
        std::fs::write(&file_path, "initial").expect("write file");

        let repo = git2::Repository::open(dir.path()).expect("open repo");
        let mut index = repo.index().expect("index");
        index.add_path(std::path::Path::new("file.txt")).expect("add");
        index.write().expect("write index");

        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let parent = repo.head().expect("head").peel_to_commit().expect("commit");
        let sig = git2::Signature::now("Test", "test@example.com").expect("sig");
        repo.commit(Some("HEAD"), &sig, &sig, "Add file", &tree, &[&parent])
            .expect("commit");

        // Modify the file
        std::fs::write(&file_path, "modified").expect("modify file");

        let app = App::new(dir.path().to_path_buf()).expect("create app");

        assert!(!app.status.files.is_empty());
    }
}
