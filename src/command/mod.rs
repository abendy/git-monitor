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
        }
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
        })
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
            assert_eq!(request.args, vec!["log", "--oneline", "-n", "10"]);
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

            assert_eq!(request.source, CommandSource::ActionMenu);
        }

        #[test]
        fn with_feedback_sets_policy() {
            let request =
                CommandRequest::git(["status"]).with_feedback(FeedbackPolicy::AlwaysPopup);

            assert_eq!(request.feedback, FeedbackPolicy::AlwaysPopup);
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
            let request =
                CommandRequest::git_alias("st", "status -sb", Path::new("/repo"));

            assert_eq!(request.program, "git");
            assert_eq!(request.args, vec!["status", "-sb"]);
            assert_eq!(request.display_name, "git st");
            assert_eq!(request.cwd, Some(PathBuf::from("/repo")));
        }

        #[test]
        fn git_alias_with_complex_command() {
            let request = CommandRequest::git_alias(
                "lg",
                "log --oneline --graph --all",
                Path::new("/repo"),
            );

            assert_eq!(request.args, vec!["log", "--oneline", "--graph", "--all"]);
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
            assert_eq!(request.source, CommandSource::ActionMenu);
            assert_eq!(request.feedback, FeedbackPolicy::AlwaysPopup);
            assert!(request.refresh_after);
            assert_eq!(request.display_name, "Push to origin");
        }
    }
}
