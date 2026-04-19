use anyhow::Result;
use std::path::Path;

use crate::domain::local_task::LocalTaskSummary;

pub trait LocalTaskSource: Clone + Send + Sync + 'static {
    fn has_task_data(&self, workdir: &Path) -> bool;

    fn list_tasks(&self, workdir: &Path) -> Result<Vec<LocalTaskSummary>>;
}
