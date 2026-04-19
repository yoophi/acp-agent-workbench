use anyhow::{Result, anyhow, bail};
use sqlx::{Row, SqlitePool};

use crate::{
    domain::{
        saved_prompt::{
            CreateSavedPromptInput, SavedPrompt, SavedPromptId, SavedPromptRunMode,
            SavedPromptScope, UpdateSavedPromptPatch,
        },
        workspace::timestamp,
    },
    ports::saved_prompt_store::SavedPromptStore,
};

#[derive(Clone)]
pub struct SqliteSavedPromptStore {
    pool: SqlitePool,
}

impl SqliteSavedPromptStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl SavedPromptStore for SqliteSavedPromptStore {
    async fn list_saved_prompts(&self, workspace_id: Option<&str>) -> Result<Vec<SavedPrompt>> {
        let rows = match workspace_id.filter(|value| !value.trim().is_empty()) {
            Some(workspace_id) => {
                sqlx::query(
                    r#"
                    SELECT id, scope, workspace_id, title, body, description, tags_json,
                           run_mode, created_at, updated_at, last_used_at, use_count
                    FROM saved_prompts
                    WHERE scope = 'global' OR workspace_id = ?
                    ORDER BY CASE WHEN workspace_id = ? THEN 0 ELSE 1 END,
                             use_count DESC, updated_at DESC, title ASC
                    "#,
                )
                .bind(workspace_id)
                .bind(workspace_id)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query(
                    r#"
                    SELECT id, scope, workspace_id, title, body, description, tags_json,
                           run_mode, created_at, updated_at, last_used_at, use_count
                    FROM saved_prompts
                    ORDER BY use_count DESC, updated_at DESC, title ASC
                    "#,
                )
                .fetch_all(&self.pool)
                .await?
            }
        };
        rows.into_iter().map(saved_prompt_from_row).collect()
    }

    async fn create_saved_prompt(&self, input: CreateSavedPromptInput) -> Result<SavedPrompt> {
        validate_prompt_parts(&input.title, &input.body)?;
        let prompt = SavedPrompt::new(normalize_create_input(input));
        upsert_saved_prompt(&self.pool, &prompt).await?;
        Ok(prompt)
    }

    async fn update_saved_prompt(
        &self,
        id: &SavedPromptId,
        patch: UpdateSavedPromptPatch,
    ) -> Result<Option<SavedPrompt>> {
        let Some(mut prompt) = get_saved_prompt(&self.pool, id).await? else {
            return Ok(None);
        };
        if let Some(scope) = patch.scope {
            prompt.scope = scope;
        }
        if let Some(workspace_id) = patch.workspace_id {
            prompt.workspace_id = workspace_id;
        }
        if let Some(title) = patch.title {
            prompt.title = title;
        }
        if let Some(body) = patch.body {
            prompt.body = body;
        }
        if let Some(description) = patch.description {
            prompt.description = description;
        }
        if let Some(tags) = patch.tags {
            prompt.tags = normalize_tags(tags);
        }
        if let Some(run_mode) = patch.run_mode {
            prompt.run_mode = run_mode;
        }
        validate_prompt_parts(&prompt.title, &prompt.body)?;
        if matches!(prompt.scope, SavedPromptScope::Global) {
            prompt.workspace_id = None;
        }
        prompt.updated_at = timestamp();
        upsert_saved_prompt(&self.pool, &prompt).await?;
        Ok(Some(prompt))
    }

    async fn delete_saved_prompt(&self, id: &SavedPromptId) -> Result<()> {
        sqlx::query("DELETE FROM saved_prompts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn record_saved_prompt_used(&self, id: &SavedPromptId) -> Result<Option<SavedPrompt>> {
        let now = timestamp();
        sqlx::query(
            r#"
            UPDATE saved_prompts
            SET last_used_at = ?, use_count = use_count + 1, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        get_saved_prompt(&self.pool, id).await
    }
}

async fn get_saved_prompt(pool: &SqlitePool, id: &str) -> Result<Option<SavedPrompt>> {
    let row = sqlx::query(
        r#"
        SELECT id, scope, workspace_id, title, body, description, tags_json,
               run_mode, created_at, updated_at, last_used_at, use_count
        FROM saved_prompts
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    row.map(saved_prompt_from_row).transpose()
}

async fn upsert_saved_prompt(pool: &SqlitePool, prompt: &SavedPrompt) -> Result<()> {
    let tags_json = serde_json::to_string(&prompt.tags)?;
    sqlx::query(
        r#"
        INSERT INTO saved_prompts (
            id, scope, workspace_id, title, body, description, tags_json, run_mode,
            created_at, updated_at, last_used_at, use_count
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            scope = excluded.scope,
            workspace_id = excluded.workspace_id,
            title = excluded.title,
            body = excluded.body,
            description = excluded.description,
            tags_json = excluded.tags_json,
            run_mode = excluded.run_mode,
            updated_at = excluded.updated_at,
            last_used_at = excluded.last_used_at,
            use_count = excluded.use_count
        "#,
    )
    .bind(&prompt.id)
    .bind(scope_to_db(&prompt.scope))
    .bind(&prompt.workspace_id)
    .bind(prompt.title.trim())
    .bind(prompt.body.trim())
    .bind(prompt.description.as_deref().map(str::trim))
    .bind(tags_json)
    .bind(run_mode_to_db(&prompt.run_mode))
    .bind(&prompt.created_at)
    .bind(&prompt.updated_at)
    .bind(&prompt.last_used_at)
    .bind(prompt.use_count)
    .execute(pool)
    .await?;
    Ok(())
}

fn saved_prompt_from_row(row: sqlx::sqlite::SqliteRow) -> Result<SavedPrompt> {
    let scope: String = row.try_get("scope")?;
    let run_mode: String = row.try_get("run_mode")?;
    let tags_json: String = row.try_get("tags_json")?;
    Ok(SavedPrompt {
        id: row.try_get("id")?,
        scope: scope_from_db(&scope)?,
        workspace_id: row.try_get("workspace_id")?,
        title: row.try_get("title")?,
        body: row.try_get("body")?,
        description: row.try_get("description")?,
        tags: serde_json::from_str(&tags_json)?,
        run_mode: run_mode_from_db(&run_mode)?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        last_used_at: row.try_get("last_used_at")?,
        use_count: row.try_get("use_count")?,
    })
}

fn normalize_create_input(mut input: CreateSavedPromptInput) -> CreateSavedPromptInput {
    input.title = input.title.trim().to_string();
    input.body = input.body.trim().to_string();
    input.description = input
        .description
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    input.tags = normalize_tags(input.tags);
    if matches!(input.scope, SavedPromptScope::Global) {
        input.workspace_id = None;
    }
    input
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim().to_string();
        if !tag.is_empty() && !normalized.contains(&tag) {
            normalized.push(tag);
        }
    }
    normalized
}

fn validate_prompt_parts(title: &str, body: &str) -> Result<()> {
    if title.trim().is_empty() {
        bail!("saved prompt title is empty");
    }
    if body.trim().is_empty() {
        bail!("saved prompt body is empty");
    }
    Ok(())
}

fn scope_to_db(scope: &SavedPromptScope) -> &'static str {
    match scope {
        SavedPromptScope::Global => "global",
        SavedPromptScope::Workspace => "workspace",
    }
}

fn scope_from_db(value: &str) -> Result<SavedPromptScope> {
    match value {
        "global" => Ok(SavedPromptScope::Global),
        "workspace" => Ok(SavedPromptScope::Workspace),
        other => Err(anyhow!("unknown saved prompt scope: {other}")),
    }
}

fn run_mode_to_db(run_mode: &SavedPromptRunMode) -> &'static str {
    match run_mode {
        SavedPromptRunMode::Insert => "insert",
        SavedPromptRunMode::Send => "send",
        SavedPromptRunMode::Enqueue => "enqueue",
    }
}

fn run_mode_from_db(value: &str) -> Result<SavedPromptRunMode> {
    match value {
        "insert" => Ok(SavedPromptRunMode::Insert),
        "send" => Ok(SavedPromptRunMode::Send),
        "enqueue" => Ok(SavedPromptRunMode::Enqueue),
        other => Err(anyhow!("unknown saved prompt run mode: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::sqlite::open_database;

    async fn temp_store() -> SqliteSavedPromptStore {
        let dir = std::env::temp_dir().join(format!("acp-saved-prompts-{}", uuid::Uuid::new_v4()));
        let pool = open_database(&dir).await.unwrap();
        SqliteSavedPromptStore::new(pool)
    }

    async fn insert_workspace(store: &SqliteSavedPromptStore, id: &str) {
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

    fn input(title: &str, workspace_id: Option<&str>) -> CreateSavedPromptInput {
        CreateSavedPromptInput {
            scope: if workspace_id.is_some() {
                SavedPromptScope::Workspace
            } else {
                SavedPromptScope::Global
            },
            workspace_id: workspace_id.map(str::to_string),
            title: title.to_string(),
            body: "Run tests".to_string(),
            description: None,
            tags: vec!["test".to_string(), "test".to_string()],
            run_mode: SavedPromptRunMode::Enqueue,
        }
    }

    #[tokio::test]
    async fn creates_lists_and_records_usage() {
        let store = temp_store().await;
        insert_workspace(&store, "ws_1").await;
        let prompt = store
            .create_saved_prompt(input("Tests", Some("ws_1")))
            .await
            .unwrap();

        let prompts = store.list_saved_prompts(Some("ws_1")).await.unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].tags, vec!["test".to_string()]);

        let used = store
            .record_saved_prompt_used(&prompt.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(used.use_count, 1);
        assert!(used.last_used_at.is_some());
    }

    #[tokio::test]
    async fn filters_workspace_prompts_with_globals() {
        let store = temp_store().await;
        insert_workspace(&store, "ws_1").await;
        insert_workspace(&store, "ws_2").await;
        store
            .create_saved_prompt(input("Global", None))
            .await
            .unwrap();
        store
            .create_saved_prompt(input("Workspace", Some("ws_1")))
            .await
            .unwrap();
        store
            .create_saved_prompt(input("Other", Some("ws_2")))
            .await
            .unwrap();

        let prompts = store.list_saved_prompts(Some("ws_1")).await.unwrap();
        let titles: Vec<_> = prompts.into_iter().map(|prompt| prompt.title).collect();
        assert_eq!(titles, vec!["Workspace".to_string(), "Global".to_string()]);
    }

    #[tokio::test]
    async fn updates_and_deletes_prompt() {
        let store = temp_store().await;
        let prompt = store.create_saved_prompt(input("Old", None)).await.unwrap();
        let updated = store
            .update_saved_prompt(
                &prompt.id,
                UpdateSavedPromptPatch {
                    title: Some("New".to_string()),
                    run_mode: Some(SavedPromptRunMode::Insert),
                    ..UpdateSavedPromptPatch::default()
                },
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.title, "New");
        assert_eq!(updated.run_mode, SavedPromptRunMode::Insert);

        store.delete_saved_prompt(&prompt.id).await.unwrap();
        assert!(store.list_saved_prompts(None).await.unwrap().is_empty());
    }
}
