use anyhow::Result;

use crate::domain::pull_request_review::{
    CreatePullRequestReviewDraftInput, PullRequestReviewDraft, PullRequestReviewDraftId,
    UpdatePullRequestReviewDraftPatch,
};

pub trait PullRequestReviewDraftStore: Clone + Send + Sync + 'static {
    async fn list_pull_request_review_drafts(
        &self,
        workspace_id: &str,
        pull_request_number: Option<u64>,
    ) -> Result<Vec<PullRequestReviewDraft>>;

    async fn create_pull_request_review_draft(
        &self,
        input: CreatePullRequestReviewDraftInput,
    ) -> Result<PullRequestReviewDraft>;

    async fn update_pull_request_review_draft(
        &self,
        id: &PullRequestReviewDraftId,
        patch: UpdatePullRequestReviewDraftPatch,
    ) -> Result<Option<PullRequestReviewDraft>>;

    async fn delete_pull_request_review_draft(&self, id: &PullRequestReviewDraftId) -> Result<()>;
}
