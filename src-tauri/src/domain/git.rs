use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGitStatus {
    pub root: String,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub is_dirty: bool,
    pub files: Vec<WorkspaceGitFileStatus>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceGitFileStatus {
    pub path: String,
    pub status_code: String,
    pub status_label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDiffSummary {
    pub status: WorkspaceGitStatus,
    pub diff_stat: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCommitRequest {
    pub workspace_id: String,
    pub checkout_id: Option<String>,
    pub message: String,
    pub files: Vec<String>,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCommitResult {
    pub commit_sha: String,
    pub status: WorkspaceGitStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePushRequest {
    pub workspace_id: String,
    pub checkout_id: Option<String>,
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub set_upstream: bool,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePushResult {
    pub remote: String,
    pub branch: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubPullRequestCreateRequest {
    pub workspace_id: String,
    pub checkout_id: Option<String>,
    pub base: String,
    pub head: Option<String>,
    pub title: String,
    pub body: String,
    pub draft: bool,
    pub confirmed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitHubPullRequestSummary {
    pub number: Option<u64>,
    pub url: String,
    pub title: String,
    pub base_ref: String,
    pub head_ref: String,
}
