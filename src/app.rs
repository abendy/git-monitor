use std::{path::PathBuf, sync::mpsc::Sender};

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
    /// Error message to display
    pub error: Option<String>,
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
            error: None,
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
        // Help overlay captures all keys
        if self.show_help {
            self.show_help = false;
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

            // Help
            KeyCode::Char('?') => {
                self.show_help = true;
            }

            // Refresh
            KeyCode::Char('r') => {
                self.refresh_status();
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
}
