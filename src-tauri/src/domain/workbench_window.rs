use serde::{Deserialize, Serialize};

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
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            is_main: label == MAIN_WORKBENCH_WINDOW_LABEL,
            label,
        }
    }
}
