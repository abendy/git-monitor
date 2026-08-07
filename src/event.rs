use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};

/// Terminal events
#[derive(Debug, Clone)]
pub enum Event {
    /// Terminal tick (for periodic updates)
    Tick,
    /// Keyboard input
    Key(KeyEvent),
    /// Mouse input
    #[allow(dead_code)]
    Mouse(MouseEvent),
    /// Terminal resize
    #[allow(dead_code)]
    Resize(u16, u16),
    /// File system change detected
    FileChanged,
}

/// Handles terminal events in a separate thread
pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,
    paused: Arc<AtomicBool>,
}

impl EventHandler {
    /// Create a new event handler with the given tick rate
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::channel();
        let event_tx = tx.clone();
        let paused = Arc::new(AtomicBool::new(false));
        let paused_clone = paused.clone();

        thread::spawn(move || {
            let mut last_tick = Instant::now();

            loop {
                // If paused, just sleep briefly and continue
                if paused_clone.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }

                // Calculate timeout until next tick
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(Duration::ZERO);

                // Poll for events
                if event::poll(timeout).unwrap_or(false) {
                    let terminal_event = match event::read() {
                        Ok(CrosstermEvent::Key(key)) => Some(Event::Key(key)),
                        Ok(CrosstermEvent::Mouse(mouse)) => Some(Event::Mouse(mouse)),
                        Ok(CrosstermEvent::Resize(width, height)) => {
                            Some(Event::Resize(width, height))
                        }
                        _ => None,
                    };

                    if terminal_event.is_some_and(|event| event_tx.send(event).is_err()) {
                        break;
                    }
                }

                // Send tick if enough time has passed
                if last_tick.elapsed() >= tick_rate {
                    if event_tx.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self { rx, tx, paused }
    }

    /// Pause event handling (for external commands)
    pub fn pause(&self) {
        self.paused
            .store(true, Ordering::Relaxed);
    }

    /// Resume event handling
    pub fn resume(&self) {
        self.paused
            .store(false, Ordering::Relaxed);
    }

    /// Get a sender for external events (file watcher, etc.)
    pub fn sender(&self) -> mpsc::Sender<Event> {
        self.tx.clone()
    }

    /// Get the next event, blocking until one is available
    pub fn next(&self) -> Result<Event> {
        self.rx.recv().map_err(Into::into)
    }
}
