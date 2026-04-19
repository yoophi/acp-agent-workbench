use anyhow::{Result, anyhow, bail};

use crate::{
    domain::{
        git::{
            GitHubPullRequestContext, GitHubPullRequestContextRequest,
            GitHubPullRequestCreateRequest, GitHubPullRequestReviewRequest,
            GitHubPullRequestReviewResult, GitHubPullRequestSummary, WorkspaceCommitRequest,
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

    pub async fn pull_request_context<H>(
        &self,
        github: H,
        request: GitHubPullRequestContextRequest,
    ) -> Result<GitHubPullRequestContext>
    where
        H: GitHubPullRequestPort,
    {
        let checkout = self
            .resolve_checkout(&request.workspace_id, request.checkout_id.as_deref())
            .await?;
        github.load_pull_request_context(&checkout.path, &request)
    }

    pub async fn submit_pull_request_review<H>(
        &self,
        github: H,
        request: GitHubPullRequestReviewRequest,
    ) -> Result<GitHubPullRequestReviewResult>
    where
        H: GitHubPullRequestPort,
    {
        require_confirmation(request.confirmed, "publish GitHub pull request review")?;
        let checkout = self
            .resolve_checkout(&request.workspace_id, request.checkout_id.as_deref())
            .await?;
        github.submit_pull_request_review(&checkout.path, &request)
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
            git::{
                GitHubPullRequestReviewDecision, GitHubPullRequestReviewResult,
                WorkspaceGitFileStatus, WorkspacePushResult,
            },
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

        async fn save_checkout(&self, _checkout: WorkspaceCheckout) -> Result<WorkspaceCheckout> {
            panic!("confirmation should be checked before checkout save")
        }

        async fn remove_workspace(&self, _workspace_id: &WorkspaceId) -> Result<()> {
            Ok(())
        }

        async fn remove_checkout(&self, _checkout_id: &CheckoutId) -> Result<()> {
            Ok(())
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
            panic!("confirmation should be checked before worktree creation")
        }

        fn remove_worktree(&self, _worktree_path: &Path, _branch_name: Option<&str>) -> Result<()> {
            panic!("confirmation should be checked before worktree cleanup")
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

        fn load_pull_request_context(
            &self,
            _workdir: &Path,
            _request: &GitHubPullRequestContextRequest,
        ) -> Result<GitHubPullRequestContext> {
            panic!("confirmation should be checked before GitHub PR context loading")
        }

        fn submit_pull_request_review(
            &self,
            _workdir: &Path,
            _request: &GitHubPullRequestReviewRequest,
        ) -> Result<GitHubPullRequestReviewResult> {
            panic!("confirmation should be checked before GitHub PR review publishing")
        }
    }

    #[derive(Clone)]
    struct ContextWorkspaceStore;

    impl WorkspaceStore for ContextWorkspaceStore {
        async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
            Ok(vec![])
        }

        async fn get_workspace(&self, id: &str) -> Result<Option<Workspace>> {
            Ok((id == "workspace-1").then(|| Workspace {
                id: "workspace-1".into(),
                name: "repo".into(),
                origin: crate::domain::workspace::GitOrigin {
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
            Ok((id == "checkout-1").then(|| WorkspaceCheckout {
                id: "checkout-1".into(),
                workspace_id: "workspace-1".into(),
                path: "/repo/worktree".into(),
                kind: crate::domain::workspace::CheckoutKind::Clone,
                branch: Some("main".into()),
                head_sha: Some("abc".into()),
                is_default: true,
            }))
        }

        async fn save_checkout(&self, _checkout: WorkspaceCheckout) -> Result<WorkspaceCheckout> {
            panic!("PR context loading should not save checkouts")
        }

        async fn remove_workspace(&self, _workspace_id: &WorkspaceId) -> Result<()> {
            Ok(())
        }

        async fn remove_checkout(&self, _checkout_id: &CheckoutId) -> Result<()> {
            Ok(())
        }

        async fn refresh_checkout(
            &self,
            _checkout_id: &CheckoutId,
        ) -> Result<Option<WorkspaceCheckout>> {
            Ok(None)
        }
    }

    #[derive(Clone)]
    struct ContextGitHubClient;

    impl GitHubPullRequestPort for ContextGitHubClient {
        fn create_pull_request(
            &self,
            _workdir: &Path,
            _status: &WorkspaceGitStatus,
            _request: &GitHubPullRequestCreateRequest,
        ) -> Result<GitHubPullRequestSummary> {
            panic!("PR context loading should not create pull requests")
        }

        fn load_pull_request_context(
            &self,
            workdir: &Path,
            request: &GitHubPullRequestContextRequest,
        ) -> Result<GitHubPullRequestContext> {
            assert_eq!(workdir, Path::new("/repo/worktree"));
            Ok(GitHubPullRequestContext {
                number: request.number,
                url: "https://github.com/owner/repo/pull/42".into(),
                title: "Review me".into(),
                body: Some("body".into()),
                author: Some("octocat".into()),
                base_ref: "main".into(),
                head_ref: "feature".into(),
                head_sha: "def".into(),
                changed_files: vec!["src/lib.rs".into()],
                diff: "diff --git a/src/lib.rs b/src/lib.rs".into(),
            })
        }

        fn submit_pull_request_review(
            &self,
            workdir: &Path,
            request: &GitHubPullRequestReviewRequest,
        ) -> Result<GitHubPullRequestReviewResult> {
            assert_eq!(workdir, Path::new("/repo/worktree"));
            Ok(GitHubPullRequestReviewResult {
                number: request.number,
                decision: request.decision.clone(),
                submitted: true,
            })
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

    #[tokio::test]
    async fn pull_request_review_publishing_requires_explicit_confirmation() {
        let result = WorkspaceGitUseCase::new(FakeWorkspaceStore, FakeGitRepository)
            .submit_pull_request_review(
                FakeGitHubPullRequestClient,
                GitHubPullRequestReviewRequest {
                    workspace_id: "workspace-1".into(),
                    checkout_id: None,
                    number: 42,
                    body: "Looks good".into(),
                    decision: GitHubPullRequestReviewDecision::Approve,
                    comments: Vec::new(),
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

    #[tokio::test]
    async fn loads_pull_request_context_from_resolved_checkout() {
        let context = WorkspaceGitUseCase::new(ContextWorkspaceStore, FakeGitRepository)
            .pull_request_context(
                ContextGitHubClient,
                GitHubPullRequestContextRequest {
                    workspace_id: "workspace-1".into(),
                    checkout_id: Some("checkout-1".into()),
                    number: 42,
                },
            )
            .await
            .unwrap();

        assert_eq!(context.number, 42);
        assert_eq!(context.changed_files, vec!["src/lib.rs"]);
    }

    #[tokio::test]
    async fn submits_pull_request_review_from_resolved_checkout() {
        let result = WorkspaceGitUseCase::new(ContextWorkspaceStore, FakeGitRepository)
            .submit_pull_request_review(
                ContextGitHubClient,
                GitHubPullRequestReviewRequest {
                    workspace_id: "workspace-1".into(),
                    checkout_id: Some("checkout-1".into()),
                    number: 42,
                    body: "Ready".into(),
                    decision: GitHubPullRequestReviewDecision::Comment,
                    comments: Vec::new(),
                    confirmed: true,
                },
            )
            .await
            .unwrap();

        assert_eq!(result.number, 42);
        assert!(result.submitted);
    }
}
