use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use anyhow::Result;

/// A single git alias
#[derive(Debug, Clone)]
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

        let content = match std::fs::read_to_string(&gitconfig_path) {
            Ok(c) => c,
            Err(_) => {
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
            }
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
            let command = parts[1].trim().trim_matches('"').to_string();
            if !name.is_empty() {
                return Some((name, command));
            }
        }

        None
    }
}
