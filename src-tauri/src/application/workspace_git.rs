use anyhow::{Result, anyhow, bail};

use crate::{
    domain::{
        git::{WorkspaceDiffSummary, WorkspaceGitStatus},
        workspace::WorkspaceCheckout,
    },
    ports::{git_repository::GitRepositoryPort, workspace_store::WorkspaceStore},
};

#[derive(Clone)]
pub struct WorkspaceGitUseCase<S, G>
where
    S: WorkspaceStore,
    G: GitRepositoryPort,
{
    store: S,
    git: G,
}

impl<S, G> WorkspaceGitUseCase<S, G>
where
    S: WorkspaceStore,
    G: GitRepositoryPort,
{
    pub fn new(store: S, git: G) -> Self {
        Self { store, git }
    }

    pub async fn status(
        &self,
        workspace_id: &str,
        checkout_id: Option<&str>,
    ) -> Result<WorkspaceGitStatus> {
        let checkout = self.resolve_checkout(workspace_id, checkout_id).await?;
        self.git.status(&checkout.path)
    }

    pub async fn diff_summary(
        &self,
        workspace_id: &str,
        checkout_id: Option<&str>,
    ) -> Result<WorkspaceDiffSummary> {
        let checkout = self.resolve_checkout(workspace_id, checkout_id).await?;
        self.git.diff_summary(&checkout.path)
    }

    async fn resolve_checkout(
        &self,
        workspace_id: &str,
        checkout_id: Option<&str>,
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

        Ok(checkout)
    }
}
