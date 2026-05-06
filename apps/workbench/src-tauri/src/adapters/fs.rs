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

#[cfg(test)]
mod tests {
    use super::LocalGoalFileReader;
    use crate::ports::goal_file::GoalFileReader;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn reads_supported_markdown_goal_file() {
        let path = temp_goal_path("md");
        fs::write(&path, "ship the ACP workbench").expect("write temp goal");

        let content = LocalGoalFileReader
            .read_goal_file(path.to_str().expect("utf8 path"))
            .expect("read goal file");

        assert_eq!(content, "ship the ACP workbench");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_unsupported_goal_file_extension() {
        let path = temp_goal_path("json");
        fs::write(&path, "{}").expect("write temp goal");

        let err = LocalGoalFileReader
            .read_goal_file(path.to_str().expect("utf8 path"))
            .expect_err("json files should be rejected");

        assert!(err.to_string().contains("Only .txt and .md"));
        let _ = fs::remove_file(path);
    }

    fn temp_goal_path(extension: &str) -> PathBuf {
        std::env::temp_dir().join(format!("acp-goal-{}.{}", Uuid::new_v4(), extension))
    }
}
