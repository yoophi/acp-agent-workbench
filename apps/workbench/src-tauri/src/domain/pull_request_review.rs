use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::workspace::{CheckoutId, WorkspaceId, timestamp};

pub type PullRequestReviewDraftId = String;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestReviewDraft {
    pub id: PullRequestReviewDraftId,
    pub workspace_id: WorkspaceId,
    pub checkout_id: Option<CheckoutId>,
    pub pull_request_number: u64,
    pub run_id: Option<String>,
    pub summary: String,
    pub decision: PullRequestReviewDecision,
    pub comments: Vec<PullRequestReviewComment>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestReviewDecision {
    Comment,
    Approve,
    RequestChanges,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PullRequestReviewCommentSide {
    Left,
    Right,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestReviewComment {
    pub path: String,
    pub line: Option<i64>,
    pub side: Option<PullRequestReviewCommentSide>,
    pub body: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreatePullRequestReviewDraftInput {
    pub workspace_id: WorkspaceId,
    pub checkout_id: Option<CheckoutId>,
    pub pull_request_number: u64,
    pub run_id: Option<String>,
    pub summary: String,
    pub decision: PullRequestReviewDecision,
    pub comments: Vec<PullRequestReviewComment>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePullRequestReviewDraftPatch {
    pub checkout_id: Option<Option<CheckoutId>>,
    pub run_id: Option<Option<String>>,
    pub summary: Option<String>,
    pub decision: Option<PullRequestReviewDecision>,
    pub comments: Option<Vec<PullRequestReviewComment>>,
}

impl PullRequestReviewDraft {
    pub fn new(input: CreatePullRequestReviewDraftInput) -> Self {
        let now = timestamp();
        Self {
            id: format!("prrd_{}", Uuid::new_v4().simple()),
            workspace_id: input.workspace_id,
            checkout_id: input.checkout_id,
            pull_request_number: input.pull_request_number,
            run_id: input.run_id,
            summary: input.summary,
            decision: input.decision,
            comments: input.comments,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
