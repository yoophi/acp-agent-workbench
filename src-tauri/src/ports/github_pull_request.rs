use anyhow::Result;
use std::path::Path;

use crate::domain::git::{
    GitHubPullRequestContext, GitHubPullRequestContextRequest, GitHubPullRequestCreateRequest,
    GitHubPullRequestReviewRequest, GitHubPullRequestReviewResult, GitHubPullRequestSummary,
    WorkspaceGitStatus,
};

pub trait GitHubPullRequestPort: Clone + Send + Sync + 'static {
    fn create_pull_request(
        &self,
        workdir: &Path,
        status: &WorkspaceGitStatus,
        request: &GitHubPullRequestCreateRequest,
    ) -> Result<GitHubPullRequestSummary>;

    fn load_pull_request_context(
        &self,
        workdir: &Path,
        request: &GitHubPullRequestContextRequest,
    ) -> Result<GitHubPullRequestContext>;

    fn submit_pull_request_review(
        &self,
        workdir: &Path,
        request: &GitHubPullRequestReviewRequest,
    ) -> Result<GitHubPullRequestReviewResult>;
}
