use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::{Local, TimeZone};

use super::{CommandType, GitCommand, GitRepo, RefDecoration};

impl GitRepo {
    /// Collect all refs (branches, tags) and map them to commit SHAs
    fn collect_refs(&self) -> HashMap<String, Vec<RefDecoration>> {
        let mut refs_map: HashMap<String, Vec<RefDecoration>> = HashMap::new();

        // Get HEAD commit for HEAD decoration
        if let Ok(head) = self.repo.head() {
            if let Some(oid) = head.target() {
                let short_sha = format!("{oid:.7}");
                refs_map
                    .entry(short_sha)
                    .or_default()
                    .push(RefDecoration::Head);
            }
        }

        // Iterate through all references
        if let Ok(refs) = self.repo.references() {
            for reference in refs.flatten() {
                let Some(name) = reference.name() else {
                    continue;
                };

                // Get the commit this ref points to
                let oid = if let Some(oid) = reference.target() {
                    oid
                } else if let Ok(resolved) = reference.resolve() {
                    match resolved.target() {
                        Some(oid) => oid,
                        None => continue,
                    }
                } else {
                    continue;
                };

                let short_sha = format!("{oid:.7}");

                // Parse the ref name into a decoration
                let decoration = if let Some(branch) = name.strip_prefix("refs/heads/") {
                    RefDecoration::LocalBranch(branch.to_string())
                } else if let Some(remote) = name.strip_prefix("refs/remotes/") {
                    // Skip HEAD refs like origin/HEAD
                    if remote.ends_with("/HEAD") {
                        continue;
                    }
                    RefDecoration::RemoteBranch(remote.to_string())
                } else if let Some(tag) = name.strip_prefix("refs/tags/") {
                    RefDecoration::Tag(tag.to_string())
                } else {
                    continue;
                };

                refs_map
                    .entry(short_sha)
                    .or_default()
                    .push(decoration);
            }
        }

        refs_map
    }

    /// Get recent activity from reflog with pagination
    #[allow(clippy::unnecessary_wraps)] // Result for API consistency
    pub fn reflog(&self, skip: usize, limit: usize) -> Result<Vec<GitCommand>> {
        let mut commands = Vec::new();

        let Ok(reflog) = self.repo.reflog("HEAD") else {
            return Ok(commands); // No reflog yet
        };

        // Collect all refs once for decoration lookup
        let refs_map = self.collect_refs();

        for entry in reflog.iter().skip(skip).take(limit) {
            let message = entry
                .message()
                .unwrap_or("")
                .to_string();
            let command_type = CommandType::from_message(&message);

            // Parse timestamp
            let sig = entry.committer();
            let timestamp = Local
                .timestamp_opt(sig.when().seconds(), 0)
                .single()
                .unwrap_or_else(Local::now);

            // Get short SHA
            let short_sha = format!("{:.7}", entry.id_new());

            // Look up decorations for this commit
            let decorations = refs_map
                .get(&short_sha)
                .cloned()
                .unwrap_or_default();

            commands.push(GitCommand {
                timestamp,
                command_type,
                message,
                sha: Some(short_sha),
                decorations,
                is_remote_only: false,
            });
        }

        Ok(commands)
    }

    /// Get total reflog entries
    #[allow(clippy::unnecessary_wraps)] // Result for API consistency
    pub fn reflog_total(&self) -> Result<usize> {
        let Ok(reflog) = self.repo.reflog("HEAD") else {
            return Ok(0);
        };

        Ok(reflog.iter().count())
    }

    #[allow(clippy::unused_self)] // Method for consistency with other methods
    fn make_commit_command(
        &self,
        commit: &git2::Commit<'_>,
        refs_map: &HashMap<String, Vec<RefDecoration>>,
        is_remote_only: bool,
    ) -> GitCommand {
        let message = commit
            .summary()
            .unwrap_or("")
            .to_string();
        let time = commit.time();
        let timestamp = Local
            .timestamp_opt(time.seconds(), 0)
            .single()
            .unwrap_or_else(Local::now);
        let short_sha = format!("{:.7}", commit.id());
        let decorations = refs_map
            .get(&short_sha)
            .cloned()
            .unwrap_or_default();
        GitCommand {
            timestamp,
            command_type: CommandType::Commit,
            message,
            sha: Some(short_sha),
            decorations,
            is_remote_only,
        }
    }

    fn walk_commits(
        &self,
        start_oid: git2::Oid,
        hide_oid: Option<git2::Oid>,
        skip: usize,
        limit: Option<usize>,
        refs_map: &HashMap<String, Vec<RefDecoration>>,
        is_remote_only: bool,
    ) -> Vec<GitCommand> {
        let mut commands = Vec::new();
        let Ok(mut revwalk) = self.repo.revwalk() else {
            return commands;
        };
        if revwalk.push(start_oid).is_err() {
            return commands;
        }
        // Intentionally ignore hide/sorting errors - these are optional optimizations.
        // If hide fails, we may see extra commits; if sorting fails, order may vary.
        // Both are acceptable degradations for a display-only feature.
        if let Some(hide_oid) = hide_oid {
            let _ = revwalk.hide(hide_oid);
        }
        let _ = revwalk.set_sorting(git2::Sort::TIME);

        if let Some(limit) = limit {
            for oid_result in revwalk.skip(skip).take(limit) {
                let Ok(oid) = oid_result else {
                    continue;
                };
                let Ok(commit) = self.repo.find_commit(oid) else {
                    continue;
                };
                commands.push(self.make_commit_command(&commit, refs_map, is_remote_only));
            }
        } else {
            for oid_result in revwalk.skip(skip) {
                let Ok(oid) = oid_result else {
                    continue;
                };
                let Ok(commit) = self.repo.find_commit(oid) else {
                    continue;
                };
                commands.push(self.make_commit_command(&commit, refs_map, is_remote_only));
            }
        }

        commands
    }

    fn count_commits(&self, start_oid: git2::Oid, hide_oid: Option<git2::Oid>) -> usize {
        let Ok(mut revwalk) = self.repo.revwalk() else {
            return 0;
        };
        if revwalk.push(start_oid).is_err() {
            return 0;
        }
        // Intentionally ignore hide/sorting errors - see walk_commits for rationale
        if let Some(hide_oid) = hide_oid {
            let _ = revwalk.hide(hide_oid);
        }
        let _ = revwalk.set_sorting(git2::Sort::TIME);

        revwalk.filter_map(Result::ok).count()
    }

    /// Get total commit count for HEAD history
    #[allow(clippy::unnecessary_wraps)] // Result for API consistency
    pub fn commit_log_total(&self) -> Result<usize> {
        let Ok(head) = self.repo.head() else {
            return Ok(0);
        };

        let Some(head_oid) = head.target() else {
            return Ok(0);
        };

        Ok(self.count_commits(head_oid, None))
    }

    /// Get commit history (git log) with pagination, including remote-only commits if tracking
    /// upstream Remote-only commits are only shown on the first page (skip = 0)
    #[allow(clippy::unnecessary_wraps)] // Result for API consistency
    pub fn commit_log(&self, skip: usize, limit: usize) -> Result<Vec<GitCommand>> {
        let mut commands = Vec::new();
        let mut remote_only_commands = Vec::new();
        let mut merge_base_sha: Option<String> = None;

        // Get HEAD
        let Ok(head) = self.repo.head() else {
            return Ok(commands); // No commits yet
        };

        let Some(head_oid) = head.target() else {
            return Ok(commands);
        };

        // Collect refs for decorations
        let refs_map = self.collect_refs();

        // Check for upstream and get remote-only commits + merge base (only on first page)
        if skip == 0 && head.is_branch() {
            if let Some(branch_name) = head.shorthand() {
                if let Ok(branch) = self
                    .repo
                    .find_branch(branch_name, git2::BranchType::Local)
                {
                    if let Ok(upstream) = branch.upstream() {
                        if let Some(upstream_oid) = upstream.get().target() {
                            // Find merge base for positioning
                            if let Ok(base_oid) = self
                                .repo
                                .merge_base(head_oid, upstream_oid)
                            {
                                merge_base_sha = Some(format!("{base_oid:.7}"));
                            }

                            remote_only_commands = self.walk_commits(
                                upstream_oid,
                                Some(head_oid),
                                0,
                                None,
                                &refs_map,
                                true,
                            );
                        }
                    }
                }
            }
        }

        let mut inserted_remote = false;
        let main_commits = self.walk_commits(
            head_oid,
            None,
            skip,
            Some(limit),
            &refs_map,
            false,
        );

        for cmd in main_commits {
            let short_sha = cmd.sha.clone().unwrap_or_default();
            if !inserted_remote && merge_base_sha.as_ref() == Some(&short_sha) {
                commands.append(&mut remote_only_commands);
                inserted_remote = true;
            }

            commands.push(cmd);
        }

        // If we never hit the merge base (e.g., it's beyond our limit), append at end
        if !inserted_remote && !remote_only_commands.is_empty() {
            commands.append(&mut remote_only_commands);
        }

        Ok(commands)
    }

    /// Get commits reachable from a branch tip
    /// Always includes at least the tip commit so branches are never empty
    pub fn commit_log_for_branch(&self, branch_name: &str) -> Result<Vec<GitCommand>> {
        // Find the branch (try local first, then remote)
        let branch = self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)
            .or_else(|_| {
                self.repo
                    .find_branch(branch_name, git2::BranchType::Remote)
            })
            .context(format!(
                "Failed to find branch '{branch_name}'"
            ))?;

        let branch_ref = branch.get();
        let Some(branch_oid) = branch_ref.target() else {
            return Ok(Vec::new());
        };

        // Collect refs for decorations
        let refs_map = self.collect_refs();
        let mut commands = self.walk_commits(
            branch_oid, None, 0, None, &refs_map, false,
        );

        if commands.is_empty() {
            if let Ok(tip_commit) = self.repo.find_commit(branch_oid) {
                commands.push(self.make_commit_command(&tip_commit, &refs_map, false));
            }
        }

        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    /// Create a test repo with an initial commit
    fn create_test_repo() -> (TempDir, GitRepo) {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let repo = Repository::init(dir.path()).expect("Failed to init repo");

        let mut config = repo
            .config()
            .expect("Failed to get config");
        config
            .set_str("user.name", "Test User")
            .expect("Failed to set user.name");
        config
            .set_str("user.email", "test@example.com")
            .expect("Failed to set user.email");
        drop(config);

        // Create initial commit
        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "initial\n").expect("Failed to write file");

        let mut index = repo
            .index()
            .expect("Failed to get index");
        index
            .add_path(Path::new("file.txt"))
            .expect("Failed to add file");
        index
            .write()
            .expect("Failed to write index");

        let tree_id = index
            .write_tree()
            .expect("Failed to write tree");
        let tree = repo
            .find_tree(tree_id)
            .expect("Failed to find tree");
        let sig = repo
            .signature()
            .expect("Failed to get signature");
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        )
        .expect("Failed to commit");

        drop(tree);
        drop(repo);

        let git_repo = GitRepo::open(dir.path()).expect("Failed to open repo");
        (dir, git_repo)
    }

    /// Add another commit to the repo
    fn add_commit(dir: &TempDir, message: &str) {
        let repo = Repository::open(dir.path()).expect("Failed to open");

        // Modify file
        let file_path = dir.path().join("file.txt");
        let content = fs::read_to_string(&file_path).unwrap_or_default();
        fs::write(
            &file_path,
            format!("{content}{message}\n"),
        )
        .expect("Failed to write");

        let mut index = repo
            .index()
            .expect("Failed to get index");
        index
            .add_path(Path::new("file.txt"))
            .expect("Failed to add");
        index
            .write()
            .expect("Failed to write index");

        let tree_id = index
            .write_tree()
            .expect("Failed to write tree");
        let tree = repo
            .find_tree(tree_id)
            .expect("Failed to find tree");
        let sig = repo
            .signature()
            .expect("Failed to get signature");
        let head = repo.head().expect("Failed to get HEAD");
        let parent = head
            .peel_to_commit()
            .expect("Failed to get commit");

        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &[&parent],
        )
        .expect("Failed to commit");
    }

    mod reflog {
        use super::*;

        #[test]
        fn returns_entries_for_repo_with_commits() {
            let (_dir, repo) = create_test_repo();

            let entries = repo
                .reflog(0, 10)
                .expect("Failed to get reflog");

            // At least one entry for initial commit
            assert!(!entries.is_empty());
        }

        #[test]
        fn entries_have_sha() {
            let (_dir, repo) = create_test_repo();

            let entries = repo
                .reflog(0, 10)
                .expect("Failed to get reflog");

            for entry in &entries {
                assert!(entry.sha.is_some());
                let sha = entry
                    .sha
                    .as_ref()
                    .expect("sha should exist");
                assert_eq!(sha.len(), 7);
            }
        }

        #[test]
        fn respects_pagination_limit() {
            let (dir, _repo) = create_test_repo();

            // Add more commits to have multiple reflog entries
            add_commit(&dir, "Second commit");
            add_commit(&dir, "Third commit");

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");
            let entries = repo
                .reflog(0, 2)
                .expect("Failed to get reflog");

            assert!(entries.len() <= 2);
        }

        #[test]
        fn respects_pagination_skip() {
            let (dir, _repo) = create_test_repo();

            add_commit(&dir, "Second commit");
            add_commit(&dir, "Third commit");

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");

            let all = repo
                .reflog(0, 100)
                .expect("Failed to get all");
            let skipped = repo
                .reflog(1, 100)
                .expect("Failed to get skipped");

            assert_eq!(
                skipped.len(),
                all.len().saturating_sub(1)
            );
        }
    }

    mod reflog_total {
        use super::*;

        #[test]
        fn counts_reflog_entries() {
            let (dir, _repo) = create_test_repo();

            add_commit(&dir, "Second commit");

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");
            let total = repo
                .reflog_total()
                .expect("Failed to get total");

            // At least 2 entries (initial + second commit)
            assert!(total >= 2);
        }
    }

    mod commit_log {
        use super::*;

        #[test]
        fn returns_commits() {
            let (_dir, repo) = create_test_repo();

            let commits = repo
                .commit_log(0, 10)
                .expect("Failed to get log");

            assert_eq!(commits.len(), 1);
            assert_eq!(commits[0].message, "Initial commit");
        }

        #[test]
        fn returns_all_commits() {
            let (dir, _repo) = create_test_repo();

            add_commit(&dir, "Second commit");
            add_commit(&dir, "Third commit");

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");
            let commits = repo
                .commit_log(0, 10)
                .expect("Failed to get log");

            assert_eq!(commits.len(), 3);

            // Verify all commits are present (order may vary with same timestamps)
            let messages: Vec<_> = commits
                .iter()
                .map(|c| c.message.as_str())
                .collect();
            assert!(messages.contains(&"Initial commit"));
            assert!(messages.contains(&"Second commit"));
            assert!(messages.contains(&"Third commit"));
        }

        #[test]
        fn respects_pagination_limit() {
            let (dir, _repo) = create_test_repo();

            add_commit(&dir, "Second commit");
            add_commit(&dir, "Third commit");

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");

            let page1 = repo
                .commit_log(0, 2)
                .expect("Failed to get page 1");
            let page2 = repo
                .commit_log(2, 2)
                .expect("Failed to get page 2");

            // Verify pagination limits work
            assert_eq!(page1.len(), 2);
            assert_eq!(page2.len(), 1);

            // All 3 commits should be covered across pages
            let all_messages: Vec<_> = page1
                .iter()
                .chain(page2.iter())
                .map(|c| c.message.as_str())
                .collect();
            assert!(all_messages.contains(&"Initial commit"));
            assert!(all_messages.contains(&"Second commit"));
            assert!(all_messages.contains(&"Third commit"));
        }

        #[test]
        fn commits_have_sha() {
            let (_dir, repo) = create_test_repo();

            let commits = repo
                .commit_log(0, 10)
                .expect("Failed to get log");

            assert!(commits[0].sha.is_some());
            let sha = commits[0]
                .sha
                .as_ref()
                .expect("sha should exist");
            assert_eq!(sha.len(), 7);
            assert!(sha
                .chars()
                .all(|c| c.is_ascii_hexdigit()));
        }
    }

    mod commit_log_total {
        use super::*;

        #[test]
        fn counts_commits() {
            let (dir, _repo) = create_test_repo();

            add_commit(&dir, "Second commit");
            add_commit(&dir, "Third commit");

            let repo = GitRepo::open(dir.path()).expect("Failed to reopen");
            let total = repo
                .commit_log_total()
                .expect("Failed to get total");

            assert_eq!(total, 3);
        }
    }

    mod commit_log_for_branch {
        use super::*;

        #[test]
        fn returns_commits_for_branch() {
            let (dir, repo) = create_test_repo();

            // Create a branch
            {
                let git_repo = Repository::open(dir.path()).expect("Failed to open");
                let head = git_repo
                    .head()
                    .expect("Failed to get HEAD");
                let commit = head
                    .peel_to_commit()
                    .expect("Failed to get commit");
                git_repo
                    .branch("feature", &commit, false)
                    .expect("Failed to create branch");
            }

            // Get the default branch name
            let branches = repo
                .list_branches()
                .expect("Failed to list");
            let default_branch = &branches[0].name;

            let commits = repo
                .commit_log_for_branch(default_branch)
                .expect("Failed to get log");

            assert_eq!(commits.len(), 1);
        }

        #[test]
        fn returns_error_for_nonexistent_branch() {
            let (_dir, repo) = create_test_repo();

            let result = repo.commit_log_for_branch("nonexistent");

            assert!(result.is_err());
        }
    }
}
