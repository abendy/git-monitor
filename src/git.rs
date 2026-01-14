use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeZone};
use git2::{Repository, Status, StatusOptions};

/// Status of a file in the repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStatus {
    /// Path relative to repository root
    pub path: PathBuf,
    /// Status in working directory
    pub working: FileState,
    /// Status in staging area (index)
    pub staged: FileState,
}

/// State of a file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FileState {
    #[default]
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    #[allow(dead_code)]
    Ignored,
    Conflicted,
}

impl FileState {
    /// Character representation (like git status --short)
    pub const fn as_char(self) -> char {
        match self {
            Self::Unmodified => ' ',
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Untracked => '?',
            Self::Ignored => '!',
            Self::Conflicted => 'U',
        }
    }

    pub const fn is_changed(self) -> bool {
        !matches!(self, Self::Unmodified | Self::Ignored)
    }
}

/// Repository state (merging, rebasing, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepoState {
    #[default]
    Normal,
    Merge,
    Rebase,
    RebaseInteractive,
    CherryPick,
    Revert,
    Bisect,
}

impl From<git2::RepositoryState> for RepoState {
    fn from(state: git2::RepositoryState) -> Self {
        match state {
            git2::RepositoryState::Merge => Self::Merge,
            git2::RepositoryState::Rebase
            | git2::RepositoryState::RebaseMerge => Self::Rebase,
            git2::RepositoryState::RebaseInteractive => Self::RebaseInteractive,
            git2::RepositoryState::CherryPick | git2::RepositoryState::CherryPickSequence => {
                Self::CherryPick
            }
            git2::RepositoryState::Revert | git2::RepositoryState::RevertSequence => Self::Revert,
            git2::RepositoryState::Bisect => Self::Bisect,
            _ => Self::Normal,
        }
    }
}

/// Complete git status snapshot
#[derive(Debug, Clone, Default)]
pub struct GitStatus {
    /// Current branch name (None if detached HEAD)
    pub branch: Option<String>,
    /// Upstream branch name if tracking
    pub upstream: Option<String>,
    /// Commits ahead of upstream
    pub ahead: usize,
    /// Commits behind upstream
    pub behind: usize,
    /// Files with changes
    pub files: Vec<FileStatus>,
    /// Repository state
    #[allow(dead_code)]
    pub state: RepoState,
}

impl GitStatus {
    /// Files changed in working directory (not staged)
    pub fn working_changes(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| f.working.is_changed())
            .collect()
    }

    /// Files staged for commit
    pub fn staged_changes(&self) -> Vec<&FileStatus> {
        self.files
            .iter()
            .filter(|f| f.staged.is_changed())
            .collect()
    }
}

/// A decoration (branch, tag, etc.) attached to a commit
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefDecoration {
    /// HEAD pointer
    Head,
    /// Local branch
    LocalBranch(String),
    /// Remote branch (e.g., origin/main)
    RemoteBranch(String),
    /// Tag
    Tag(String),
}

/// A git command/action from the reflog
#[derive(Debug, Clone)]
pub struct GitCommand {
    /// When the command was executed
    pub timestamp: DateTime<Local>,
    /// Type of command
    pub command_type: CommandType,
    /// Command message/description
    pub message: String,
    /// Short SHA if available
    pub sha: Option<String>,
    /// Decorations (branches, tags) pointing to this commit
    pub decorations: Vec<RefDecoration>,
    /// True if this commit only exists on remote (not reachable from HEAD)
    pub is_remote_only: bool,
}

/// Detailed commit information for expanded view
#[derive(Debug, Clone)]
pub struct CommitDetail {
    /// Full 40-character SHA
    pub full_sha: String,
    /// Author name
    pub author_name: String,
    /// Author email
    pub author_email: String,
    /// Author timestamp
    pub author_time: DateTime<Local>,
    /// Committer name
    pub committer_name: String,
    /// Committer email
    pub committer_email: String,
    /// Committer timestamp
    #[allow(dead_code)]
    pub committer_time: DateTime<Local>,
    /// Full commit message (summary + body)
    pub message: String,
    /// GPG signature status (if signed)
    pub gpg_status: Option<String>,
    /// Files changed in this commit
    pub files: Vec<CommitFile>,
    /// Total lines added
    pub insertions: usize,
    /// Total lines deleted
    pub deletions: usize,
}

/// A file changed in a commit
#[derive(Debug, Clone)]
pub struct CommitFile {
    /// File path
    pub path: String,
    /// Change status
    pub status: FileState,
    /// Lines added in this file
    pub insertions: usize,
    /// Lines deleted in this file
    pub deletions: usize,
}

/// Information about a local branch
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name
    pub name: String,
    /// Whether this is the current (checked out) branch
    pub is_current: bool,
    /// Whether this is a remote tracking branch
    pub is_remote: bool,
}

/// Types of git commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    Commit,
    Checkout,
    Merge,
    Rebase,
    Pull,
    #[allow(dead_code)]
    Push,
    Reset,
    CherryPick,
    Revert,
    Branch,
    Clone,
    Init,
    #[allow(dead_code)]
    Fetch,
    #[allow(dead_code)]
    Stash,
    Other,
}

impl CommandType {
    /// Parse command type from reflog message
    pub fn from_message(msg: &str) -> Self {
        let msg_lower = msg.to_lowercase();

        if msg_lower.starts_with("commit") {
            Self::Commit
        } else if msg_lower.starts_with("checkout") {
            Self::Checkout
        } else if msg_lower.starts_with("merge") {
            Self::Merge
        } else if msg_lower.starts_with("rebase") {
            Self::Rebase
        } else if msg_lower.starts_with("pull") {
            Self::Pull
        } else if msg_lower.starts_with("reset") {
            Self::Reset
        } else if msg_lower.starts_with("cherry-pick") {
            Self::CherryPick
        } else if msg_lower.starts_with("revert") {
            Self::Revert
        } else if msg_lower.starts_with("branch") {
            Self::Branch
        } else if msg_lower.starts_with("clone") {
            Self::Clone
        } else if msg_lower.contains("initial") {
            Self::Init
        } else {
            Self::Other
        }
    }

    /// Icon for this command type
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Commit => "●",
            Self::Checkout => "⎇",
            Self::Merge => "⑂",
            Self::Rebase => "↺",
            Self::Pull => "↓",
            Self::Push => "↑",
            Self::Fetch => "⟳",
            Self::Reset => "↩",
            Self::CherryPick => "❋",
            Self::Revert => "⊗",
            Self::Stash => "□",
            Self::Branch => "⌥",
            Self::Clone => "⊕",
            Self::Init => "★",
            Self::Other => "•",
        }
    }
}

/// Format a timestamp as relative time (e.g., "2 hr ago", "3 days ago")
pub fn format_relative_time(time: DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(time);

    let seconds = duration.num_seconds();
    if seconds < 60 {
        return "just now".to_string();
    }

    let minutes = duration.num_minutes();
    if minutes < 60 {
        return format!("{} min ago", minutes);
    }

    let hours = duration.num_hours();
    if hours < 24 {
        return format!("{} hr ago", hours);
    }

    let days = duration.num_days();
    if days < 7 {
        return format!("{} days ago", days);
    }

    let weeks = days / 7;
    if weeks < 4 {
        return format!("{} wk ago", weeks);
    }

    let months = days / 30;
    if months < 12 {
        return format!("{} mo ago", months);
    }

    let years = days / 365;
    format!("{} yr ago", years)
}

/// Git repository wrapper
pub struct GitRepo {
    repo: Repository,
}

impl GitRepo {
    /// Open a git repository at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::discover(path)
            .with_context(|| format!("No git repository found at {}", path.display()))?;

        Ok(Self { repo })
    }

    /// Get the repository root path
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// Get current git status
    pub fn status(&self) -> Result<GitStatus> {
        let mut status = GitStatus {
            state: self.repo.state().into(),
            ..Default::default()
        };

        // Get branch info
        self.populate_branch_info(&mut status)?;

        // Get file statuses
        self.populate_file_statuses(&mut status)?;

        Ok(status)
    }

    /// Populate branch and upstream info
    fn populate_branch_info(&self, status: &mut GitStatus) -> Result<()> {
        // Get HEAD reference
        let head = match self.repo.head() {
            Ok(head) => head,
            Err(_) => return Ok(()), // No commits yet
        };

        // Get branch name
        if head.is_branch() {
            status.branch = head.shorthand().map(String::from);

            // Get upstream tracking info
            if let Some(branch_name) = &status.branch {
                if let Ok(branch) = self.repo.find_branch(branch_name, git2::BranchType::Local) {
                    if let Ok(upstream) = branch.upstream() {
                        status.upstream = upstream.name().ok().flatten().map(String::from);

                        // Get ahead/behind counts
                        if let (Some(local_oid), Some(upstream_oid)) =
                            (head.target(), upstream.get().target())
                        {
                            if let Ok((ahead, behind)) =
                                self.repo.graph_ahead_behind(local_oid, upstream_oid)
                            {
                                status.ahead = ahead;
                                status.behind = behind;
                            }
                        }
                    }
                }
            }
        } else {
            // Detached HEAD - show short commit hash
            if let Some(oid) = head.target() {
                status.branch = Some(format!("{:.7}", oid));
            }
        }

        Ok(())
    }

    /// Stage a file
    pub fn stage(&self, path: &Path) -> Result<()> {
        let mut index = self.repo.index().context("Failed to get index")?;
        index
            .add_path(path)
            .with_context(|| format!("Failed to stage {}", path.display()))?;
        index.write().context("Failed to write index")?;
        Ok(())
    }

    /// Unstage a file
    pub fn unstage(&self, path: &Path) -> Result<()> {
        let head = self.repo.head().context("Failed to get HEAD")?;
        let head_commit = head.peel_to_commit().context("Failed to get HEAD commit")?;
        let head_tree = head_commit.tree().context("Failed to get HEAD tree")?;

        self.repo
            .reset_default(Some(&head_commit.as_object()), [path])
            .or_else(|_| -> std::result::Result<(), git2::Error> {
                // If reset fails (file is new), remove from index
                let mut index = self.repo.index()?;
                index.remove_path(path)?;
                index.write()?;
                Ok(())
            })
            .with_context(|| format!("Failed to unstage {}", path.display()))?;

        drop(head_tree);
        Ok(())
    }

    /// Get diff for a file (working directory changes)
    pub fn diff_file(&self, path: &Path, staged: bool) -> Result<String> {
        use std::fmt::Write;

        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.pathspec(path);

        let diff = if staged {
            // Staged: diff HEAD to index
            let head = self.repo.head().ok();
            let head_tree = head.and_then(|h| h.peel_to_tree().ok());
            self.repo
                .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut diff_opts))
        } else {
            // Unstaged: diff index to workdir
            self.repo
                .diff_index_to_workdir(None, Some(&mut diff_opts))
        }
        .context("Failed to get diff")?;

        let mut output = String::new();

        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' => "+",
                '-' => "-",
                ' ' => " ",
                _ => "",
            };
            let content = std::str::from_utf8(line.content()).unwrap_or("");
            let _ = write!(output, "{prefix}{content}");
            true
        })
        .context("Failed to print diff")?;

        if output.is_empty() {
            output = String::from("(no changes)");
        }

        Ok(output)
    }

    /// Collect all refs (branches, tags) and map them to commit SHAs
    fn collect_refs(&self) -> HashMap<String, Vec<RefDecoration>> {
        let mut refs_map: HashMap<String, Vec<RefDecoration>> = HashMap::new();

        // Get HEAD commit for HEAD decoration
        if let Ok(head) = self.repo.head() {
            if let Some(oid) = head.target() {
                let short_sha = format!("{:.7}", oid);
                refs_map.entry(short_sha).or_default().push(RefDecoration::Head);
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

                refs_map.entry(short_sha).or_default().push(decoration);
            }
        }

        refs_map
    }

    /// Get recent activity from reflog
    pub fn reflog(&self, limit: usize) -> Result<Vec<GitCommand>> {
        let mut commands = Vec::new();

        let reflog = match self.repo.reflog("HEAD") {
            Ok(reflog) => reflog,
            Err(_) => return Ok(commands), // No reflog yet
        };

        // Collect all refs once for decoration lookup
        let refs_map = self.collect_refs();

        for entry in reflog.iter().take(limit) {
            let message = entry.message().unwrap_or("").to_string();
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
            let decorations = refs_map.get(&short_sha).cloned().unwrap_or_default();

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

    /// Get commit history (git log), including remote-only commits if tracking upstream
    pub fn commit_log(&self, limit: usize) -> Result<Vec<GitCommand>> {
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

        // Helper to create GitCommand from commit
        let make_command =
            |commit: &git2::Commit<'_>,
             refs_map: &HashMap<String, Vec<RefDecoration>>,
             is_remote_only: bool| {
                let message = commit.summary().unwrap_or("").to_string();
                let time = commit.time();
                let timestamp = Local
                    .timestamp_opt(time.seconds(), 0)
                    .single()
                    .unwrap_or_else(Local::now);
                let short_sha = format!("{:.7}", commit.id());
                let decorations = refs_map.get(&short_sha).cloned().unwrap_or_default();
                GitCommand {
                    timestamp,
                    command_type: CommandType::Commit,
                    message,
                    sha: Some(short_sha),
                    decorations,
                    is_remote_only,
                }
            };

        // Check for upstream and get remote-only commits + merge base
        if head.is_branch() {
            if let Some(branch_name) = head.shorthand() {
                if let Ok(branch) = self.repo.find_branch(branch_name, git2::BranchType::Local) {
                    if let Ok(upstream) = branch.upstream() {
                        if let Some(upstream_oid) = upstream.get().target() {
                            // Find merge base for positioning
                            if let Ok(base_oid) = self.repo.merge_base(head_oid, upstream_oid) {
                                merge_base_sha = Some(format!("{:.7}", base_oid));
                            }

                            // Walk commits from upstream that are not reachable from HEAD
                            if let Ok(mut revwalk) = self.repo.revwalk() {
                                let _ = revwalk.push(upstream_oid);
                                let _ = revwalk.hide(head_oid);
                                revwalk.set_sorting(git2::Sort::TIME).ok();

                                for oid_result in revwalk {
                                    let oid = match oid_result {
                                        Ok(oid) => oid,
                                        Err(_) => continue,
                                    };
                                    if let Ok(commit) = self.repo.find_commit(oid) {
                                        remote_only_commands
                                            .push(make_command(&commit, &refs_map, true));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Walk local commits from HEAD
        let mut revwalk = self.repo.revwalk().context("Failed to create revwalk")?;
        revwalk.push(head_oid).context("Failed to push HEAD")?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut inserted_remote = false;
        for oid_result in revwalk.take(limit) {
            let oid = match oid_result {
                Ok(oid) => oid,
                Err(_) => continue,
            };

            let short_sha = format!("{:.7}", oid);

            // Insert remote-only commits right before the merge base
            if !inserted_remote && merge_base_sha.as_ref() == Some(&short_sha) {
                commands.append(&mut remote_only_commands);
                inserted_remote = true;
            }

            if let Ok(commit) = self.repo.find_commit(oid) {
                commands.push(make_command(&commit, &refs_map, false));
            }
        }

        // If we never hit the merge base (e.g., it's beyond our limit), append at end
        if !inserted_remote && !remote_only_commands.is_empty() {
            commands.append(&mut remote_only_commands);
        }

        Ok(commands)
    }

    /// Get commits unique to a branch (not reachable from current branch)
    /// Always includes at least the tip commit so branches are never empty
    pub fn commit_log_for_branch(&self, branch_name: &str) -> Result<Vec<GitCommand>> {
        let mut commands = Vec::new();

        // Find the branch (try local first, then remote)
        let branch = self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)
            .or_else(|_| self.repo.find_branch(branch_name, git2::BranchType::Remote))
            .context(format!("Failed to find branch '{branch_name}'"))?;

        let branch_ref = branch.get();
        let branch_oid = match branch_ref.target() {
            Some(oid) => oid,
            None => return Ok(commands),
        };

        // Compare against current branch (HEAD) to show only unique commits
        let current_branch_oid = self
            .repo
            .head()
            .ok()
            .and_then(|h| h.target());

        // Collect refs for decorations
        let refs_map = self.collect_refs();

        // Helper to create GitCommand from commit
        let make_command = |commit: &git2::Commit<'_>, refs_map: &HashMap<String, Vec<RefDecoration>>| {
            let message = commit.summary().unwrap_or("").to_string();
            let time = commit.time();
            let timestamp = Local
                .timestamp_opt(time.seconds(), 0)
                .single()
                .unwrap_or_else(Local::now);
            let short_sha = format!("{:.7}", commit.id());
            let decorations = refs_map.get(&short_sha).cloned().unwrap_or_default();
            GitCommand {
                timestamp,
                command_type: CommandType::Commit,
                message,
                sha: Some(short_sha),
                decorations,
                is_remote_only: false,
            }
        };

        // Walk commits from the branch tip, excluding current branch
        let mut revwalk = self.repo.revwalk().context("Failed to create revwalk")?;
        revwalk.push(branch_oid).context("Failed to push branch OID")?;
        if let Some(current_oid) = current_branch_oid {
            let _ = revwalk.hide(current_oid); // Exclude commits reachable from current branch
        }
        revwalk.set_sorting(git2::Sort::TIME)?;

        for oid_result in revwalk {
            let oid = match oid_result {
                Ok(oid) => oid,
                Err(_) => continue,
            };

            let commit = match self.repo.find_commit(oid) {
                Ok(c) => c,
                Err(_) => continue,
            };

            commands.push(make_command(&commit, &refs_map));
        }

        // Always show at least the tip commit (for branches like main that share ancestry)
        if commands.is_empty() {
            if let Ok(tip_commit) = self.repo.find_commit(branch_oid) {
                commands.push(make_command(&tip_commit, &refs_map));
            }
        }

        Ok(commands)
    }

    /// List all branches (local and remote) with their info
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        let mut branches = Vec::new();

        // Get current branch name for comparison
        let current_branch = self
            .repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from));

        // Get upstream branch name to filter it from remote list
        let upstream_name = current_branch.as_ref().and_then(|branch_name| {
            self.repo
                .find_branch(branch_name, git2::BranchType::Local)
                .ok()
                .and_then(|branch| branch.upstream().ok())
                .and_then(|upstream| upstream.name().ok().flatten().map(String::from))
        });

        // Iterate through local branches
        let branch_iter = self.repo.branches(Some(git2::BranchType::Local))?;

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

        // Iterate through remote branches
        let remote_iter = self.repo.branches(Some(git2::BranchType::Remote))?;

        for branch_result in remote_iter {
            let (branch, _branch_type) = branch_result?;

            let name = match branch.name()? {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Skip HEAD refs (e.g., origin/HEAD)
            if name.ends_with("/HEAD") {
                continue;
            }

            // Skip upstream of current branch (already shown in history header)
            if upstream_name.as_ref() == Some(&name) {
                continue;
            }

            branches.push(BranchInfo {
                name,
                is_current: false,
                is_remote: true,
            });
        }

        // Sort: current branch first, then local branches, then remote branches
        branches.sort_by(|a, b| {
            match (a.is_current, b.is_current) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            match (a.is_remote, b.is_remote) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
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
        let author_name = author.name().unwrap_or("Unknown").to_string();
        let author_email = author.email().unwrap_or("").to_string();
        let author_time = {
            let time = author.when();
            let secs = time.seconds();
            let offset_mins = time.offset_minutes();
            let offset = chrono::FixedOffset::east_opt(offset_mins * 60)
                .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).unwrap());
            DateTime::from_timestamp(secs, 0)
                .map(|dt| dt.with_timezone(&offset).with_timezone(&Local))
                .unwrap_or_else(Local::now)
        };

        // Extract committer info
        let committer = commit.committer();
        let committer_name = committer.name().unwrap_or("Unknown").to_string();
        let committer_email = committer.email().unwrap_or("").to_string();
        let committer_time = {
            let time = committer.when();
            let secs = time.seconds();
            let offset_mins = time.offset_minutes();
            let offset = chrono::FixedOffset::east_opt(offset_mins * 60)
                .unwrap_or_else(|| chrono::FixedOffset::east_opt(0).unwrap());
            DateTime::from_timestamp(secs, 0)
                .map(|dt| dt.with_timezone(&offset).with_timezone(&Local))
                .unwrap_or_else(Local::now)
        };

        // Get full commit message
        let message = commit.message().unwrap_or("").to_string();

        // Check for GPG signature
        let gpg_status = commit
            .raw_header()
            .and_then(|header| {
                if header.contains("gpgsig") {
                    Some("Signed".to_string())
                } else {
                    None
                }
            });

        // Get files changed by diffing against parent
        let tree = commit.tree().context("Failed to get commit tree")?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .context("Failed to diff trees")?;

        // Get overall stats
        let stats = diff.stats().context("Failed to get diff stats")?;
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

                file_stats.borrow_mut().insert(path, (status, 0, 0));
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
            .map(|(path, (status, ins, del))| CommitFile {
                path,
                status,
                insertions: ins,
                deletions: del,
            })
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

        let tree = commit.tree().context("Failed to get commit tree")?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

        // Create diff with path filter
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(file_path);

        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))
            .context("Failed to diff trees")?;

        // Format as patch
        let mut output = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let prefix = match line.origin() {
                '+' | '-' | ' ' => format!("{}", line.origin()),
                _ => String::new(),
            };
            if let Ok(content) = std::str::from_utf8(line.content()) {
                output.push_str(&prefix);
                output.push_str(content);
            }
            true
        })?;

        if output.is_empty() {
            output = format!("(No changes for {file_path})");
        }

        Ok(output)
    }

    /// Populate file statuses
    fn populate_file_statuses(&self, status: &mut GitStatus) -> Result<()> {
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false)
            .include_unmodified(false);

        let statuses = self
            .repo
            .statuses(Some(&mut opts))
            .context("Failed to get git status")?;

        for entry in statuses.iter() {
            let path = entry
                .path()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("<invalid>"));

            let git_status = entry.status();

            let file_status = FileStatus {
                path,
                working: working_state_from_git2(git_status),
                staged: staged_state_from_git2(git_status),
            };

            // Only add if there's an actual change
            if file_status.working.is_changed() || file_status.staged.is_changed() {
                status.files.push(file_status);
            }
        }

        // Sort by path
        status.files.sort_by(|a, b| a.path.cmp(&b.path));

        Ok(())
    }
}

/// Extract working directory state from git2 status
fn working_state_from_git2(status: Status) -> FileState {
    if status.is_wt_new() {
        FileState::Untracked
    } else if status.is_wt_modified() {
        FileState::Modified
    } else if status.is_wt_deleted() {
        FileState::Deleted
    } else if status.is_wt_renamed() {
        FileState::Renamed
    } else if status.is_conflicted() {
        FileState::Conflicted
    } else {
        FileState::Unmodified
    }
}

/// Extract staged (index) state from git2 status
fn staged_state_from_git2(status: Status) -> FileState {
    if status.is_index_new() {
        FileState::Added
    } else if status.is_index_modified() {
        FileState::Modified
    } else if status.is_index_deleted() {
        FileState::Deleted
    } else if status.is_index_renamed() {
        FileState::Renamed
    } else {
        FileState::Unmodified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod file_state {
        use super::*;

        #[test]
        fn as_char_returns_correct_characters() {
            assert_eq!(FileState::Unmodified.as_char(), ' ');
            assert_eq!(FileState::Modified.as_char(), 'M');
            assert_eq!(FileState::Added.as_char(), 'A');
            assert_eq!(FileState::Deleted.as_char(), 'D');
            assert_eq!(FileState::Renamed.as_char(), 'R');
            assert_eq!(FileState::Untracked.as_char(), '?');
            assert_eq!(FileState::Ignored.as_char(), '!');
            assert_eq!(FileState::Conflicted.as_char(), 'U');
        }

        #[test]
        fn is_changed_returns_false_for_unchanged_states() {
            assert!(!FileState::Unmodified.is_changed());
            assert!(!FileState::Ignored.is_changed());
        }

        #[test]
        fn is_changed_returns_true_for_changed_states() {
            assert!(FileState::Modified.is_changed());
            assert!(FileState::Added.is_changed());
            assert!(FileState::Deleted.is_changed());
            assert!(FileState::Renamed.is_changed());
            assert!(FileState::Untracked.is_changed());
            assert!(FileState::Conflicted.is_changed());
        }
    }

    mod command_type {
        use super::*;

        #[test]
        fn from_message_parses_commit() {
            assert_eq!(CommandType::from_message("commit: initial commit"), CommandType::Commit);
            assert_eq!(CommandType::from_message("Commit (amend): fix typo"), CommandType::Commit);
        }

        #[test]
        fn from_message_parses_checkout() {
            assert_eq!(CommandType::from_message("checkout: moving from main to feature"), CommandType::Checkout);
        }

        #[test]
        fn from_message_parses_merge() {
            assert_eq!(CommandType::from_message("merge feature-branch: Fast-forward"), CommandType::Merge);
        }

        #[test]
        fn from_message_parses_rebase() {
            assert_eq!(CommandType::from_message("rebase (finish): refs/heads/main onto abc123"), CommandType::Rebase);
        }

        #[test]
        fn from_message_parses_pull() {
            assert_eq!(CommandType::from_message("pull: Fast-forward"), CommandType::Pull);
        }

        #[test]
        fn from_message_parses_reset() {
            assert_eq!(CommandType::from_message("reset: moving to HEAD~1"), CommandType::Reset);
        }

        #[test]
        fn from_message_parses_cherry_pick() {
            assert_eq!(CommandType::from_message("cherry-pick: picked commit abc123"), CommandType::CherryPick);
        }

        #[test]
        fn from_message_parses_revert() {
            assert_eq!(CommandType::from_message("revert: reverting abc123"), CommandType::Revert);
        }

        #[test]
        fn from_message_parses_branch() {
            assert_eq!(CommandType::from_message("branch: created from HEAD"), CommandType::Branch);
        }

        #[test]
        fn from_message_parses_clone() {
            assert_eq!(CommandType::from_message("clone: from https://github.com/user/repo"), CommandType::Clone);
        }

        #[test]
        fn from_message_parses_init() {
            // "initial" anywhere in the message (that doesn't start with other keywords)
            assert_eq!(CommandType::from_message("initial commit"), CommandType::Init);
        }

        #[test]
        fn from_message_returns_other_for_unknown() {
            assert_eq!(CommandType::from_message("unknown operation"), CommandType::Other);
            assert_eq!(CommandType::from_message(""), CommandType::Other);
        }

        #[test]
        fn icon_returns_non_empty_string() {
            let types = [
                CommandType::Commit,
                CommandType::Checkout,
                CommandType::Merge,
                CommandType::Rebase,
                CommandType::Pull,
                CommandType::Push,
                CommandType::Fetch,
                CommandType::Reset,
                CommandType::CherryPick,
                CommandType::Revert,
                CommandType::Branch,
                CommandType::Clone,
                CommandType::Init,
                CommandType::Stash,
                CommandType::Other,
            ];
            for cmd_type in types {
                assert!(!cmd_type.icon().is_empty(), "{cmd_type:?} should have non-empty icon");
            }
        }
    }

    mod git_status {
        use super::*;
        use std::path::PathBuf;

        fn make_file(path: &str, working: FileState, staged: FileState) -> FileStatus {
            FileStatus {
                path: PathBuf::from(path),
                working,
                staged,
            }
        }

        #[test]
        fn working_changes_filters_modified_files() {
            let status = GitStatus {
                files: vec![
                    make_file("changed.txt", FileState::Modified, FileState::Unmodified),
                    make_file("unchanged.txt", FileState::Unmodified, FileState::Unmodified),
                    make_file("added.txt", FileState::Added, FileState::Unmodified),
                ],
                ..Default::default()
            };
            let changes = status.working_changes();
            assert_eq!(changes.len(), 2);
            assert_eq!(changes[0].path, PathBuf::from("changed.txt"));
            assert_eq!(changes[1].path, PathBuf::from("added.txt"));
        }

        #[test]
        fn staged_changes_filters_staged_files() {
            let status = GitStatus {
                files: vec![
                    make_file("staged.txt", FileState::Unmodified, FileState::Modified),
                    make_file("unstaged.txt", FileState::Modified, FileState::Unmodified),
                    make_file("both.txt", FileState::Modified, FileState::Added),
                ],
                ..Default::default()
            };
            let staged = status.staged_changes();
            assert_eq!(staged.len(), 2);
            assert_eq!(staged[0].path, PathBuf::from("staged.txt"));
            assert_eq!(staged[1].path, PathBuf::from("both.txt"));
        }
    }
}
