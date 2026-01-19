use anyhow::{Context, Result};
use tracing::debug;

use super::{BranchInfo, GitRepo};

impl GitRepo {
    /// List local branches with their info
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        let mut branches = Vec::new();

        // Get current branch name for comparison
        let current_branch = self
            .repo
            .head()
            .map_err(|e| {
                debug!("Failed to get HEAD for branch comparison: {} (detached HEAD?)", e);
                e
            })
            .ok()
            .and_then(|h| h.shorthand().map(String::from));

        // Iterate through local branches
        let branch_iter = self
            .repo
            .branches(Some(git2::BranchType::Local))?;

        for branch_result in branch_iter {
            let (branch, _branch_type) = branch_result?;

            let name = match branch.name()? {
                Some(n) => n.to_string(),
                None => continue,
            };

            if name.is_empty() {
                continue;
            }

            let is_current = current_branch.as_ref() == Some(&name);

            branches.push(BranchInfo {
                name,
                is_current,
                is_remote: false,
            });
        }

        // Sort: current branch first, then local branches
        branches.sort_by(|a, b| {
            match (a.is_current, b.is_current) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            a.name.cmp(&b.name)
        });

        Ok(branches)
    }

    /// Checkout a branch by name
    pub fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let branch = self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)
            .with_context(|| format!("Branch '{branch_name}' not found"))?;

        let reference = branch.get();
        let oid = reference
            .target()
            .ok_or_else(|| anyhow::anyhow!("Branch has no target"))?;

        let commit = self.repo.find_commit(oid)?;

        // Check for uncommitted changes
        let statuses = self.repo.statuses(None)?;
        let has_changes = statuses.iter().any(|s| {
            let status = s.status();
            status.is_wt_modified()
                || status.is_wt_deleted()
                || status.is_index_modified()
                || status.is_index_deleted()
                || status.is_index_new()
        });

        if has_changes {
            return Err(anyhow::anyhow!(
                "Cannot checkout: you have uncommitted changes"
            ));
        }

        // Checkout the tree
        self.repo.checkout_tree(
            commit.as_object(),
            Some(git2::build::CheckoutBuilder::new().safe()),
        )?;

        // Update HEAD
        self.repo
            .set_head(&format!("refs/heads/{branch_name}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    /// Create a test repo with an initial commit on master/main
    fn create_test_repo() -> (TempDir, GitRepo) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(dir.path()).expect("Failed to init repo");

        let mut config = repo.config().expect("Failed to get config");
        config
            .set_str("user.name", "Test User")
            .expect("Failed to set user.name");
        config
            .set_str("user.email", "test@example.com")
            .expect("Failed to set user.email");
        drop(config);

        // Create initial commit
        let file_path = dir.path().join("initial.txt");
        fs::write(&file_path, "initial content\n").expect("Failed to write file");

        let mut index = repo.index().expect("Failed to get index");
        index
            .add_path(Path::new("initial.txt"))
            .expect("Failed to add file");
        index.write().expect("Failed to write index");

        let tree_id = index.write_tree().expect("Failed to write tree");
        let tree = repo.find_tree(tree_id).expect("Failed to find tree");
        let sig = repo.signature().expect("Failed to get signature");
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("Failed to commit");

        drop(tree);
        drop(repo);

        let git_repo = GitRepo::open(dir.path()).expect("Failed to open repo");
        (dir, git_repo)
    }

    /// Create additional branch in the repo
    fn create_branch(dir: &TempDir, branch_name: &str) {
        let repo = Repository::open(dir.path()).expect("Failed to open");
        let head = repo.head().expect("Failed to get HEAD");
        let commit = head.peel_to_commit().expect("Failed to get commit");
        repo.branch(branch_name, &commit, false)
            .expect("Failed to create branch");
    }

    mod list_branches {
        use super::*;

        #[test]
        fn returns_single_branch_for_new_repo() {
            let (_dir, repo) = create_test_repo();

            let branches = repo.list_branches().expect("Failed to list branches");

            assert_eq!(branches.len(), 1);
            // Default branch is either master or main
            assert!(branches[0].name == "master" || branches[0].name == "main");
            assert!(branches[0].is_current);
            assert!(!branches[0].is_remote);
        }

        #[test]
        fn lists_multiple_branches() {
            let (dir, repo) = create_test_repo();

            create_branch(&dir, "feature-a");
            create_branch(&dir, "feature-b");

            let branches = repo.list_branches().expect("Failed to list branches");

            assert_eq!(branches.len(), 3);
        }

        #[test]
        fn marks_current_branch() {
            let (dir, repo) = create_test_repo();

            create_branch(&dir, "other");

            let branches = repo.list_branches().expect("Failed to list branches");

            let current_count = branches.iter().filter(|b| b.is_current).count();
            assert_eq!(current_count, 1);
        }

        #[test]
        fn sorts_current_branch_first() {
            let (dir, repo) = create_test_repo();

            // Create branches that would sort before master/main alphabetically
            create_branch(&dir, "aaa-branch");
            create_branch(&dir, "bbb-branch");

            let branches = repo.list_branches().expect("Failed to list branches");

            // Current branch should be first regardless of name
            assert!(branches[0].is_current);
        }

        #[test]
        fn sorts_remaining_branches_alphabetically() {
            let (dir, repo) = create_test_repo();

            create_branch(&dir, "zebra");
            create_branch(&dir, "alpha");
            create_branch(&dir, "beta");

            let branches = repo.list_branches().expect("Failed to list branches");

            // Skip first (current branch), check remaining are alphabetical
            let non_current: Vec<_> =
                branches.iter().skip(1).map(|b| b.name.as_str()).collect();

            let mut sorted = non_current.clone();
            sorted.sort();
            assert_eq!(non_current, sorted);
        }
    }

    mod checkout_branch {
        use super::*;

        #[test]
        fn checkouts_existing_branch() {
            let (dir, repo) = create_test_repo();

            create_branch(&dir, "feature");

            let result = repo.checkout_branch("feature");
            assert!(result.is_ok());

            // Verify HEAD now points to feature
            let git_repo = Repository::open(dir.path()).expect("Failed to open");
            let head = git_repo.head().expect("Failed to get HEAD");
            assert_eq!(head.shorthand(), Some("feature"));
        }

        #[test]
        fn returns_error_for_nonexistent_branch() {
            let (_dir, repo) = create_test_repo();

            let result = repo.checkout_branch("nonexistent");

            assert!(result.is_err());
            let err = result.err().expect("should be error");
            assert!(err.to_string().contains("not found"));
        }

        #[test]
        fn returns_error_with_uncommitted_changes() {
            let (dir, repo) = create_test_repo();

            create_branch(&dir, "feature");

            // Create uncommitted changes
            fs::write(dir.path().join("initial.txt"), "modified\n")
                .expect("Failed to write");

            let result = repo.checkout_branch("feature");

            assert!(result.is_err());
            let err = result.err().expect("should be error");
            assert!(err.to_string().contains("uncommitted changes"));
        }

        #[test]
        fn returns_error_with_staged_changes() {
            let (dir, repo) = create_test_repo();

            create_branch(&dir, "feature");

            // Create and stage a new file
            fs::write(dir.path().join("new.txt"), "new content\n")
                .expect("Failed to write");
            repo.stage(Path::new("new.txt")).expect("Failed to stage");

            let result = repo.checkout_branch("feature");

            assert!(result.is_err());
            let err = result.err().expect("should be error");
            assert!(err.to_string().contains("uncommitted changes"));
        }
    }
}
