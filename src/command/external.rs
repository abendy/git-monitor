//! External command execution (TUI-suspending commands).
//!
//! External commands run with inherited stdio and require TUI suspension.
//! Examples: pager diffs, difftool, interactive rebase.

use std::path::Path;
use std::process::{Command, Stdio};

/// Commands that require TUI suspension to run interactively
#[derive(Debug, Clone)]
#[allow(dead_code)] // InteractiveRebase variant reserved for upcoming feature
pub enum ExternalCommand {
    /// Show commit file diff with pager (uses core.pager from gitconfig)
    PagerDiff {
        commit_sha: String,
        file_path: String,
    },
    /// Show commit file diff with difftool (uses diff.tool from gitconfig)
    DiffTool {
        commit_sha: String,
        file_path: String,
    },
    /// Show working/staged file diff with pager
    FilePagerDiff { file_path: String, staged: bool },
    /// Show working/staged file diff with difftool
    FileDiffTool { file_path: String, staged: bool },
    /// Interactive rebase (suspends TUI for editor)
    InteractiveRebase {
        /// Base commit for rebase (e.g., "HEAD~3", commit SHA)
        onto: String,
    },
}

impl ExternalCommand {
    /// Build and execute the command in the given directory.
    ///
    /// Returns the exit status. Caller is responsible for TUI suspension.
    pub fn execute(&self, repo_path: &Path) -> std::io::Result<std::process::ExitStatus> {
        let mut cmd = self.build_command();
        cmd.current_dir(repo_path)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
    }

    /// Build the command without executing it.
    #[must_use]
    fn build_command(&self) -> Command {
        let mut cmd = Command::new("git");

        match self {
            Self::PagerDiff {
                commit_sha,
                file_path,
            } => {
                cmd.args(["--paginate", "show", commit_sha, "--", file_path]);
            }
            Self::DiffTool {
                commit_sha,
                file_path,
            } => {
                cmd.args([
                    "difftool",
                    "--no-prompt",
                    &format!("{commit_sha}~1..{commit_sha}"),
                    "--",
                    file_path,
                ]);
            }
            Self::FilePagerDiff { file_path, staged } => {
                let mut args = vec!["--paginate", "diff"];
                if *staged {
                    args.push("--staged");
                }
                args.extend(["--", file_path]);
                cmd.args(&args);
            }
            Self::FileDiffTool { file_path, staged } => {
                let mut args = vec!["difftool", "--no-prompt"];
                if *staged {
                    args.push("--staged");
                }
                args.extend(["--", file_path]);
                cmd.args(&args);
            }
            Self::InteractiveRebase { onto } => {
                cmd.args(["rebase", "-i", onto]);
            }
        }

        cmd
    }

    /// Human-readable description for display/logging
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::PagerDiff {
                commit_sha,
                file_path,
            } => format!("git show {commit_sha} -- {file_path}"),
            Self::DiffTool {
                commit_sha,
                file_path,
            } => format!("git difftool {commit_sha} -- {file_path}"),
            Self::FilePagerDiff { file_path, staged } => {
                if *staged {
                    format!("git diff --staged -- {file_path}")
                } else {
                    format!("git diff -- {file_path}")
                }
            }
            Self::FileDiffTool { file_path, staged } => {
                if *staged {
                    format!("git difftool --staged -- {file_path}")
                } else {
                    format!("git difftool -- {file_path}")
                }
            }
            Self::InteractiveRebase { onto } => format!("git rebase -i {onto}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_pager_diff_description() {
        let cmd = ExternalCommand::FilePagerDiff {
            file_path: "src/main.rs".to_string(),
            staged: false,
        };
        assert_eq!(
            cmd.description(),
            "git diff -- src/main.rs"
        );

        let staged = ExternalCommand::FilePagerDiff {
            file_path: "src/main.rs".to_string(),
            staged: true,
        };
        assert_eq!(
            staged.description(),
            "git diff --staged -- src/main.rs"
        );
    }

    #[test]
    fn test_interactive_rebase_description() {
        let cmd = ExternalCommand::InteractiveRebase {
            onto: "HEAD~3".to_string(),
        };
        assert_eq!(
            cmd.description(),
            "git rebase -i HEAD~3"
        );
    }
}
