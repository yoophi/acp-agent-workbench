use anyhow::{Result, anyhow, bail};

use crate::{
    domain::{local_task::LocalTaskList, workspace::WorkspaceCheckout},
    ports::{local_task_source::LocalTaskSource, workspace_store::WorkspaceStore},
};

#[derive(Clone)]
pub struct ListLocalTasksUseCase<S, T>
where
    S: WorkspaceStore,
    T: LocalTaskSource,
{
    store: S,
    task_source: T,
}

impl<S, T> ListLocalTasksUseCase<S, T>
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
    ) -> Result<LocalTaskList> {
        let checkout = self.resolve_checkout(workspace_id, checkout_id).await?;
        let workdir = checkout.path.to_string_lossy().to_string();
        let detected = self.task_source.has_task_data(&checkout.path);
        if !detected {
            return Ok(LocalTaskList::unavailable(
                workspace_id.trim().to_string(),
                checkout.id,
                workdir,
                false,
                "beads task data not found in workspace",
            ));
        }

        match self.task_source.list_tasks(&checkout.path) {
            Ok(tasks) => Ok(LocalTaskList::available(
                workspace_id.trim().to_string(),
                checkout.id,
                workdir,
                tasks,
            )),
            Err(err) => Ok(LocalTaskList::unavailable(
                workspace_id.trim().to_string(),
                checkout.id,
                workdir,
                true,
                err.to_string(),
            )),
        }
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
            local_task::{LocalTaskStatus, LocalTaskSummary},
            workspace::{CheckoutId, GitOrigin, Workspace, WorkspaceCheckout, WorkspaceId},
        },
        ports::{local_task_source::LocalTaskSource, workspace_store::WorkspaceStore},
    };
    use anyhow::Result;
    use std::path::{Path, PathBuf};

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
    }

    impl LocalTaskSource for FakeTaskSource {
        fn has_task_data(&self, _workdir: &Path) -> bool {
            self.detected
        }

        fn list_tasks(&self, _workdir: &Path) -> Result<Vec<LocalTaskSummary>> {
            Ok(vec![LocalTaskSummary {
                id: "bd-1".into(),
                title: "Wire task list".into(),
                description: Some("Show tasks".into()),
                status: Some("open".into()),
                priority: Some("1".into()),
                labels: vec!["backend".into()],
                dependencies: vec![],
                blocked: false,
                acceptance_criteria: Some("Tasks render".into()),
            }])
        }

        fn update_status(
            &self,
            _workdir: &Path,
            _task_id: &str,
            _status: LocalTaskStatus,
        ) -> Result<LocalTaskSummary> {
            unimplemented!("list tests do not update task status")
        }
    }

    #[tokio::test]
    async fn returns_tasks_for_detected_workspace_data() {
        let result =
            ListLocalTasksUseCase::new(FakeWorkspaceStore, FakeTaskSource { detected: true })
                .execute("workspace-1", None)
                .await
                .unwrap();

        assert!(result.detected);
        assert!(result.available);
        assert_eq!(result.checkout_id, "checkout-1");
        assert_eq!(result.tasks[0].id, "bd-1");
    }

    #[tokio::test]
    async fn reports_recoverable_missing_task_data() {
        let result =
            ListLocalTasksUseCase::new(FakeWorkspaceStore, FakeTaskSource { detected: false })
                .execute("workspace-1", None)
                .await
                .unwrap();

        assert!(!result.detected);
        assert!(!result.available);
        assert_eq!(result.tasks, Vec::<LocalTaskSummary>::new());
        assert_eq!(
            result.error.as_deref(),
            Some("beads task data not found in workspace")
        );
    }
}
