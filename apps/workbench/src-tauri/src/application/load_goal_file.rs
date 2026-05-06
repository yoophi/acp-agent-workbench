use anyhow::Result;

use crate::ports::goal_file::GoalFileReader;

pub struct LoadGoalFileUseCase<R>
where
    R: GoalFileReader,
{
    reader: R,
}

impl<R> LoadGoalFileUseCase<R>
where
    R: GoalFileReader,
{
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn execute(&self, path: &str) -> Result<String> {
        self.reader.read_goal_file(path)
    }
}
