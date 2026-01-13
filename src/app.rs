use std::{path::PathBuf, process::Command, sync::mpsc::Sender};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::warn;

use crate::{
    config::GitConfig,
    event::Event,
    git::{GitCommand, GitRepo, GitStatus},
    tui::Tui,
    ui,
    watcher::{RepoWatcher, WatchEvent},
};

/// Maximum number of activity entries to keep
const MAX_ACTIVITY: usize = 50;

/// Maximum number of commands to keep in history
const MAX_COMMAND_HISTORY: usize = 100;

/// History display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryMode {
    /// Show reflog (git actions)
    Reflog,
    /// Show commit log
    #[default]
    CommitLog,
}

/// Application state
pub struct App {
    /// Path to the repository
    pub repo_path: PathBuf,
    /// Git repository handle
    repo: GitRepo,
    /// Current git status
    pub status: GitStatus,
    /// Recent git activity
    pub activity: Vec<GitCommand>,
    /// Git config with aliases
    pub config: GitConfig,
    /// File watcher
    #[allow(dead_code)]
    watcher: Option<RepoWatcher>,
    /// Whether the application is running
    pub running: bool,
    /// Selected index in main list (command → staged → working → activity)
    pub selected: usize,
    /// Show help overlay
    pub show_help: bool,
    /// Show diff overlay
    pub show_diff: bool,
    /// Current diff content
    pub diff_content: String,
    /// Diff file path (for title)
    pub diff_path: String,
    /// Error message to display
    pub error: Option<String>,
    /// Whether in command input mode
    pub command_mode: bool,
    /// Current command input buffer
    pub command_input: String,
    /// Last command output (stdout/stderr combined)
    pub command_output: String,
    /// Whether last command succeeded
    pub command_success: bool,
    /// Command history
    pub command_history: Vec<String>,
    /// Current position in history (for navigation)
    pub history_index: Option<usize>,
    /// Whether showing alias categories popup
    pub show_aliases: bool,
    /// Selected alias category index
    pub alias_section_selected: usize,
    /// Whether viewing aliases within a section
    pub show_section_aliases: bool,
    /// Selected alias within section
    pub alias_selected: usize,
    /// Current history display mode (reflog vs commit log)
    pub history_mode: HistoryMode,
}

impl App {
    /// Create a new application instance
    pub fn new(path: PathBuf) -> Result<Self> {
        let repo = GitRepo::open(&path)?;
        let repo_path = repo
            .workdir()
            .map(PathBuf::from)
            .unwrap_or_else(|| path.clone());

        let status = repo.status().unwrap_or_default();
        // Load commit log by default (matches HistoryMode::default())
        let activity = repo.commit_log(MAX_ACTIVITY).unwrap_or_default();
        let config = GitConfig::load(&repo_path).unwrap_or_default();

        Ok(Self {
            repo_path,
            repo,
            status,
            activity,
            config,
            watcher: None,
            running: true,
            selected: 0,
            show_help: false,
            show_diff: false,
            diff_content: String::new(),
            diff_path: String::new(),
            error: None,
            command_mode: false,
            command_input: String::new(),
            command_output: String::new(),
            command_success: true,
            command_history: Vec::new(),
            history_index: None,
            show_aliases: false,
            alias_section_selected: 0,
            show_section_aliases: false,
            alias_selected: 0,
            history_mode: HistoryMode::default(),
        })
    }

    /// Set up file watcher
    pub fn setup_watcher(&mut self, event_tx: Sender<Event>) -> Result<()> {
        // Create a channel to receive watch events
        let (watch_tx, watch_rx) = std::sync::mpsc::channel::<WatchEvent>();

        // Spawn a thread to forward watch events to the main event loop
        let tx = event_tx;
        std::thread::spawn(move || {
            while let Ok(_event) = watch_rx.recv() {
                // Any file change triggers a refresh
                if tx.send(Event::FileChanged).is_err() {
                    break;
                }
            }
        });

        // Create the watcher
        let watcher = RepoWatcher::new(&self.repo_path, watch_tx)?;
        self.watcher = Some(watcher);

        Ok(())
    }

    /// Refresh git status and activity
    pub fn refresh_status(&mut self) {
        match self.repo.status() {
            Ok(status) => {
                self.status = status;
                self.error = None;
            }
            Err(e) => {
                warn!("Failed to refresh git status: {}", e);
                self.error = Some(format!("Git error: {e}"));
            }
        }

        // Refresh activity log based on history mode
        self.refresh_activity();
    }

    /// Refresh activity based on current history mode
    fn refresh_activity(&mut self) {
        let result = match self.history_mode {
            HistoryMode::Reflog => self.repo.reflog(MAX_ACTIVITY),
            HistoryMode::CommitLog => self.repo.commit_log(MAX_ACTIVITY),
        };

        if let Ok(activity) = result {
            self.activity = activity;
        }
    }

    /// Toggle history mode between reflog and commit log
    fn toggle_history_mode(&mut self) {
        self.history_mode = match self.history_mode {
            HistoryMode::Reflog => HistoryMode::CommitLog,
            HistoryMode::CommitLog => HistoryMode::Reflog,
        };
        self.refresh_activity();
    }

    /// Check if selection is in the history section
    fn is_in_history(&self) -> bool {
        let staged_len = self.status.staged_changes().len();
        let working_len = self.status.working_changes().len();
        let files_total = staged_len + working_len;

        // Index 0 is command, files are 1..=files_total, history starts after
        self.selected > files_total
    }

    /// Jump to first history item
    fn jump_to_history(&mut self) {
        let staged_len = self.status.staged_changes().len();
        let working_len = self.status.working_changes().len();
        let files_total = staged_len + working_len;

        // First history item is at index files_total + 1
        if !self.activity.is_empty() {
            self.selected = files_total + 1;
        }
    }

    /// Run the main application loop
    pub fn run(&mut self, tui: &mut Tui) -> Result<()> {
        while self.running {
            // Draw the UI
            tui.draw(|frame| ui::render(frame, self))?;

            // Handle events
            match tui.events.next()? {
                Event::Key(key) => self.handle_key(key),
                Event::Tick => self.on_tick(),
                Event::FileChanged => self.refresh_status(),
                Event::Resize(_, _) => {}
                Event::Mouse(_) => {}
            }
        }

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyEvent) {
        // Command mode captures all input
        if self.command_mode {
            self.handle_command_mode_key(key);
            return;
        }

        // Alias browsing mode captures keys
        if self.show_aliases {
            self.handle_alias_mode_key(key);
            return;
        }

        // Overlays capture keys
        if self.show_help {
            self.show_help = false;
            return;
        }

        if self.show_diff {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('d') => {
                    self.show_diff = false;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            // Quit
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }

            // Enter command mode
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_input.clear();
                self.history_index = None;
            }

            // Show aliases (when on command section)
            KeyCode::Char('a') => {
                if self.selected == 0 && !self.config.sections.is_empty() {
                    self.show_aliases = true;
                    self.alias_section_selected = 0;
                    self.show_section_aliases = false;
                }
            }

            // Help
            KeyCode::Char('?') => {
                self.show_help = true;
            }

            // Refresh
            KeyCode::Char('r') => {
                self.refresh_status();
            }

            // History mode toggle / focus
            KeyCode::Char('h') => {
                if self.is_in_history() {
                    // Already in history, toggle mode
                    self.toggle_history_mode();
                } else {
                    // Just jump to history section
                    self.jump_to_history();
                }
            }

            // Stage/Unstage
            KeyCode::Char('s') => {
                self.toggle_stage();
            }

            // Diff or activate command mode
            KeyCode::Char('d') => {
                self.show_diff();
            }
            KeyCode::Enter => {
                if self.selected == 0 {
                    // Activate command mode when on command section
                    self.command_mode = true;
                    self.command_input.clear();
                    self.history_index = None;
                } else {
                    self.show_diff();
                }
            }

            // List navigation
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_prev();
            }
            KeyCode::Char('g') => {
                self.select_first();
            }
            KeyCode::Char('G') => {
                self.select_last();
            }

            // Copy sha to clipboard (for history items)
            KeyCode::Char('y') => {
                if let Some(sha) = self.selected_activity_sha() {
                    self.copy_to_clipboard(&sha);
                    self.error = Some(format!("Copied: {sha}"));
                }
            }

            _ => {}
        }
    }

    /// Handle keyboard input in command mode
    fn handle_command_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            // Cancel command mode
            KeyCode::Esc => {
                self.command_mode = false;
                self.command_input.clear();
                self.history_index = None;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.command_mode = false;
                self.command_input.clear();
                self.history_index = None;
            }

            // Execute command
            KeyCode::Enter => {
                self.command_mode = false;
                self.execute_command();
            }

            // Type characters
            KeyCode::Char(c) => {
                self.command_input.push(c);
                self.history_index = None;
            }

            // Backspace
            KeyCode::Backspace => {
                self.command_input.pop();
                self.history_index = None;
            }

            // History navigation
            KeyCode::Up => {
                self.history_prev();
            }
            KeyCode::Down => {
                self.history_next();
            }

            _ => {}
        }
    }

    /// Handle keyboard input in alias browsing mode
    fn handle_alias_mode_key(&mut self, key: KeyEvent) {
        match key.code {
            // Go back / close
            KeyCode::Esc | KeyCode::Char('q') => {
                if self.show_section_aliases {
                    // Go back to category list
                    self.show_section_aliases = false;
                    self.alias_selected = 0;
                } else {
                    // Close alias browser
                    self.show_aliases = false;
                }
            }

            // Select category or run alias
            KeyCode::Enter => {
                if self.show_section_aliases {
                    // Run selected alias
                    self.run_selected_alias();
                } else {
                    // Enter category to show aliases
                    self.show_section_aliases = true;
                    self.alias_selected = 0;
                }
            }

            // Navigation
            KeyCode::Char('j') | KeyCode::Down => {
                if self.show_section_aliases {
                    // Navigate aliases within section
                    if let Some(section) = self.config.sections.get(self.alias_section_selected) {
                        if self.alias_selected < section.aliases.len().saturating_sub(1) {
                            self.alias_selected += 1;
                        }
                    }
                } else {
                    // Navigate categories
                    if self.alias_section_selected < self.config.sections.len().saturating_sub(1) {
                        self.alias_section_selected += 1;
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.show_section_aliases {
                    if self.alias_selected > 0 {
                        self.alias_selected -= 1;
                    }
                } else if self.alias_section_selected > 0 {
                    self.alias_section_selected -= 1;
                }
            }

            _ => {}
        }
    }

    /// Run the currently selected alias
    fn run_selected_alias(&mut self) {
        let Some(section) = self.config.sections.get(self.alias_section_selected) else {
            return;
        };
        let Some(alias) = section.aliases.get(self.alias_selected) else {
            return;
        };

        // Close alias browser
        self.show_aliases = false;
        self.show_section_aliases = false;

        // Set up command and execute
        self.command_input = format!("git {}", alias.name);
        self.execute_command();
    }

    /// Total count of all items (command + staged + working + activity)
    pub fn total_count(&self) -> usize {
        1 // Command section at index 0
            + self.status.staged_changes().len()
            + self.status.working_changes().len()
            + self.activity.len()
    }

    /// Get the selected file and whether it's staged
    /// Returns (path, is_staged) or None if selection is on command or activity
    fn selected_file_info(&self) -> Option<(PathBuf, bool)> {
        // Index 0 is command section
        if self.selected == 0 {
            return None;
        }

        let staged = self.status.staged_changes();
        let working = self.status.working_changes();
        let staged_len = staged.len();

        // Adjust for command section at index 0
        let file_idx = self.selected - 1;

        if file_idx < staged_len {
            // Selected is in staged
            staged.get(file_idx).map(|f| (f.path.clone(), true))
        } else if file_idx < staged_len + working.len() {
            // Selected is in working
            let working_idx = file_idx - staged_len;
            working.get(working_idx).map(|f| (f.path.clone(), false))
        } else {
            // Selected is in activity
            None
        }
    }

    /// Get the selected activity item's SHA (if on a history item)
    fn selected_activity_sha(&self) -> Option<String> {
        let staged_len = self.status.staged_changes().len();
        let working_len = self.status.working_changes().len();
        let files_total = staged_len + working_len;

        // Index 0 is command, files are 1..=files_total, activity starts after
        if self.selected > files_total {
            let activity_idx = self.selected - 1 - files_total;
            self.activity
                .get(activity_idx)
                .and_then(|cmd| cmd.sha.clone())
        } else {
            None
        }
    }

    /// Copy text to clipboard (macOS)
    fn copy_to_clipboard(&self, text: &str) {
        let _ = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(text.as_bytes())?;
                }
                child.wait()
            });
    }

    /// Toggle stage/unstage for selected file
    fn toggle_stage(&mut self) {
        let Some((path, is_staged)) = self.selected_file_info() else {
            return; // Can't stage/unstage activity items
        };

        let result = if is_staged {
            self.repo.unstage(&path)
        } else {
            self.repo.stage(&path)
        };

        if let Err(e) = result {
            self.error = Some(format!("Error: {e}"));
        } else {
            self.refresh_status();
        }
    }

    /// Show diff for selected file
    fn show_diff(&mut self) {
        let Some((path, is_staged)) = self.selected_file_info() else {
            return; // Can't show diff for activity items
        };

        match self.repo.diff_file(&path, is_staged) {
            Ok(content) => {
                self.diff_content = content;
                self.diff_path = path.to_string_lossy().to_string();
                self.show_diff = true;
            }
            Err(e) => {
                self.error = Some(format!("Diff error: {e}"));
            }
        }
    }

    /// Select next item
    fn select_next(&mut self) {
        let len = self.total_count();
        if len > 0 && self.selected < len - 1 {
            self.selected += 1;
        }
    }

    /// Select previous item
    fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Select first item
    fn select_first(&mut self) {
        self.selected = 0;
    }

    /// Select last item
    fn select_last(&mut self) {
        let len = self.total_count();
        if len > 0 {
            self.selected = len - 1;
        }
    }

    /// Handle tick events (periodic updates)
    fn on_tick(&mut self) {
        // Tick is now just for UI updates, not git status refresh
        // Status is refreshed via file watcher events
    }

    /// Execute a command (must start with "git")
    fn execute_command(&mut self) {
        let input = self.command_input.trim();
        if input.is_empty() {
            return;
        }

        // Parse command
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        // Only allow git commands for safety
        if parts[0] != "git" {
            self.command_output = String::from("Error: Only git commands are allowed");
            self.command_success = false;
            return;
        }

        // Add to history (avoid duplicates of last command)
        if self.command_history.last().map(String::as_str) != Some(input) {
            self.command_history.push(input.to_string());
            if self.command_history.len() > MAX_COMMAND_HISTORY {
                self.command_history.remove(0);
            }
        }

        // Execute the command
        let result = Command::new("git")
            .args(&parts[1..])
            .current_dir(&self.repo_path)
            .output();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                self.command_success = output.status.success();

                if self.command_success {
                    self.command_output = if stdout.is_empty() {
                        String::from("(no output)")
                    } else {
                        stdout.to_string()
                    };
                } else {
                    self.command_output = if stderr.is_empty() {
                        stdout.to_string()
                    } else {
                        stderr.to_string()
                    };
                }

                // Refresh status after any git command (it might have changed state)
                self.refresh_status();
            }
            Err(e) => {
                self.command_output = format!("Failed to execute: {e}");
                self.command_success = false;
            }
        }

        // Clear input and reset history navigation
        self.command_input.clear();
        self.history_index = None;
    }

    /// Navigate command history (older)
    fn history_prev(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // Start from most recent
                self.history_index = Some(self.command_history.len() - 1);
            }
            Some(idx) if idx > 0 => {
                self.history_index = Some(idx - 1);
            }
            _ => {}
        }

        if let Some(idx) = self.history_index {
            self.command_input = self.command_history[idx].clone();
        }
    }

    /// Navigate command history (newer)
    fn history_next(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        match self.history_index {
            Some(idx) if idx < self.command_history.len() - 1 => {
                self.history_index = Some(idx + 1);
                self.command_input = self.command_history[idx + 1].clone();
            }
            Some(_) => {
                // Past end of history, clear input
                self.history_index = None;
                self.command_input.clear();
            }
            None => {}
        }
    }
}
