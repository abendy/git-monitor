use std::sync::mpsc::Sender;

use anyhow::Result;
use tracing::warn;

use super::{App, PAGE_SIZE};
use crate::command::ExternalCommand;
use crate::event::Event;
use crate::git::SnapshotRequest;
use crate::tui::Tui;
use crate::ui;
use crate::watcher::{RepoWatcher, WatchEvent};

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

        // Create the watcher using Git's resolved paths so linked worktrees
        // monitor their external index, HEAD, refs, and reflogs.
        let common_dir = self.repo.common_dir();
        let watcher = RepoWatcher::new(
            &self.repo_path,
            self.repo.git_dir(),
            &common_dir,
            watch_tx,
        )?;
        self.watcher = Some(watcher);

        Ok(())
    }

    /// Reload Git data and keep the cursor on the same item
    pub fn refresh_status(&mut self) {
        let selection = self.selection_key();

        if self.load_snapshot() {
            self.feedback.error = None;
        }
        self.update_sections();

        // Keep the cursor on the same item now that rows may have moved
        self.restore_selection(selection.as_ref());
    }

    /// Read a new snapshot for the current view and swap it in.
    ///
    /// On failure the previous snapshot stays and the error is shown.
    /// Returns whether the swap happened.
    pub(super) fn load_snapshot(&mut self) -> bool {
        let request = SnapshotRequest {
            history_mode: self.history_mode,
            history_page: self.history_page,
            page_size: PAGE_SIZE,
            expanded_branch: self.expanded_branch.clone(),
        };
        match self.repo.snapshot(&request) {
            Ok(snapshot) => {
                self.history_page = snapshot.history_page;
                self.snapshot = snapshot;
                // Forget an expanded branch that no longer exists
                if let Some(name) = &self.expanded_branch {
                    if !self
                        .snapshot
                        .branches
                        .iter()
                        .any(|b| &b.name == name)
                    {
                        self.expanded_branch = None;
                    }
                }
                true
            }
            Err(e) => {
                warn!("Failed to refresh repository: {e:#}");
                self.feedback.error = Some(format!("Git error: {e:#}"));
                false
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
                Event::Resize(_, _) | Event::Mouse(_) => {}
            }
        }

        Ok(())
    }

    /// Run an external command with TUI suspension
    #[allow(clippy::needless_pass_by_value)] // Takes ownership of command for execution
    pub(super) fn run_external_command(
        &mut self,
        tui: &mut Tui,
        cmd: ExternalCommand,
    ) -> Result<()> {
        use std::io::Read as _;

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
        // Intentionally ignore read errors - we just need to pause, and if
        // stdin is broken the TUI will resume anyway
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
