use anyhow::{Result, anyhow, bail};

use crate::{
    domain::{
        git::{
            GitHubPullRequestCreateRequest, GitHubPullRequestSummary, WorkspaceCommitRequest,
            WorkspaceCommitResult, WorkspaceDiffSummary, WorkspaceGitStatus, WorkspacePushRequest,
            WorkspacePushResult,
        },
        workspace::WorkspaceCheckout,
    },
    ports::{
        git_repository::GitRepositoryPort, github_pull_request::GitHubPullRequestPort,
        workspace_store::WorkspaceStore,
    },
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

    pub async fn commit(&self, request: WorkspaceCommitRequest) -> Result<WorkspaceCommitResult> {
        require_confirmation(request.confirmed, "commit workspace changes")?;
        let checkout = self
            .resolve_checkout(&request.workspace_id, request.checkout_id.as_deref())
            .await?;
        self.git
            .commit(&checkout.path, &request.message, request.files.as_slice())
    }

    pub async fn push(&self, request: WorkspacePushRequest) -> Result<WorkspacePushResult> {
        require_confirmation(request.confirmed, "push workspace branch")?;
        let checkout = self
            .resolve_checkout(&request.workspace_id, request.checkout_id.as_deref())
            .await?;
        let status = self.git.status(&checkout.path)?;
        let branch = request
            .branch
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or(status.branch.as_deref())
            .ok_or_else(|| anyhow!("branch is required"))?;
        let remote = request
            .remote
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("origin");
        self.git
            .push(&checkout.path, remote, branch, request.set_upstream)
    }

    pub async fn create_pull_request<H>(
        &self,
        github: H,
        request: GitHubPullRequestCreateRequest,
    ) -> Result<GitHubPullRequestSummary>
    where
        H: GitHubPullRequestPort,
    {
        require_confirmation(request.confirmed, "create GitHub pull request")?;
        let checkout = self
            .resolve_checkout(&request.workspace_id, request.checkout_id.as_deref())
            .await?;
        let status = self.git.status(&checkout.path)?;
        github.create_pull_request(&checkout.path, &status, &request)
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

fn require_confirmation(confirmed: bool, action: &str) -> Result<()> {
    if confirmed {
        Ok(())
    } else {
        bail!("explicit confirmation is required to {action}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            git::{WorkspaceGitFileStatus, WorkspacePushResult},
            workspace::{CheckoutId, Workspace, WorkspaceCheckout, WorkspaceId},
        },
        ports::github_pull_request::GitHubPullRequestPort,
    };
    use std::path::Path;

    #[derive(Clone)]
    struct FakeWorkspaceStore;

    impl WorkspaceStore for FakeWorkspaceStore {
        async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
            Ok(vec![])
        }

        async fn get_workspace(&self, _id: &str) -> Result<Option<Workspace>> {
            panic!("confirmation should be checked before workspace lookup")
        }

        async fn list_checkouts(&self, _workspace_id: &str) -> Result<Vec<WorkspaceCheckout>> {
            Ok(vec![])
        }

        async fn get_checkout(&self, _id: &str) -> Result<Option<WorkspaceCheckout>> {
            Ok(None)
        }

        async fn remove_workspace(&self, _workspace_id: &WorkspaceId) -> Result<()> {
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
    struct FakeGitRepository;

    impl GitRepositoryPort for FakeGitRepository {
        fn status(&self, _workdir: &Path) -> Result<WorkspaceGitStatus> {
            Ok(WorkspaceGitStatus {
                root: "/repo".into(),
                branch: Some("feature".into()),
                head_sha: Some("abc".into()),
                is_dirty: false,
                files: Vec::<WorkspaceGitFileStatus>::new(),
            })
        }

        fn diff_summary(&self, _workdir: &Path) -> Result<WorkspaceDiffSummary> {
            panic!("confirmation should be checked before diff access")
        }

        fn commit(
            &self,
            _workdir: &Path,
            _message: &str,
            _files: &[String],
        ) -> Result<WorkspaceCommitResult> {
            panic!("confirmation should be checked before commit")
        }

        fn push(
            &self,
            _workdir: &Path,
            _remote: &str,
            _branch: &str,
            _set_upstream: bool,
        ) -> Result<WorkspacePushResult> {
            panic!("confirmation should be checked before push")
        }

        fn create_worktree(
            &self,
            _source_workdir: &Path,
            _branch_name: &str,
            _worktree_path: &Path,
        ) -> Result<WorkspaceGitStatus> {
            panic!("confirmation should be checked before worktree provisioning")
        }
    }

    #[derive(Clone)]
    struct FakeGitHubPullRequestClient;

    impl GitHubPullRequestPort for FakeGitHubPullRequestClient {
        fn create_pull_request(
            &self,
            _workdir: &Path,
            _status: &WorkspaceGitStatus,
            _request: &GitHubPullRequestCreateRequest,
        ) -> Result<GitHubPullRequestSummary> {
            panic!("confirmation should be checked before GitHub PR creation")
        }
    }

    #[tokio::test]
    async fn commit_requires_explicit_confirmation() {
        let result = WorkspaceGitUseCase::new(FakeWorkspaceStore, FakeGitRepository)
            .commit(WorkspaceCommitRequest {
                workspace_id: "workspace-1".into(),
                checkout_id: None,
                message: "commit".into(),
                files: vec!["src/lib.rs".into()],
                confirmed: false,
            })
            .await;

        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("explicit confirmation")
        );
    }

    #[tokio::test]
    async fn push_requires_explicit_confirmation() {
        let result = WorkspaceGitUseCase::new(FakeWorkspaceStore, FakeGitRepository)
            .push(WorkspacePushRequest {
                workspace_id: "workspace-1".into(),
                checkout_id: None,
                remote: Some("origin".into()),
                branch: Some("feature".into()),
                set_upstream: true,
                confirmed: false,
            })
            .await;

        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("explicit confirmation")
        );
    }

    #[tokio::test]
    async fn pull_request_creation_requires_explicit_confirmation() {
        let result = WorkspaceGitUseCase::new(FakeWorkspaceStore, FakeGitRepository)
            .create_pull_request(
                FakeGitHubPullRequestClient,
                GitHubPullRequestCreateRequest {
                    workspace_id: "workspace-1".into(),
                    checkout_id: None,
                    base: "main".into(),
                    head: Some("feature".into()),
                    title: "Title".into(),
                    body: "Body".into(),
                    draft: false,
                    confirmed: false,
                },
            )
            .await;

        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("explicit confirmation")
        );
    }
}
