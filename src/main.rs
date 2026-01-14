use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

mod app;
mod config;
mod event;
mod git;
mod tui;
mod ui;
mod watcher;

#[derive(Parser, Debug)]
#[command(name = "git-monitor")]
#[command(about = "Real-time TUI for monitoring git repository activity")]
#[command(version)]
struct Args {
    /// Path to git repository (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    path: PathBuf,

    /// Tick rate in milliseconds
    #[arg(short, long, default_value = "250")]
    tick_rate: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_target(false)
        .init();

    // Run the application
    let mut app = app::App::new(args.path)?;
    let mut tui = tui::Tui::new(args.tick_rate)?;

    tui.enter()?;

    // Set up file watcher
    app.setup_watcher(tui.events.sender())?;

    let result = app.run(&mut tui);
    app.save_history();
    tui.exit()?;

    result
}
