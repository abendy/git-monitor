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
            });
        }

        Ok(commands)
    }

    /// Get commit history (git log)
    pub fn commit_log(&self, limit: usize) -> Result<Vec<GitCommand>> {
        let mut commands = Vec::new();

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

        // Walk commits
        let mut revwalk = self.repo.revwalk().context("Failed to create revwalk")?;
        revwalk.push(head_oid).context("Failed to push HEAD")?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        for oid_result in revwalk.take(limit) {
            let oid = match oid_result {
                Ok(oid) => oid,
                Err(_) => continue,
            };

            let commit = match self.repo.find_commit(oid) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Get commit message (first line)
            let message = commit
                .summary()
                .unwrap_or("")
                .to_string();

            // Parse timestamp
            let time = commit.time();
            let timestamp = Local
                .timestamp_opt(time.seconds(), 0)
                .single()
                .unwrap_or_else(Local::now);

            // Get short SHA
            let short_sha = format!("{:.7}", oid);

            // Look up decorations
            let decorations = refs_map.get(&short_sha).cloned().unwrap_or_default();

            commands.push(GitCommand {
                timestamp,
                command_type: CommandType::Commit,
                message,
                sha: Some(short_sha),
                decorations,
            });
        }

        Ok(commands)
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
