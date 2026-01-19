use std::path::Path;

use anyhow::{Context, Result};
use git2::Repository;

use super::GitRepo;

impl GitRepo {
    /// Open a git repository at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::discover(path).with_context(|| {
            format!(
                "No git repository found at {}",
                path.display()
            )
        })?;

        Ok(Self { repo })
    }

    /// Get the repository root path
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// Stage a file
    pub fn stage(&self, path: &Path) -> Result<()> {
        let mut index = self
            .repo
            .index()
            .context("Failed to get index")?;
        index
            .add_path(path)
            .with_context(|| format!("Failed to stage {}", path.display()))?;
        index
            .write()
            .context("Failed to write index")?;
        Ok(())
    }

    /// Unstage a file
    pub fn unstage(&self, path: &Path) -> Result<()> {
        let head = self
            .repo
            .head()
            .context("Failed to get HEAD")?;
        let head_commit = head
            .peel_to_commit()
            .context("Failed to get HEAD commit")?;
        let head_tree = head_commit
            .tree()
            .context("Failed to get HEAD tree")?;

        self.repo
            .reset_default(Some(head_commit.as_object()), [path])
            .or_else(
                |_| -> std::result::Result<(), git2::Error> {
                    // If reset fails (file is new), remove from index
                    let mut index = self.repo.index()?;
                    index.remove_path(path)?;
                    index.write()?;
                    Ok(())
                },
            )
            .with_context(|| format!("Failed to unstage {}", path.display()))?;

        drop(head_tree);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use tempfile::TempDir;

    /// Create a temporary git repository for testing
    fn create_test_repo() -> (TempDir, GitRepo) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(dir.path()).expect("Failed to init repo");

        // Configure git user for commits
        let mut config = repo.config().expect("Failed to get config");
        config
            .set_str("user.name", "Test User")
            .expect("Failed to set user.name");
        config
            .set_str("user.email", "test@example.com")
            .expect("Failed to set user.email");

        drop(config);
        drop(repo);

        let git_repo = GitRepo::open(dir.path()).expect("Failed to open repo");
        (dir, git_repo)
    }

    /// Create a test repo with an initial commit (required for unstage)
    fn create_test_repo_with_commit() -> (TempDir, GitRepo) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(dir.path()).expect("Failed to init repo");

        // Configure git user for commits
        let mut config = repo.config().expect("Failed to get config");
        config
            .set_str("user.name", "Test User")
            .expect("Failed to set user.name");
        config
            .set_str("user.email", "test@example.com")
            .expect("Failed to set user.email");
        drop(config);

        // Create initial file and commit
        let file_path = dir.path().join("initial.txt");
        fs::write(&file_path, "initial content").expect("Failed to write file");

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

    mod open {
        use super::*;

        #[test]
        fn opens_valid_repository() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            Repository::init(dir.path()).expect("Failed to init repo");

            let result = GitRepo::open(dir.path());
            assert!(result.is_ok());
        }

        #[test]
        fn returns_error_for_non_repository() {
            let dir = TempDir::new().expect("Failed to create temp dir");

            let result = GitRepo::open(dir.path());
            assert!(result.is_err());

            let err = result.err().expect("should be error");
            assert!(err.to_string().contains("No git repository found"));
        }

        #[test]
        fn discovers_repository_from_subdirectory() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            Repository::init(dir.path()).expect("Failed to init repo");

            // Create a subdirectory
            let subdir = dir.path().join("subdir");
            fs::create_dir(&subdir).expect("Failed to create subdir");

            let result = GitRepo::open(&subdir);
            assert!(result.is_ok());
        }

        #[test]
        fn workdir_returns_repository_root() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            Repository::init(dir.path()).expect("Failed to init repo");

            let repo = GitRepo::open(dir.path()).expect("Failed to open");
            let workdir = repo.workdir();

            assert!(workdir.is_some());
            assert_eq!(
                workdir.expect("workdir should exist").canonicalize().ok(),
                dir.path().canonicalize().ok()
            );
        }
    }

    mod stage {
        use super::*;

        #[test]
        fn stages_new_file() {
            let (dir, repo) = create_test_repo();

            // Create a new file
            let file_path = dir.path().join("test.txt");
            fs::write(&file_path, "hello world").expect("Failed to write file");

            // Stage it
            let result = repo.stage(Path::new("test.txt"));
            assert!(result.is_ok());

            // Verify it's staged by checking the index
            let git_repo = Repository::open(dir.path()).expect("Failed to open");
            let index = git_repo.index().expect("Failed to get index");
            let entry = index.get_path(Path::new("test.txt"), 0);
            assert!(entry.is_some());
        }

        #[test]
        fn stages_modified_file() {
            let (dir, repo) = create_test_repo_with_commit();

            // Modify the initial file
            let file_path = dir.path().join("initial.txt");
            fs::write(&file_path, "modified content").expect("Failed to write file");

            // Stage it
            let result = repo.stage(Path::new("initial.txt"));
            assert!(result.is_ok());
        }

        #[test]
        fn returns_error_for_nonexistent_file() {
            let (_dir, repo) = create_test_repo();

            let result = repo.stage(Path::new("nonexistent.txt"));
            assert!(result.is_err());
        }
    }

    mod unstage {
        use super::*;

        #[test]
        fn unstages_newly_added_file() {
            let (dir, repo) = create_test_repo_with_commit();

            // Create and stage a new file
            let file_path = dir.path().join("new.txt");
            fs::write(&file_path, "new content").expect("Failed to write file");
            repo.stage(Path::new("new.txt"))
                .expect("Failed to stage");

            // Verify it's staged
            let git_repo = Repository::open(dir.path()).expect("Failed to open");
            let index = git_repo.index().expect("Failed to get index");
            assert!(index.get_path(Path::new("new.txt"), 0).is_some());
            drop(index);
            drop(git_repo);

            // Unstage it
            let result = repo.unstage(Path::new("new.txt"));
            assert!(result.is_ok());

            // Verify it's no longer staged
            let git_repo = Repository::open(dir.path()).expect("Failed to open");
            let index = git_repo.index().expect("Failed to get index");
            assert!(index.get_path(Path::new("new.txt"), 0).is_none());
        }

        #[test]
        fn unstages_modified_file() {
            let (dir, repo) = create_test_repo_with_commit();

            // Modify and stage the initial file
            let file_path = dir.path().join("initial.txt");
            fs::write(&file_path, "modified content").expect("Failed to write file");
            repo.stage(Path::new("initial.txt"))
                .expect("Failed to stage");

            // Unstage it
            let result = repo.unstage(Path::new("initial.txt"));
            assert!(result.is_ok());
        }
    }
}
