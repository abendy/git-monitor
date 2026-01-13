use std::path::PathBuf;

use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{event::Event, tui::Tui, ui};

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
    /// Whether the application is running
    pub running: bool,
    /// Currently active panel
    pub active_panel: Panel,
    /// Show help overlay
    pub show_help: bool,
}

impl App {
    /// Create a new application instance
    pub fn new(path: PathBuf) -> Result<Self> {
        let repo_path = path
            .canonicalize()
            .with_context(|| format!("Invalid path: {}", path.display()))?;

        Ok(Self {
            repo_path,
            running: true,
            active_panel: Panel::default(),
            show_help: false,
        })
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
                Event::Resize(_, _) => {}
                Event::Mouse(_) => {}
            }
        }

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key(&mut self, key: KeyEvent) {
        // Global keys
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                } else {
                    self.running = false;
                }
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.running = false;
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Tab => {
                self.active_panel = self.active_panel.next();
            }
            KeyCode::BackTab => {
                self.active_panel = self.active_panel.prev();
            }
            _ => {}
        }
    }

    /// Handle tick events (periodic updates)
    fn on_tick(&mut self) {
        // Future: refresh git status, update timestamps, etc.
    }
}
