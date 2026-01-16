use anyhow::{Context, Result};

use super::{BranchInfo, GitRepo};

impl GitRepo {
    /// List local branches with their info
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>> {
        let mut branches = Vec::new();

        // Get current branch name for comparison
        let current_branch = self
            .repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from));

        // Iterate through local branches
        let branch_iter = self
            .repo
            .branches(Some(git2::BranchType::Local))?;

        for branch_result in branch_iter {
            let (branch, _branch_type) = branch_result?;

            let name = match branch.name()? {
                Some(n) => n.to_string(),
                None => continue,
            };

            if name.is_empty() {
                continue;
            }

            let is_current = current_branch.as_ref() == Some(&name);

            branches.push(BranchInfo {
                name,
                is_current,
                is_remote: false,
            });
        }

        // Sort: current branch first, then local branches
        branches.sort_by(|a, b| {
            match (a.is_current, b.is_current) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }
            a.name.cmp(&b.name)
        });

        Ok(branches)
    }

    /// Checkout a branch by name
    pub fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        let branch = self
            .repo
            .find_branch(branch_name, git2::BranchType::Local)
            .with_context(|| format!("Branch '{branch_name}' not found"))?;

        let reference = branch.get();
        let oid = reference
            .target()
            .ok_or_else(|| anyhow::anyhow!("Branch has no target"))?;

        let commit = self.repo.find_commit(oid)?;

        // Check for uncommitted changes
        let statuses = self.repo.statuses(None)?;
        let has_changes = statuses.iter().any(|s| {
            let status = s.status();
            status.is_wt_modified()
                || status.is_wt_deleted()
                || status.is_index_modified()
                || status.is_index_deleted()
                || status.is_index_new()
        });

        if has_changes {
            return Err(anyhow::anyhow!(
                "Cannot checkout: you have uncommitted changes"
            ));
        }

        // Checkout the tree
        self.repo.checkout_tree(
            commit.as_object(),
            Some(git2::build::CheckoutBuilder::new().safe()),
        )?;

        // Update HEAD
        self.repo
            .set_head(&format!("refs/heads/{branch_name}"))?;

        Ok(())
    }
}
