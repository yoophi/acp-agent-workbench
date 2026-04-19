use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ResumePolicy {
    #[default]
    Fresh,
    ResumeIfAvailable,
    ResumeRequired,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunRequest {
    pub goal: String,
    pub agent_id: String,
    pub workspace_id: Option<String>,
    pub checkout_id: Option<String>,
    pub cwd: Option<String>,
    pub agent_command: Option<String>,
    pub stdio_buffer_limit_mb: Option<usize>,
    pub auto_allow: Option<bool>,
    pub run_id: Option<String>,
    pub resume_session_id: Option<String>,
    pub resume_policy: Option<ResumePolicy>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub id: String,
    pub goal: String,
    pub agent_id: String,
}

impl AgentRun {
    pub fn new(goal: String, agent_id: String) -> Self {
        Self::with_id(Uuid::new_v4().to_string(), goal, agent_id)
    }

    pub fn with_id(id: String, goal: String, agent_id: String) -> Self {
        Self { id, goal, agent_id }
    }
}
