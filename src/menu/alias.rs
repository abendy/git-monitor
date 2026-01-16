//! Alias browser menus.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent};

use super::{Menu, MenuAction, MenuItem, MenuResult};
use crate::command::{CommandRequest, CommandSource};
use crate::config::{Alias, AliasSection};

/// Menu for browsing alias sections
pub struct AliasSectionMenu {
    sections: Vec<AliasSection>,
    repo_path: PathBuf,
    selected: usize,
}

impl AliasSectionMenu {
    /// Create a new alias section menu
    pub fn new(sections: Vec<AliasSection>, repo_path: PathBuf) -> Self {
        Self {
            sections,
            repo_path,
            selected: 0,
        }
    }
}

impl Menu for AliasSectionMenu {
    fn title(&self) -> &str {
        "Alias Sections"
    }

    fn items(&self) -> Vec<MenuItem> {
        self.sections
            .iter()
            .map(|section| {
                MenuItem::new(&section.name).with_description(format!(
                    "{} aliases",
                    section.aliases.len()
                ))
            })
            .collect()
    }

    fn selected(&self) -> usize {
        self.selected
    }

    fn set_selected(&mut self, idx: usize) {
        if idx < self.sections.len() {
            self.selected = idx;
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> MenuResult {
        match key.code {
            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_prev();
                MenuResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                MenuResult::Continue
            }

            // Drill into section
            KeyCode::Enter => {
                if let Some(section) = self.sections.get(self.selected) {
                    let items_menu = AliasItemsMenu::new(
                        section.name.clone(),
                        section.aliases.clone(),
                        self.repo_path.clone(),
                    );
                    MenuResult::Push(Box::new(items_menu))
                } else {
                    MenuResult::Continue
                }
            }

            // Close
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('a') => MenuResult::Close,

            _ => MenuResult::Continue,
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        super::render_menu(self, frame, area);
    }
}

/// Menu for browsing aliases within a section
pub struct AliasItemsMenu {
    section_name: String,
    aliases: Vec<Alias>,
    repo_path: PathBuf,
    selected: usize,
}

impl AliasItemsMenu {
    /// Create a new alias items menu
    pub fn new(section_name: String, aliases: Vec<Alias>, repo_path: PathBuf) -> Self {
        Self {
            section_name,
            aliases,
            repo_path,
            selected: 0,
        }
    }
}

impl Menu for AliasItemsMenu {
    fn title(&self) -> &str {
        &self.section_name
    }

    fn items(&self) -> Vec<MenuItem> {
        self.aliases
            .iter()
            .map(|alias| {
                MenuItem::new(&alias.name)
                    .with_key(&alias.name)
                    .with_description(&alias.command)
            })
            .collect()
    }

    fn selected(&self) -> usize {
        self.selected
    }

    fn set_selected(&mut self, idx: usize) {
        if idx < self.aliases.len() {
            self.selected = idx;
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> MenuResult {
        match key.code {
            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_prev();
                MenuResult::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                MenuResult::Continue
            }

            // Execute alias
            KeyCode::Enter => {
                if let Some(alias) = self.aliases.get(self.selected) {
                    let request = CommandRequest::git_alias(
                        &alias.name,
                        &alias.command,
                        &self.repo_path,
                    )
                    .with_source(CommandSource::AliasBrowser);

                    MenuResult::Execute(MenuAction::Command(request))
                } else {
                    MenuResult::Continue
                }
            }

            // Go back to sections
            KeyCode::Esc | KeyCode::Char('q') => MenuResult::Pop,

            // Close entire browser
            KeyCode::Char('a') => MenuResult::CloseAll,

            _ => MenuResult::Continue,
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, area: ratatui::layout::Rect) {
        super::render_menu(self, frame, area);
    }
}
