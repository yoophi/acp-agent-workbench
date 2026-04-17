use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEventEnvelope {
    pub run_id: String,
    pub event: RunEvent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RunEvent {
    Lifecycle {
        status: LifecycleStatus,
        message: String,
    },
    AgentMessage {
        text: String,
    },
    Thought {
        text: String,
    },
    Plan {
        entries: Vec<PlanEntry>,
    },
    Tool {
        status: String,
        title: String,
        locations: Vec<String>,
    },
    Usage {
        used: i64,
        size: i64,
    },
    Permission {
        permission_id: Option<String>,
        title: String,
        input: Option<Value>,
        options: Vec<PermissionOption>,
        selected: Option<String>,
        requires_response: bool,
    },
    FileSystem {
        operation: String,
        path: String,
    },
    Terminal {
        operation: String,
        terminal_id: Option<String>,
        message: String,
    },
    Diagnostic {
        message: String,
    },
    Raw {
        method: String,
        payload: Value,
    },
    Error {
        message: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LifecycleStatus {
    Started,
    Initialized,
    SessionCreated,
    PromptSent,
    PromptCompleted,
    Cancelled,
    Completed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanEntry {
    pub status: String,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOption {
    pub name: String,
    pub kind: String,
    pub option_id: String,
}
