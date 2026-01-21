use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use anyhow::Result;

/// A single git alias
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias {
    /// Alias name (e.g., "co")
    pub name: String,
    /// Alias command (e.g., "checkout")
    pub command: String,
}

/// A section/category of aliases
#[derive(Debug, Clone)]
pub struct AliasSection {
    /// Section name (e.g., "fetch", "commit")
    pub name: String,
    /// Aliases in this section
    pub aliases: Vec<Alias>,
}

/// Git configuration with aliases grouped by section
#[derive(Debug, Clone, Default)]
pub struct GitConfig {
    /// Alias sections in order
    pub sections: Vec<AliasSection>,
}

impl GitConfig {
    /// Load git config from repository
    pub fn load(repo_path: &Path) -> Result<Self> {
        // Get all aliases from git config
        let output = Command::new("git")
            .args(["config", "--get-regexp", "^alias\\."])
            .current_dir(repo_path)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse aliases into a map
        let mut alias_map: BTreeMap<String, String> = BTreeMap::new();
        for line in stdout.lines() {
            if let Some(rest) = line.strip_prefix("alias.") {
                if let Some((name, cmd)) = rest.split_once(' ') {
                    alias_map.insert(name.to_string(), cmd.to_string());
                }
            }
        }

        // Try to read gitconfig to find section groupings
        let sections = Self::parse_sections_from_gitconfig(&alias_map);

        Ok(Self { sections })
    }

    /// Parse sections from gitconfig file
    fn parse_sections_from_gitconfig(alias_map: &BTreeMap<String, String>) -> Vec<AliasSection> {
        // Try global gitconfig first, then local
        let home = std::env::var("HOME").unwrap_or_default();
        let gitconfig_path = format!("{home}/.gitconfig");

        let Ok(content) = std::fs::read_to_string(&gitconfig_path) else {
            // Fall back to a single "All" section
            return vec![AliasSection {
                name: "all".to_string(),
                aliases: alias_map
                    .iter()
                    .map(|(name, command)| Alias {
                        name: name.clone(),
                        command: command.clone(),
                    })
                    .collect(),
            }];
        };

        let mut sections: Vec<AliasSection> = Vec::new();
        let mut in_alias_block = false;
        let mut seen_aliases: std::collections::HashSet<String> = std::collections::HashSet::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Check for [alias] section start
            if trimmed.eq_ignore_ascii_case("[alias]") {
                in_alias_block = true;
                continue;
            }

            // Check for another section starting (end of alias block)
            if trimmed.starts_with('[') && !trimmed.eq_ignore_ascii_case("[alias]") {
                in_alias_block = false;
                continue;
            }

            if !in_alias_block {
                continue;
            }

            // Check for section header comment: # --- name ---
            if let Some(section_name) = Self::parse_section_header(trimmed) {
                sections.push(AliasSection {
                    name: section_name,
                    aliases: Vec::new(),
                });
                continue;
            }

            // Check for alias definition
            if let Some((name, _)) = Self::parse_alias_line(trimmed) {
                if let Some(command) = alias_map.get(&name) {
                    if !seen_aliases.contains(&name) {
                        seen_aliases.insert(name.clone());
                        let alias = Alias {
                            name,
                            command: command.clone(),
                        };

                        if let Some(section) = sections.last_mut() {
                            section.aliases.push(alias);
                        } else {
                            // No section yet, create "other"
                            sections.push(AliasSection {
                                name: "other".to_string(),
                                aliases: vec![alias],
                            });
                        }
                    }
                }
            }
        }

        // Add any aliases not in sections to "other"
        let remaining: Vec<Alias> = alias_map
            .iter()
            .filter(|(name, _)| !seen_aliases.contains(*name))
            .map(|(name, command)| Alias {
                name: name.clone(),
                command: command.clone(),
            })
            .collect();

        if !remaining.is_empty() {
            sections.push(AliasSection {
                name: "other".to_string(),
                aliases: remaining,
            });
        }

        // Remove empty sections
        sections.retain(|s| !s.aliases.is_empty());

        sections
    }

    /// Parse section header like "# --- fetch ---" or "# fetch"
    fn parse_section_header(line: &str) -> Option<String> {
        if !line.starts_with('#') {
            return None;
        }

        let content = line.trim_start_matches('#').trim();

        // Match pattern: --- name ---
        if content.starts_with("---") && content.ends_with("---") {
            let inner = content
                .trim_start_matches('-')
                .trim_end_matches('-')
                .trim();
            if !inner.is_empty() && !inner.contains(' ') {
                return Some(inner.to_lowercase());
            }
        }

        None
    }

    /// Parse an alias line like "co = checkout"
    fn parse_alias_line(line: &str) -> Option<(String, String)> {
        if line.starts_with('#') || line.is_empty() {
            return None;
        }

        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() == 2 {
            let name = parts[0].trim().to_string();
            let command = parts[1]
                .trim()
                .trim_matches('"')
                .to_string();
            if !name.is_empty() {
                return Some((name, command));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_section_header {
        use super::*;

        #[test]
        fn parses_dashed_header() {
            let result = GitConfig::parse_section_header("# --- fetch ---");

            assert_eq!(result, Some("fetch".to_string()));
        }

        #[test]
        fn parses_dashed_header_with_extra_spaces() {
            let result = GitConfig::parse_section_header("#   ---   commit   ---");

            assert_eq!(result, Some("commit".to_string()));
        }

        #[test]
        fn returns_none_for_non_comment() {
            let result = GitConfig::parse_section_header("co = checkout");

            assert!(result.is_none());
        }

        #[test]
        fn returns_none_for_plain_comment() {
            let result = GitConfig::parse_section_header("# This is a comment");

            assert!(result.is_none());
        }

        #[test]
        fn returns_none_for_multi_word_header() {
            let result = GitConfig::parse_section_header("# --- fetch remote ---");

            assert!(result.is_none());
        }

        #[test]
        fn lowercases_header_name() {
            let result = GitConfig::parse_section_header("# --- FETCH ---");

            assert_eq!(result, Some("fetch".to_string()));
        }
    }

    mod parse_alias_line {
        use super::*;

        #[test]
        fn parses_simple_alias() {
            let result = GitConfig::parse_alias_line("co = checkout");

            assert_eq!(
                result,
                Some(("co".to_string(), "checkout".to_string()))
            );
        }

        #[test]
        fn parses_alias_with_arguments() {
            let result = GitConfig::parse_alias_line("lg = log --oneline --graph");

            assert_eq!(
                result,
                Some((
                    "lg".to_string(),
                    "log --oneline --graph".to_string()
                ))
            );
        }

        #[test]
        fn parses_quoted_alias() {
            let result = GitConfig::parse_alias_line("st = \"status -sb\"");

            assert_eq!(
                result,
                Some((
                    "st".to_string(),
                    "status -sb".to_string()
                ))
            );
        }

        #[test]
        fn handles_extra_whitespace() {
            let result = GitConfig::parse_alias_line("  br   =   branch  ");

            assert_eq!(
                result,
                Some(("br".to_string(), "branch".to_string()))
            );
        }

        #[test]
        fn returns_none_for_comment() {
            let result = GitConfig::parse_alias_line("# co = checkout");

            assert!(result.is_none());
        }

        #[test]
        fn returns_none_for_empty_line() {
            let result = GitConfig::parse_alias_line("");

            assert!(result.is_none());
        }

        #[test]
        fn returns_none_for_line_without_equals() {
            let result = GitConfig::parse_alias_line("checkout");

            assert!(result.is_none());
        }

        #[test]
        fn handles_equals_in_command() {
            let result = GitConfig::parse_alias_line("cfg = config --global user.name=test");

            assert_eq!(
                result,
                Some((
                    "cfg".to_string(),
                    "config --global user.name=test".to_string()
                ))
            );
        }
    }

    mod alias {
        use super::*;

        #[test]
        fn alias_equality() {
            let a1 = Alias {
                name: "co".to_string(),
                command: "checkout".to_string(),
            };
            let a2 = Alias {
                name: "co".to_string(),
                command: "checkout".to_string(),
            };
            let a3 = Alias {
                name: "br".to_string(),
                command: "branch".to_string(),
            };

            assert_eq!(a1, a2);
            assert_ne!(a1, a3);
        }
    }

    mod git_config {
        use super::*;

        #[test]
        fn default_creates_empty_config() {
            let config = GitConfig::default();

            assert!(config.sections.is_empty());
        }
    }

    mod parse_sections_from_gitconfig {
        use super::*;

        #[test]
        fn fallback_to_all_section_when_no_gitconfig() {
            // When HOME points to a directory without .gitconfig,
            // all aliases should be in an "all" section
            let mut alias_map = BTreeMap::new();
            alias_map.insert("co".to_string(), "checkout".to_string());
            alias_map.insert("br".to_string(), "branch".to_string());

            // Save and override HOME to a nonexistent path
            let original_home = std::env::var("HOME").ok();
            std::env::set_var(
                "HOME",
                "/nonexistent/path/that/does/not/exist",
            );

            let sections = GitConfig::parse_sections_from_gitconfig(&alias_map);

            // Restore HOME
            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            }

            assert_eq!(sections.len(), 1);
            assert_eq!(sections[0].name, "all");
            assert_eq!(sections[0].aliases.len(), 2);
        }

        #[test]
        fn empty_alias_map_returns_empty_sections() {
            let alias_map = BTreeMap::new();

            // Override HOME to ensure we use fallback
            let original_home = std::env::var("HOME").ok();
            std::env::set_var("HOME", "/nonexistent/path");

            let sections = GitConfig::parse_sections_from_gitconfig(&alias_map);

            if let Some(home) = original_home {
                std::env::set_var("HOME", home);
            }

            // With fallback, we get an "all" section but with 0 aliases
            // The retain call removes empty sections
            assert!(sections.is_empty() || sections[0].aliases.is_empty());
        }
    }

    mod load {
        use std::path::PathBuf;
        use std::process::Command;

        use tempfile::TempDir;

        use super::*;

        struct IsolatedRepo {
            dir: TempDir,
            empty_config: PathBuf,
        }

        impl IsolatedRepo {
            fn path(&self) -> &Path {
                self.dir.path()
            }

            /// Load config with isolation from global/system git config
            fn load_config(&self) -> Result<GitConfig> {
                // Temporarily set environment to isolate from global config
                std::env::set_var("GIT_CONFIG_GLOBAL", &self.empty_config);
                std::env::set_var("GIT_CONFIG_SYSTEM", &self.empty_config);

                let result = GitConfig::load(self.dir.path());

                // Clean up environment (restore to previous state)
                std::env::remove_var("GIT_CONFIG_GLOBAL");
                std::env::remove_var("GIT_CONFIG_SYSTEM");

                result
            }
        }

        fn setup_git_repo() -> IsolatedRepo {
            let dir = TempDir::new().expect("create temp dir");

            // Create empty config file to isolate from user's global config
            let empty_config = dir.path().join(".empty_gitconfig");
            std::fs::write(&empty_config, "").expect("create empty config");

            // Initialize git repo (isolated from global config)
            Command::new("git")
                .args(["init"])
                .current_dir(dir.path())
                .env("GIT_CONFIG_GLOBAL", &empty_config)
                .env("GIT_CONFIG_SYSTEM", &empty_config)
                .output()
                .expect("git init");

            // Configure user for the repo (isolated from global config)
            Command::new("git")
                .args(["config", "user.email", "test@test.com"])
                .current_dir(dir.path())
                .env("GIT_CONFIG_GLOBAL", &empty_config)
                .env("GIT_CONFIG_SYSTEM", &empty_config)
                .output()
                .expect("git config email");

            Command::new("git")
                .args(["config", "user.name", "Test User"])
                .current_dir(dir.path())
                .env("GIT_CONFIG_GLOBAL", &empty_config)
                .env("GIT_CONFIG_SYSTEM", &empty_config)
                .output()
                .expect("git config name");

            IsolatedRepo { dir, empty_config }
        }

        #[test]
        fn loads_aliases_from_repo() {
            let repo = setup_git_repo();

            // Add some aliases to the repo's local config
            Command::new("git")
                .args(["config", "alias.co", "checkout"])
                .current_dir(repo.path())
                .env("GIT_CONFIG_GLOBAL", &repo.empty_config)
                .env("GIT_CONFIG_SYSTEM", &repo.empty_config)
                .output()
                .expect("add alias");

            Command::new("git")
                .args(["config", "alias.br", "branch"])
                .current_dir(repo.path())
                .env("GIT_CONFIG_GLOBAL", &repo.empty_config)
                .env("GIT_CONFIG_SYSTEM", &repo.empty_config)
                .output()
                .expect("add alias");

            let config = repo.load_config().expect("load config");

            // Should have sections with our aliases
            let all_aliases: Vec<_> = config
                .sections
                .iter()
                .flat_map(|s| &s.aliases)
                .collect();

            assert!(all_aliases
                .iter()
                .any(|a| a.name == "co" && a.command == "checkout"));
            assert!(all_aliases
                .iter()
                .any(|a| a.name == "br" && a.command == "branch"));
        }

        #[test]
        fn loads_empty_config_when_no_aliases() {
            let repo = setup_git_repo();

            let config = repo.load_config().expect("load config");

            // No aliases means empty sections (after retain removes empty ones)
            let total_aliases: usize = config
                .sections
                .iter()
                .map(|s| s.aliases.len())
                .sum();
            assert_eq!(total_aliases, 0);
        }

        #[test]
        fn loads_alias_with_arguments() {
            let repo = setup_git_repo();

            Command::new("git")
                .args(["config", "alias.lg", "log --oneline --graph"])
                .current_dir(repo.path())
                .env("GIT_CONFIG_GLOBAL", &repo.empty_config)
                .env("GIT_CONFIG_SYSTEM", &repo.empty_config)
                .output()
                .expect("add alias");

            let config = repo.load_config().expect("load config");

            let all_aliases: Vec<_> = config
                .sections
                .iter()
                .flat_map(|s| &s.aliases)
                .collect();

            let lg_alias = all_aliases
                .iter()
                .find(|a| a.name == "lg");
            assert!(lg_alias.is_some());
            assert_eq!(
                lg_alias.unwrap().command,
                "log --oneline --graph"
            );
        }

        #[test]
        fn handles_complex_alias_commands() {
            let repo = setup_git_repo();

            // Add an alias with shell command
            Command::new("git")
                .args(["config", "alias.last", "log -1 HEAD"])
                .current_dir(repo.path())
                .env("GIT_CONFIG_GLOBAL", &repo.empty_config)
                .env("GIT_CONFIG_SYSTEM", &repo.empty_config)
                .output()
                .expect("add alias");

            let config = repo.load_config().expect("load config");

            let all_aliases: Vec<_> = config
                .sections
                .iter()
                .flat_map(|s| &s.aliases)
                .collect();

            assert!(all_aliases
                .iter()
                .any(|a| a.name == "last"));
        }
    }

    mod alias_section {
        use super::*;

        #[test]
        fn section_stores_name_and_aliases() {
            let section = AliasSection {
                name: "fetch".to_string(),
                aliases: vec![
                    Alias {
                        name: "f".to_string(),
                        command: "fetch".to_string(),
                    },
                    Alias {
                        name: "fa".to_string(),
                        command: "fetch --all".to_string(),
                    },
                ],
            };

            assert_eq!(section.name, "fetch");
            assert_eq!(section.aliases.len(), 2);
            assert_eq!(section.aliases[0].name, "f");
            assert_eq!(section.aliases[1].name, "fa");
        }

        #[test]
        fn section_can_be_empty() {
            let section = AliasSection {
                name: "empty".to_string(),
                aliases: vec![],
            };

            assert!(section.aliases.is_empty());
        }
    }
}
