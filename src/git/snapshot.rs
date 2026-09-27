//! Repository snapshots.
//!
//! A snapshot holds the Git data the dashboard shows. It is read in one pass
//! and replaced whole, so the view never mixes status from one refresh with
//! history from another. Snapshots know nothing about cursors, expansion, or
//! layout; the app keeps that state separately.

use anyhow::{Context as _, Result};

use super::{BranchInfo, GitCommand, GitRepo, GitStatus};

/// History display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HistoryMode {
    /// Show reflog (git actions)
    Reflog,
    /// Show commit log
    #[default]
    CommitLog,
}

/// What a snapshot should load, derived from the current view
#[derive(Debug, Clone)]
pub struct SnapshotRequest {
    /// Which history list to load
    pub history_mode: HistoryMode,
    /// Requested history page (0-indexed); clamped to the pages available
    pub history_page: usize,
    /// Entries per history page
    pub page_size: usize,
    /// Branch whose commits the view shows, if any
    pub expanded_branch: Option<String>,
}

/// Git data for one refresh
#[derive(Debug, Clone, Default)]
pub struct RepoSnapshot {
    /// Working tree and index status
    pub status: GitStatus,
    /// Entries on the loaded history page
    pub history: Vec<GitCommand>,
    /// Total entries across all history pages
    pub history_total: usize,
    /// Number of history pages
    pub history_pages: usize,
    /// History page actually loaded (the request, clamped)
    pub history_page: usize,
    /// Commit HEAD points at, if any
    pub head: Option<String>,
    /// Local and remote branches
    pub branches: Vec<BranchInfo>,
    /// Commits unique to the requested branch; empty if none was requested
    /// or it no longer exists
    pub branch_commits: Vec<GitCommand>,
}

impl GitRepo {
    /// Read a complete snapshot, or fail without partial results
    pub fn snapshot(&self, request: &SnapshotRequest) -> Result<RepoSnapshot> {
        let status = self
            .status()
            .context("reading status")?;

        let history_total = match request.history_mode {
            HistoryMode::Reflog => self.reflog_total(),
            HistoryMode::CommitLog => self.commit_log_total(),
        }
        .context("counting history")?;
        let history_pages = history_total.div_ceil(request.page_size.max(1));
        let history_page = request
            .history_page
            .min(history_pages.saturating_sub(1));

        let skip = history_page * request.page_size;
        let history = match request.history_mode {
            HistoryMode::Reflog => self.reflog(skip, request.page_size),
            HistoryMode::CommitLog => self.commit_log(skip, request.page_size),
        }
        .context("reading history")?;

        let branches = self
            .list_branches()
            .context("listing branches")?;

        let branch_commits = match &request.expanded_branch {
            Some(name) if branches.iter().any(|b| &b.name == name) => self
                .commit_log_for_branch(name)
                .context("reading branch history")?,
            _ => Vec::new(),
        };

        let head = self
            .repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .map(|oid| oid.to_string());

        Ok(RepoSnapshot {
            status,
            history,
            history_total,
            history_pages,
            history_page,
            head,
            branches,
            branch_commits,
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Repo with `count` empty commits on `main`
    fn repo_with_commits(count: usize) -> (TempDir, GitRepo) {
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

        let mut parent: Option<git2::Oid> = None;
        for i in 0..count {
            let parent_commit = parent.map(|oid| {
                repo.find_commit(oid)
                    .expect("find parent")
            });
            let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();
            let oid = repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("Commit {i}"),
                    &tree,
                    &parents,
                )
                .expect("commit");
            parent = Some(oid);
        }

        let git = GitRepo::open(dir.path()).expect("open repo");
        (dir, git)
    }

    fn request(page: usize) -> SnapshotRequest {
        SnapshotRequest {
            history_mode: HistoryMode::CommitLog,
            history_page: page,
            page_size: 2,
            expanded_branch: None,
        }
    }

    #[test]
    fn loads_requested_history_page() {
        let (_dir, git) = repo_with_commits(5);

        let snapshot = git
            .snapshot(&request(1))
            .expect("snapshot");

        assert_eq!(snapshot.history_total, 5);
        assert_eq!(snapshot.history_pages, 3);
        assert_eq!(snapshot.history_page, 1);
        assert_eq!(snapshot.history.len(), 2);
    }

    #[test]
    fn clamps_page_past_the_end() {
        let (_dir, git) = repo_with_commits(3);

        let snapshot = git
            .snapshot(&request(9))
            .expect("snapshot");

        assert_eq!(snapshot.history_pages, 2);
        assert_eq!(snapshot.history_page, 1);
        assert_eq!(snapshot.history.len(), 1);
    }

    #[test]
    fn empty_repo_has_no_history() {
        let (_dir, git) = repo_with_commits(0);

        let snapshot = git
            .snapshot(&request(0))
            .expect("snapshot");

        assert_eq!(snapshot.history_total, 0);
        assert_eq!(snapshot.history_pages, 0);
        assert_eq!(snapshot.history_page, 0);
        assert!(snapshot.history.is_empty());
    }
}
