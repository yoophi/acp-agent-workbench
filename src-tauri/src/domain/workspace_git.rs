use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGitFileStatus {
    pub path: String,
    pub index_status: String,
    pub worktree_status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGitStatus {
    pub workspace_id: String,
    pub checkout_id: String,
    pub path: String,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub is_clean: bool,
    pub files: Vec<WorkspaceGitFileStatus>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDiffSummary {
    pub workspace_id: String,
    pub checkout_id: String,
    pub path: String,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub staged_stat: String,
    pub unstaged_stat: String,
    pub untracked_files: Vec<String>,
}
