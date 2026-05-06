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
    pub last_window: bool,
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
    pub fn new(active_run_count: usize, last_window: bool) -> Self {
        Self {
            active_run_count,
            last_window,
        }
    }
}

pub fn should_confirm_last_window_close(label: &str, open_window_count: usize) -> bool {
    should_protect_last_window() && label == MAIN_WORKBENCH_WINDOW_LABEL && open_window_count <= 1
}

#[cfg(target_os = "macos")]
fn should_protect_last_window() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
fn should_protect_last_window() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{
        MAIN_WORKBENCH_WINDOW_LABEL, WorkbenchWindowCloseRequest, should_confirm_last_window_close,
    };

    #[test]
    fn close_request_tracks_last_window_context() {
        assert_eq!(
            WorkbenchWindowCloseRequest::new(2, true),
            WorkbenchWindowCloseRequest {
                active_run_count: 2,
                last_window: true,
            }
        );
    }

    #[test]
    fn last_window_close_confirmation_is_platform_dependent() {
        #[cfg(target_os = "macos")]
        assert!(!should_confirm_last_window_close(
            MAIN_WORKBENCH_WINDOW_LABEL,
            1
        ));

        #[cfg(not(target_os = "macos"))]
        assert!(should_confirm_last_window_close(
            MAIN_WORKBENCH_WINDOW_LABEL,
            1
        ));

        assert!(!should_confirm_last_window_close("workbench-detached", 1));
        assert!(!should_confirm_last_window_close(
            MAIN_WORKBENCH_WINDOW_LABEL,
            2
        ));
    }
}
