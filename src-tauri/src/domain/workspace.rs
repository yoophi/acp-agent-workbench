use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

pub type WorkspaceId = String;
pub type CheckoutId = String;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub origin: GitOrigin,
    pub default_checkout_id: Option<CheckoutId>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitOrigin {
    pub raw_url: String,
    pub canonical_url: String,
    pub host: String,
    pub owner: String,
    pub repo: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceCheckout {
    pub id: CheckoutId,
    pub workspace_id: WorkspaceId,
    pub path: PathBuf,
    pub kind: CheckoutKind,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub is_default: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CheckoutKind {
    Clone,
    Worktree,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredWorkspace {
    pub workspace: Workspace,
    pub checkout: WorkspaceCheckout,
}

impl Workspace {
    pub fn from_origin(origin: GitOrigin) -> Self {
        let now = timestamp();
        Self {
            id: workspace_id(&origin.canonical_url),
            name: origin.repo.clone(),
            origin,
            default_checkout_id: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl WorkspaceCheckout {
    pub fn new(
        workspace_id: WorkspaceId,
        canonical_origin: &str,
        path: PathBuf,
        branch: Option<String>,
        head_sha: Option<String>,
        is_default: bool,
    ) -> Self {
        Self {
            id: checkout_id(canonical_origin, &path),
            workspace_id,
            path,
            kind: CheckoutKind::Clone,
            branch,
            head_sha,
            is_default,
        }
    }

    pub fn new_worktree(
        workspace_id: WorkspaceId,
        canonical_origin: &str,
        path: PathBuf,
        branch: Option<String>,
        head_sha: Option<String>,
    ) -> Self {
        Self {
            id: checkout_id(canonical_origin, &path),
            workspace_id,
            path,
            kind: CheckoutKind::Worktree,
            branch,
            head_sha,
            is_default: false,
        }
    }
}

pub fn workspace_id(canonical_origin: &str) -> WorkspaceId {
    format!("ws_{}", deterministic_id(canonical_origin))
}

pub fn checkout_id(canonical_origin: &str, path: &PathBuf) -> CheckoutId {
    format!(
        "co_{}",
        deterministic_id(&format!("{}:{}", canonical_origin, path.display()))
    )
}

pub fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("{secs}")
}

fn deterministic_id(value: &str) -> String {
    Uuid::new_v5(&Uuid::NAMESPACE_URL, value.as_bytes())
        .simple()
        .to_string()
}
