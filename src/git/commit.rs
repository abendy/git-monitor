use anyhow::{Context, Result};
use chrono::{DateTime, Local, Offset};

use super::{CommitDetail, CommitFile, FileState, GitRepo};

impl GitRepo {
    /// Get detailed commit information for a given SHA
    pub fn commit_detail(&self, short_sha: &str) -> Result<CommitDetail> {
        // Parse the short SHA to find the commit
        let obj = self
            .repo
            .revparse_single(short_sha)
            .with_context(|| format!("Failed to find commit {short_sha}"))?;
        let commit = obj
            .peel_to_commit()
            .with_context(|| format!("Object {short_sha} is not a commit"))?;

        let oid = commit.id();
        let full_sha = format!("{oid}");

        // Extract author info
        let author = commit.author();
        let author_name = author
            .name()
            .unwrap_or("Unknown")
            .to_string();
        let author_email = author.email().unwrap_or("").to_string();
        let author_time = {
            let time = author.when();
            let secs = time.seconds();
            let offset_mins = time.offset_minutes();
            let offset =
                chrono::FixedOffset::east_opt(offset_mins * 60).unwrap_or(chrono::Utc.fix());
            DateTime::from_timestamp(secs, 0)
                .map(|dt| {
                    dt.with_timezone(&offset)
                        .with_timezone(&Local)
                })
                .unwrap_or_else(Local::now)
        };

        // Extract committer info
        let committer = commit.committer();
        let committer_name = committer
            .name()
            .unwrap_or("Unknown")
            .to_string();
        let committer_email = committer
            .email()
            .unwrap_or("")
            .to_string();
        let committer_time = {
            let time = committer.when();
            let secs = time.seconds();
            let offset_mins = time.offset_minutes();
            let offset =
                chrono::FixedOffset::east_opt(offset_mins * 60).unwrap_or(chrono::Utc.fix());
            DateTime::from_timestamp(secs, 0)
                .map(|dt| {
                    dt.with_timezone(&offset)
                        .with_timezone(&Local)
                })
                .unwrap_or_else(Local::now)
        };

        // Get full commit message
        let message = commit
            .message()
            .unwrap_or("")
            .to_string();

        // Check for GPG signature
        let gpg_status = commit.raw_header().and_then(|header| {
            if header.contains("gpgsig") {
                Some("Signed".to_string())
            } else {
                None
            }
        });

        // Get files changed by diffing against parent
        let tree = commit
            .tree()
            .context("Failed to get commit tree")?;
        let parent_tree = commit
            .parent(0)
            .ok()
            .and_then(|p| p.tree().ok());

        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .context("Failed to diff trees")?;

        // Get overall stats
        let stats = diff
            .stats()
            .context("Failed to get diff stats")?;
        let insertions = stats.insertions();
        let deletions = stats.deletions();

        // Collect files with per-file stats using RefCell for interior mutability
        use std::cell::RefCell;
        use std::collections::HashMap;
        let file_stats: RefCell<HashMap<String, (FileState, usize, usize)>> =
            RefCell::new(HashMap::new());

        diff.foreach(
            &mut |delta, _progress| {
                let path = delta
                    .new_file()
                    .path()
                    .or_else(|| delta.old_file().path())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "<unknown>".to_string());

                let status = match delta.status() {
                    git2::Delta::Added => FileState::Added,
                    git2::Delta::Deleted => FileState::Deleted,
                    git2::Delta::Modified => FileState::Modified,
                    git2::Delta::Renamed => FileState::Renamed,
                    git2::Delta::Copied => FileState::Added,
                    _ => FileState::Modified,
                };

                file_stats
                    .borrow_mut()
                    .insert(path, (status, 0, 0));
                true
            },
            None,
            None,
            Some(&mut |delta, _hunk, line| {
                if let Some(path) = delta
                    .new_file()
                    .path()
                    .or_else(|| delta.old_file().path())
                    .map(|p| p.to_string_lossy().to_string())
                {
                    if let Some(entry) = file_stats.borrow_mut().get_mut(&path) {
                        match line.origin() {
                            '+' => entry.1 += 1,
                            '-' => entry.2 += 1,
                            _ => {}
                        }
                    }
                }
                true
            }),
        )?;

        // Convert to Vec and sort
        let mut files: Vec<CommitFile> = file_stats
            .into_inner()
            .into_iter()
            .map(
                |(path, (status, ins, del))| CommitFile {
                    path,
                    status,
                    insertions: ins,
                    deletions: del,
                },
            )
            .collect();
        files.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(CommitDetail {
            full_sha,
            author_name,
            author_email,
            author_time,
            committer_name,
            committer_email,
            committer_time,
            message,
            gpg_status,
            files,
            insertions,
            deletions,
        })
    }

    /// Get diff for a specific file in a commit (vs its parent)
    pub fn commit_file_diff(&self, commit_sha: &str, file_path: &str) -> Result<String> {
        let obj = self
            .repo
            .revparse_single(commit_sha)
            .with_context(|| format!("Failed to find commit {commit_sha}"))?;
        let commit = obj
            .peel_to_commit()
            .with_context(|| format!("Object {commit_sha} is not a commit"))?;

        let tree = commit
            .tree()
            .context("Failed to get commit tree")?;
        let parent_tree = commit
            .parent(0)
            .ok()
            .and_then(|p| p.tree().ok());

        // Create diff with path filter
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(file_path);

        let diff = self
            .repo
            .diff_tree_to_tree(
                parent_tree.as_ref(),
                Some(&tree),
                Some(&mut opts),
            )
            .context("Failed to diff trees")?;

        // Format as patch
        let mut output = String::new();
        diff.print(
            git2::DiffFormat::Patch,
            |_delta, _hunk, line| {
                let prefix = match line.origin() {
                    '+' | '-' | ' ' => format!("{}", line.origin()),
                    _ => String::new(),
                };
                if let Ok(content) = std::str::from_utf8(line.content()) {
                    output.push_str(&prefix);
                    output.push_str(content);
                }
                true
            },
        )?;

        if output.is_empty() {
            output = format!("(No changes for {file_path})");
        }

        Ok(output)
    }
}
