//! Integration tests for git workflow operations.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::path::Path;

use git_monitor::git::{FileState, GitRepo};
use tempfile::TempDir;

/// Helper to create a test git repository with an initial commit
fn create_test_repo() -> (TempDir, GitRepo) {
    let dir = TempDir::new().expect("create temp dir");

    // Create the repository and initial commit in a separate scope
    {
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
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        )
        .expect("commit");
    }

    let git_repo = GitRepo::open(dir.path()).expect("open GitRepo");
    (dir, git_repo)
}

mod stage_unstage_workflow {
    use super::*;

    #[test]
    fn stage_new_file() {
        let (dir, repo) = create_test_repo();

        // Create a new file
        std::fs::write(dir.path().join("new.txt"), "content").expect("write");

        // Stage it
        repo.stage(Path::new("new.txt"))
            .expect("stage");

        // Verify it's staged
        let status = repo.status().expect("status");
        let file = status
            .files
            .iter()
            .find(|f| f.path == Path::new("new.txt"));
        assert!(file.is_some());
        assert_eq!(file.unwrap().staged, FileState::Added);
    }

    #[test]
    fn unstage_staged_file() {
        let (dir, repo) = create_test_repo();

        // Create and stage a file
        std::fs::write(dir.path().join("new.txt"), "content").expect("write");
        repo.stage(Path::new("new.txt"))
            .expect("stage");

        // Unstage it
        repo.unstage(Path::new("new.txt"))
            .expect("unstage");

        // Verify it's no longer staged
        let status = repo.status().expect("status");
        let file = status
            .files
            .iter()
            .find(|f| f.path == Path::new("new.txt"));
        assert!(file.is_some());
        assert_eq!(
            file.unwrap().staged,
            FileState::Unmodified
        );
        assert_eq!(
            file.unwrap().working,
            FileState::Untracked
        );
    }

    #[test]
    fn stage_modified_file() {
        let (dir, repo) = create_test_repo();

        // Create, stage, and commit a file
        let file_path = dir.path().join("file.txt");
        std::fs::write(&file_path, "initial").expect("write");
        repo.stage(Path::new("file.txt"))
            .expect("stage");

        // Commit it using git2 directly
        {
            let git_repo = git2::Repository::open(dir.path()).expect("open");
            let mut index = git_repo.index().expect("index");
            let tree_id = index.write_tree().expect("tree");
            let tree = git_repo
                .find_tree(tree_id)
                .expect("find tree");
            let parent = git_repo
                .head()
                .expect("head")
                .peel_to_commit()
                .expect("commit");
            let sig = git2::Signature::now("Test", "test@example.com").expect("sig");
            git_repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    "Add file",
                    &tree,
                    &[&parent],
                )
                .expect("commit");
        }

        // Modify the file
        std::fs::write(&file_path, "modified").expect("modify");

        // Stage the modification
        repo.stage(Path::new("file.txt"))
            .expect("stage");

        // Verify staged modification
        let status = repo.status().expect("status");
        let file = status
            .files
            .iter()
            .find(|f| f.path == Path::new("file.txt"));
        assert!(file.is_some());
        assert_eq!(
            file.unwrap().staged,
            FileState::Modified
        );
    }
}

mod diff_workflow {
    use super::*;

    #[test]
    fn diff_working_changes() {
        let (dir, repo) = create_test_repo();

        // Create and commit a file
        let file_path = dir.path().join("file.txt");
        std::fs::write(&file_path, "line1\nline2\n").expect("write");

        {
            let git_repo = git2::Repository::open(dir.path()).expect("open");
            let mut index = git_repo.index().expect("index");
            index
                .add_path(Path::new("file.txt"))
                .expect("add");
            index.write().expect("write");
            let tree_id = index.write_tree().expect("tree");
            let tree = git_repo
                .find_tree(tree_id)
                .expect("find tree");
            let parent = git_repo
                .head()
                .expect("head")
                .peel_to_commit()
                .expect("commit");
            let sig = git2::Signature::now("Test", "test@example.com").expect("sig");
            git_repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    "Add file",
                    &tree,
                    &[&parent],
                )
                .expect("commit");
        }

        // Modify the file
        std::fs::write(&file_path, "line1\nline2\nline3\n").expect("modify");

        // Get working diff using GitRepo method
        let diff = repo
            .diff_file(Path::new("file.txt"), false)
            .expect("diff");

        assert!(diff.contains("+line3"));
    }

    #[test]
    fn diff_staged_changes() {
        let (dir, repo) = create_test_repo();

        // Create and stage a new file
        std::fs::write(
            dir.path().join("new.txt"),
            "new content\n",
        )
        .expect("write");
        repo.stage(Path::new("new.txt"))
            .expect("stage");

        // Get staged diff using GitRepo method
        let diff = repo
            .diff_file(Path::new("new.txt"), true)
            .expect("diff");

        assert!(diff.contains("+new content"));
    }
}

mod branch_workflow {
    use super::*;

    #[test]
    fn list_branches_shows_current() {
        let (_dir, repo) = create_test_repo();

        let branches = repo.list_branches().expect("branches");

        assert!(!branches.is_empty());
        assert!(branches.iter().any(|b| b.is_current));
    }

    #[test]
    fn create_and_checkout_branch() {
        let (dir, repo) = create_test_repo();

        // Create a new branch using git2
        {
            let git_repo = git2::Repository::open(dir.path()).expect("open");
            let head = git_repo
                .head()
                .expect("head")
                .peel_to_commit()
                .expect("commit");
            git_repo
                .branch("feature", &head, false)
                .expect("create branch");
        }

        // Checkout using GitRepo method
        repo.checkout_branch("feature")
            .expect("checkout");

        // Verify we're on the new branch
        let branches = repo.list_branches().expect("branches");
        let current = branches.iter().find(|b| b.is_current);
        assert!(current.is_some());
        assert_eq!(current.unwrap().name, "feature");
    }

    #[test]
    fn checkout_nonexistent_branch_fails() {
        let (_dir, repo) = create_test_repo();

        let result = repo.checkout_branch("nonexistent");

        assert!(result.is_err());
    }
}

mod history_workflow {
    use super::*;

    #[test]
    fn commit_log_returns_commits() {
        let (_dir, repo) = create_test_repo();

        let commits = repo.commit_log(0, 10).expect("log");

        assert!(!commits.is_empty());
        assert!(commits
            .iter()
            .any(|c| c.message.contains("Initial commit")));
    }

    #[test]
    fn commit_log_respects_limit() {
        let (dir, _repo) = create_test_repo();

        // Create more commits
        {
            let git_repo = git2::Repository::open(dir.path()).expect("open");
            let sig = git2::Signature::now("Test", "test@example.com").expect("sig");

            for i in 0..5 {
                let tree_id = git_repo
                    .index()
                    .expect("index")
                    .write_tree()
                    .expect("tree");
                let tree = git_repo
                    .find_tree(tree_id)
                    .expect("find tree");
                let parent = git_repo
                    .head()
                    .expect("head")
                    .peel_to_commit()
                    .expect("commit");
                git_repo
                    .commit(
                        Some("HEAD"),
                        &sig,
                        &sig,
                        &format!("Commit {i}"),
                        &tree,
                        &[&parent],
                    )
                    .expect("commit");
            }
        }

        // Reopen to get fresh GitRepo
        let repo = GitRepo::open(dir.path()).expect("reopen");
        let commits = repo.commit_log(0, 3).expect("log");

        assert_eq!(commits.len(), 3);
    }

    #[test]
    fn reflog_returns_entries() {
        let (_dir, repo) = create_test_repo();

        let entries = repo.reflog(0, 10).expect("reflog");

        assert!(!entries.is_empty());
    }
}
