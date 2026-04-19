use anyhow::{Result, anyhow, bail};

use crate::{
    domain::{
        local_task::{LocalTaskStatus, LocalTaskSummary},
        workspace::WorkspaceCheckout,
    },
    ports::{local_task_source::LocalTaskSource, workspace_store::WorkspaceStore},
};

#[derive(Clone)]
pub struct UpdateLocalTaskStatusUseCase<S, T>
where
    S: WorkspaceStore,
    T: LocalTaskSource,
{
    store: S,
    task_source: T,
}

impl<S, T> UpdateLocalTaskStatusUseCase<S, T>
where
    S: WorkspaceStore,
    T: LocalTaskSource,
{
    pub fn new(store: S, task_source: T) -> Self {
        Self { store, task_source }
    }

    pub async fn execute(
        &self,
        workspace_id: &str,
        checkout_id: Option<&str>,
        task_id: &str,
        status: LocalTaskStatus,
    ) -> Result<LocalTaskSummary> {
        let task_id = task_id.trim();
        if task_id.is_empty() {
            bail!("task id is required");
        }

        let checkout = self.resolve_checkout(workspace_id, checkout_id).await?;
        if !self.task_source.has_task_data(&checkout.path) {
            bail!("beads task data not found in workspace");
        }

        self.task_source
            .update_status(&checkout.path, task_id, status)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            local_task::LocalTaskSummary,
            workspace::{CheckoutId, GitOrigin, Workspace, WorkspaceCheckout, WorkspaceId},
        },
        ports::{local_task_source::LocalTaskSource, workspace_store::WorkspaceStore},
    };
    use anyhow::Result;
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    #[derive(Clone)]
    struct FakeWorkspaceStore;

    impl WorkspaceStore for FakeWorkspaceStore {
        async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
            Ok(vec![])
        }

        async fn get_workspace(&self, id: &str) -> Result<Option<Workspace>> {
            Ok((id == "workspace-1").then(|| Workspace {
                id: "workspace-1".into(),
                name: "repo".into(),
                origin: GitOrigin {
                    raw_url: "git@github.com:owner/repo.git".into(),
                    canonical_url: "github.com/owner/repo".into(),
                    host: "github.com".into(),
                    owner: "owner".into(),
                    repo: "repo".into(),
                },
                default_checkout_id: Some("checkout-1".into()),
                created_at: "1".into(),
                updated_at: "1".into(),
            }))
        }

        async fn list_checkouts(&self, _workspace_id: &str) -> Result<Vec<WorkspaceCheckout>> {
            Ok(vec![])
        }

        async fn get_checkout(&self, id: &str) -> Result<Option<WorkspaceCheckout>> {
            Ok((id == "checkout-1").then(|| {
                let mut checkout = WorkspaceCheckout::new(
                    "workspace-1".into(),
                    "github.com/owner/repo",
                    PathBuf::from("/repo"),
                    Some("main".into()),
                    Some("abc123".into()),
                    true,
                );
                checkout.id = "checkout-1".into();
                checkout
            }))
        }

        async fn remove_workspace(&self, _workspace_id: &WorkspaceId) -> Result<()> {
            Ok(())
        }

        async fn remove_checkout(&self, _checkout_id: &CheckoutId) -> Result<()> {
            Ok(())
        }

        async fn save_checkout(&self, checkout: WorkspaceCheckout) -> Result<WorkspaceCheckout> {
            Ok(checkout)
        }

        async fn refresh_checkout(
            &self,
            _checkout_id: &CheckoutId,
        ) -> Result<Option<WorkspaceCheckout>> {
            Ok(None)
        }
    }

    #[derive(Clone)]
    struct FakeTaskSource {
        detected: bool,
        updated: Arc<Mutex<Vec<(String, LocalTaskStatus)>>>,
    }

    impl LocalTaskSource for FakeTaskSource {
        fn has_task_data(&self, _workdir: &Path) -> bool {
            self.detected
        }

        fn list_tasks(&self, _workdir: &Path) -> Result<Vec<LocalTaskSummary>> {
            Ok(vec![])
        }

        fn update_status(
            &self,
            _workdir: &Path,
            task_id: &str,
            status: LocalTaskStatus,
        ) -> Result<LocalTaskSummary> {
            self.updated
                .lock()
                .unwrap()
                .push((task_id.to_string(), status.clone()));
            Ok(LocalTaskSummary {
                id: task_id.into(),
                title: "Wire task status".into(),
                description: None,
                status: Some(status.as_beads_status().into()),
                priority: None,
                labels: vec![],
                dependencies: vec![],
                blocked: false,
                acceptance_criteria: None,
            })
        }
    }

    #[tokio::test]
    async fn updates_status_for_detected_workspace_data() {
        let updates = Arc::new(Mutex::new(vec![]));
        let result = UpdateLocalTaskStatusUseCase::new(
            FakeWorkspaceStore,
            FakeTaskSource {
                detected: true,
                updated: updates.clone(),
            },
        )
        .execute("workspace-1", None, "bd-1", LocalTaskStatus::InProgress)
        .await
        .unwrap();

        assert_eq!(result.status.as_deref(), Some("in_progress"));
        assert_eq!(
            updates.lock().unwrap().as_slice(),
            &[("bd-1".into(), LocalTaskStatus::InProgress)]
        );
    }

    #[tokio::test]
    async fn rejects_missing_task_data() {
        let result = UpdateLocalTaskStatusUseCase::new(
            FakeWorkspaceStore,
            FakeTaskSource {
                detected: false,
                updated: Arc::new(Mutex::new(vec![])),
            },
        )
        .execute("workspace-1", None, "bd-1", LocalTaskStatus::Closed)
        .await;

        assert_eq!(
            result.unwrap_err().to_string(),
            "beads task data not found in workspace"
        );
    }
}
