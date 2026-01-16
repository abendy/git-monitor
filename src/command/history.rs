//! Command history management.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// Maximum number of commands to keep in history
const MAX_HISTORY: usize = 100;

/// History file name in home directory
const HISTORY_FILE: &str = ".git-monitor-history";

/// Manages command history with persistence
#[derive(Debug, Clone)]
pub struct CommandHistory {
    /// Commands in history (oldest first)
    commands: Vec<String>,
    /// Current navigation index (None = not navigating)
    nav_index: Option<usize>,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHistory {
    /// Create a new history, loading from disk
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: load_from_disk(),
            nav_index: None,
        }
    }

    /// Add a command to history (deduplicates against last entry)
    pub fn add(&mut self, command: impl Into<String>) {
        let command = command.into();
        if command.trim().is_empty() {
            return;
        }

        // Don't add if it's the same as the last command
        if self.commands.last().map(String::as_str) == Some(&command) {
            return;
        }

        self.commands.push(command);

        // Trim to max size
        while self.commands.len() > MAX_HISTORY {
            self.commands.remove(0);
        }

        // Save to disk
        save_to_disk(&self.commands);
    }

    /// Get all commands (oldest first)
    #[must_use]
    pub fn commands(&self) -> &[String] {
        &self.commands
    }

    /// Start or continue navigation to older command
    pub fn navigate_older(&mut self) -> Option<&str> {
        if self.commands.is_empty() {
            return None;
        }

        let new_idx = match self.nav_index {
            None => self.commands.len().saturating_sub(1),
            Some(idx) => idx.saturating_sub(1),
        };

        self.nav_index = Some(new_idx);
        self.commands
            .get(new_idx)
            .map(String::as_str)
    }

    /// Navigate to newer command
    pub fn navigate_newer(&mut self) -> Option<&str> {
        let idx = self.nav_index?;
        let new_idx = idx + 1;

        if new_idx >= self.commands.len() {
            // Past the end, exit navigation
            self.nav_index = None;
            return None;
        }

        self.nav_index = Some(new_idx);
        self.commands
            .get(new_idx)
            .map(String::as_str)
    }

    /// Reset navigation state
    pub fn reset_navigation(&mut self) {
        self.nav_index = None;
    }

    /// Check if currently navigating history
    #[must_use]
    #[allow(dead_code)] // API for future use
    pub fn is_navigating(&self) -> bool {
        self.nav_index.is_some()
    }
}

/// Get the path to the history file
fn history_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(HISTORY_FILE))
}

/// Load history from disk
fn load_from_disk() -> Vec<String> {
    let Some(path) = history_file_path() else {
        return Vec::new();
    };

    let Ok(file) = File::open(&path) else {
        return Vec::new();
    };

    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .take(MAX_HISTORY)
        .collect()
}

/// Save history to disk
fn save_to_disk(history: &[String]) {
    let Some(path) = history_file_path() else {
        return;
    };

    let Ok(mut file) = File::create(&path) else {
        return;
    };

    for cmd in history.iter().take(MAX_HISTORY) {
        let _ = writeln!(file, "{cmd}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_deduplicates() {
        let mut history = CommandHistory {
            commands: vec!["git status".to_string()],
            nav_index: None,
        };

        history.add("git status");
        assert_eq!(history.commands.len(), 1);

        history.add("git log");
        assert_eq!(history.commands.len(), 2);
    }

    #[test]
    fn test_navigation() {
        let mut history = CommandHistory {
            commands: vec![
                "git status".to_string(),
                "git log".to_string(),
                "git diff".to_string(),
            ],
            nav_index: None,
        };

        assert_eq!(
            history.navigate_older(),
            Some("git diff")
        );
        assert_eq!(
            history.navigate_older(),
            Some("git log")
        );
        assert_eq!(
            history.navigate_older(),
            Some("git status")
        );
        assert_eq!(
            history.navigate_older(),
            Some("git status")
        ); // Stays at oldest

        assert_eq!(
            history.navigate_newer(),
            Some("git log")
        );
        assert_eq!(
            history.navigate_newer(),
            Some("git diff")
        );
        assert_eq!(history.navigate_newer(), None); // Past end
    }
}
