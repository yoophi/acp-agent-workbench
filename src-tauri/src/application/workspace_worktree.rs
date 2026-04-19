use anyhow::{Result, anyhow, bail};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::{
    domain::workspace::WorkspaceCheckout,
    ports::{git_repository::GitRepositoryPort, workspace_store::WorkspaceStore},
};

#[derive(Clone)]
pub struct WorkspaceTaskWorktreeUseCase<S, G>
where
    S: WorkspaceStore,
    G: GitRepositoryPort,
{
    store: S,
    git: G,
}

impl<S, G> WorkspaceTaskWorktreeUseCase<S, G>
where
    S: WorkspaceStore,
    G: GitRepositoryPort,
{
    pub fn new(store: S, git: G) -> Self {
        Self { store, git }
    }

    pub async fn provision(
        &self,
        workspace_id: &str,
        checkout_id: Option<&str>,
        task_slug: Option<&str>,
    ) -> Result<WorkspaceCheckout> {
        let workspace_id = workspace_id.trim();
        if workspace_id.is_empty() {
            bail!("workspace id is required");
        }

        let workspace = self
            .store
            .get_workspace(workspace_id)
            .await?
            .ok_or_else(|| anyhow!("workspace not found: {workspace_id}"))?;

        let checkout_id = checkout_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or(workspace.default_checkout_id.as_deref())
            .ok_or_else(|| anyhow!("workspace has no checkout: {workspace_id}"))?;

        let checkout = self
            .store
            .get_checkout(checkout_id)
            .await?
            .ok_or_else(|| anyhow!("checkout not found: {checkout_id}"))?;

        if checkout.workspace_id != workspace.id {
            bail!("checkout {checkout_id} does not belong to workspace {workspace_id}");
        }

        let slug = task_worktree_slug(task_slug);
        let branch = format!("worktree/{slug}");
        let path = derive_worktree_path(&checkout.path, &slug)?;
        let status = self.git.create_worktree(&checkout.path, &branch, &path)?;
        let worktree = WorkspaceCheckout::new_worktree(
            workspace.id,
            &workspace.origin.canonical_url,
            PathBuf::from(status.root),
            status.branch,
            status.head_sha,
        );

        self.store.save_checkout(worktree).await
    }
}

fn derive_worktree_path(checkout_path: &Path, slug: &str) -> Result<PathBuf> {
    let parent = checkout_path
        .parent()
        .ok_or_else(|| anyhow!("checkout path has no parent: {}", checkout_path.display()))?;
    let checkout_name = checkout_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            anyhow!(
                "checkout path has no directory name: {}",
                checkout_path.display()
            )
        })?;
    Ok(parent.join(format!("{checkout_name}-{slug}")))
}

fn task_worktree_slug(value: Option<&str>) -> String {
    let slug = sanitize_worktree_slug(value.unwrap_or_default());
    if slug.is_empty() {
        let id = Uuid::new_v4().simple().to_string();
        format!("task-{}", &id[..8])
    } else {
        slug
    }
}

fn sanitize_worktree_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::{derive_worktree_path, sanitize_worktree_slug};
    use std::path::Path;

    #[test]
    fn sanitizes_task_identifier_for_branch_and_path() {
        assert_eq!(
            sanitize_worktree_slug("Issue #63: Worktree Isolation"),
            "issue-63-worktree-isolation"
        );
        assert_eq!(sanitize_worktree_slug("  ///  "), "");
    }

    #[test]
    fn derives_sibling_worktree_path_from_checkout_name() {
        let path =
            derive_worktree_path(Path::new("/repo/acp-agent-workbench"), "issue-63").unwrap();
        assert_eq!(path, Path::new("/repo/acp-agent-workbench-issue-63"));
    }
}
