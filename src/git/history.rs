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
                let short_sha = format!("{:.7}", oid);
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

                let short_sha = format!("{:.7}", oid);

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
    pub fn reflog(&self, skip: usize, limit: usize) -> Result<Vec<GitCommand>> {
        let mut commands = Vec::new();

        let reflog = match self.repo.reflog("HEAD") {
            Ok(reflog) => reflog,
            Err(_) => return Ok(commands), // No reflog yet
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
    pub fn reflog_total(&self) -> Result<usize> {
        let reflog = match self.repo.reflog("HEAD") {
            Ok(reflog) => reflog,
            Err(_) => return Ok(0),
        };

        Ok(reflog.iter().count())
    }

    fn make_commit_command(
        &self,
        commit: &git2::Commit<'_>,
        refs_map: &HashMap<String, Vec<RefDecoration>>,
        is_remote_only: bool,
    ) -> GitCommand {
        let message = commit.summary().unwrap_or("").to_string();
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
        let mut revwalk = match self.repo.revwalk() {
            Ok(revwalk) => revwalk,
            Err(_) => return commands,
        };
        if revwalk.push(start_oid).is_err() {
            return commands;
        }
        if let Some(hide_oid) = hide_oid {
            let _ = revwalk.hide(hide_oid);
        }
        let _ = revwalk.set_sorting(git2::Sort::TIME);

        if let Some(limit) = limit {
            for oid_result in revwalk.skip(skip).take(limit) {
                let oid = match oid_result {
                    Ok(oid) => oid,
                    Err(_) => continue,
                };
                let commit = match self.repo.find_commit(oid) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                commands.push(self.make_commit_command(
                    &commit,
                    refs_map,
                    is_remote_only,
                ));
            }
        } else {
            for oid_result in revwalk.skip(skip) {
                let oid = match oid_result {
                    Ok(oid) => oid,
                    Err(_) => continue,
                };
                let commit = match self.repo.find_commit(oid) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                commands.push(self.make_commit_command(
                    &commit,
                    refs_map,
                    is_remote_only,
                ));
            }
        }

        commands
    }

    fn count_commits(
        &self,
        start_oid: git2::Oid,
        hide_oid: Option<git2::Oid>,
    ) -> usize {
        let mut revwalk = match self.repo.revwalk() {
            Ok(revwalk) => revwalk,
            Err(_) => return 0,
        };
        if revwalk.push(start_oid).is_err() {
            return 0;
        }
        if let Some(hide_oid) = hide_oid {
            let _ = revwalk.hide(hide_oid);
        }
        let _ = revwalk.set_sorting(git2::Sort::TIME);

        revwalk.filter_map(Result::ok).count()
    }

    /// Get total commit count for HEAD history
    pub fn commit_log_total(&self) -> Result<usize> {
        let head = match self.repo.head() {
            Ok(head) => head,
            Err(_) => return Ok(0),
        };

        let head_oid = match head.target() {
            Some(oid) => oid,
            None => return Ok(0),
        };

        Ok(self.count_commits(head_oid, None))
    }

    /// Get commit history (git log) with pagination, including remote-only commits if tracking
    /// upstream Remote-only commits are only shown on the first page (skip = 0)
    pub fn commit_log(&self, skip: usize, limit: usize) -> Result<Vec<GitCommand>> {
        let mut commands = Vec::new();
        let mut remote_only_commands = Vec::new();
        let mut merge_base_sha: Option<String> = None;

        // Get HEAD
        let head = match self.repo.head() {
            Ok(head) => head,
            Err(_) => return Ok(commands), // No commits yet
        };

        let head_oid = match head.target() {
            Some(oid) => oid,
            None => return Ok(commands),
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
                                merge_base_sha = Some(format!("{:.7}", base_oid));
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
        let branch_oid = match branch_ref.target() {
            Some(oid) => oid,
            None => return Ok(Vec::new()),
        };

        // Collect refs for decorations
        let refs_map = self.collect_refs();
        let mut commands = self.walk_commits(
            branch_oid,
            None,
            0,
            None,
            &refs_map,
            false,
        );

        if commands.is_empty() {
            if let Ok(tip_commit) = self.repo.find_commit(branch_oid) {
                commands.push(self.make_commit_command(
                    &tip_commit,
                    &refs_map,
                    false,
                ));
            }
        }

        Ok(commands)
    }
}
