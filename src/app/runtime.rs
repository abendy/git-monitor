use std::sync::mpsc::Sender;

use anyhow::Result;
use tracing::warn;

use crate::command::ExternalCommand;
use crate::event::Event;
use crate::tui::Tui;
use crate::ui;
use crate::watcher::{RepoWatcher, WatchEvent};

use super::{App, HistoryMode, PAGE_SIZE};

impl App {
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
    pub(super) fn refresh_activity(&mut self) {
        self.history_total_items = match self.history_mode {
            HistoryMode::Reflog => self.repo.reflog_total().unwrap_or(0),
            HistoryMode::CommitLog => {
                self.repo.commit_log_total().unwrap_or(0)
            }
        };
        self.history_total_pages = if self.history_total_items == 0 {
            0
        } else {
            (self.history_total_items + PAGE_SIZE - 1) / PAGE_SIZE
        };

        if self.history_total_pages == 0 {
            self.history_page = 0;
        } else if self.history_page >= self.history_total_pages {
            self.history_page = self.history_total_pages - 1;
        }

        let skip = self.history_page * PAGE_SIZE;
        let result = match self.history_mode {
            HistoryMode::Reflog => self.repo.reflog(skip, PAGE_SIZE),
            HistoryMode::CommitLog => self.repo.commit_log(skip, PAGE_SIZE),
        };

        if let Ok(activity) = result {
            self.activity = activity;
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
    pub(super) fn run_external_command(&mut self, tui: &mut Tui, cmd: ExternalCommand) -> Result<()> {
        // Pause event handler so it doesn't consume input meant for the external command
        tui.events.pause();
        // Give the event thread time to finish any pending poll
        std::thread::sleep(std::time::Duration::from_millis(100));
        tui.suspend()?;

        // Execute the external command (inherits stdio)
        if let Err(e) = cmd.execute(&self.repo_path) {
            self.feedback.error = Some(format!(
                "Failed to run {}: {e}",
                cmd.description()
            ));
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

    /// Handle tick events (periodic updates)
    pub(super) fn on_tick(&mut self) {
        // Handle time-based feedback updates (toast auto-dismiss)
        self.feedback.tick();
    }
}
