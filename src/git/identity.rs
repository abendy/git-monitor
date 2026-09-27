//! Stable identity for a repository and each of its worktrees.
//!
//! Identity is where the repository lives on disk: the canonical path of its
//! common Git directory. Linked worktrees share that directory, so they share
//! the repository identity; each worktree adds its own canonical path.
//!
//! Defined behavior (ADR-008):
//! - Restarting, refreshing, or opening from a subdirectory or a symlinked path
//!   gives the same identity.
//! - Moving or renaming the repository gives a new identity.
//! - Adding, changing, or removing remotes does not change it. Which forge
//!   repository a checkout belongs to is a separate question.

use std::fmt;
use std::path::Path;

use anyhow::{Context, Result};

use super::GitRepo;

/// Identity of a repository, shared by all of its worktrees
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepoId(String);

impl RepoId {
    /// The identity as text
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RepoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Identity of one worktree of a repository
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorktreeId {
    repo: RepoId,
    path: String,
}

impl WorktreeId {
    /// The repository this worktree belongs to
    #[must_use]
    pub const fn repo(&self) -> &RepoId {
        &self.repo
    }

    /// The worktree's identity as text
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.path
    }
}

impl fmt::Display for WorktreeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.path)
    }
}

impl GitRepo {
    /// Identity of this repository, the same for every one of its worktrees
    pub fn repo_id(&self) -> Result<RepoId> {
        canonical(&self.common_dir()).map(RepoId)
    }

    /// Identity of the worktree this handle was opened in
    ///
    /// A bare repository has no working tree, so its Git directory stands in.
    pub fn worktree_id(&self) -> Result<WorktreeId> {
        let root = self
            .workdir()
            .unwrap_or_else(|| self.git_dir());
        Ok(WorktreeId {
            repo: self.repo_id()?,
            path: canonical(root)?,
        })
    }
}

/// Canonical path as text: symlinks resolved, no trailing separator
fn canonical(path: &Path) -> Result<String> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("resolving {}", path.display()))?;
    Ok(canonical.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Repo with one empty commit, so worktrees can be added
    fn repo_with_commit() -> TempDir {
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
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        )
        .expect("commit");
        dir
    }

    fn open(path: &Path) -> GitRepo {
        GitRepo::open(path).expect("open repo")
    }

    #[test]
    fn linked_worktrees_share_the_repo_but_not_the_worktree() {
        let main = repo_with_commit();
        let linked_parent = TempDir::new().expect("create worktree parent");
        let linked_path = linked_parent.path().join("linked");
        git2::Repository::open(main.path())
            .expect("open main")
            .worktree("linked", &linked_path, None)
            .expect("add linked worktree");

        let main_id = open(main.path())
            .worktree_id()
            .expect("main id");
        let linked_id = open(&linked_path)
            .worktree_id()
            .expect("linked id");

        assert_eq!(
            main_id.repo(),
            linked_id.repo(),
            "same repository"
        );
        assert_ne!(
            main_id, linked_id,
            "different worktrees"
        );
    }

    #[test]
    fn identity_is_stable_across_reopening_and_subdirectories() {
        let dir = repo_with_commit();
        let sub = dir.path().join("src/deep");
        std::fs::create_dir_all(&sub).expect("create subdir");

        let first = open(dir.path())
            .worktree_id()
            .expect("first open");
        let again = open(dir.path())
            .worktree_id()
            .expect("reopen");
        let from_sub = open(&sub)
            .worktree_id()
            .expect("open from subdirectory");

        assert_eq!(
            first, again,
            "restart gives the same identity"
        );
        assert_eq!(
            first, from_sub,
            "subdirectory gives the same identity"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_path_gives_the_same_identity() {
        let dir = repo_with_commit();
        let links = TempDir::new().expect("create link dir");
        let link = links.path().join("via-link");
        std::os::unix::fs::symlink(dir.path(), &link).expect("symlink");

        assert_eq!(
            open(dir.path())
                .worktree_id()
                .expect("direct"),
            open(&link)
                .worktree_id()
                .expect("via symlink")
        );
    }

    #[test]
    fn remotes_do_not_change_identity() {
        let dir = repo_with_commit();
        let before = open(dir.path())
            .repo_id()
            .expect("before");

        git2::Repository::open(dir.path())
            .expect("open repo")
            .remote(
                "origin",
                "https://example.com/owner/repo.git",
            )
            .expect("add remote");

        assert_eq!(
            open(dir.path())
                .repo_id()
                .expect("after"),
            before
        );
    }

    #[test]
    fn moving_the_repository_gives_a_new_identity() {
        let parent = TempDir::new().expect("create parent");
        let original = parent.path().join("project");
        let moved = parent.path().join("renamed");
        let seed = repo_with_commit();
        std::fs::rename(seed.path(), &original).expect("move repo into parent");

        let before = open(&original)
            .repo_id()
            .expect("before move");
        std::fs::rename(&original, &moved).expect("rename repo");
        let after = open(&moved)
            .repo_id()
            .expect("after move");

        assert_ne!(
            before, after,
            "a moved repository is a new location"
        );
    }
}
