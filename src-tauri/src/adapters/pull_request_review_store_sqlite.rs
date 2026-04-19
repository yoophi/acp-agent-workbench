use anyhow::{Result, anyhow, bail};
use sqlx::{Row, SqlitePool};

use crate::{
    domain::{
        pull_request_review::{
            CreatePullRequestReviewDraftInput, PullRequestReviewComment, PullRequestReviewDecision,
            PullRequestReviewDraft, PullRequestReviewDraftId, UpdatePullRequestReviewDraftPatch,
        },
        workspace::timestamp,
    },
    ports::pull_request_review_store::PullRequestReviewDraftStore,
};

#[derive(Clone)]
pub struct SqlitePullRequestReviewDraftStore {
    pool: SqlitePool,
}

impl SqlitePullRequestReviewDraftStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl PullRequestReviewDraftStore for SqlitePullRequestReviewDraftStore {
    async fn list_pull_request_review_drafts(
        &self,
        workspace_id: &str,
        pull_request_number: Option<u64>,
    ) -> Result<Vec<PullRequestReviewDraft>> {
        if workspace_id.trim().is_empty() {
            bail!("workspace id is required");
        }
        let rows = match pull_request_number {
            Some(number) => {
                if number == 0 {
                    bail!("pull request number is required");
                }
                sqlx::query(
                    r#"
                    SELECT id, workspace_id, checkout_id, pull_request_number, run_id, summary,
                           decision, comments_json, created_at, updated_at
                    FROM pull_request_review_drafts
                    WHERE workspace_id = ? AND pull_request_number = ?
                    ORDER BY updated_at DESC
                    "#,
                )
                .bind(workspace_id)
                .bind(number as i64)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(
                    r#"
                    SELECT id, workspace_id, checkout_id, pull_request_number, run_id, summary,
                           decision, comments_json, created_at, updated_at
                    FROM pull_request_review_drafts
                    WHERE workspace_id = ?
                    ORDER BY updated_at DESC
                    "#,
                )
                .bind(workspace_id)
                .fetch_all(&self.pool)
                .await?
            }
        };
        rows.into_iter().map(draft_from_row).collect()
    }

    async fn create_pull_request_review_draft(
        &self,
        input: CreatePullRequestReviewDraftInput,
    ) -> Result<PullRequestReviewDraft> {
        let draft = PullRequestReviewDraft::new(normalize_create_input(input)?);
        upsert_draft(&self.pool, &draft).await?;
        Ok(draft)
    }

    async fn update_pull_request_review_draft(
        &self,
        id: &PullRequestReviewDraftId,
        patch: UpdatePullRequestReviewDraftPatch,
    ) -> Result<Option<PullRequestReviewDraft>> {
        let Some(mut draft) = get_draft(&self.pool, id).await? else {
            return Ok(None);
        };
        if let Some(checkout_id) = patch.checkout_id {
            draft.checkout_id = checkout_id;
        }
        if let Some(run_id) = patch.run_id {
            draft.run_id = run_id
                .map(normalize_optional_string)
                .and_then(|value| value);
        }
        if let Some(summary) = patch.summary {
            draft.summary = summary.trim().to_string();
        }
        if let Some(decision) = patch.decision {
            draft.decision = decision;
        }
        if let Some(comments) = patch.comments {
            draft.comments = normalize_comments(comments)?;
        }
        validate_draft(
            &draft.workspace_id,
            draft.pull_request_number,
            &draft.comments,
        )?;
        draft.updated_at = timestamp();
        upsert_draft(&self.pool, &draft).await?;
        Ok(Some(draft))
    }

    async fn delete_pull_request_review_draft(&self, id: &PullRequestReviewDraftId) -> Result<()> {
        sqlx::query("DELETE FROM pull_request_review_drafts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

async fn get_draft(pool: &SqlitePool, id: &str) -> Result<Option<PullRequestReviewDraft>> {
    let row = sqlx::query(
        r#"
        SELECT id, workspace_id, checkout_id, pull_request_number, run_id, summary,
               decision, comments_json, created_at, updated_at
        FROM pull_request_review_drafts
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(draft_from_row).transpose()
}

async fn upsert_draft(pool: &SqlitePool, draft: &PullRequestReviewDraft) -> Result<()> {
    let comments_json = serde_json::to_string(&draft.comments)?;
    sqlx::query(
        r#"
        INSERT INTO pull_request_review_drafts (
            id, workspace_id, checkout_id, pull_request_number, run_id, summary, decision,
            comments_json, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            checkout_id = excluded.checkout_id,
            run_id = excluded.run_id,
            summary = excluded.summary,
            decision = excluded.decision,
            comments_json = excluded.comments_json,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&draft.id)
    .bind(&draft.workspace_id)
    .bind(&draft.checkout_id)
    .bind(draft.pull_request_number as i64)
    .bind(&draft.run_id)
    .bind(draft.summary.trim())
    .bind(decision_to_db(&draft.decision))
    .bind(comments_json)
    .bind(&draft.created_at)
    .bind(&draft.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

fn draft_from_row(row: sqlx::sqlite::SqliteRow) -> Result<PullRequestReviewDraft> {
    let decision: String = row.try_get("decision")?;
    let comments_json: String = row.try_get("comments_json")?;
    let number: i64 = row.try_get("pull_request_number")?;
    Ok(PullRequestReviewDraft {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        checkout_id: row.try_get("checkout_id")?,
        pull_request_number: number.try_into()?,
        run_id: row.try_get("run_id")?,
        summary: row.try_get("summary")?,
        decision: decision_from_db(&decision)?,
        comments: serde_json::from_str(&comments_json)?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn normalize_create_input(
    mut input: CreatePullRequestReviewDraftInput,
) -> Result<CreatePullRequestReviewDraftInput> {
    input.workspace_id = input.workspace_id.trim().to_string();
    input.checkout_id = input
        .checkout_id
        .map(normalize_optional_string)
        .and_then(|value| value);
    input.run_id = input
        .run_id
        .map(normalize_optional_string)
        .and_then(|value| value);
    input.summary = input.summary.trim().to_string();
    input.comments = normalize_comments(input.comments)?;
    validate_draft(
        &input.workspace_id,
        input.pull_request_number,
        &input.comments,
    )?;
    Ok(input)
}

fn normalize_optional_string(value: String) -> Option<String> {
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn normalize_comments(
    comments: Vec<PullRequestReviewComment>,
) -> Result<Vec<PullRequestReviewComment>> {
    comments
        .into_iter()
        .map(|comment| {
            let path = comment.path.trim().to_string();
            let body = comment.body.trim().to_string();
            if path.is_empty() {
                bail!("review comment path is required");
            }
            if body.is_empty() {
                bail!("review comment body is required");
            }
            if matches!(comment.line, Some(line) if line <= 0) {
                bail!("review comment line must be positive");
            }
            Ok(PullRequestReviewComment {
                path,
                line: comment.line,
                side: comment.side,
                body,
            })
        })
        .collect()
}

fn validate_draft(
    workspace_id: &str,
    pull_request_number: u64,
    comments: &[PullRequestReviewComment],
) -> Result<()> {
    if workspace_id.trim().is_empty() {
        bail!("workspace id is required");
    }
    if pull_request_number == 0 {
        bail!("pull request number is required");
    }
    for comment in comments {
        if comment.path.trim().is_empty() || comment.body.trim().is_empty() {
            bail!("review comments require path and body");
        }
    }
    Ok(())
}

fn decision_to_db(decision: &PullRequestReviewDecision) -> &'static str {
    match decision {
        PullRequestReviewDecision::Comment => "comment",
        PullRequestReviewDecision::Approve => "approve",
        PullRequestReviewDecision::RequestChanges => "request_changes",
    }
}

fn decision_from_db(value: &str) -> Result<PullRequestReviewDecision> {
    match value {
        "comment" => Ok(PullRequestReviewDecision::Comment),
        "approve" => Ok(PullRequestReviewDecision::Approve),
        "request_changes" => Ok(PullRequestReviewDecision::RequestChanges),
        other => Err(anyhow!("unknown pull request review decision: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::sqlite::open_database, domain::pull_request_review::PullRequestReviewCommentSide,
    };

    async fn temp_store() -> SqlitePullRequestReviewDraftStore {
        let dir =
            std::env::temp_dir().join(format!("acp-pr-review-drafts-{}", uuid::Uuid::new_v4()));
        let pool = open_database(&dir).await.unwrap();
        SqlitePullRequestReviewDraftStore::new(pool)
    }

    async fn insert_workspace(store: &SqlitePullRequestReviewDraftStore, id: &str) {
        sqlx::query(
            r#"
            INSERT INTO workspaces (
                id, name, raw_origin_url, canonical_origin_url, host, owner, repo, created_at, updated_at
            )
            VALUES (?, 'repo', 'git@github.com:owner/repo.git', ?, 'github.com', 'owner', 'repo', '1', '1')
            "#,
        )
        .bind(id)
        .bind(format!("github.com/owner/repo-{id}"))
        .execute(&store.pool)
        .await
        .unwrap();
    }

    fn input(workspace_id: &str, number: u64) -> CreatePullRequestReviewDraftInput {
        CreatePullRequestReviewDraftInput {
            workspace_id: workspace_id.to_string(),
            checkout_id: None,
            pull_request_number: number,
            run_id: Some("run-1".to_string()),
            summary: "Looks good overall".to_string(),
            decision: PullRequestReviewDecision::Comment,
            comments: vec![PullRequestReviewComment {
                path: "src/lib.rs".to_string(),
                line: Some(12),
                side: Some(PullRequestReviewCommentSide::Right),
                body: "Please add a regression test.".to_string(),
            }],
        }
    }

    #[tokio::test]
    async fn creates_lists_updates_and_deletes_review_drafts() {
        let store = temp_store().await;
        insert_workspace(&store, "ws-1").await;
        let draft = store
            .create_pull_request_review_draft(input("ws-1", 42))
            .await
            .unwrap();

        let drafts = store
            .list_pull_request_review_drafts("ws-1", Some(42))
            .await
            .unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].comments[0].path, "src/lib.rs");

        let updated = store
            .update_pull_request_review_draft(
                &draft.id,
                UpdatePullRequestReviewDraftPatch {
                    summary: Some("Request changes".to_string()),
                    decision: Some(PullRequestReviewDecision::RequestChanges),
                    comments: Some(vec![]),
                    ..UpdatePullRequestReviewDraftPatch::default()
                },
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.summary, "Request changes");
        assert_eq!(updated.decision, PullRequestReviewDecision::RequestChanges);
        assert!(updated.comments.is_empty());

        store
            .delete_pull_request_review_draft(&draft.id)
            .await
            .unwrap();
        assert!(
            store
                .list_pull_request_review_drafts("ws-1", Some(42))
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn filters_review_drafts_by_workspace_and_pull_request() {
        let store = temp_store().await;
        insert_workspace(&store, "ws-1").await;
        insert_workspace(&store, "ws-2").await;
        store
            .create_pull_request_review_draft(input("ws-1", 42))
            .await
            .unwrap();
        store
            .create_pull_request_review_draft(input("ws-1", 43))
            .await
            .unwrap();
        store
            .create_pull_request_review_draft(input("ws-2", 42))
            .await
            .unwrap();

        let drafts = store
            .list_pull_request_review_drafts("ws-1", Some(42))
            .await
            .unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].workspace_id, "ws-1");
        assert_eq!(drafts[0].pull_request_number, 42);
    }

    #[tokio::test]
    async fn validates_required_review_draft_fields() {
        let store = temp_store().await;
        let err = store
            .create_pull_request_review_draft(input("", 0))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("workspace id"));
    }
}
