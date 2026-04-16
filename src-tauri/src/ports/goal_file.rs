use anyhow::Result;

pub trait GoalFileReader: Clone + Send + Sync + 'static {
    fn read_goal_file(&self, path: &str) -> Result<String>;
}
