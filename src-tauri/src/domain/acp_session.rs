use serde::{Deserialize, Serialize};

use crate::domain::{run::AgentRunRequest, workspace::timestamp};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcpSessionRecord {
    pub run_id: String,
    pub session_id: String,
    pub workspace_id: Option<String>,
    pub checkout_id: Option<String>,
    pub workdir: Option<String>,
    pub agent_id: String,
    pub agent_command: Option<String>,
    pub task: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AcpSessionLookup {
    pub workspace_id: Option<String>,
    pub checkout_id: Option<String>,
    pub workdir: Option<String>,
    pub agent_id: String,
}

impl AcpSessionRecord {
    pub fn from_request(run_id: &str, session_id: &str, request: &AgentRunRequest) -> Self {
        let now = timestamp();
        Self {
            run_id: run_id.to_string(),
            session_id: session_id.to_string(),
            workspace_id: request.workspace_id.clone(),
            checkout_id: request.checkout_id.clone(),
            workdir: request.cwd.clone(),
            agent_id: request.agent_id.clone(),
            agent_command: request.agent_command.clone(),
            task: request.goal.clone(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl AcpSessionLookup {
    pub fn from_request(request: &AgentRunRequest) -> Self {
        Self {
            workspace_id: request.workspace_id.clone(),
            checkout_id: request.checkout_id.clone(),
            workdir: request.cwd.clone(),
            agent_id: request.agent_id.clone(),
        }
    }
}
