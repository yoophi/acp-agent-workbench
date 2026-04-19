use anyhow::Result;
use std::path::Path;

use crate::domain::git::{
    GitHubPullRequestCreateRequest, GitHubPullRequestSummary, WorkspaceGitStatus,
};

pub trait GitHubPullRequestPort: Clone + Send + Sync + 'static {
    fn create_pull_request(
        &self,
        workdir: &Path,
        status: &WorkspaceGitStatus,
        request: &GitHubPullRequestCreateRequest,
    ) -> Result<GitHubPullRequestSummary>;
}
