use anyhow::{Result, anyhow, bail};

use crate::{
    domain::workspace::{WorkspaceCheckout, WorkspaceId},
    domain::workspace_git::{WorkspaceDiffSummary, WorkspaceGitStatus},
    ports::{workspace_git::WorkspaceGitInspector, workspace_store::WorkspaceStore},
};

#[derive(Clone)]
pub struct WorkspaceGitUseCase<S, G>
where
    S: WorkspaceStore,
    G: WorkspaceGitInspector,
{
    store: S,
    git: G,
}

impl<S, G> WorkspaceGitUseCase<S, G>
where
    S: WorkspaceStore,
    G: WorkspaceGitInspector,
{
    pub fn new(store: S, git: G) -> Self {
        Self { store, git }
    }

    pub async fn status(
        &self,
        workspace_id: &WorkspaceId,
        checkout_id: Option<&str>,
    ) -> Result<WorkspaceGitStatus> {
        let checkout = self.resolve_checkout(workspace_id, checkout_id).await?;
        self.git.status(workspace_id.clone(), checkout).await
    }

    pub async fn diff_summary(
        &self,
        workspace_id: &WorkspaceId,
        checkout_id: Option<&str>,
    ) -> Result<WorkspaceDiffSummary> {
        let checkout = self.resolve_checkout(workspace_id, checkout_id).await?;
        self.git.diff_summary(workspace_id.clone(), checkout).await
    }

    async fn resolve_checkout(
        &self,
        workspace_id: &WorkspaceId,
        checkout_id: Option<&str>,
    ) -> Result<WorkspaceCheckout> {
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
        if checkout.workspace_id != *workspace_id {
            bail!("checkout {checkout_id} does not belong to workspace {workspace_id}");
        }
        Ok(checkout)
    }
}
