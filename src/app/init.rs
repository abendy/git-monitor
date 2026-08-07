use std::path::PathBuf;

use anyhow::Result;
use tracing::warn;

use super::{App, HistoryMode, ViewMode};
use crate::actions::ActionRegistry;
use crate::command::{CommandExecutor, CommandHistory};
use crate::config::GitConfig;
use crate::git::{GitRepo, GitStatus};
use crate::input::Keymap;
use crate::menu::MenuStack;
use crate::section::{
    BranchesSection, CommandSection, FileListSection, HistorySection, SectionRegistry,
};

impl App {
    /// Create a new application instance
    #[allow(clippy::needless_pass_by_value)] // PathBuf API is cleaner than &Path
    pub fn new(path: PathBuf) -> Result<Self> {
        let repo = GitRepo::open(&path)?;
        let repo_path = repo
            .workdir()
            .map_or_else(|| path.clone(), PathBuf::from);

        let status = repo.status().unwrap_or_else(|e| {
            warn!("Failed to get git status: {e}");
            GitStatus::default()
        });
        let activity = Vec::new();
        let config = GitConfig::load(&repo_path).unwrap_or_else(|e| {
            warn!("Failed to load git config: {e}");
            GitConfig::default()
        });
        let branches = repo
            .list_branches()
            .unwrap_or_else(|e| {
                warn!("Failed to list branches: {e}");
                Vec::new()
            });

        // Initialize action registry with aliases
        let mut action_registry = ActionRegistry::new();
        let all_aliases: Vec<_> = config
            .sections
            .iter()
            .flat_map(|s| s.aliases.iter().cloned())
            .collect();
        action_registry.add_alias_actions(&all_aliases);

        // Initialize command executor
        let executor = CommandExecutor::new(&repo_path);
        let keymap = Keymap::from_registry(&action_registry);

        let mut app = Self {
            repo_path,
            repo,
            status,
            activity,
            config,
            watcher: None,
            running: true,
            selected: None,
            show_help: false,
            view_mode: ViewMode::default(),
            command_input: String::new(),
            command_draft: None,
            command_history: CommandHistory::new(),
            history_mode: HistoryMode::default(),
            feedback: crate::feedback::FeedbackManager::new(),
            expanded_commit: None,
            expanded_detail: None,
            expanded_file_idx: None,
            branches,
            history_collapsed: false,
            history_page: 0,
            history_total_items: 0,
            history_total_pages: 0,
            expanded_branch: None,
            expanded_branch_commits: Vec::new(),
            pending_external: None,
            action_registry,
            executor,
            menu_stack: MenuStack::new(),
            keymap,
            command_section: CommandSection::new(),
            staged_section: FileListSection::staged(),
            working_section: FileListSection::working(),
            history_section: HistorySection::new(),
            branches_section: BranchesSection::new(),
            section_registry: SectionRegistry::new(),
        };

        app.refresh_activity();
        // Update sections with initial state
        app.update_sections();
        app.select_default_section();

        Ok(app)
    }
}
