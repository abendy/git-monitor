use std::{path::PathBuf, sync::mpsc::Sender};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::warn;

use crate::{
    actions::{Action, ActionRegistry, AppAction, AppState, Context},
    command::{CommandExecutor, CommandHistory, CommandRequest, CommandSource, ExternalCommand},
    config::GitConfig,
    event::Event,
    feedback::{Feedback, FeedbackManager, PopupContent},
    git::{BranchInfo, CommitDetail, FileState, GitCommand, GitRepo, GitStatus},
    input::Keymap,
    menu::{ActionMenu, AliasSectionMenu, MenuResult, MenuStack, PushConfirmMenu},
    section::{
        BranchesSection, BranchesSectionData, CommandSection, CommandSectionData, HistorySection,
        HistorySectionData, Section, SectionId, SectionItemCounts, SectionRegistry, StagedSection,
        StagedSectionData, WorkingSection, WorkingSectionData,
    },
    tui::Tui,
    ui,
    watcher::{RepoWatcher, WatchEvent},
};

/// Page size for history pagination
const PAGE_SIZE: usize = 50;

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
    /// Current command input buffer
    pub command_input: String,
    /// Command history (managed by CommandHistory module)
    pub command_history: CommandHistory,
    /// Current history display mode (reflog vs commit log)
    pub history_mode: HistoryMode,
    /// Feedback manager (handles output, popups, toasts, errors)
    pub feedback: FeedbackManager,
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
    /// Declarative keymap for action lookup
    keymap: Keymap,
    // ─────────────────────────────────────────────────────────────────────────
    // Section instances (for modular architecture)
    // ─────────────────────────────────────────────────────────────────────────
    /// Command input section
    pub command_section: CommandSection,
    /// Staged files section
    pub staged_section: StagedSection,
    /// Working files section
    pub working_section: WorkingSection,
    /// History section (commits/reflog)
    pub history_section: HistorySection,
    /// Branches section
    pub branches_section: BranchesSection,
    /// Section registry for index calculations
    section_registry: SectionRegistry,
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

        let mut app = Self {
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
            command_input: String::new(),
            command_history: CommandHistory::new(),
            history_mode: HistoryMode::default(),
            feedback: FeedbackManager::new(),
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
            keymap: Keymap::with_defaults(),
            command_section: CommandSection::new(),
            staged_section: StagedSection::new(),
            working_section: WorkingSection::new(),
            history_section: HistorySection::new(),
            branches_section: BranchesSection::new(),
            section_registry: SectionRegistry::new(),
        };

        // Update sections with initial state
        app.update_sections();

        Ok(app)
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
        self.command_history.reset_navigation();
        self.update_sections();
    }

    /// Exit command mode (return to normal)
    pub fn exit_command_mode(&mut self) {
        self.view_mode = ViewMode::Normal;
        self.command_input.clear();
        self.command_history.reset_navigation();
        self.update_sections();
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
    // Section management
    // ─────────────────────────────────────────────────────────────────────────

    /// Update all sections with current app state
    ///
    /// Called after state changes (refresh, mode change, etc.) to keep
    /// section data in sync with app state.
    pub fn update_sections(&mut self) {
        // Update command section
        self.command_section.update(CommandSectionData {
            input: self.command_input.clone(),
            is_active: self.is_command_mode(),
            output: self.feedback.output().cloned(),
            menu_active: self.menu_stack.is_active(),
        });

        // Update staged section
        self.staged_section.update(StagedSectionData {
            files: self
                .status
                .staged_changes()
                .into_iter()
                .cloned()
                .collect(),
            command_mode_active: self.is_command_mode(),
            action_registry: Some(self.action_registry.clone()),
            app_state: Some(self.app_state()),
        });

        // Update working section
        self.working_section.update(WorkingSectionData {
            files: self
                .status
                .working_changes()
                .into_iter()
                .cloned()
                .collect(),
            command_mode_active: self.is_command_mode(),
            action_registry: Some(self.action_registry.clone()),
            app_state: Some(self.app_state()),
        });

        // Update history section
        self.history_section.update(HistorySectionData {
            activity: self.activity.clone(),
            history_mode: self.history_mode,
            is_collapsed: self.history_collapsed,
            page: self.history_page,
            current_branch: self.branches.iter().find(|b| b.is_current).cloned(),
            status: self.status.clone(),
            expanded_commit: self.expanded_commit.clone(),
            expanded_detail: self.expanded_detail.clone(),
            command_mode_active: self.is_command_mode(),
            action_registry: Some(self.action_registry.clone()),
            app_state: Some(self.app_state()),
        });

        // Update branches section
        self.branches_section.update(BranchesSectionData {
            branches: self.branches.clone(),
            expanded_branch: self.expanded_branch.clone(),
            expanded_branch_commits: self.expanded_branch_commits.clone(),
            expanded_commit: self.expanded_commit.clone(),
            expanded_detail: self.expanded_detail.clone(),
            command_mode_active: self.is_command_mode(),
            action_registry: Some(self.action_registry.clone()),
            app_state: Some(self.app_state()),
        });
    }

    /// Get item counts for all sections (for registry calculations)
    #[must_use]
    pub fn section_item_counts(&self) -> SectionItemCounts {
        SectionItemCounts::from_sections(
            &self.command_section,
            &self.staged_section,
            &self.working_section,
            &self.history_section,
            &self.branches_section,
        )
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
                self.feedback.error = None;
            }
            Err(e) => {
                warn!("Failed to refresh git status: {}", e);
                self.feedback.error = Some(format!("Git error: {e}"));
            }
        }

        // Refresh activity log based on history mode
        self.refresh_activity();

        // Refresh branches
        if let Ok(branches) = self.repo.list_branches() {
            self.branches = branches;
        }

        // Update section data
        self.update_sections();
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
        let counts = self.section_item_counts();
        self.section_registry
            .section_start_index(SectionId::Branches, &counts)
            .unwrap_or(0)
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

        // Check if in expanded commit files (special case not in registry)
        if self.expanded_commit.is_some() && self.expanded_file_idx.is_some() {
            return Context::CommitFiles;
        }

        // Use registry for standard section context resolution
        let counts = self.section_item_counts();
        self.section_registry.context_for_index(selected, &counts)
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
                self.feedback.popup.open(PopupContent::Diff {
                    path: file_path.to_string(),
                    content: diff_content,
                    is_staged: false,
                });
            }
            Err(e) => {
                self.feedback.error = Some(format!("Failed to get diff: {e}"));
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
                self.feedback.error = Some(format!("Failed to load branch history: {e}"));
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
                self.feedback.error = Some("Already on this branch".to_string());
                return;
            }
            Some(branch) => branch.name.clone(),
            None => return,
        };

        match self.repo.checkout_branch(&branch_name) {
            Ok(()) => {
                self.feedback.error = Some(format!("Switched to branch '{branch_name}'"));
                self.refresh_status();
            }
            Err(e) => {
                self.feedback.error = Some(e.to_string());
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

        // Execute the external command (inherits stdio)
        if let Err(e) = cmd.execute(&self.repo_path) {
            self.feedback.error = Some(format!("Failed to run {}: {e}", cmd.description()));
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
        if self.feedback.popup.is_open() {
            self.handle_popup_key(key);
            return;
        }

        // Help overlay captures keys
        if self.show_help {
            self.show_help = false;
            return;
        }

        // Try declarative keymap lookup first
        let context = self.current_context();
        if let Some(action) = self.keymap.lookup(key, context) {
            self.execute_app_action(action);
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
                // First check if we're on a working/staged file
                if let Some((path, is_staged, _)) = self.selected_file_info() {
                    self.pending_external = Some(ExternalCommand::FileDiffTool {
                        file_path: path.to_string_lossy().to_string(),
                        staged: is_staged,
                    });
                } else if let (Some(sha), Some(file_idx)) =
                    (self.expanded_commit.clone(), self.expanded_file_idx)
                {
                    // Commit file difftool
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
                if self.selected == Some(0) && !self.command_history.commands().is_empty() {
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
                        self.feedback.error = Some(format!("Copied: {sha}"));
                    } else {
                        self.feedback.error = Some("Failed to copy to clipboard".to_string());
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
                                self.feedback.error = Some(format!("Copied: {}", detail.full_sha));
                            } else {
                                self.feedback.error = Some("Failed to copy to clipboard".to_string());
                            }
                            return;
                        }
                    }
                    // Copy short sha
                    if self.copy_to_clipboard(&sha) {
                        self.feedback.error = Some(format!("Copied: {sha}"));
                    } else {
                        self.feedback.error = Some("Failed to copy to clipboard".to_string());
                    }
                }
            }

            // Interactive rebase onto selected commit (history only)
            KeyCode::Char('R') => {
                if self.is_in_history() {
                    self.execute_app_action(AppAction::InteractiveRebase);
                }
            }

            // Toggle expand/collapse for headers, or show commit details
            KeyCode::Char(' ') => {
                // First check if we're on a staged/working file - open in pager
                if let Some((path, is_staged, _)) = self.selected_file_info() {
                    self.pending_external = Some(ExternalCommand::FilePagerDiff {
                        file_path: path.to_string_lossy().to_string(),
                        staged: is_staged,
                    });
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
                if self.selected == Some(0) && self.feedback.has_output() {
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
                self.command_history.reset_navigation();
            }

            // Backspace
            KeyCode::Backspace => {
                self.command_input.pop();
                self.command_history.reset_navigation();
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
            AppAction::FilePagerDiff => {
                if let Some((path, is_staged, _)) = self.selected_file_info() {
                    self.pending_external = Some(ExternalCommand::FilePagerDiff {
                        file_path: path.to_string_lossy().to_string(),
                        staged: is_staged,
                    });
                }
            }
            AppAction::FileDiffTool => {
                if let Some((path, is_staged, _)) = self.selected_file_info() {
                    self.pending_external = Some(ExternalCommand::FileDiffTool {
                        file_path: path.to_string_lossy().to_string(),
                        staged: is_staged,
                    });
                }
            }

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
                        self.feedback.error = Some(format!("Copied: {sha}"));
                    }
                }
            }
            AppAction::CopyFullSha => {
                if let Some(sha) = self.selected_activity_sha() {
                    if let Some(detail) = &self.expanded_detail {
                        if self.expanded_commit.as_ref() == Some(&sha) {
                            if self.copy_to_clipboard(&detail.full_sha) {
                                self.feedback.error = Some(format!("Copied: {}", detail.full_sha));
                            }
                            return;
                        }
                    }
                    if self.copy_to_clipboard(&sha) {
                        self.feedback.error = Some(format!("Copied: {sha}"));
                    }
                }
            }
            AppAction::NextPage => self.next_history_page(),
            AppAction::PrevPage => self.prev_history_page(),
            AppAction::InteractiveRebase => {
                // Start interactive rebase onto the selected commit
                if let Some(sha) = self.selected_activity_sha() {
                    self.pending_external = Some(ExternalCommand::InteractiveRebase { onto: sha });
                }
            }

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
                self.feedback.popup.close();
            }

            // Scroll down
            KeyCode::Char('j') | KeyCode::Down => {
                self.feedback.popup.scroll_down(1);
            }

            // Scroll up
            KeyCode::Char('k') | KeyCode::Up => {
                self.feedback.popup.scroll_up(1);
            }

            // Page down
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.feedback.popup.page_down();
            }

            // Page up
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.feedback.popup.page_up();
            }

            // Jump to top
            KeyCode::Char('g') => {
                self.feedback.popup.scroll_to_top();
            }

            // Jump to bottom
            KeyCode::Char('G') => {
                self.feedback.popup.scroll_to_bottom();
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
        self.feedback.expand_output();
    }

    /// Open a diff in the popup
    fn open_diff_popup(&mut self, path: String, content: String, is_staged: bool) {
        self.feedback.popup.open(PopupContent::Diff {
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
                self.feedback.error = Some("No branch checked out".to_string());
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
        let counts = self.section_item_counts();
        self.section_registry.total_items(&counts)
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
            self.feedback.error = Some(format!("Error: {e}"));
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
                    self.feedback.error = Some(format!("Error reading file: {e}"));
                }
            }
            return;
        }

        match self.repo.diff_file(&path, is_staged) {
            Ok(content) => {
                self.open_diff_popup(path_str, content, is_staged);
            }
            Err(e) => {
                self.feedback.error = Some(format!("Diff error: {e}"));
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
                if let Some(expanded_name) = self.expanded_branch.clone() {
                    let branches_start = self.branches_start_index();
                    let last_commit_idx = branches_start + self.expanded_branch_commits.len() - 1;

                    if idx == last_commit_idx {
                        // Find next branch
                        let branch_names: Vec<String> = self.other_branches().iter().map(|b| b.name.clone()).collect();
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
                if let Some(expanded_name) = self.expanded_branch.clone() {
                    let branches_start = self.branches_start_index();
                    if idx == branches_start {
                        // On first commit of expanded branch - go to previous branch or history
                        let other_branches = self.other_branches();
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
        // Handle time-based feedback updates (toast auto-dismiss)
        self.feedback.tick();
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

        // Record in history (handles deduplication and persistence)
        self.command_history.add(request.display_name.clone());

        // Show feedback (handles inline output, popup logic, etc.)
        self.feedback.show(
            Feedback::CommandOutput {
                command: request.display_name.clone(),
                result,
                source: request.source,
            },
            request.feedback,
        );

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
            self.feedback.error = Some(String::from("Only git commands are allowed"));
            self.command_input.clear();
            self.command_history.reset_navigation();
            return;
        }

        // Execute using unified framework (source = Palette)
        self.run_command(request.with_source(CommandSource::Palette));

        // Clear input and reset history navigation
        self.command_input.clear();
        self.command_history.reset_navigation();
    }

    /// Navigate command history (older)
    fn history_prev(&mut self) {
        if let Some(cmd) = self.command_history.navigate_older() {
            self.command_input = cmd.to_string();
        }
    }

    /// Navigate command history (newer)
    fn history_next(&mut self) {
        match self.command_history.navigate_newer() {
            Some(cmd) => {
                self.command_input = cmd.to_string();
            }
            None => {
                // Past end of history, clear input
                self.command_input.clear();
            }
        }
    }
}
