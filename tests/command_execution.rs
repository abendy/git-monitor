//! Integration tests for command execution.

use std::path::Path;

use git_monitor::command::{CommandExecutor, CommandRequest};
use tempfile::TempDir;

/// Helper to create a test git repository
fn create_test_repo() -> TempDir {
    let dir = TempDir::new().expect("create temp dir");

    // Initialize git repo
    {
        let repo = git2::Repository::init(dir.path()).expect("init repo");
        let sig = git2::Signature::now("Test", "test@example.com").expect("signature");
        let tree_id = repo.index().expect("index").write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("commit");
    }

    dir
}

mod executor_basic {
    use super::*;

    #[test]
    fn executes_simple_command() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::git(["status"]);
        let result = executor.execute(&request);

        assert!(result.success);
    }

    #[test]
    fn captures_stdout() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::git(["status", "-sb"]);
        let result = executor.execute(&request);

        assert!(result.success);
        // Short status should contain branch info
        assert!(result.stdout.contains("##"));
    }

    #[test]
    fn reports_failure_for_invalid_command() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::git(["invalid-command-that-does-not-exist"]);
        let result = executor.execute(&request);

        assert!(!result.success);
    }

    #[test]
    fn captures_error_output() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        // Attempt to checkout nonexistent branch
        let request = CommandRequest::git(["checkout", "nonexistent-branch"]);
        let result = executor.execute(&request);

        assert!(!result.success);
        // Error message should be in stderr
        assert!(!result.stderr.is_empty());
    }

    #[test]
    fn tracks_execution_duration() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::git(["status"]);
        let result = executor.execute(&request);

        // Duration should be positive
        assert!(!result.duration.is_zero());
    }

    #[test]
    fn handles_nonexistent_program() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::from_input("nonexistent-program-xyz123").unwrap();
        let result = executor.execute(&request);

        assert!(!result.success);
        assert!(result.exit_code.is_none());
        assert!(result.stderr.contains("Failed to execute"));
    }
}

mod executor_git_operations {
    use super::*;

    #[test]
    fn git_log_returns_commits() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::git(["log", "--oneline"]);
        let result = executor.execute(&request);

        assert!(result.success);
        assert!(result.stdout.contains("Initial commit"));
    }

    #[test]
    fn git_branch_lists_branches() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::git(["branch"]);
        let result = executor.execute(&request);

        assert!(result.success);
        // Should have at least one branch (master or main)
        assert!(!result.stdout.is_empty());
    }

    #[test]
    fn git_diff_on_clean_repo() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::git(["diff"]);
        let result = executor.execute(&request);

        assert!(result.success);
        // Clean repo should have no diff
        assert!(result.stdout.is_empty());
    }

    #[test]
    fn git_diff_shows_changes() {
        let dir = create_test_repo();

        // Create a tracked file
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "initial").expect("write");

        {
            let repo = git2::Repository::open(dir.path()).expect("open");
            let mut index = repo.index().expect("index");
            index.add_path(Path::new("test.txt")).expect("add");
            index.write().expect("write");
            let tree_id = index.write_tree().expect("tree");
            let tree = repo.find_tree(tree_id).expect("find tree");
            let parent = repo.head().expect("head").peel_to_commit().expect("commit");
            let sig = git2::Signature::now("Test", "test@example.com").expect("sig");
            repo.commit(Some("HEAD"), &sig, &sig, "Add test file", &tree, &[&parent])
                .expect("commit");
        }

        // Modify the file
        std::fs::write(&file_path, "modified").expect("modify");

        let executor = CommandExecutor::new(dir.path());
        let request = CommandRequest::git(["diff"]);
        let result = executor.execute(&request);

        assert!(result.success);
        assert!(result.stdout.contains("modified"));
    }

    #[test]
    fn git_add_stages_file() {
        let dir = create_test_repo();

        // Create a new file
        std::fs::write(dir.path().join("new.txt"), "content").expect("write");

        let executor = CommandExecutor::new(dir.path());
        let add_request = CommandRequest::git(["add", "new.txt"]);
        let add_result = executor.execute(&add_request);

        assert!(add_result.success);

        // Verify file is staged
        let status_request = CommandRequest::git(["status", "--porcelain"]);
        let status_result = executor.execute(&status_request);

        assert!(status_result.stdout.contains("A  new.txt"));
    }

    #[test]
    fn git_reset_unstages_file() {
        let dir = create_test_repo();

        // Create and stage a new file
        std::fs::write(dir.path().join("new.txt"), "content").expect("write");

        let executor = CommandExecutor::new(dir.path());
        executor.execute(&CommandRequest::git(["add", "new.txt"]));

        // Reset to unstage
        let reset_request = CommandRequest::git(["reset", "HEAD", "new.txt"]);
        let reset_result = executor.execute(&reset_request);

        assert!(reset_result.success);

        // Verify file is unstaged (should be untracked)
        let status_request = CommandRequest::git(["status", "--porcelain"]);
        let status_result = executor.execute(&status_request);

        assert!(status_result.stdout.contains("?? new.txt"));
    }
}

mod command_result {
    use super::*;

    #[test]
    fn display_output_shows_stderr_for_errors() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::git(["checkout", "nonexistent"]);
        let result = executor.execute(&request);

        // For failed git commands, error goes to stderr
        let output = result.display_output();
        assert!(!output.is_empty());
    }

    #[test]
    fn line_count_matches_output() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::git(["log", "--oneline"]);
        let result = executor.execute(&request);

        let expected_lines = result.display_output().lines().count();
        assert_eq!(result.line_count(), expected_lines);
    }
}

mod custom_working_directory {
    use super::*;

    #[test]
    fn uses_request_cwd_when_specified() {
        let dir = create_test_repo();

        // Create a subdirectory
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).expect("create subdir");

        // Executor has default cwd as repo root
        let executor = CommandExecutor::new(dir.path());

        // But request specifies subdir
        let request = CommandRequest::from_input("pwd").unwrap().with_cwd(subdir.clone());
        let result = executor.execute(&request);

        assert!(result.success);
        // pwd output should end with "subdir"
        assert!(result.stdout.trim().ends_with("subdir"));
    }

    #[test]
    fn uses_default_cwd_when_not_specified() {
        let dir = create_test_repo();
        let executor = CommandExecutor::new(dir.path());

        let request = CommandRequest::from_input("pwd").unwrap();
        let result = executor.execute(&request);

        assert!(result.success);
        // Should be the repo root, not subdir
        assert!(!result.stdout.trim().ends_with("subdir"));
    }
}
