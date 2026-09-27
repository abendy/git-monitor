//! Unified command execution framework.
//!
//! Provides a single entry point for executing commands (git, system, external)
//! with consistent output handling, history recording, and feedback.

mod executor;
mod external;
mod history;

use std::path::{Path, PathBuf};

pub use executor::{CommandExecutor, CommandResult};
pub use external::ExternalCommand;
pub use history::CommandHistory;

/// Source of command execution - affects feedback behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)] // Variants reserved for expanded feedback policy
pub enum CommandSource {
    /// Direct keyboard shortcut
    #[default]
    Keyboard,
    /// Command palette (: mode)
    Palette,
    /// Context action menu (m key)
    ActionMenu,
    /// Alias browser (a key)
    AliasBrowser,
    /// Internal/automatic command
    Internal,
}

/// Policy for displaying command feedback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)] // Variants reserved for expanded feedback policy
pub enum FeedbackPolicy {
    /// Use source-based defaults
    #[default]
    Default,
    /// Always show popup regardless of output length
    AlwaysPopup,
    /// Only show inline, never auto-popup
    InlineOnly,
    /// No visible feedback (background operations)
    Silent,
    /// External command requiring TUI suspension
    External,
}

/// A request to execute a command
#[derive(Debug, Clone)]
pub struct CommandRequest {
    /// Program to execute (e.g., "git", "cargo", "make")
    pub program: String,
    /// Arguments to pass to the program
    pub args: Vec<String>,
    /// Human-readable description for history/popup title
    pub display_name: String,
    /// Working directory (None = use default repo path)
    pub cwd: Option<PathBuf>,
    /// How this command was triggered
    pub source: CommandSource,
    /// Feedback display policy
    pub feedback: FeedbackPolicy,
    /// Whether to refresh git status after execution
    pub refresh_after: bool,
    /// Whether the user already confirmed this command in a dialog
    pub confirmed: bool,
}

impl CommandRequest {
    /// Create a new git command request
    pub fn git(args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let args: Vec<String> = args
            .into_iter()
            .map(Into::into)
            .collect();
        let display_name = format!("git {}", args.join(" "));
        Self {
            program: "git".to_string(),
            args,
            display_name,
            cwd: None,
            source: CommandSource::default(),
            feedback: FeedbackPolicy::default(),
            refresh_after: true,
            confirmed: false,
        }
    }

    /// Create a request that runs a typed command line through the user's shell.
    ///
    /// The whole line goes to `$SHELL -c` (falling back to `sh`), so quotes,
    /// pipes, and variables behave as they do at a prompt. Use only for text
    /// the user typed; never assemble the line from other data.
    pub fn shell_line(input: &str) -> Option<Self> {
        let line = input.trim();
        if line.is_empty() {
            return None;
        }

        let shell = std::env::var("SHELL")
            .ok()
            .filter(|shell| !shell.trim().is_empty())
            .unwrap_or_else(|| "sh".to_string());

        Some(Self {
            program: shell,
            args: vec!["-c".to_string(), line.to_string()],
            display_name: line.to_string(),
            cwd: None,
            source: CommandSource::Palette,
            feedback: FeedbackPolicy::default(),
            refresh_after: true,
            confirmed: false,
        })
    }

    /// Create a command request from a full command string
    pub fn from_input(input: &str) -> Option<Self> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let program = parts[0].to_string();
        let args: Vec<String> = parts[1..]
            .iter()
            .map(|s| (*s).to_string())
            .collect();

        Some(Self {
            program,
            args,
            display_name: input.to_string(),
            cwd: None,
            source: CommandSource::Palette,
            feedback: FeedbackPolicy::default(),
            refresh_after: true,
            confirmed: false,
        })
    }

    /// Mark the command as confirmed by the user
    #[must_use]
    pub const fn confirmed(mut self) -> Self {
        self.confirmed = true;
        self
    }

    /// Plain-language description of what this command can destroy or publish,
    /// if it is a Git command that must be confirmed before running from a key
    /// or menu.
    #[must_use]
    pub fn destructive_effect(&self) -> Option<&'static str> {
        if self.program != "git" {
            return None;
        }

        // Skip global options (`-C <path>`, `-c <key=value>`) to find the subcommand
        let mut args = self.args.iter().map(String::as_str);
        let subcommand = loop {
            match args.next()? {
                "-C" | "-c" => {
                    args.next()?;
                }
                arg if arg.starts_with('-') => {}
                arg => break arg,
            }
        };
        let rest: Vec<&str> = args.collect();
        let has = |flags: &[&str]| {
            rest.iter()
                .any(|arg| flags.contains(arg))
        };

        match subcommand {
            "reset" => {
                Some("Moves the current branch. With --hard it also discards uncommitted changes.")
            }
            "rebase" => Some("Rewrites commits on the current branch."),
            "branch" if has(&["-d", "-D", "--delete"]) => Some("Deletes a branch."),
            "push"
                if has(&["-f", "--force", "-d", "--delete", "--mirror"])
                    || rest.iter().any(|arg| {
                        arg.starts_with("--force-with-lease")
                            || arg.starts_with("--force-if-includes")
                            || arg.starts_with('+')
                            || arg.starts_with(':')
                    }) =>
            {
                Some("Force-pushes or deletes on the remote.")
            }
            "clean" if !has(&["-n", "--dry-run"]) => Some("Deletes untracked files."),
            "stash" if has(&["drop", "clear"]) => Some("Deletes stashed changes."),
            _ => None,
        }
    }

    /// Set the command source
    #[must_use]
    pub const fn with_source(mut self, source: CommandSource) -> Self {
        self.source = source;
        self
    }

    /// Set the feedback policy
    #[must_use]
    #[allow(dead_code)] // Builder method for future use
    pub const fn with_feedback(mut self, policy: FeedbackPolicy) -> Self {
        self.feedback = policy;
        self
    }

    /// Set whether to refresh after execution
    #[must_use]
    #[allow(dead_code)] // Builder method for future use
    pub const fn with_refresh(mut self, refresh: bool) -> Self {
        self.refresh_after = refresh;
        self
    }

    /// Set custom working directory
    #[must_use]
    #[allow(dead_code)] // Builder method for future use
    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// Set custom display name
    #[must_use]
    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = name.into();
        self
    }

    /// Create a git alias command request
    pub fn git_alias(name: &str, command: &str, repo_path: &Path) -> Self {
        // Git aliases are expanded as "git <command>"
        let args: Vec<String> = command
            .split_whitespace()
            .map(String::from)
            .collect();
        Self {
            program: "git".to_string(),
            args,
            display_name: format!("git {name}"),
            cwd: Some(repo_path.to_path_buf()),
            source: CommandSource::default(),
            feedback: FeedbackPolicy::default(),
            refresh_after: true,
            confirmed: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod command_source {
        use super::*;

        #[test]
        fn default_is_keyboard() {
            let source = CommandSource::default();
            assert_eq!(source, CommandSource::Keyboard);
        }
    }

    mod feedback_policy {
        use super::*;

        #[test]
        fn default_is_default_policy() {
            let policy = FeedbackPolicy::default();
            assert_eq!(policy, FeedbackPolicy::Default);
        }
    }

    mod command_request {
        use super::*;

        #[test]
        fn git_creates_git_command() {
            let request = CommandRequest::git(["status", "-sb"]);

            assert_eq!(request.program, "git");
            assert_eq!(request.args, vec!["status", "-sb"]);
            assert_eq!(request.display_name, "git status -sb");
            assert!(request.refresh_after);
        }

        #[test]
        fn git_with_vec_args() {
            let args = vec!["log", "--oneline", "-n", "10"];
            let request = CommandRequest::git(args);

            assert_eq!(request.program, "git");
            assert_eq!(
                request.args,
                vec!["log", "--oneline", "-n", "10"]
            );
        }

        fn effect(line: &str) -> Option<&'static str> {
            CommandRequest::from_input(line)
                .unwrap()
                .destructive_effect()
        }

        #[test]
        fn destructive_git_commands_need_confirmation() {
            for line in [
                "git reset --hard HEAD~1",
                "git reset HEAD~1",
                "git rebase -i main",
                "git branch -D feature",
                "git branch --delete feature",
                "git push --force",
                "git push -f origin main",
                "git push --force-with-lease",
                "git push origin +main",
                "git push origin :old-branch",
                "git push --delete origin old-branch",
                "git clean -fd",
                "git stash drop",
                "git stash clear",
                "git -C sub reset --hard",
            ] {
                assert!(
                    effect(line).is_some(),
                    "should confirm: {line}"
                );
            }
        }

        #[test]
        fn safe_git_commands_run_without_confirmation() {
            for line in [
                "git status",
                "git push",
                "git push -u origin main",
                "git branch -a",
                "git clean -n",
                "git stash list",
                "git log --oneline",
                "git -c color.ui=always status",
                "ls -la",
            ] {
                assert!(
                    effect(line).is_none(),
                    "should not confirm: {line}"
                );
            }
        }

        #[test]
        fn shell_line_passes_the_whole_line_to_the_shell() {
            let line = r#"echo "two words" | tr a-z A-Z"#;
            let request = CommandRequest::shell_line(&format!("  {line}  ")).unwrap();

            assert_eq!(request.args, vec!["-c", line]);
            assert_eq!(request.display_name, line);
        }

        #[test]
        fn shell_line_returns_none_for_blank_input() {
            assert!(CommandRequest::shell_line("   ").is_none());
        }

        #[test]
        fn from_input_parses_simple_command() {
            let request = CommandRequest::from_input("ls -la").unwrap();

            assert_eq!(request.program, "ls");
            assert_eq!(request.args, vec!["-la"]);
            assert_eq!(request.display_name, "ls -la");
            assert_eq!(request.source, CommandSource::Palette);
        }

        #[test]
        fn from_input_parses_git_command() {
            let request = CommandRequest::from_input("git status").unwrap();

            assert_eq!(request.program, "git");
            assert_eq!(request.args, vec!["status"]);
        }

        #[test]
        fn from_input_returns_none_for_empty() {
            let request = CommandRequest::from_input("");

            assert!(request.is_none());
        }

        #[test]
        fn from_input_returns_none_for_whitespace() {
            let request = CommandRequest::from_input("   ");

            assert!(request.is_none());
        }

        #[test]
        fn from_input_handles_multiple_spaces() {
            let request = CommandRequest::from_input("git   status  -sb").unwrap();

            assert_eq!(request.program, "git");
            assert_eq!(request.args, vec!["status", "-sb"]);
        }

        #[test]
        fn with_source_sets_source() {
            let request = CommandRequest::git(["status"]).with_source(CommandSource::ActionMenu);

            assert_eq!(
                request.source,
                CommandSource::ActionMenu
            );
        }

        #[test]
        fn with_feedback_sets_policy() {
            let request =
                CommandRequest::git(["status"]).with_feedback(FeedbackPolicy::AlwaysPopup);

            assert_eq!(
                request.feedback,
                FeedbackPolicy::AlwaysPopup
            );
        }

        #[test]
        fn with_refresh_sets_refresh() {
            let request = CommandRequest::git(["log"]).with_refresh(false);

            assert!(!request.refresh_after);
        }

        #[test]
        fn with_cwd_sets_directory() {
            let request = CommandRequest::git(["status"]).with_cwd(PathBuf::from("/tmp"));

            assert_eq!(request.cwd, Some(PathBuf::from("/tmp")));
        }

        #[test]
        fn with_display_name_sets_name() {
            let request = CommandRequest::git(["status"]).with_display_name("Check status");

            assert_eq!(request.display_name, "Check status");
        }

        #[test]
        fn git_alias_creates_alias_command() {
            let request = CommandRequest::git_alias("st", "status -sb", Path::new("/repo"));

            assert_eq!(request.program, "git");
            assert_eq!(request.args, vec!["status", "-sb"]);
            assert_eq!(request.display_name, "git st");
            assert_eq!(
                request.cwd,
                Some(PathBuf::from("/repo"))
            );
        }

        #[test]
        fn git_alias_with_complex_command() {
            let request = CommandRequest::git_alias(
                "lg",
                "log --oneline --graph --all",
                Path::new("/repo"),
            );

            assert_eq!(
                request.args,
                vec!["log", "--oneline", "--graph", "--all"]
            );
            assert_eq!(request.display_name, "git lg");
        }

        #[test]
        fn builder_chain_works() {
            let request = CommandRequest::git(["push"])
                .with_source(CommandSource::ActionMenu)
                .with_feedback(FeedbackPolicy::AlwaysPopup)
                .with_refresh(true)
                .with_display_name("Push to origin");

            assert_eq!(request.program, "git");
            assert_eq!(
                request.source,
                CommandSource::ActionMenu
            );
            assert_eq!(
                request.feedback,
                FeedbackPolicy::AlwaysPopup
            );
            assert!(request.refresh_after);
            assert_eq!(request.display_name, "Push to origin");
        }
    }
}
