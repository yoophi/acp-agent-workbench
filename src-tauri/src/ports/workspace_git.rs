use anyhow::Result;
use std::future::Future;

use crate::domain::{
    workspace::WorkspaceCheckout,
    workspace_git::{WorkspaceDiffSummary, WorkspaceGitStatus},
};

pub trait WorkspaceGitInspector: Clone + Send + Sync + 'static {
    fn status(
        &self,
        workspace_id: String,
        checkout: WorkspaceCheckout,
    ) -> impl Future<Output = Result<WorkspaceGitStatus>> + Send;

    fn diff_summary(
        &self,
        workspace_id: String,
        checkout: WorkspaceCheckout,
    ) -> impl Future<Output = Result<WorkspaceDiffSummary>> + Send;
}
