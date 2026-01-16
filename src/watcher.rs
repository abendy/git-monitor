use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind, Debouncer};

/// File system watcher for git repository changes
pub struct RepoWatcher {
    #[allow(dead_code)]
    debouncer: Debouncer<notify::RecommendedWatcher>,
}

impl RepoWatcher {
    /// Create a new watcher for the given repository path
    pub fn new(repo_path: &Path, tx: Sender<WatchEvent>) -> Result<Self> {
        let event_tx = tx.clone();

        // Create debouncer with 100ms delay
        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            move |result: Result<Vec<DebouncedEvent>, notify::Error>| {
                if let Ok(events) = result {
                    for event in events {
                        if event.kind == DebouncedEventKind::Any {
                            let watch_event = categorize_path(&event.path);
                            // Send event, ignore if receiver dropped
                            let _ = event_tx.send(watch_event);
                        }
                    }
                }
            },
        )
        .context("Failed to create file watcher")?;

        // Watch the working directory
        debouncer
            .watcher()
            .watch(repo_path, RecursiveMode::Recursive)
            .with_context(|| {
                format!(
                    "Failed to watch {}",
                    repo_path.display()
                )
            })?;

        // Also watch .git directory for index changes
        let git_dir = repo_path.join(".git");
        if git_dir.exists() {
            // Watch specific git files, not the whole .git (too noisy)
            let index_path = git_dir.join("index");
            if index_path.exists() {
                let _ = debouncer
                    .watcher()
                    .watch(&index_path, RecursiveMode::NonRecursive);
            }

            let head_path = git_dir.join("HEAD");
            if head_path.exists() {
                let _ = debouncer
                    .watcher()
                    .watch(&head_path, RecursiveMode::NonRecursive);
            }

            // Watch refs for branch changes
            let refs_path = git_dir.join("refs");
            if refs_path.exists() {
                let _ = debouncer
                    .watcher()
                    .watch(&refs_path, RecursiveMode::Recursive);
            }

            // Watch logs for reflog changes
            let logs_path = git_dir.join("logs");
            if logs_path.exists() {
                let _ = debouncer
                    .watcher()
                    .watch(&logs_path, RecursiveMode::Recursive);
            }
        }

        Ok(Self { debouncer })
    }
}

/// Types of watch events
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// Working directory file changed
    #[allow(dead_code)]
    WorkingDirectory(PathBuf),
    /// Git index (staging area) changed
    GitIndex,
    /// Git HEAD changed (branch switch, commit)
    GitHead,
    /// Git refs changed
    GitRefs,
}

/// Categorize a path into a watch event type
fn categorize_path(path: &Path) -> WatchEvent {
    let path_str = path.to_string_lossy();

    if path_str.contains(".git/index") {
        WatchEvent::GitIndex
    } else if path_str.contains(".git/HEAD") {
        WatchEvent::GitHead
    } else if path_str.contains(".git/refs") {
        WatchEvent::GitRefs
    } else {
        WatchEvent::WorkingDirectory(path.to_path_buf())
    }
}
