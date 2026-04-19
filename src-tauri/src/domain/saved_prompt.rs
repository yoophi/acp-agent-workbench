use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::workspace::{WorkspaceId, timestamp};

pub type SavedPromptId = String;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SavedPrompt {
    pub id: SavedPromptId,
    pub scope: SavedPromptScope,
    pub workspace_id: Option<WorkspaceId>,
    pub title: String,
    pub body: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub run_mode: SavedPromptRunMode,
    pub created_at: String,
    pub updated_at: String,
    pub last_used_at: Option<String>,
    pub use_count: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SavedPromptScope {
    Global,
    Workspace,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SavedPromptRunMode {
    Insert,
    Send,
    Enqueue,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateSavedPromptInput {
    pub scope: SavedPromptScope,
    pub workspace_id: Option<WorkspaceId>,
    pub title: String,
    pub body: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub run_mode: SavedPromptRunMode,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSavedPromptPatch {
    pub scope: Option<SavedPromptScope>,
    pub workspace_id: Option<Option<WorkspaceId>>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub description: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub run_mode: Option<SavedPromptRunMode>,
}

impl SavedPrompt {
    pub fn new(input: CreateSavedPromptInput) -> Self {
        let now = timestamp();
        Self {
            id: format!("sp_{}", Uuid::new_v4().simple()),
            scope: input.scope,
            workspace_id: input.workspace_id,
            title: input.title,
            body: input.body,
            description: input.description,
            tags: input.tags,
            run_mode: input.run_mode,
            created_at: now.clone(),
            updated_at: now,
            last_used_at: None,
            use_count: 0,
        }
    }
}
