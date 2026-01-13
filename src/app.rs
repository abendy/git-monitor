use std::{path::PathBuf, process::Command, sync::mpsc::Sender};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::warn;

use crate::{
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

/// Active panel in the UI
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    #[default]
    Working,
    Staged,
    Activity,
}

impl Panel {
    pub const fn next(self) -> Self {
        match self {
            Self::Working => Self::Staged,
            Self::Staged => Self::Activity,
            Self::Activity => Self::Working,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Working => Self::Activity,
            Self::Staged => Self::Working,
            Self::Activity => Self::Staged,
        }
    }
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
    /// File watcher
    #[allow(dead_code)]
    watcher: Option<RepoWatcher>,
    /// Whether the application is running
    pub running: bool,
    /// Currently active panel
    pub active_panel: Panel,
    /// Selected index in working panel
    pub working_selected: usize,
    /// Selected index in staged panel
    pub staged_selected: usize,
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
        let activity = repo.reflog(MAX_ACTIVITY).unwrap_or_default();

        Ok(Self {
            repo_path,
            repo,
            status,
            activity,
            watcher: None,
            running: true,
            active_panel: Panel::default(),
            working_selected: 0,
            staged_selected: 0,
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

                // Clamp selection indices
                let working_len = self.status.working_changes().len();
                if self.working_selected >= working_len && working_len > 0 {
                    self.working_selected = working_len - 1;
                }

                let staged_len = self.status.staged_changes().len();
                if self.staged_selected >= staged_len && staged_len > 0 {
                    self.staged_selected = staged_len - 1;
                }
            }
            Err(e) => {
                warn!("Failed to refresh git status: {}", e);
                self.error = Some(format!("Git error: {e}"));
            }
        }

        // Refresh activity log
        if let Ok(activity) = self.repo.reflog(MAX_ACTIVITY) {
            self.activity = activity;
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

            // Help
            KeyCode::Char('?') => {
                self.show_help = true;
            }

            // Refresh
            KeyCode::Char('r') => {
                self.refresh_status();
            }

            // Stage/Unstage
            KeyCode::Char('s') => {
                self.toggle_stage();
            }

            // Diff
            KeyCode::Char('d') | KeyCode::Enter => {
                self.show_diff();
            }

            // Panel navigation
            KeyCode::Tab => {
                self.active_panel = self.active_panel.next();
            }
            KeyCode::BackTab => {
                self.active_panel = self.active_panel.prev();
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

    /// Toggle stage/unstage for selected file
    fn toggle_stage(&mut self) {
        let result = match self.active_panel {
            Panel::Working => {
                // Stage the selected working file
                let changes = self.status.working_changes();
                if let Some(file) = changes.get(self.working_selected) {
                    let path = file.path.clone();
                    self.repo.stage(&path)
                } else {
                    return;
                }
            }
            Panel::Staged => {
                // Unstage the selected staged file
                let changes = self.status.staged_changes();
                if let Some(file) = changes.get(self.staged_selected) {
                    let path = file.path.clone();
                    self.repo.unstage(&path)
                } else {
                    return;
                }
            }
            Panel::Activity => return,
        };

        if let Err(e) = result {
            self.error = Some(format!("Error: {e}"));
        } else {
            self.refresh_status();
        }
    }

    /// Show diff for selected file
    fn show_diff(&mut self) {
        let (path, staged) = match self.active_panel {
            Panel::Working => {
                let changes = self.status.working_changes();
                if let Some(file) = changes.get(self.working_selected) {
                    (file.path.clone(), false)
                } else {
                    return;
                }
            }
            Panel::Staged => {
                let changes = self.status.staged_changes();
                if let Some(file) = changes.get(self.staged_selected) {
                    (file.path.clone(), true)
                } else {
                    return;
                }
            }
            Panel::Activity => return,
        };

        match self.repo.diff_file(&path, staged) {
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

    /// Select next item in current panel
    fn select_next(&mut self) {
        match self.active_panel {
            Panel::Working => {
                let len = self.status.working_changes().len();
                if len > 0 && self.working_selected < len - 1 {
                    self.working_selected += 1;
                }
            }
            Panel::Staged => {
                let len = self.status.staged_changes().len();
                if len > 0 && self.staged_selected < len - 1 {
                    self.staged_selected += 1;
                }
            }
            Panel::Activity => {}
        }
    }

    /// Select previous item in current panel
    fn select_prev(&mut self) {
        match self.active_panel {
            Panel::Working => {
                if self.working_selected > 0 {
                    self.working_selected -= 1;
                }
            }
            Panel::Staged => {
                if self.staged_selected > 0 {
                    self.staged_selected -= 1;
                }
            }
            Panel::Activity => {}
        }
    }

    /// Select first item
    fn select_first(&mut self) {
        match self.active_panel {
            Panel::Working => self.working_selected = 0,
            Panel::Staged => self.staged_selected = 0,
            Panel::Activity => {}
        }
    }

    /// Select last item
    fn select_last(&mut self) {
        match self.active_panel {
            Panel::Working => {
                let len = self.status.working_changes().len();
                if len > 0 {
                    self.working_selected = len - 1;
                }
            }
            Panel::Staged => {
                let len = self.status.staged_changes().len();
                if len > 0 {
                    self.staged_selected = len - 1;
                }
            }
            Panel::Activity => {}
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
