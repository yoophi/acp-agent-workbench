use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalTaskStatus {
    Open,
    InProgress,
    Closed,
}

impl LocalTaskStatus {
    pub fn as_beads_status(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Closed => "closed",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalTaskSummary {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub labels: Vec<String>,
    pub dependencies: Vec<String>,
    pub blocked: bool,
    pub acceptance_criteria: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalTaskList {
    pub workspace_id: String,
    pub checkout_id: String,
    pub workdir: String,
    pub source: LocalTaskSourceKind,
    pub detected: bool,
    pub available: bool,
    pub tasks: Vec<LocalTaskSummary>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LocalTaskSourceKind {
    Beads,
}

impl LocalTaskList {
    pub fn available(
        workspace_id: String,
        checkout_id: String,
        workdir: String,
        tasks: Vec<LocalTaskSummary>,
    ) -> Self {
        Self {
            workspace_id,
            checkout_id,
            workdir,
            source: LocalTaskSourceKind::Beads,
            detected: true,
            available: true,
            tasks,
            error: None,
        }
    }

    pub fn unavailable(
        workspace_id: String,
        checkout_id: String,
        workdir: String,
        detected: bool,
        error: impl Into<String>,
    ) -> Self {
        Self {
            workspace_id,
            checkout_id,
            workdir,
            source: LocalTaskSourceKind::Beads,
            detected,
            available: false,
            tasks: vec![],
            error: Some(error.into()),
        }
    }
}
