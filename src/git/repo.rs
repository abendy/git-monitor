use std::path::Path;

use anyhow::{Context, Result};
use git2::Repository;

use super::GitRepo;

impl GitRepo {
    /// Open a git repository at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let repo = Repository::discover(path).with_context(|| {
            format!(
                "No git repository found at {}",
                path.display()
            )
        })?;

        Ok(Self { repo })
    }

    /// Get the repository root path
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// Stage a file
    pub fn stage(&self, path: &Path) -> Result<()> {
        let mut index = self
            .repo
            .index()
            .context("Failed to get index")?;
        index
            .add_path(path)
            .with_context(|| format!("Failed to stage {}", path.display()))?;
        index
            .write()
            .context("Failed to write index")?;
        Ok(())
    }

    /// Unstage a file
    pub fn unstage(&self, path: &Path) -> Result<()> {
        let head = self
            .repo
            .head()
            .context("Failed to get HEAD")?;
        let head_commit = head
            .peel_to_commit()
            .context("Failed to get HEAD commit")?;
        let head_tree = head_commit
            .tree()
            .context("Failed to get HEAD tree")?;

        self.repo
            .reset_default(Some(head_commit.as_object()), [path])
            .or_else(
                |_| -> std::result::Result<(), git2::Error> {
                    // If reset fails (file is new), remove from index
                    let mut index = self.repo.index()?;
                    index.remove_path(path)?;
                    index.write()?;
                    Ok(())
                },
            )
            .with_context(|| format!("Failed to unstage {}", path.display()))?;

        drop(head_tree);
        Ok(())
    }
}
