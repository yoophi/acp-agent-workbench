use anyhow::Result;
use std::path::Path;

use crate::domain::git::{
    WorkspaceCommitResult, WorkspaceDiffSummary, WorkspaceGitStatus, WorkspacePushResult,
};

pub trait GitRepositoryPort: Clone + Send + Sync + 'static {
    fn status(&self, workdir: &Path) -> Result<WorkspaceGitStatus>;

    fn diff_summary(&self, workdir: &Path) -> Result<WorkspaceDiffSummary>;

    fn commit(
        &self,
        workdir: &Path,
        message: &str,
        files: &[String],
    ) -> Result<WorkspaceCommitResult>;

    fn push(
        &self,
        workdir: &Path,
        remote: &str,
        branch: &str,
        set_upstream: bool,
    ) -> Result<WorkspacePushResult>;

    fn create_worktree(
        &self,
        source_workdir: &Path,
        branch_name: &str,
        worktree_path: &Path,
    ) -> Result<WorkspaceGitStatus>;

    fn remove_worktree(&self, worktree_path: &Path, branch_name: Option<&str>) -> Result<()>;
}
