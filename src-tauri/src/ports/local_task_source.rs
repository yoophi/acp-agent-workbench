use anyhow::Result;
use std::path::Path;

use crate::domain::local_task::{LocalTaskStatus, LocalTaskSummary};

pub trait LocalTaskSource: Clone + Send + Sync + 'static {
    fn has_task_data(&self, workdir: &Path) -> bool;

    fn list_tasks(&self, workdir: &Path) -> Result<Vec<LocalTaskSummary>>;

    fn update_status(
        &self,
        workdir: &Path,
        task_id: &str,
        status: LocalTaskStatus,
    ) -> Result<LocalTaskSummary>;
}
