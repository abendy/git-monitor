use std::io::{self, Stdout};
use std::panic;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::event::EventHandler;

pub type Frame<'a> = ratatui::Frame<'a>;

/// Terminal user interface wrapper
pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    pub events: EventHandler,
}

impl Tui {
    /// Create a new TUI instance
    pub fn new(tick_rate: u64) -> Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        let events = EventHandler::new(Duration::from_millis(tick_rate));

        Ok(Self { terminal, events })
    }

    /// Enter the terminal UI mode
    pub fn enter(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;

        // Set up panic hook to restore terminal on panic
        let panic_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            // Intentionally ignore reset errors - we're already panicking,
            // and a double-panic would abort without showing the error
            let _ = Self::reset();
            panic_hook(panic_info);
        }));

        self.terminal.hide_cursor()?;
        self.terminal.clear()?;

        Ok(())
    }

    /// Exit the terminal UI mode
    pub fn exit(&mut self) -> Result<()> {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Reset terminal to normal state
    fn reset() -> Result<()> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        Ok(())
    }

    /// Draw the UI
    pub fn draw<F>(&mut self, render: F) -> Result<()>
    where
        F: FnOnce(&mut Frame<'_>),
    {
        self.terminal.draw(render)?;
        Ok(())
    }

    /// Suspend the TUI to run an external command
    pub fn suspend(&mut self) -> Result<()> {
        Self::reset()?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Resume the TUI after an external command
    pub fn resume(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }
}
