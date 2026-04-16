use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

use crate::ports::goal_file::GoalFileReader;

#[derive(Clone, Default)]
pub struct LocalGoalFileReader;

impl GoalFileReader for LocalGoalFileReader {
    fn read_goal_file(&self, path: &str) -> Result<String> {
        let path = Path::new(path);
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "txt" | "md") {
            bail!("Only .txt and .md goal files are supported");
        }

        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
    }
}
