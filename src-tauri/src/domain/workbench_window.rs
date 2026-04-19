use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAIN_WORKBENCH_WINDOW_LABEL: &str = "main";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchWindowInfo {
    pub label: String,
    pub is_main: bool,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchWindowBootstrap {
    pub label: String,
    pub is_main: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detached_tab: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchWindowCloseRequest {
    pub active_run_count: usize,
}

impl WorkbenchWindowInfo {
    pub fn new(label: impl Into<String>, title: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            is_main: label == MAIN_WORKBENCH_WINDOW_LABEL,
            label,
            title: title.into(),
        }
    }
}

impl WorkbenchWindowBootstrap {
    pub fn new(label: impl Into<String>, detached_tab: Option<Value>) -> Self {
        let label = label.into();
        Self {
            is_main: label == MAIN_WORKBENCH_WINDOW_LABEL,
            label,
            detached_tab,
        }
    }
}

impl WorkbenchWindowCloseRequest {
    pub fn new(active_run_count: usize) -> Self {
        Self { active_run_count }
    }
}
