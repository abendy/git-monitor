use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::Command,
    sync::mpsc::Sender,
};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::warn;

use crate::{
    config::GitConfig,
    event::Event,
    git::{BranchInfo, CommitDetail, FileState, GitCommand, GitRepo, GitStatus},
    tui::Tui,
    ui,
    watcher::{RepoWatcher, WatchEvent},
};

/// Maximum number of activity entries to keep
const MAX_ACTIVITY: usize = 50;

/// Maximum number of commands to keep in history
const MAX_COMMAND_HISTORY: usize = 100;

/// Auto-open popup if command output exceeds this many lines
const AUTO_POPUP_LINE_THRESHOLD: usize = 5;

/// History file name in home directory
const HISTORY_FILE: &str = ".git-monitor-history";

/// Get the path to the history file
fn history_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(HISTORY_FILE))
}

/// Load command history from file
fn load_history() -> Vec<String> {
    let Some(path) = history_file_path() else {
        return Vec::new();
    };

    let Ok(file) = File::open(&path) else {
        return Vec::new();
    };

    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .take(MAX_COMMAND_HISTORY)
        .collect()
}

/// Save command history to file
fn save_history(history: &[String]) {
    let Some(path) = history_file_path() else {
        return;
    };

    let Ok(mut file) = File::create(&path) else {
        return;
    };

    for cmd in history.iter().take(MAX_COMMAND_HISTORY) {
        let _ = writeln!(file, "{cmd}");
    }
}

/// History display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryMode {
    /// Show reflog (git actions)
    Reflog,
    /// Show commit log
    #[default]
    CommitLog,
}

/// Content displayed in the popup
#[derive(Debug, Clone, Default)]
pub enum PopupContent {
    /// No popup active
    #[default]
    None,
    /// Command output (from : command mode)
    CommandOutput {
        command: String,
        output: String,
        success: bool,
    },
    /// File diff
    Diff {
        path: String,
        content: String,
        #[allow(dead_code)] // Reserved for future staged/unstaged indicator
        is_staged: bool,
    },
    // Future: CommitDetail, RebaseTool, etc.
}

impl PopupContent {
    /// Check if popup is active
    pub fn is_active(&self) -> bool {
        !matches!(self, PopupContent::None)
    }

    /// Get the number of lines without allocating
    pub fn line_count(&self) -> usize {
        match self {
            PopupContent::None => 0,
            PopupContent::CommandOutput { output, .. } => output.lines().count(),
            PopupContent::Diff { content, .. } => content.lines().count(),
        }
    }

    /// Get the title for the popup
    pub fn title(&self) -> String {
        match self {
            PopupContent::None => String::new(),
            PopupContent::CommandOutput { command, .. } => format!(" Output: {command} "),
            PopupContent::Diff { path, .. } => format!(" Diff: {path} "),
        }
    }
}

/// Popup state with scroll position
#[derive(Debug, Clone, Default)]
pub struct PopupState {
    /// Content being displayed
    pub content: PopupContent,
    /// Scroll offset (line number at top of view)
    pub scroll_offset: usize,
    /// Visible height from last render (for scroll calculations)
    pub visible_height: usize,
}

impl PopupState {
    /// Open popup with content
    pub fn open(&mut self, content: PopupContent) {
        self.content = content;
        self.scroll_offset = 0;
    }

    /// Close popup
    pub fn close(&mut self) {
        self.content = PopupContent::None;
        self.scroll_offset = 0;
    }

    /// Check if popup is open
    pub fn is_open(&self) -> bool {
        self.content.is_active()
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize, max_lines: usize, visible_height: usize) {
        let max_offset = max_lines.saturating_sub(visible_height);
        self.scroll_offset = (self.scroll_offset + n).min(max_offset);
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Jump to top
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Jump to bottom
    pub fn scroll_to_bottom(&mut self, max_lines: usize, visible_height: usize) {
        self.scroll_offset = max_lines.saturating_sub(visible_height);
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
    /// Popup state (for full-screen overlays: output, diff, etc.)
    pub popup: PopupState,
    /// SHA of currently expanded commit in history (None = collapsed)
    pub expanded_commit: Option<String>,
    /// Cached detail for expanded commit
    pub expanded_detail: Option<CommitDetail>,
    /// Selected file index within expanded commit (None = on commit header)
    pub expanded_file_idx: Option<usize>,
    /// List of local branches
    pub branches: Vec<BranchInfo>,
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
        let branches = repo.list_branches().unwrap_or_default();

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
            error: None,
            command_mode: false,
            command_input: String::new(),
            command_output: String::new(),
            command_success: true,
            command_history: load_history(),
            history_index: None,
            show_aliases: false,
            alias_section_selected: 0,
            show_section_aliases: false,
            alias_selected: 0,
            history_mode: HistoryMode::default(),
            popup: PopupState::default(),
            expanded_commit: None,
            expanded_detail: None,
            expanded_file_idx: None,
            branches,
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

        // Refresh branches
        if let Ok(branches) = self.repo.list_branches() {
            self.branches = branches;
        }
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
        let activity_len = self.activity.len();

        // Index 0 is command, files are 1..=files_total, history is next
        let history_start = 1 + files_total;
        let history_end = history_start + activity_len;
        self.selected >= history_start && self.selected < history_end
    }

    /// Check if selection is in the branches section
    fn is_in_branches(&self) -> bool {
        let staged_len = self.status.staged_changes().len();
        let working_len = self.status.working_changes().len();
        let activity_len = self.activity.len();
        let files_total = staged_len + working_len;

        // Branches start after: command(1) + files + activity
        let branches_start = 1 + files_total + activity_len;
        self.selected >= branches_start && self.selected < branches_start + self.branches.len()
    }

    /// Check if currently selected item is the expanded commit
    fn is_on_expanded_commit(&self) -> bool {
        if let Some(expanded_sha) = &self.expanded_commit {
            if let Some(selected_sha) = self.selected_activity_sha() {
                return expanded_sha == &selected_sha;
            }
        }
        false
    }

    /// Open diff popup for a file in a specific commit
    fn open_commit_file_diff(&mut self, commit_sha: &str, file_path: &str) {
        match self.repo.commit_file_diff(commit_sha, file_path) {
            Ok(diff_content) => {
                self.popup.open(PopupContent::Diff {
                    path: file_path.to_string(),
                    content: diff_content,
                    is_staged: false,
                });
            }
            Err(e) => {
                self.error = Some(format!("Failed to get diff: {e}"));
            }
        }
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

    /// Jump to first branch item
    fn jump_to_branches(&mut self) {
        let staged_len = self.status.staged_changes().len();
        let working_len = self.status.working_changes().len();
        let activity_len = self.activity.len();
        let files_total = staged_len + working_len;

        // Branches come after: command(1) + files + activity
        if !self.branches.is_empty() {
            self.selected = 1 + files_total + activity_len;
            self.close_expanded_commit();
        }
    }

    /// Get the selected branch info (if on a branch item)
    fn selected_branch(&self) -> Option<&BranchInfo> {
        let staged_len = self.status.staged_changes().len();
        let working_len = self.status.working_changes().len();
        let activity_len = self.activity.len();
        let files_total = staged_len + working_len;

        let branches_start = 1 + files_total + activity_len;

        if self.selected >= branches_start {
            let branch_idx = self.selected - branches_start;
            self.branches.get(branch_idx)
        } else {
            None
        }
    }

    /// Checkout the currently selected branch
    fn checkout_selected_branch(&mut self) {
        let branch_name = match self.selected_branch() {
            Some(branch) if branch.is_current => {
                self.error = Some("Already on this branch".to_string());
                return;
            }
            Some(branch) => branch.name.clone(),
            None => return,
        };

        match self.repo.checkout_branch(&branch_name) {
            Ok(()) => {
                self.error = Some(format!("Switched to branch '{branch_name}'"));
                self.refresh_status();
            }
            Err(e) => {
                self.error = Some(e.to_string());
            }
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

        // Popup mode captures keys (full-screen overlays)
        if self.popup.is_open() {
            self.handle_popup_key(key);
            return;
        }

        // Help overlay captures keys
        if self.show_help {
            self.show_help = false;
            return;
        }

        // Auto-enter command mode when typing on command section
        // (except for quit, help, and special keys)
        if self.selected == 0 {
            if let KeyCode::Char(c) = key.code {
                if !matches!(c, 'q' | '?' | ':' | 'o') && !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.command_mode = true;
                    self.command_input.clear();
                    self.history_index = None;
                    self.command_input.push(c);
                    return;
                }
            }
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

            // Show aliases (universal shortcut)
            KeyCode::Char('a') => {
                if !self.config.sections.is_empty() {
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

            // Jump to branches
            KeyCode::Char('b') => {
                self.jump_to_branches();
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
                } else if self.is_in_branches() {
                    // Checkout selected branch
                    self.checkout_selected_branch();
                } else {
                    self.show_diff();
                }
            }

            // List navigation (with expanded commit support)
            KeyCode::Char('j') | KeyCode::Down => {
                if self.is_on_expanded_commit() {
                    // Navigate within expanded commit
                    let file_count = self
                        .expanded_detail
                        .as_ref()
                        .map(|d| d.files.len())
                        .unwrap_or(0);

                    match self.expanded_file_idx {
                        None if file_count > 0 => {
                            // Move from header to first file
                            self.expanded_file_idx = Some(0);
                        }
                        Some(idx) if idx + 1 < file_count => {
                            // Move to next file
                            self.expanded_file_idx = Some(idx + 1);
                        }
                        Some(_) | None => {
                            // At last file or no files - move to next commit
                            self.expanded_file_idx = None;
                            self.select_next();
                        }
                    }
                } else {
                    self.select_next();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected == 0 && !self.command_history.is_empty() {
                    // On command section - enter command mode and show history
                    self.command_mode = true;
                    self.history_prev();
                } else if self.is_on_expanded_commit() {
                    // Navigate within expanded commit
                    match self.expanded_file_idx {
                        Some(0) => {
                            // Move from first file back to header
                            self.expanded_file_idx = None;
                        }
                        Some(idx) => {
                            // Move to previous file
                            self.expanded_file_idx = Some(idx - 1);
                        }
                        None => {
                            // On header - move to previous item
                            self.select_prev();
                        }
                    }
                } else {
                    self.select_prev();
                }
            }
            KeyCode::Char('g') => {
                self.select_first();
            }
            KeyCode::Char('G') => {
                self.select_last();
            }

            // Copy sha to clipboard (for history items) - 'y' copies short sha
            KeyCode::Char('y') => {
                if let Some(sha) = self.selected_activity_sha() {
                    if self.copy_to_clipboard(&sha) {
                        self.error = Some(format!("Copied: {sha}"));
                    } else {
                        self.error = Some("Failed to copy to clipboard".to_string());
                    }
                }
            }

            // Copy full sha to clipboard (for history items)
            KeyCode::Char('c') => {
                if self.is_in_history() {
                    // If expanded, copy full sha from detail
                    if let Some(detail) = &self.expanded_detail {
                        if self.copy_to_clipboard(&detail.full_sha) {
                            self.error = Some(format!("Copied: {}", detail.full_sha));
                        } else {
                            self.error = Some("Failed to copy to clipboard".to_string());
                        }
                    } else if let Some(sha) = self.selected_activity_sha() {
                        // If not expanded, copy short sha
                        if self.copy_to_clipboard(&sha) {
                            self.error = Some(format!("Copied: {sha}"));
                        } else {
                            self.error = Some("Failed to copy to clipboard".to_string());
                        }
                    }
                }
            }

            // Show diff for files, or toggle commit detail expansion
            KeyCode::Char(' ') => {
                // First check if we're on a staged/working file
                if self.selected_file_info().is_some() {
                    self.show_diff();
                } else if let Some(sha) = self.selected_activity_sha() {
                    // History item handling
                    if self.expanded_commit.as_ref() == Some(&sha) {
                        // Already expanded - check if we're on a file
                        if let Some(file_idx) = self.expanded_file_idx {
                            // Open diff for this file - clone path to avoid borrow issues
                            let file_path = self
                                .expanded_detail
                                .as_ref()
                                .and_then(|d| d.files.get(file_idx))
                                .map(|f| f.path.clone());

                            if let Some(path) = file_path {
                                self.open_commit_file_diff(&sha, &path);
                            }
                        } else {
                            // On commit header - collapse
                            self.close_expanded_commit();
                        }
                    } else {
                        // Expand - fetch details
                        self.expanded_commit = Some(sha.clone());
                        self.expanded_detail = self.repo.commit_detail(&sha).ok();
                        self.expanded_file_idx = None;
                    }
                }
            }

            // Open output popup (when on command section with output)
            KeyCode::Char('o') => {
                if self.selected == 0 && !self.command_output.is_empty() {
                    self.open_output_popup();
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

    /// Handle keyboard input in popup mode
    fn handle_popup_key(&mut self, key: KeyEvent) {
        // Use visible height from last render (defaults to 20 if not yet rendered)
        let visible_height = if self.popup.visible_height > 0 {
            self.popup.visible_height
        } else {
            20
        };
        let max_lines = self.popup.content.line_count();

        match key.code {
            // Close popup
            KeyCode::Esc | KeyCode::Char('q') => {
                self.popup.close();
            }

            // Scroll down
            KeyCode::Char('j') | KeyCode::Down => {
                self.popup.scroll_down(1, max_lines, visible_height);
            }

            // Scroll up
            KeyCode::Char('k') | KeyCode::Up => {
                self.popup.scroll_up(1);
            }

            // Page down
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.popup.scroll_down(visible_height / 2, max_lines, visible_height);
            }

            // Page up
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.popup.scroll_up(visible_height / 2);
            }

            // Jump to top
            KeyCode::Char('g') => {
                self.popup.scroll_to_top();
            }

            // Jump to bottom
            KeyCode::Char('G') => {
                self.popup.scroll_to_bottom(max_lines, visible_height);
            }

            _ => {}
        }
    }

    /// Open the output popup with current command output
    fn open_output_popup(&mut self) {
        let command = self.command_history.last().cloned().unwrap_or_default();
        self.popup.open(PopupContent::CommandOutput {
            command,
            output: self.command_output.clone(),
            success: self.command_success,
        });
    }

    /// Open a diff in the popup
    fn open_diff_popup(&mut self, path: String, content: String, is_staged: bool) {
        self.popup.open(PopupContent::Diff {
            path,
            content,
            is_staged,
        });
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
            + self.branches.len()
    }

    /// Get the selected file info
    /// Returns (path, is_staged, file_state) or None if selection is on command or activity
    fn selected_file_info(&self) -> Option<(PathBuf, bool, FileState)> {
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
            staged.get(file_idx).map(|f| (f.path.clone(), true, f.staged))
        } else if file_idx < staged_len + working.len() {
            // Selected is in working
            let working_idx = file_idx - staged_len;
            working.get(working_idx).map(|f| (f.path.clone(), false, f.working))
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

    /// Copy text to clipboard (cross-platform)
    fn copy_to_clipboard(&self, text: &str) -> bool {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(text))
            .is_ok()
    }

    /// Toggle stage/unstage for selected file
    fn toggle_stage(&mut self) {
        let Some((path, is_staged, _state)) = self.selected_file_info() else {
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

    /// Show diff for selected file (using new popup system)
    fn show_diff(&mut self) {
        let Some((path, is_staged, state)) = self.selected_file_info() else {
            return; // Can't show diff for activity items
        };

        let path_str = path.to_string_lossy().to_string();

        // For untracked files, show the file content instead of diff
        if state == FileState::Untracked {
            let full_path = self.repo_path.join(&path);
            match std::fs::read_to_string(&full_path) {
                Ok(content) => {
                    // Format as a "new file" diff-like view
                    let formatted = content
                        .lines()
                        .map(|line| format!("+{line}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let header = format!("(new file)\n\n{formatted}");
                    self.open_diff_popup(path_str, header, is_staged);
                }
                Err(e) => {
                    self.error = Some(format!("Error reading file: {e}"));
                }
            }
            return;
        }

        match self.repo.diff_file(&path, is_staged) {
            Ok(content) => {
                self.open_diff_popup(path_str, content, is_staged);
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
            self.close_expanded_commit();
        }
    }

    /// Select previous item
    fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.close_expanded_commit();
        }
    }

    /// Select first item
    fn select_first(&mut self) {
        self.selected = 0;
        self.close_expanded_commit();
    }

    /// Select last item
    fn select_last(&mut self) {
        let len = self.total_count();
        if len > 0 {
            self.selected = len - 1;
            self.close_expanded_commit();
        }
    }

    /// Close expanded commit detail
    fn close_expanded_commit(&mut self) {
        self.expanded_commit = None;
        self.expanded_detail = None;
        self.expanded_file_idx = None;
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

        // Auto-open popup if output exceeds threshold
        if self.command_output.lines().count() > AUTO_POPUP_LINE_THRESHOLD {
            self.open_output_popup();
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

    /// Save command history to persistent storage
    pub fn save_history(&self) {
        save_history(&self.command_history);
    }
}
