use std::path::Path;

use anyhow::{Context, Result};

use super::GitRepo;

impl GitRepo {
    /// Get diff for a file (working directory changes)
    pub fn diff_file(&self, path: &Path, staged: bool) -> Result<String> {
        use std::fmt::Write;

        let mut diff_opts = git2::DiffOptions::new();
        diff_opts.pathspec(path);

        let diff = if staged {
            // Staged: diff HEAD to index
            let head = self.repo.head().ok();
            let head_tree = head.and_then(|h| h.peel_to_tree().ok());
            self.repo.diff_tree_to_index(
                head_tree.as_ref(),
                None,
                Some(&mut diff_opts),
            )
        } else {
            // Unstaged: diff index to workdir
            self.repo
                .diff_index_to_workdir(None, Some(&mut diff_opts))
        }
        .context("Failed to get diff")?;

        let mut output = String::new();

        diff.print(
            git2::DiffFormat::Patch,
            |_delta, _hunk, line| {
                let prefix = match line.origin() {
                    '+' => "+",
                    '-' => "-",
                    ' ' => " ",
                    _ => "",
                };
                let content = std::str::from_utf8(line.content()).unwrap_or("");
                let _ = write!(output, "{prefix}{content}");
                true
            },
        )
        .context("Failed to print diff")?;

        if output.is_empty() {
            output = String::from("(no changes)");
        }

        Ok(output)
    }
}
