use anyhow::Result;
use std::path::Path;

use crate::domain::git::{WorkspaceDiffSummary, WorkspaceGitStatus};

pub trait GitRepositoryPort: Clone + Send + Sync + 'static {
    fn status(&self, workdir: &Path) -> Result<WorkspaceGitStatus>;

    fn diff_summary(&self, workdir: &Path) -> Result<WorkspaceDiffSummary>;
}
