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
    /// Open a file or directory in $EDITOR
    OpenEditor { editor: String, path: String },
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
        match self {
            Self::PagerDiff {
                commit_sha,
                file_path,
            } => {
                let mut cmd = Command::new("git");
                cmd.args(["--paginate", "show", commit_sha, "--", file_path]);
                cmd
            }
            Self::DiffTool {
                commit_sha,
                file_path,
            } => {
                let mut cmd = Command::new("git");
                cmd.args([
                    "difftool",
                    "--no-prompt",
                    &format!("{commit_sha}~1..{commit_sha}"),
                    "--",
                    file_path,
                ]);
                cmd
            }
            Self::FilePagerDiff { file_path, staged } => {
                let mut cmd = Command::new("git");
                let mut args = vec!["--paginate", "diff"];
                if *staged {
                    args.push("--staged");
                }
                args.extend(["--", file_path]);
                cmd.args(&args);
                cmd
            }
            Self::FileDiffTool { file_path, staged } => {
                let mut cmd = Command::new("git");
                let mut args = vec!["difftool", "--no-prompt"];
                if *staged {
                    args.push("--staged");
                }
                args.extend(["--", file_path]);
                cmd.args(&args);
                cmd
            }
            Self::InteractiveRebase { onto } => {
                let mut cmd = Command::new("git");
                cmd.args(["rebase", "-i", onto]);
                cmd
            }
            Self::OpenEditor { editor, path } => build_editor_command(editor, path),
        }
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
            Self::OpenEditor { editor, path } => format!("{editor} {path}"),
        }
    }
}

fn build_editor_command(editor: &str, path: &str) -> Command {
    let mut parts = editor.split_whitespace();
    let program = parts.next().unwrap_or(editor);
    let mut cmd = Command::new(program);

    for arg in parts {
        cmd.arg(arg);
    }

    cmd.arg(path);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    mod description {
        use super::*;

        #[test]
        fn pager_diff_shows_commit_and_file() {
            let cmd = ExternalCommand::PagerDiff {
                commit_sha: "abc1234".to_string(),
                file_path: "src/main.rs".to_string(),
            };

            assert_eq!(cmd.description(), "git show abc1234 -- src/main.rs");
        }

        #[test]
        fn diff_tool_shows_commit_and_file() {
            let cmd = ExternalCommand::DiffTool {
                commit_sha: "abc1234".to_string(),
                file_path: "src/lib.rs".to_string(),
            };

            assert_eq!(cmd.description(), "git difftool abc1234 -- src/lib.rs");
        }

        #[test]
        fn file_pager_diff_unstaged_shows_diff() {
            let cmd = ExternalCommand::FilePagerDiff {
                file_path: "src/main.rs".to_string(),
                staged: false,
            };

            assert_eq!(cmd.description(), "git diff -- src/main.rs");
        }

        #[test]
        fn file_pager_diff_staged_includes_flag() {
            let cmd = ExternalCommand::FilePagerDiff {
                file_path: "src/main.rs".to_string(),
                staged: true,
            };

            assert_eq!(cmd.description(), "git diff --staged -- src/main.rs");
        }

        #[test]
        fn file_diff_tool_unstaged_shows_difftool() {
            let cmd = ExternalCommand::FileDiffTool {
                file_path: "src/app.rs".to_string(),
                staged: false,
            };

            assert_eq!(cmd.description(), "git difftool -- src/app.rs");
        }

        #[test]
        fn file_diff_tool_staged_includes_flag() {
            let cmd = ExternalCommand::FileDiffTool {
                file_path: "src/app.rs".to_string(),
                staged: true,
            };

            assert_eq!(cmd.description(), "git difftool --staged -- src/app.rs");
        }

        #[test]
        fn interactive_rebase_shows_onto() {
            let cmd = ExternalCommand::InteractiveRebase {
                onto: "HEAD~3".to_string(),
            };

            assert_eq!(cmd.description(), "git rebase -i HEAD~3");
        }

        #[test]
        fn interactive_rebase_with_sha() {
            let cmd = ExternalCommand::InteractiveRebase {
                onto: "abc1234".to_string(),
            };

            assert_eq!(cmd.description(), "git rebase -i abc1234");
        }

        #[test]
        fn open_editor_shows_command_and_path() {
            let cmd = ExternalCommand::OpenEditor {
                editor: "vim".to_string(),
                path: "/path/to/file.rs".to_string(),
            };

            assert_eq!(cmd.description(), "vim /path/to/file.rs");
        }

        #[test]
        fn open_editor_with_editor_args() {
            let cmd = ExternalCommand::OpenEditor {
                editor: "code --wait".to_string(),
                path: "/path/to/file.rs".to_string(),
            };

            assert_eq!(cmd.description(), "code --wait /path/to/file.rs");
        }
    }

    mod build_editor_command {
        use super::*;

        #[test]
        fn simple_editor_name() {
            let cmd = build_editor_command("vim", "file.txt");

            assert_eq!(cmd.get_program(), "vim");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["file.txt"]);
        }

        #[test]
        fn editor_with_single_arg() {
            let cmd = build_editor_command("code --wait", "file.txt");

            assert_eq!(cmd.get_program(), "code");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["--wait", "file.txt"]);
        }

        #[test]
        fn editor_with_multiple_args() {
            let cmd = build_editor_command("emacs -nw --no-splash", "project/main.rs");

            assert_eq!(cmd.get_program(), "emacs");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["-nw", "--no-splash", "project/main.rs"]);
        }

        #[test]
        fn empty_editor_uses_empty_as_program() {
            let cmd = build_editor_command("", "file.txt");

            assert_eq!(cmd.get_program(), "");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["file.txt"]);
        }
    }

    mod build_command {
        use super::*;

        #[test]
        fn pager_diff_uses_git_paginate() {
            let external = ExternalCommand::PagerDiff {
                commit_sha: "abc1234".to_string(),
                file_path: "src/main.rs".to_string(),
            };

            let cmd = external.build_command();

            assert_eq!(cmd.get_program(), "git");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["--paginate", "show", "abc1234", "--", "src/main.rs"]);
        }

        #[test]
        fn diff_tool_uses_parent_commit_range() {
            let external = ExternalCommand::DiffTool {
                commit_sha: "abc1234".to_string(),
                file_path: "src/lib.rs".to_string(),
            };

            let cmd = external.build_command();

            assert_eq!(cmd.get_program(), "git");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(
                args,
                vec!["difftool", "--no-prompt", "abc1234~1..abc1234", "--", "src/lib.rs"]
            );
        }

        #[test]
        fn file_pager_diff_unstaged() {
            let external = ExternalCommand::FilePagerDiff {
                file_path: "src/app.rs".to_string(),
                staged: false,
            };

            let cmd = external.build_command();

            assert_eq!(cmd.get_program(), "git");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["--paginate", "diff", "--", "src/app.rs"]);
        }

        #[test]
        fn file_pager_diff_staged() {
            let external = ExternalCommand::FilePagerDiff {
                file_path: "src/app.rs".to_string(),
                staged: true,
            };

            let cmd = external.build_command();

            assert_eq!(cmd.get_program(), "git");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["--paginate", "diff", "--staged", "--", "src/app.rs"]);
        }

        #[test]
        fn file_diff_tool_unstaged() {
            let external = ExternalCommand::FileDiffTool {
                file_path: "README.md".to_string(),
                staged: false,
            };

            let cmd = external.build_command();

            assert_eq!(cmd.get_program(), "git");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["difftool", "--no-prompt", "--", "README.md"]);
        }

        #[test]
        fn file_diff_tool_staged() {
            let external = ExternalCommand::FileDiffTool {
                file_path: "Cargo.toml".to_string(),
                staged: true,
            };

            let cmd = external.build_command();

            assert_eq!(cmd.get_program(), "git");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["difftool", "--no-prompt", "--staged", "--", "Cargo.toml"]);
        }

        #[test]
        fn interactive_rebase_command() {
            let external = ExternalCommand::InteractiveRebase {
                onto: "HEAD~5".to_string(),
            };

            let cmd = external.build_command();

            assert_eq!(cmd.get_program(), "git");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["rebase", "-i", "HEAD~5"]);
        }

        #[test]
        fn open_editor_command() {
            let external = ExternalCommand::OpenEditor {
                editor: "nvim".to_string(),
                path: "/tmp/test.txt".to_string(),
            };

            let cmd = external.build_command();

            assert_eq!(cmd.get_program(), "nvim");
            let args: Vec<_> = cmd.get_args().collect();
            assert_eq!(args, vec!["/tmp/test.txt"]);
        }
    }

    mod external_command_variants {
        use super::*;

        #[test]
        fn pager_diff_is_debug_printable() {
            let cmd = ExternalCommand::PagerDiff {
                commit_sha: "abc".to_string(),
                file_path: "test".to_string(),
            };
            let debug = format!("{:?}", cmd);
            assert!(debug.contains("PagerDiff"));
        }

        #[test]
        fn diff_tool_is_clonable() {
            let cmd = ExternalCommand::DiffTool {
                commit_sha: "abc".to_string(),
                file_path: "test".to_string(),
            };
            let cloned = cmd.clone();
            assert_eq!(cmd.description(), cloned.description());
        }
    }
}
