use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::Sender,
};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::warn;

use crate::{
    actions::{Action, ActionRegistry, AppAction, AppState, Context},
    command::{CommandExecutor, CommandRequest, CommandSource, FeedbackPolicy},
    config::GitConfig,
    event::Event,
    feedback::{PopupContent, PopupState},
    git::{BranchInfo, CommitDetail, FileState, GitCommand, GitRepo, GitStatus},
    menu::{ActionMenu, AliasSectionMenu, MenuResult, MenuStack, PushConfirmMenu},
    tui::Tui,
    ui,
    watcher::{RepoWatcher, WatchEvent},
};

/// Page size for history pagination
const PAGE_SIZE: usize = 50;

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

/// View mode for the application body
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Normal navigation mode (default)
    #[default]
    Normal,
    /// Command input mode (typing git commands)
    Command,
}

/// External command requiring TUI suspension
#[derive(Debug, Clone)]
pub enum ExternalCommand {
    /// Show commit file diff with pager (uses core.pager from gitconfig)
    PagerDiff { commit_sha: String, file_path: String },
    /// Show commit file diff with difftool (uses diff.tool from gitconfig)
    DiffTool { commit_sha: String, file_path: String },
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
    /// Selected index in main list (None = nothing focused, Some(0) = command, etc.)
    pub selected: Option<usize>,
    /// Show help overlay
    pub show_help: bool,
    /// Current view mode
    pub view_mode: ViewMode,
    /// Error message to display
    pub error: Option<String>,
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
    /// Whether the History section (current branch) is collapsed
    pub history_collapsed: bool,
    /// Current page offset for history pagination (0-indexed)
    pub history_page: usize,
    /// Which non-current branch is expanded (showing its commits)
    pub expanded_branch: Option<String>,
    /// Cached commits for the expanded branch
    pub expanded_branch_commits: Vec<GitCommand>,
    /// Pending external command (requires TUI suspension)
    pub pending_external: Option<ExternalCommand>,
    /// Action registry for contextual actions
    pub action_registry: ActionRegistry,
    /// Command executor for running commands
    executor: CommandExecutor,
    /// Menu stack for modal dialogs
    pub menu_stack: MenuStack,
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
        let activity = repo.commit_log(0, PAGE_SIZE).unwrap_or_default();
        let config = GitConfig::load(&repo_path).unwrap_or_default();
        let branches = repo.list_branches().unwrap_or_default();

        // Initialize action registry with aliases
        let mut action_registry = ActionRegistry::new();
        let all_aliases: Vec<_> = config
            .sections
            .iter()
            .flat_map(|s| s.aliases.iter().cloned())
            .collect();
        action_registry.add_alias_actions(&all_aliases);

        // Initialize command executor
        let executor = CommandExecutor::new(&repo_path);

        Ok(Self {
            repo_path,
            repo,
            status,
            activity,
            config,
            watcher: None,
            running: true,
            selected: None,
            show_help: false,
            view_mode: ViewMode::default(),
            error: None,
            command_input: String::new(),
            command_output: String::new(),
            command_success: true,
            command_history: load_history(),
            history_index: None,
            history_mode: HistoryMode::default(),
            popup: PopupState::default(),
            expanded_commit: None,
            expanded_detail: None,
            expanded_file_idx: None,
            branches,
            history_collapsed: false,
            history_page: 0,
            expanded_branch: None,
            expanded_branch_commits: Vec::new(),
            pending_external: None,
            action_registry,
            executor,
            menu_stack: MenuStack::new(),
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // ViewMode helper methods
    // ─────────────────────────────────────────────────────────────────────────

    /// Check if in command mode
    pub fn is_command_mode(&self) -> bool {
        matches!(self.view_mode, ViewMode::Command)
    }

    /// Enter command mode
    pub fn enter_command_mode(&mut self) {
        self.view_mode = ViewMode::Command;
        self.command_input.clear();
        self.history_index = None;
    }

    /// Exit command mode (return to normal)
    pub fn exit_command_mode(&mut self) {
        self.view_mode = ViewMode::Normal;
        self.command_input.clear();
        self.history_index = None;
    }

    /// Enter alias browser using the menu stack
    pub fn enter_alias_browser(&mut self) {
        if self.config.sections.is_empty() {
            return;
        }

        let menu = AliasSectionMenu::new(
            self.config.sections.clone(),
            self.repo_path.clone(),
        );
        self.menu_stack.push(Box::new(menu));
    }

    /// Open action menu for current context using the menu stack
    pub fn open_action_menu(&mut self) {
        let context = self.current_context();
        let state = self.app_state();
        let actions: Vec<Action> = self
            .action_registry
            .actions_for_context(context, &state)
            .into_iter()
            .cloned()
            .collect();

        if actions.is_empty() {
            return;
        }

        let menu = ActionMenu::new(context, actions, self.repo_path.clone());
        self.menu_stack.push(Box::new(menu));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Watcher and refresh methods
    // ─────────────────────────────────────────────────────────────────────────

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

    /// Refresh activity based on current history mode and page
    fn refresh_activity(&mut self) {
        let skip = self.history_page * PAGE_SIZE;
        let result = match self.history_mode {
            HistoryMode::Reflog => self.repo.reflog(skip, PAGE_SIZE),
            HistoryMode::CommitLog => self.repo.commit_log(skip, PAGE_SIZE),
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
        self.history_page = 0; // Reset to first page when switching modes
        self.refresh_activity();
    }

    /// Go to next history page
    fn next_history_page(&mut self) {
        // Only advance if we got a full page (more data likely exists)
        // Use >= because remote-only commits may add extra items beyond PAGE_SIZE
        if self.activity.len() >= PAGE_SIZE {
            self.history_page += 1;
            self.refresh_activity();
            self.select_first_history_commit();
        }
    }

    /// Go to previous history page
    fn prev_history_page(&mut self) {
        if self.history_page > 0 {
            self.history_page -= 1;
            self.refresh_activity();
            self.select_first_history_commit();
        }
    }

    /// Select the first commit in the history section
    fn select_first_history_commit(&mut self) {
        if !self.activity.is_empty() {
            let files_total = self.status.staged_changes().len() + self.status.working_changes().len();
            // First commit is after header: 1 (command) + files_total + 1 (header)
            self.selected = Some(1 + files_total + 1);
        }
    }

    /// Go to first history page
    #[allow(dead_code)]
    fn first_history_page(&mut self) {
        if self.history_page != 0 {
            self.history_page = 0;
            self.refresh_activity();
        }
    }

    /// Check if selection is on the History header
    fn is_on_history_header(&self) -> bool {
        let Some(selected) = self.selected else {
            return false;
        };
        let files_total = self.status.staged_changes().len() + self.status.working_changes().len();
        // History header is at files_total + 1
        selected == 1 + files_total
    }

    /// Check if selection is in the history commits section (not on header)
    fn is_in_history(&self) -> bool {
        if self.history_collapsed {
            return false;
        }
        let Some(selected) = self.selected else {
            return false;
        };
        let files_total = self.status.staged_changes().len() + self.status.working_changes().len();
        let activity_len = self.activity.len();

        // History header is at files_total + 1
        // History commits start at files_total + 2
        let history_commits_start = 1 + files_total + 1;
        let history_commits_end = history_commits_start + activity_len;
        selected >= history_commits_start && selected < history_commits_end
    }

    /// Check if selection is in the branches section (expanded branch commits)
    fn is_in_branches(&self) -> bool {
        if self.expanded_branch.is_none() || self.expanded_branch_commits.is_empty() {
            return false;
        }
        let Some(selected) = self.selected else {
            return false;
        };
        let branches_start = self.branches_start_index();
        let branches_end = branches_start + self.expanded_branch_commits.len();
        selected >= branches_start && selected < branches_end
    }

    /// Check if selection is on a branch header (always None - headers not selectable)
    fn is_on_branch_header(&self) -> Option<String> {
        None
    }

    /// Get the index where branches section starts
    fn branches_start_index(&self) -> usize {
        let files_total = self.status.staged_changes().len() + self.status.working_changes().len();
        let history_items = if self.history_collapsed {
            1 // Just header
        } else {
            1 + self.activity.len() // Header + commits
        };
        1 + files_total + history_items
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

    /// Determine the current context based on selection and state
    #[must_use]
    pub fn current_context(&self) -> Context {
        // Handle special modes first
        if self.is_command_mode() {
            return Context::Command;
        }

        let Some(selected) = self.selected else {
            return Context::Global;
        };

        // Index 0 is command section
        if selected == 0 {
            return Context::Command;
        }

        // Check if in expanded commit files
        if self.expanded_commit.is_some() && self.expanded_file_idx.is_some() {
            return Context::CommitFiles;
        }

        // Check files section
        if self.selected_file_info().is_some() {
            let staged_len = self.status.staged_changes().len();
            let file_idx = selected - 1;
            if file_idx < staged_len {
                return Context::StagedFiles;
            }
            return Context::WorkingFiles;
        }

        // Check history header
        if self.is_on_history_header() {
            return Context::HistoryHeader;
        }

        // Check history commits
        if self.is_in_history() {
            return Context::HistoryCommits;
        }

        // Check branch commits
        if self.is_in_branches() {
            return Context::BranchCommits;
        }

        Context::Global
    }

    /// Get current app state for condition evaluation
    #[must_use]
    pub fn app_state(&self) -> AppState {
        AppState {
            ahead: self.status.ahead,
            behind: self.status.behind,
            has_upstream: self.status.upstream.is_some(),
            staged_count: self.status.staged_changes().len(),
            working_count: self.status.working_changes().len(),
            untracked_count: self
                .status
                .files
                .iter()
                .filter(|f| f.working == FileState::Untracked)
                .count(),
        }
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

    /// Jump to working area (first file in staged or working changes)
    fn jump_to_working(&mut self) {
        let files_total =
            self.status.staged_changes().len() + self.status.working_changes().len();
        if files_total == 0 {
            return;
        }

        // Select first file (index 1, after any spacer)
        self.selected = Some(1);
        self.close_expanded_commit();
    }

    /// Jump to history and expand it, landing on most recent commit
    fn jump_to_history(&mut self) {
        if self.activity.is_empty() {
            return;
        }

        // Expand history (collapses any expanded branch)
        self.expand_history();

        let staged_len = self.status.staged_changes().len();
        let working_len = self.status.working_changes().len();
        let files_total = staged_len + working_len;

        // Select first commit (skip header)
        let history_header_idx = 1 + files_total;
        self.selected = Some(history_header_idx + 1);
        self.close_expanded_commit();
    }

    /// Jump to first branch and expand it, landing on first commit
    fn jump_to_branches(&mut self) {
        let other_branches = self.other_branches();
        if other_branches.is_empty() {
            return;
        }

        // Get first branch name and expand it
        let first_branch_name = other_branches[0].name.clone();
        self.expand_branch(&first_branch_name);

        // Select first commit in the branch
        let branches_start = self.branches_start_index();
        self.selected = Some(branches_start);
        self.close_expanded_commit();
    }

    /// Get branches excluding the current one (for accordion display)
    pub fn other_branches(&self) -> Vec<&BranchInfo> {
        self.branches.iter().filter(|b| !b.is_current).collect()
    }

    /// Expand a branch to show its commits
    fn expand_branch(&mut self, branch_name: &str) {
        // Collapse history when expanding a branch
        self.history_collapsed = true;

        // Collapse any previously expanded branch
        if self.expanded_branch.as_deref() == Some(branch_name) {
            return; // Already expanded
        }

        // Fetch commits unique to this branch
        match self.repo.commit_log_for_branch(branch_name) {
            Ok(commits) => {
                self.expanded_branch = Some(branch_name.to_string());
                self.expanded_branch_commits = commits;
            }
            Err(e) => {
                self.error = Some(format!("Failed to load branch history: {e}"));
            }
        }
    }

    /// Collapse the currently expanded branch
    fn collapse_branch(&mut self) {
        self.expanded_branch = None;
        self.expanded_branch_commits.clear();
    }

    /// Collapse the History section
    fn collapse_history(&mut self) {
        self.history_collapsed = true;
        self.close_expanded_commit();
    }

    /// Expand the History section
    fn expand_history(&mut self) {
        self.history_collapsed = false;
        // Collapse any expanded branch when expanding history
        self.collapse_branch();
    }

    /// Get the selected branch info (if on a branch header)
    fn selected_branch(&self) -> Option<&BranchInfo> {
        let branch_name = self.is_on_branch_header()?;
        self.branches.iter().find(|b| b.name == branch_name)
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
            // Handle pending external command (requires TUI suspension)
            if let Some(cmd) = self.pending_external.take() {
                self.run_external_command(tui, cmd)?;
                continue;
            }

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

    /// Run an external command with TUI suspension
    fn run_external_command(&mut self, tui: &mut Tui, cmd: ExternalCommand) -> Result<()> {
        // Pause event handler so it doesn't consume input meant for the external command
        tui.events.pause();
        // Give the event thread time to finish any pending poll
        std::thread::sleep(std::time::Duration::from_millis(100));
        tui.suspend()?;

        let result = match &cmd {
            ExternalCommand::PagerDiff { commit_sha, file_path } => {
                // Use git show with --paginate to force pager usage
                Command::new("git")
                    .args(["--paginate", "show", commit_sha, "--", file_path])
                    .current_dir(&self.repo_path)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status()
            }
            ExternalCommand::DiffTool { commit_sha, file_path } => {
                // Use git difftool which respects diff.tool from gitconfig
                Command::new("git")
                    .args([
                        "difftool",
                        "--no-prompt",
                        &format!("{commit_sha}~1..{commit_sha}"),
                        "--",
                        file_path,
                    ])
                    .current_dir(&self.repo_path)
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status()
            }
        };

        if let Err(e) = result {
            self.error = Some(format!("Failed to run external command: {e}"));
        }

        // Wait for user to press Enter before resuming TUI
        // See ADR-001 for rationale
        use std::io::Read;
        println!("\n[Press Enter to continue]");
        let _ = std::io::stdin().read(&mut [0u8]);

        tui.resume()?;
        tui.events.resume();
        Ok(())
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyEvent) {
        // Menu stack takes priority when active
        if self.menu_stack.is_active() {
            self.handle_menu_key(key);
            return;
        }

        // ViewMode-based dispatch (command and alias modes capture all input)
        match &self.view_mode {
            ViewMode::Command => {
                self.handle_command_mode_key(key);
                return;
            }
            ViewMode::Normal => {}
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
        if self.selected == Some(0) {
            if let KeyCode::Char(c) = key.code {
                if !matches!(c, 'q' | '?' | ':' | 'o') && !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.enter_command_mode();
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
                self.enter_command_mode();
            }

            // Show aliases (universal shortcut)
            KeyCode::Char('a') => {
                self.enter_alias_browser();
            }

            // Open action menu for current context
            KeyCode::Char('m') => {
                self.open_action_menu();
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

            // History pagination: next page (only in history section)
            KeyCode::Char(']') => {
                if self.is_in_history() || self.is_on_history_header() {
                    // Use >= because remote-only commits may add extra items beyond PAGE_SIZE
                    if self.activity.len() >= PAGE_SIZE {
                        self.next_history_page();
                    }
                }
            }

            // History pagination: previous page (only in history section)
            KeyCode::Char('[') => {
                if self.is_in_history() || self.is_on_history_header() {
                    self.prev_history_page();
                }
            }

            // Select first item (vim-style)
            KeyCode::Char('g') => {
                self.select_first();
            }

            // Jump to branches
            KeyCode::Char('b') => {
                self.jump_to_branches();
            }

            // Jump to working area
            KeyCode::Char('w') => {
                self.jump_to_working();
            }

            // Stage/Unstage
            KeyCode::Char('s') => {
                self.toggle_stage();
            }

            // Push current branch (universal shortcut in normal mode)
            KeyCode::Char('P') => {
                self.show_push_confirm(false);
            }

            // Pull (lowercase p)
            KeyCode::Char('p') => {
                self.execute_pull();
            }

            // Fetch
            KeyCode::Char('f') => {
                self.execute_fetch();
            }

            // Diff (inline popup)
            KeyCode::Char('d') => {
                // Check if we're on a file in an expanded commit
                if let (Some(sha), Some(file_idx)) =
                    (self.expanded_commit.clone(), self.expanded_file_idx)
                {
                    let file_path = self
                        .expanded_detail
                        .as_ref()
                        .and_then(|d| d.files.get(file_idx))
                        .map(|f| f.path.clone());

                    if let Some(path) = file_path {
                        self.open_commit_file_diff(&sha, &path);
                    }
                } else {
                    // Working directory file diff
                    self.show_diff();
                }
            }

            // External difftool (uses gitconfig diff.tool)
            KeyCode::Char('M') => {
                if let (Some(sha), Some(file_idx)) =
                    (self.expanded_commit.clone(), self.expanded_file_idx)
                {
                    let file_path = self
                        .expanded_detail
                        .as_ref()
                        .and_then(|d| d.files.get(file_idx))
                        .map(|f| f.path.clone());

                    if let Some(path) = file_path {
                        self.pending_external = Some(ExternalCommand::DiffTool {
                            commit_sha: sha,
                            file_path: path,
                        });
                    }
                }
            }
            KeyCode::Enter => {
                if self.selected == Some(0) {
                    // Activate command mode when on command section
                    self.enter_command_mode();
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
                if self.selected == Some(0) && !self.command_history.is_empty() {
                    // On command section - enter command mode and show history
                    self.enter_command_mode();
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

            // Copy full sha to clipboard (for history or branch commit items)
            KeyCode::Char('c') => {
                // Works for both history commits and expanded branch commits
                if let Some(sha) = self.selected_activity_sha() {
                    // If expanded, copy full sha from detail
                    if let Some(detail) = &self.expanded_detail {
                        if self.expanded_commit.as_ref() == Some(&sha) {
                            if self.copy_to_clipboard(&detail.full_sha) {
                                self.error = Some(format!("Copied: {}", detail.full_sha));
                            } else {
                                self.error = Some("Failed to copy to clipboard".to_string());
                            }
                            return;
                        }
                    }
                    // Copy short sha
                    if self.copy_to_clipboard(&sha) {
                        self.error = Some(format!("Copied: {sha}"));
                    } else {
                        self.error = Some("Failed to copy to clipboard".to_string());
                    }
                }
            }

            // Toggle expand/collapse for headers, or show commit details
            KeyCode::Char(' ') => {
                // First check if we're on a staged/working file
                if self.selected_file_info().is_some() {
                    self.show_diff();
                } else if self.is_on_history_header() {
                    // Toggle history section expand/collapse
                    if self.history_collapsed {
                        self.expand_history();
                    } else {
                        self.collapse_history();
                    }
                } else if let Some(branch_name) = self.is_on_branch_header() {
                    // Toggle branch expand/collapse
                    if self.expanded_branch.as_deref() == Some(&branch_name) {
                        self.collapse_branch();
                    } else {
                        self.expand_branch(&branch_name);
                    }
                } else if let Some(sha) = self.selected_activity_sha() {
                    // Commit item handling (in history or expanded branch)
                    if self.expanded_commit.as_ref() == Some(&sha) {
                        // Already expanded - check if we're on a file
                        if let Some(file_idx) = self.expanded_file_idx {
                            // Open external pager diff (uses gitconfig core.pager)
                            let file_path = self
                                .expanded_detail
                                .as_ref()
                                .and_then(|d| d.files.get(file_idx))
                                .map(|f| f.path.clone());

                            if let Some(path) = file_path {
                                self.pending_external = Some(ExternalCommand::PagerDiff {
                                    commit_sha: sha,
                                    file_path: path,
                                });
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
                if self.selected == Some(0) && !self.command_output.is_empty() {
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
                self.exit_command_mode();
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit_command_mode();
            }

            // Execute command
            KeyCode::Enter => {
                self.view_mode = ViewMode::Normal;
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

    /// Execute a built-in app action
    fn execute_app_action(&mut self, action: AppAction) {
        match action {
            // File actions
            AppAction::ToggleStage => self.toggle_stage(),
            AppAction::ShowDiff => self.show_diff(),

            // History actions
            AppAction::ToggleHistoryMode => self.toggle_history_mode(),
            AppAction::ExpandCommit => {
                // Handled via space key behavior
                if let Some(sha) = self.selected_activity_sha() {
                    self.expanded_commit = Some(sha.clone());
                    self.expanded_detail = self.repo.commit_detail(&sha).ok();
                    self.expanded_file_idx = None;
                }
            }
            AppAction::CopyShortSha => {
                if let Some(sha) = self.selected_activity_sha() {
                    if self.copy_to_clipboard(&sha) {
                        self.error = Some(format!("Copied: {sha}"));
                    }
                }
            }
            AppAction::CopyFullSha => {
                if let Some(sha) = self.selected_activity_sha() {
                    if let Some(detail) = &self.expanded_detail {
                        if self.expanded_commit.as_ref() == Some(&sha) {
                            if self.copy_to_clipboard(&detail.full_sha) {
                                self.error = Some(format!("Copied: {}", detail.full_sha));
                            }
                            return;
                        }
                    }
                    if self.copy_to_clipboard(&sha) {
                        self.error = Some(format!("Copied: {sha}"));
                    }
                }
            }
            AppAction::NextPage => self.next_history_page(),
            AppAction::PrevPage => self.prev_history_page(),

            // Commit file actions
            AppAction::PagerDiff => {
                if let (Some(sha), Some(file_idx)) =
                    (self.expanded_commit.clone(), self.expanded_file_idx)
                {
                    let file_path = self
                        .expanded_detail
                        .as_ref()
                        .and_then(|d| d.files.get(file_idx))
                        .map(|f| f.path.clone());

                    if let Some(path) = file_path {
                        self.pending_external = Some(ExternalCommand::PagerDiff {
                            commit_sha: sha,
                            file_path: path,
                        });
                    }
                }
            }
            AppAction::InlineDiff => {
                if let (Some(sha), Some(file_idx)) =
                    (self.expanded_commit.clone(), self.expanded_file_idx)
                {
                    let file_path = self
                        .expanded_detail
                        .as_ref()
                        .and_then(|d| d.files.get(file_idx))
                        .map(|f| f.path.clone());

                    if let Some(path) = file_path {
                        self.open_commit_file_diff(&sha, &path);
                    }
                }
            }
            AppAction::DiffTool => {
                if let (Some(sha), Some(file_idx)) =
                    (self.expanded_commit.clone(), self.expanded_file_idx)
                {
                    let file_path = self
                        .expanded_detail
                        .as_ref()
                        .and_then(|d| d.files.get(file_idx))
                        .map(|f| f.path.clone());

                    if let Some(path) = file_path {
                        self.pending_external = Some(ExternalCommand::DiffTool {
                            commit_sha: sha,
                            file_path: path,
                        });
                    }
                }
            }

            // Branch actions
            AppAction::Checkout => self.checkout_selected_branch(),
            AppAction::ExpandBranch => {
                if let Some(branch_name) = self.is_on_branch_header() {
                    self.expand_branch(&branch_name);
                }
            }

            // Global actions
            AppAction::Push => self.show_push_confirm(false),
            AppAction::Pull => self.execute_pull(),
            AppAction::Fetch => self.execute_fetch(),
            AppAction::Refresh => self.refresh_status(),
            AppAction::EnterCommandMode => self.enter_command_mode(),
            AppAction::BrowseAliases => self.enter_alias_browser(),
            AppAction::ShowHelp => self.show_help = true,
            AppAction::Quit => self.running = false,

            // Navigation
            AppAction::JumpToWorking => self.jump_to_working(),
            AppAction::JumpToHistory => self.jump_to_history(),
            AppAction::JumpToBranches => self.jump_to_branches(),
        }
    }

    /// Handle keyboard input in popup mode
    fn handle_popup_key(&mut self, key: KeyEvent) {
        match key.code {
            // Close popup
            KeyCode::Esc | KeyCode::Char('q') => {
                self.popup.close();
            }

            // Scroll down
            KeyCode::Char('j') | KeyCode::Down => {
                self.popup.scroll_down(1);
            }

            // Scroll up
            KeyCode::Char('k') | KeyCode::Up => {
                self.popup.scroll_up(1);
            }

            // Page down
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.popup.page_down();
            }

            // Page up
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.popup.page_up();
            }

            // Jump to top
            KeyCode::Char('g') => {
                self.popup.scroll_to_top();
            }

            // Jump to bottom
            KeyCode::Char('G') => {
                self.popup.scroll_to_bottom();
            }

            _ => {}
        }
    }

    /// Handle key events when menu stack is active
    fn handle_menu_key(&mut self, key: KeyEvent) {
        use crate::menu::MenuAction;

        // Get the result from the active menu
        let result = if let Some(menu) = self.menu_stack.current_mut() {
            menu.handle_key(key)
        } else {
            return;
        };

        // Process the result
        match result {
            MenuResult::Continue => {}
            MenuResult::Close => {
                self.menu_stack.pop();
            }
            MenuResult::Execute(action) => {
                self.menu_stack.clear();
                match action {
                    MenuAction::Command(request) => {
                        self.run_command(request);
                    }
                    MenuAction::App(app_action) => {
                        self.execute_app_action(app_action);
                    }
                    MenuAction::Custom(callback) => {
                        callback();
                    }
                }
            }
            MenuResult::Push(menu) => {
                self.menu_stack.push(menu);
            }
            MenuResult::Pop => {
                self.menu_stack.pop();
            }
            MenuResult::CloseAll => {
                self.menu_stack.clear();
            }
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

    /// Show push confirmation menu
    fn show_push_confirm(&mut self, force: bool) {
        let branch = match &self.status.branch {
            Some(b) => b.clone(),
            None => {
                self.error = Some("No branch checked out".to_string());
                return;
            }
        };

        // Extract remote name from upstream (e.g., "origin/main" -> "origin")
        let (remote, has_upstream) = match &self.status.upstream {
            Some(upstream) => {
                let remote = upstream.split('/').next().unwrap_or("origin").to_string();
                (remote, true)
            }
            None => ("origin".to_string(), false),
        };

        let menu = PushConfirmMenu::new(
            branch,
            remote,
            has_upstream,
            self.status.ahead,
            force,
        );
        self.menu_stack.push(Box::new(menu));
    }

    /// Execute git pull
    fn execute_pull(&mut self) {
        let request = CommandRequest::git(["pull"])
            .with_source(CommandSource::Keyboard);
        self.run_command(request);
    }

    /// Execute git fetch
    fn execute_fetch(&mut self) {
        let request = CommandRequest::git(["fetch"])
            .with_source(CommandSource::Keyboard);
        self.run_command(request);
    }

    /// Total count of all selectable items
    pub fn total_count(&self) -> usize {
        let files_total = self.status.staged_changes().len() + self.status.working_changes().len();

        // History section: 1 header + (commits if expanded)
        let history_items = if self.history_collapsed {
            1
        } else {
            1 + self.activity.len()
        };

        // Branches section: only expanded branch commits are selectable (not headers)
        let branch_items = if self.expanded_branch.is_some() {
            self.expanded_branch_commits.len()
        } else {
            0
        };

        1 + files_total + history_items + branch_items
    }

    /// Get the selected file info
    /// Returns (path, is_staged, file_state) or None if selection is on command or activity
    fn selected_file_info(&self) -> Option<(PathBuf, bool, FileState)> {
        let selected = self.selected?;

        // Index 0 is command section
        if selected == 0 {
            return None;
        }

        let staged = self.status.staged_changes();
        let working = self.status.working_changes();
        let staged_len = staged.len();

        // Adjust for command section at index 0
        let file_idx = selected - 1;

        if file_idx < staged_len {
            // Selected is in staged
            staged.get(file_idx).map(|f| (f.path.clone(), true, f.staged))
        } else if file_idx < staged_len + working.len() {
            // Selected is in working
            let working_idx = file_idx - staged_len;
            working.get(working_idx).map(|f| (f.path.clone(), false, f.working))
        } else {
            // Selected is in activity or branches
            None
        }
    }

    /// Get the selected commit's SHA (from history or expanded branch)
    fn selected_activity_sha(&self) -> Option<String> {
        let selected = self.selected?;
        let files_total = self.status.staged_changes().len() + self.status.working_changes().len();

        // History header is at files_total + 1
        let history_header_idx = 1 + files_total;

        // Check if in history commits (not collapsed)
        if !self.history_collapsed && selected > history_header_idx {
            let history_commits_start = history_header_idx + 1;
            let history_commits_end = history_commits_start + self.activity.len();

            if selected >= history_commits_start && selected < history_commits_end {
                let commit_idx = selected - history_commits_start;
                return self.activity.get(commit_idx).and_then(|cmd| cmd.sha.clone());
            }
        }

        // Check if in expanded branch commits
        let branches_start = self.branches_start_index();
        if self.expanded_branch.is_some() && selected >= branches_start {
            let commit_idx = selected - branches_start;
            return self.expanded_branch_commits.get(commit_idx).and_then(|cmd| cmd.sha.clone());
        }

        None
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

    /// Select next item with accordion auto-expand/collapse
    fn select_next(&mut self) {
        let len = self.total_count();
        if len == 0 {
            return;
        }

        match self.selected {
            None => {
                self.selected = Some(0);
            }
            Some(idx) => {
                self.close_expanded_commit();

                let files_total = self.status.staged_changes().len() + self.status.working_changes().len();
                let history_header_idx = 1 + files_total;

                // Are we on the last history commit about to move to branches?
                let on_last_history_commit = !self.history_collapsed
                    && !self.activity.is_empty()
                    && idx == history_header_idx + self.activity.len();

                if on_last_history_commit {
                    // Moving from last history commit to first branch's first commit
                    let other_branches = self.other_branches();
                    if let Some(first_branch) = other_branches.first() {
                        let branch_name = first_branch.name.clone();
                        self.expand_branch(&branch_name);
                        self.selected = Some(self.branches_start_index());
                        return;
                    }
                }

                // Don't go past the last item if no branches to expand
                if idx >= len - 1 {
                    return;
                }

                // Check if we're on an expanded branch's last commit
                if self.expanded_branch.is_some() {
                    let branches_start = self.branches_start_index();
                    let last_commit_idx = branches_start + self.expanded_branch_commits.len() - 1;

                    if idx == last_commit_idx {
                        // Find next branch
                        let branch_names: Vec<String> = self.other_branches().iter().map(|b| b.name.clone()).collect();
                        let expanded_name = self.expanded_branch.as_ref().unwrap().clone();
                        let mut found = false;
                        let mut next_branch: Option<String> = None;
                        for name in &branch_names {
                            if found {
                                next_branch = Some(name.clone());
                                break;
                            }
                            if name == &expanded_name {
                                found = true;
                            }
                        }
                        if let Some(next_name) = next_branch {
                            self.expand_branch(&next_name);
                            self.selected = Some(self.branches_start_index());
                        }
                        // No next branch or expanded next - stay at last commit
                        return;
                    }
                }

                // Normal navigation
                let new_idx = idx + 1;
                self.selected = Some(new_idx);

                // Skip history header - auto-expand and go to first commit
                if new_idx == history_header_idx && !self.activity.is_empty() {
                    self.expand_history();
                    self.selected = Some(new_idx + 1);
                }
            }
        }
    }

    /// Select previous item with accordion auto-expand/collapse
    fn select_prev(&mut self) {
        match self.selected {
            Some(idx) if idx > 0 => {
                self.close_expanded_commit();

                let files_total = self.status.staged_changes().len() + self.status.working_changes().len();
                let history_header_idx = 1 + files_total;

                // Check if we're on the first commit of an expanded branch
                if self.expanded_branch.is_some() {
                    let branches_start = self.branches_start_index();
                    if idx == branches_start {
                        // On first commit of expanded branch - go to previous branch or history
                        let other_branches = self.other_branches();
                        let expanded_name = self.expanded_branch.as_ref().unwrap().clone();
                        let mut prev_branch: Option<String> = None;

                        for branch in other_branches {
                            if branch.name == expanded_name {
                                break;
                            }
                            prev_branch = Some(branch.name.clone());
                        }

                        if let Some(prev_name) = prev_branch {
                            // Expand previous branch and go to its last commit
                            self.expand_branch(&prev_name);
                            let new_start = self.branches_start_index();
                            self.selected = Some(new_start + self.expanded_branch_commits.len() - 1);
                        } else {
                            // First branch - go to history
                            self.expand_history();
                            self.selected = Some(history_header_idx + self.activity.len());
                        }
                        return;
                    }
                }

                // Normal navigation
                let new_idx = idx - 1;
                self.selected = Some(new_idx);

                // Skip history header - go to previous item (last file or command)
                if new_idx == history_header_idx {
                    self.selected = Some(new_idx - 1);
                }
            }
            _ => {}
        }
    }

    /// Select first item
    fn select_first(&mut self) {
        self.selected = Some(0);
        self.close_expanded_commit();
    }

    /// Select last item
    fn select_last(&mut self) {
        let len = self.total_count();
        if len > 0 {
            self.selected = Some(len - 1);
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

    // ─────────────────────────────────────────────────────────────────────────
    // Unified Command Execution
    // ─────────────────────────────────────────────────────────────────────────

    /// Execute a command using the unified framework
    ///
    /// This is the single entry point for all command execution. It handles:
    /// - Running the command via CommandExecutor
    /// - Recording in history
    /// - Displaying output (inline or popup based on source and result)
    /// - Refreshing git status if requested
    fn run_command(&mut self, request: CommandRequest) {
        // Execute via the executor
        let result = self.executor.execute(&request);

        // Record in history (deduplicated)
        if self.command_history.last().map(String::as_str) != Some(&request.display_name) {
            self.command_history.push(request.display_name.clone());
            if self.command_history.len() > MAX_COMMAND_HISTORY {
                self.command_history.remove(0);
            }
        }

        // Store output for display
        self.command_output = result.display_output().to_string();
        self.command_success = result.success;

        // Determine if popup should auto-open based on source and result
        let should_popup = match request.feedback {
            FeedbackPolicy::AlwaysPopup => true,
            FeedbackPolicy::InlineOnly => false,
            FeedbackPolicy::Silent => false,
            FeedbackPolicy::External => false,
            FeedbackPolicy::Default => {
                // Source-based defaults:
                // - ActionMenu/AliasBrowser: Always popup (deliberate selection deserves feedback)
                // - Keyboard/Palette: Popup on failure or long output
                match request.source {
                    CommandSource::ActionMenu | CommandSource::AliasBrowser => true,
                    CommandSource::Keyboard | CommandSource::Palette | CommandSource::Internal => {
                        !result.success || result.line_count() > AUTO_POPUP_LINE_THRESHOLD
                    }
                }
            }
        };

        if should_popup {
            self.open_output_popup();
        }

        // Refresh git status if requested
        if request.refresh_after {
            self.refresh_status();
        }
    }

    /// Execute command from the command palette (: mode)
    fn execute_command(&mut self) {
        let input = self.command_input.trim();
        if input.is_empty() {
            return;
        }

        // Parse command - for now still restrict to git commands for safety
        // TODO: Make this configurable for allowed command prefixes
        let Some(request) = CommandRequest::from_input(input) else {
            return;
        };

        if request.program != "git" {
            self.command_output = String::from("Error: Only git commands are allowed");
            self.command_success = false;
            self.command_input.clear();
            self.history_index = None;
            return;
        }

        // Execute using unified framework (source = Palette)
        self.run_command(request.with_source(CommandSource::Palette));

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
