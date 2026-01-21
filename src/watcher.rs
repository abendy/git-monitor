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

    mod repo_watcher {
        use std::fs::{self, File};
        use std::io::Write;
        use std::sync::mpsc;
        use std::thread;
        use std::time::Duration;

        use tempfile::TempDir;

        use super::*;

        fn create_git_repo() -> TempDir {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let git_dir = dir.path().join(".git");
            fs::create_dir(&git_dir).expect("Failed to create .git dir");
            // Create essential git files
            File::create(git_dir.join("index")).expect("Failed to create index");
            File::create(git_dir.join("HEAD")).expect("Failed to create HEAD");
            fs::create_dir(git_dir.join("refs")).expect("Failed to create refs dir");
            fs::create_dir(git_dir.join("logs")).expect("Failed to create logs dir");
            dir
        }

        #[test]
        fn new_initializes_with_valid_directory() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let (tx, _rx) = mpsc::channel();

            let result = RepoWatcher::new(dir.path(), tx);
            assert!(result.is_ok());
        }

        #[test]
        fn new_initializes_with_git_repository() {
            let dir = create_git_repo();
            let (tx, _rx) = mpsc::channel();

            let result = RepoWatcher::new(dir.path(), tx);
            assert!(result.is_ok());
        }

        #[test]
        fn new_fails_for_nonexistent_path() {
            let (tx, _rx) = mpsc::channel();
            let nonexistent = PathBuf::from("/nonexistent/path/that/should/not/exist");

            let result = RepoWatcher::new(&nonexistent, tx);
            assert!(result.is_err());
        }

        #[test]
        fn sends_working_directory_event_on_file_change() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let (tx, rx) = mpsc::channel();

            let _watcher = RepoWatcher::new(dir.path(), tx).expect("Failed to create watcher");

            // Create a file after watcher is set up
            let file_path = dir.path().join("test.txt");
            thread::sleep(Duration::from_millis(200)); // Allow watcher to initialize
            File::create(&file_path)
                .expect("Failed to create file")
                .write_all(b"test")
                .expect("Failed to write");

            // Wait for events
            thread::sleep(Duration::from_millis(300));

            // Collect all events
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }

            assert!(
                !events.is_empty(),
                "Should receive at least one event"
            );

            // Check that we received a working directory event for our file
            let has_test_file_event = events.iter().any(|e| {
                if let WatchEvent::WorkingDirectory(path) = e {
                    path.to_string_lossy()
                        .contains("test.txt")
                } else {
                    false
                }
            });
            assert!(
                has_test_file_event,
                "Should receive WorkingDirectory event for test.txt, got: {:?}",
                events
            );
        }

        #[test]
        fn sends_git_index_event_on_index_change() {
            let dir = create_git_repo();
            let (tx, rx) = mpsc::channel();

            let _watcher = RepoWatcher::new(dir.path(), tx).expect("Failed to create watcher");

            // Modify the git index
            let index_path = dir.path().join(".git/index");
            thread::sleep(Duration::from_millis(200));

            // Write to index file
            let mut file = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&index_path)
                .expect("Failed to open index");
            file.write_all(b"DIRC")
                .expect("Failed to write");
            drop(file); // Ensure file is closed

            // Wait for events - file watcher behavior can vary by platform
            thread::sleep(Duration::from_millis(500));

            // Collect all events
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }

            // We should receive at least some event (may be GitIndex or WorkingDirectory
            // depending on which watcher picks it up first)
            assert!(
                !events.is_empty(),
                "Should receive at least one event"
            );

            // Check that the event is correctly categorized based on path
            // The categorize_path function is tested separately
            let has_git_event = events.iter().any(|e| match e {
                WatchEvent::GitIndex => true,
                WatchEvent::WorkingDirectory(p) => p
                    .to_string_lossy()
                    .contains(".git/index"),
                _ => false,
            });
            assert!(
                has_git_event,
                "Should receive event related to git index, got: {:?}",
                events
            );
        }

        #[test]
        fn sends_git_head_event_on_head_change() {
            let dir = create_git_repo();
            let (tx, rx) = mpsc::channel();

            let _watcher = RepoWatcher::new(dir.path(), tx).expect("Failed to create watcher");

            // Modify HEAD
            let head_path = dir.path().join(".git/HEAD");
            thread::sleep(Duration::from_millis(150));
            let mut file = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&head_path)
                .expect("Failed to open HEAD");
            file.write_all(b"ref: refs/heads/main\n")
                .expect("Failed to write");

            // Wait for events and look for GitHead
            thread::sleep(Duration::from_millis(300));

            let mut found_head_event = false;
            while let Ok(event) = rx.try_recv() {
                if matches!(event, WatchEvent::GitHead) {
                    found_head_event = true;
                    break;
                }
            }
            assert!(
                found_head_event,
                "Should receive a GitHead event"
            );
        }

        #[test]
        fn sends_git_refs_event_on_refs_change() {
            let dir = create_git_repo();
            let refs_heads = dir.path().join(".git/refs/heads");
            fs::create_dir_all(&refs_heads).expect("Failed to create refs/heads");

            let (tx, rx) = mpsc::channel();

            let _watcher = RepoWatcher::new(dir.path(), tx).expect("Failed to create watcher");

            // Create a new branch ref
            thread::sleep(Duration::from_millis(150));
            let branch_path = refs_heads.join("main");
            File::create(&branch_path)
                .expect("Failed to create branch")
                .write_all(b"0000000000000000000000000000000000000000")
                .expect("Failed to write");

            // Wait for events and look for GitRefs
            thread::sleep(Duration::from_millis(300));

            let mut found_refs_event = false;
            while let Ok(event) = rx.try_recv() {
                if matches!(event, WatchEvent::GitRefs) {
                    found_refs_event = true;
                    break;
                }
            }
            assert!(
                found_refs_event,
                "Should receive a GitRefs event"
            );
        }

        #[test]
        fn debounces_rapid_changes() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let (tx, rx) = mpsc::channel();

            let _watcher = RepoWatcher::new(dir.path(), tx).expect("Failed to create watcher");

            // Make rapid changes
            let file_path = dir.path().join("rapid.txt");
            thread::sleep(Duration::from_millis(150));
            for i in 0..5 {
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&file_path)
                    .expect("Failed to open file");
                file.write_all(format!("change {i}").as_bytes())
                    .expect("Failed to write");
                thread::sleep(Duration::from_millis(10)); // Rapid changes
            }

            // Wait for debounced event
            thread::sleep(Duration::from_millis(200));

            // Drain all events
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }

            // Should have fewer events than changes due to debouncing
            assert!(
                events.len() < 5,
                "Expected debouncing to reduce events, got {} events",
                events.len()
            );
        }

        #[test]
        fn watcher_handles_missing_git_index() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let git_dir = dir.path().join(".git");
            fs::create_dir(&git_dir).expect("Failed to create .git dir");
            // Don't create index file
            File::create(git_dir.join("HEAD")).expect("Failed to create HEAD");
            fs::create_dir(git_dir.join("refs")).expect("Failed to create refs dir");

            let (tx, _rx) = mpsc::channel();

            // Should still initialize successfully (logs warning but doesn't fail)
            let result = RepoWatcher::new(dir.path(), tx);
            assert!(result.is_ok());
        }

        #[test]
        fn watcher_handles_missing_git_refs() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let git_dir = dir.path().join(".git");
            fs::create_dir(&git_dir).expect("Failed to create .git dir");
            File::create(git_dir.join("index")).expect("Failed to create index");
            File::create(git_dir.join("HEAD")).expect("Failed to create HEAD");
            // Don't create refs dir

            let (tx, _rx) = mpsc::channel();

            // Should still initialize successfully
            let result = RepoWatcher::new(dir.path(), tx);
            assert!(result.is_ok());
        }

        #[test]
        fn watcher_handles_missing_git_logs() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            let git_dir = dir.path().join(".git");
            fs::create_dir(&git_dir).expect("Failed to create .git dir");
            File::create(git_dir.join("index")).expect("Failed to create index");
            File::create(git_dir.join("HEAD")).expect("Failed to create HEAD");
            fs::create_dir(git_dir.join("refs")).expect("Failed to create refs dir");
            // Don't create logs dir

            let (tx, _rx) = mpsc::channel();

            // Should still initialize successfully
            let result = RepoWatcher::new(dir.path(), tx);
            assert!(result.is_ok());
        }

        #[test]
        fn watcher_works_without_git_directory() {
            let dir = TempDir::new().expect("Failed to create temp dir");
            // No .git directory at all

            let (tx, rx) = mpsc::channel();

            let _watcher = RepoWatcher::new(dir.path(), tx).expect("Failed to create watcher");

            // Create a file
            let file_path = dir.path().join("test.txt");
            thread::sleep(Duration::from_millis(150));
            File::create(&file_path)
                .expect("Failed to create file")
                .write_all(b"test")
                .expect("Failed to write");

            // Should still receive working directory events
            let event = rx.recv_timeout(Duration::from_secs(2));
            assert!(event.is_ok(), "Should receive an event");
            assert!(matches!(
                event.unwrap(),
                WatchEvent::WorkingDirectory(_)
            ));
        }
    }

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
            assert!(matches!(
                event,
                WatchEvent::WorkingDirectory(_)
            ));

            if let WatchEvent::WorkingDirectory(p) = event {
                assert_eq!(p, PathBuf::from("/repo/src/main.rs"));
            }
        }

        #[test]
        fn detects_working_directory_nested() {
            let path = Path::new("/repo/src/git/mod.rs");
            let event = categorize_path(path);
            // "git" in path is not ".git", so it's working directory
            assert!(matches!(
                event,
                WatchEvent::WorkingDirectory(_)
            ));
        }
    }
}
