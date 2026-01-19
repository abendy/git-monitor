use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, DebouncedEventKind, Debouncer};
use tracing::warn;

/// File system watcher for git repository changes
pub struct RepoWatcher {
    #[allow(dead_code)]
    debouncer: Debouncer<notify::RecommendedWatcher>,
}

impl RepoWatcher {
    /// Create a new watcher for the given repository path
    pub fn new(repo_path: &Path, tx: Sender<WatchEvent>) -> Result<Self> {
        let event_tx = tx;

        // Create debouncer with 100ms delay
        let mut debouncer = new_debouncer(
            Duration::from_millis(100),
            move |result: Result<Vec<DebouncedEvent>, notify::Error>| {
                if let Ok(events) = result {
                    for event in events {
                        if event.kind == DebouncedEventKind::Any {
                            let watch_event = categorize_path(&event.path);
                            // Send event to main loop. Intentionally ignore send errors -
                            // this happens during shutdown when the receiver is dropped,
                            // which is expected and harmless.
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
                if let Err(e) = debouncer
                    .watcher()
                    .watch(&index_path, RecursiveMode::NonRecursive)
                {
                    warn!("Failed to watch git index: {}", e);
                }
            }

            let head_path = git_dir.join("HEAD");
            if head_path.exists() {
                if let Err(e) = debouncer
                    .watcher()
                    .watch(&head_path, RecursiveMode::NonRecursive)
                {
                    warn!("Failed to watch git HEAD: {}", e);
                }
            }

            // Watch refs for branch changes
            let refs_path = git_dir.join("refs");
            if refs_path.exists() {
                if let Err(e) = debouncer
                    .watcher()
                    .watch(&refs_path, RecursiveMode::Recursive)
                {
                    warn!("Failed to watch git refs: {}", e);
                }
            }

            // Watch logs for reflog changes
            let logs_path = git_dir.join("logs");
            if logs_path.exists() {
                if let Err(e) = debouncer
                    .watcher()
                    .watch(&logs_path, RecursiveMode::Recursive)
                {
                    warn!("Failed to watch git logs: {}", e);
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    mod categorize_path {
        use super::*;

        #[test]
        fn detects_git_index() {
            let path = Path::new("/repo/.git/index");
            let event = categorize_path(path);
            assert!(matches!(event, WatchEvent::GitIndex));
        }

        #[test]
        fn detects_git_index_lock() {
            let path = Path::new("/repo/.git/index.lock");
            let event = categorize_path(path);
            // index.lock contains "index" so it's still GitIndex
            assert!(matches!(event, WatchEvent::GitIndex));
        }

        #[test]
        fn detects_git_head() {
            let path = Path::new("/repo/.git/HEAD");
            let event = categorize_path(path);
            assert!(matches!(event, WatchEvent::GitHead));
        }

        #[test]
        fn detects_git_refs() {
            let path = Path::new("/repo/.git/refs/heads/main");
            let event = categorize_path(path);
            assert!(matches!(event, WatchEvent::GitRefs));
        }

        #[test]
        fn detects_git_refs_remotes() {
            let path = Path::new("/repo/.git/refs/remotes/origin/main");
            let event = categorize_path(path);
            assert!(matches!(event, WatchEvent::GitRefs));
        }

        #[test]
        fn detects_working_directory_file() {
            let path = Path::new("/repo/src/main.rs");
            let event = categorize_path(path);
            assert!(matches!(event, WatchEvent::WorkingDirectory(_)));

            if let WatchEvent::WorkingDirectory(p) = event {
                assert_eq!(p, PathBuf::from("/repo/src/main.rs"));
            }
        }

        #[test]
        fn detects_working_directory_nested() {
            let path = Path::new("/repo/src/git/mod.rs");
            let event = categorize_path(path);
            // "git" in path is not ".git", so it's working directory
            assert!(matches!(event, WatchEvent::WorkingDirectory(_)));
        }
    }
}
