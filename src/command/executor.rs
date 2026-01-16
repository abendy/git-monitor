//! Command execution implementation.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use super::CommandRequest;

/// Result of command execution
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields reserved for future diagnostics
pub struct CommandResult {
    /// Whether the command succeeded (exit code 0)
    pub success: bool,
    /// Exit code if available
    pub exit_code: Option<i32>,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// How long the command took
    pub duration: Duration,
}

impl CommandResult {
    /// Get the output to display.
    ///
    /// Prefers stderr for git commands (which output progress to stderr),
    /// falls back to stdout, then a placeholder.
    #[must_use]
    pub fn display_output(&self) -> &str {
        if !self.stderr.is_empty() {
            &self.stderr
        } else if !self.stdout.is_empty() {
            &self.stdout
        } else {
            "(no output)"
        }
    }

    /// Get line count of display output
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.display_output().lines().count()
    }
}

/// Executes commands with consistent handling
pub struct CommandExecutor {
    /// Default working directory for commands
    default_cwd: std::path::PathBuf,
}

impl CommandExecutor {
    /// Create a new executor with the given default working directory
    #[must_use]
    pub fn new(default_cwd: impl Into<std::path::PathBuf>) -> Self {
        Self {
            default_cwd: default_cwd.into(),
        }
    }

    /// Execute a command request
    pub fn execute(&self, request: &CommandRequest) -> CommandResult {
        let start = Instant::now();
        let cwd = request
            .cwd
            .as_deref()
            .unwrap_or(&self.default_cwd);

        self.execute_in_dir(
            &request.program,
            &request.args,
            cwd,
            start,
        )
    }

    /// Execute a command in a specific directory
    fn execute_in_dir(
        &self,
        program: &str,
        args: &[String],
        cwd: &Path,
        start: Instant,
    ) -> CommandResult {
        let result = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .output();

        match result {
            Ok(output) => CommandResult {
                success: output.status.success(),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                duration: start.elapsed(),
            },
            Err(e) => CommandResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Failed to execute: {e}"),
                duration: start.elapsed(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_output_prefers_stderr() {
        let result = CommandResult {
            success: true,
            exit_code: Some(0),
            stdout: "stdout content".to_string(),
            stderr: "stderr content".to_string(),
            duration: Duration::from_millis(100),
        };
        assert_eq!(
            result.display_output(),
            "stderr content"
        );
    }

    #[test]
    fn test_display_output_falls_back_to_stdout() {
        let result = CommandResult {
            success: true,
            exit_code: Some(0),
            stdout: "stdout content".to_string(),
            stderr: String::new(),
            duration: Duration::from_millis(100),
        };
        assert_eq!(
            result.display_output(),
            "stdout content"
        );
    }

    #[test]
    fn test_display_output_placeholder() {
        let result = CommandResult {
            success: true,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            duration: Duration::from_millis(100),
        };
        assert_eq!(result.display_output(), "(no output)");
    }
}
