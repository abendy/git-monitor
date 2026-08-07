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
    pub const fn new(sections: Vec<AliasSection>, repo_path: PathBuf) -> Self {
        Self {
            sections,
            repo_path,
            selected: 0,
        }
    }
}

impl Menu for AliasSectionMenu {
    fn title(&self) -> &'static str {
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
            KeyCode::Esc | KeyCode::Char('q' | 'a') => MenuResult::Close,

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
    pub const fn new(section_name: String, aliases: Vec<Alias>, repo_path: PathBuf) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_aliases() -> Vec<Alias> {
        vec![
            Alias {
                name: "co".to_string(),
                command: "checkout".to_string(),
            },
            Alias {
                name: "br".to_string(),
                command: "branch".to_string(),
            },
            Alias {
                name: "st".to_string(),
                command: "status -sb".to_string(),
            },
        ]
    }

    fn test_sections() -> Vec<AliasSection> {
        vec![
            AliasSection {
                name: "checkout".to_string(),
                aliases: vec![Alias {
                    name: "co".to_string(),
                    command: "checkout".to_string(),
                }],
            },
            AliasSection {
                name: "branch".to_string(),
                aliases: vec![
                    Alias {
                        name: "br".to_string(),
                        command: "branch".to_string(),
                    },
                    Alias {
                        name: "bra".to_string(),
                        command: "branch -a".to_string(),
                    },
                ],
            },
        ]
    }

    mod alias_section_menu {
        use super::*;

        #[test]
        fn new_creates_menu() {
            let sections = test_sections();
            let menu = AliasSectionMenu::new(sections, PathBuf::from("/repo"));

            assert_eq!(menu.title(), "Alias Sections");
            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn items_returns_sections_with_counts() {
            let sections = test_sections();
            let menu = AliasSectionMenu::new(sections, PathBuf::from("/repo"));

            let items = menu.items();

            assert_eq!(items.len(), 2);
            assert_eq!(items[0].label, "checkout");
            assert_eq!(
                items[0].description,
                Some("1 aliases".to_string())
            );
            assert_eq!(items[1].label, "branch");
            assert_eq!(
                items[1].description,
                Some("2 aliases".to_string())
            );
        }

        #[test]
        fn set_selected_updates_selection() {
            let sections = test_sections();
            let mut menu = AliasSectionMenu::new(sections, PathBuf::from("/repo"));

            menu.set_selected(1);

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn set_selected_ignores_out_of_bounds() {
            let sections = test_sections();
            let mut menu = AliasSectionMenu::new(sections, PathBuf::from("/repo"));

            menu.set_selected(10);

            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn handle_key_esc_closes() {
            let sections = test_sections();
            let mut menu = AliasSectionMenu::new(sections, PathBuf::from("/repo"));

            let result = menu.handle_key(KeyEvent::from(KeyCode::Esc));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn handle_key_q_closes() {
            let sections = test_sections();
            let mut menu = AliasSectionMenu::new(sections, PathBuf::from("/repo"));

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('q')));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn handle_key_a_closes() {
            let sections = test_sections();
            let mut menu = AliasSectionMenu::new(sections, PathBuf::from("/repo"));

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('a')));

            assert!(matches!(result, MenuResult::Close));
        }

        #[test]
        fn handle_key_enter_pushes_items_menu() {
            let sections = test_sections();
            let mut menu = AliasSectionMenu::new(sections, PathBuf::from("/repo"));

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            assert!(matches!(result, MenuResult::Push(_)));
        }

        #[test]
        fn handle_key_down_navigates() {
            let sections = test_sections();
            let mut menu = AliasSectionMenu::new(sections, PathBuf::from("/repo"));

            menu.handle_key(KeyEvent::from(KeyCode::Down));

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn handle_key_j_navigates_down() {
            let sections = test_sections();
            let mut menu = AliasSectionMenu::new(sections, PathBuf::from("/repo"));

            menu.handle_key(KeyEvent::from(KeyCode::Char('j')));

            assert_eq!(menu.selected(), 1);
        }
    }

    mod alias_items_menu {
        use super::*;

        #[test]
        fn new_creates_menu() {
            let aliases = test_aliases();
            let menu = AliasItemsMenu::new(
                "checkout".to_string(),
                aliases,
                PathBuf::from("/repo"),
            );

            assert_eq!(menu.title(), "checkout");
            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn items_returns_aliases() {
            let aliases = test_aliases();
            let menu = AliasItemsMenu::new(
                "section".to_string(),
                aliases,
                PathBuf::from("/repo"),
            );

            let items = menu.items();

            assert_eq!(items.len(), 3);
            assert_eq!(items[0].label, "co");
            assert_eq!(
                items[0].key_hint,
                Some("co".to_string())
            );
            assert_eq!(
                items[0].description,
                Some("checkout".to_string())
            );
            assert_eq!(
                items[2].description,
                Some("status -sb".to_string())
            );
        }

        #[test]
        fn set_selected_updates_selection() {
            let aliases = test_aliases();
            let mut menu = AliasItemsMenu::new(
                "section".to_string(),
                aliases,
                PathBuf::from("/repo"),
            );

            menu.set_selected(2);

            assert_eq!(menu.selected(), 2);
        }

        #[test]
        fn set_selected_ignores_out_of_bounds() {
            let aliases = test_aliases();
            let mut menu = AliasItemsMenu::new(
                "section".to_string(),
                aliases,
                PathBuf::from("/repo"),
            );

            menu.set_selected(10);

            assert_eq!(menu.selected(), 0);
        }

        #[test]
        fn handle_key_esc_pops() {
            let aliases = test_aliases();
            let mut menu = AliasItemsMenu::new(
                "section".to_string(),
                aliases,
                PathBuf::from("/repo"),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Esc));

            assert!(matches!(result, MenuResult::Pop));
        }

        #[test]
        fn handle_key_q_pops() {
            let aliases = test_aliases();
            let mut menu = AliasItemsMenu::new(
                "section".to_string(),
                aliases,
                PathBuf::from("/repo"),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('q')));

            assert!(matches!(result, MenuResult::Pop));
        }

        #[test]
        fn handle_key_a_closes_all() {
            let aliases = test_aliases();
            let mut menu = AliasItemsMenu::new(
                "section".to_string(),
                aliases,
                PathBuf::from("/repo"),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Char('a')));

            assert!(matches!(result, MenuResult::CloseAll));
        }

        #[test]
        fn handle_key_enter_executes_alias() {
            let aliases = test_aliases();
            let mut menu = AliasItemsMenu::new(
                "section".to_string(),
                aliases,
                PathBuf::from("/repo"),
            );

            let result = menu.handle_key(KeyEvent::from(KeyCode::Enter));

            assert!(matches!(
                result,
                MenuResult::Execute(MenuAction::Command(_))
            ));
        }

        #[test]
        fn handle_key_down_navigates() {
            let aliases = test_aliases();
            let mut menu = AliasItemsMenu::new(
                "section".to_string(),
                aliases,
                PathBuf::from("/repo"),
            );

            menu.handle_key(KeyEvent::from(KeyCode::Down));

            assert_eq!(menu.selected(), 1);
        }

        #[test]
        fn handle_key_up_navigates() {
            let aliases = test_aliases();
            let mut menu = AliasItemsMenu::new(
                "section".to_string(),
                aliases,
                PathBuf::from("/repo"),
            );
            menu.set_selected(2);

            menu.handle_key(KeyEvent::from(KeyCode::Up));

            assert_eq!(menu.selected(), 1);
        }
    }
}
