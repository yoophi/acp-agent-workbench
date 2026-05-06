use anyhow::Result;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{path::Path, time::Duration};

const WORKSPACE_SCHEMA_VERSION: i64 = 1;
const SAVED_PROMPTS_SCHEMA_VERSION: i64 = 2;
const ACP_SESSIONS_SCHEMA_VERSION: i64 = 3;
const PULL_REQUEST_REVIEW_DRAFTS_SCHEMA_VERSION: i64 = 4;

pub async fn open_database(app_data_dir: &Path) -> Result<SqlitePool> {
    tokio::fs::create_dir_all(app_data_dir).await?;
    let db_path = app_data_dir.join("workbench.sqlite");
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_millis(5_000));
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    configure_database(&pool).await?;
    migrate_database(&pool).await?;
    Ok(pool)
}

async fn configure_database(pool: &SqlitePool) -> Result<()> {
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA busy_timeout = 5000")
        .execute(pool)
        .await?;
    Ok(())
}

async fn migrate_database(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    if !migration_applied(pool, WORKSPACE_SCHEMA_VERSION).await? {
        migrate_workspace_schema(pool).await?;
    }
    if !migration_applied(pool, SAVED_PROMPTS_SCHEMA_VERSION).await? {
        migrate_saved_prompts_schema(pool).await?;
    }
    if !migration_applied(pool, ACP_SESSIONS_SCHEMA_VERSION).await? {
        migrate_acp_sessions_schema(pool).await?;
    }
    if !migration_applied(pool, PULL_REQUEST_REVIEW_DRAFTS_SCHEMA_VERSION).await? {
        migrate_pull_request_review_drafts_schema(pool).await?;
    }
    Ok(())
}

async fn migration_applied(pool: &SqlitePool, version: i64) -> Result<bool> {
    let applied: Option<(i64,)> =
        sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
            .bind(version)
            .fetch_optional(pool)
            .await?;
    Ok(applied.is_some())
}

async fn migrate_workspace_schema(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS workspaces (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            raw_origin_url TEXT NOT NULL,
            canonical_origin_url TEXT NOT NULL UNIQUE,
            host TEXT NOT NULL,
            owner TEXT NOT NULL,
            repo TEXT NOT NULL,
            default_checkout_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS workspace_checkouts (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
            path TEXT NOT NULL,
            kind TEXT NOT NULL,
            branch TEXT,
            head_sha TEXT,
            is_default INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(workspace_id, path)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_workspace_checkouts_workspace_id
        ON workspace_checkouts(workspace_id)
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO schema_migrations (version) VALUES (?)")
        .bind(WORKSPACE_SCHEMA_VERSION)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn migrate_saved_prompts_schema(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS saved_prompts (
            id TEXT PRIMARY KEY,
            scope TEXT NOT NULL,
            workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            description TEXT,
            tags_json TEXT NOT NULL DEFAULT '[]',
            run_mode TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_used_at TEXT,
            use_count INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_saved_prompts_scope_workspace
        ON saved_prompts(scope, workspace_id, updated_at DESC)
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO schema_migrations (version) VALUES (?)")
        .bind(SAVED_PROMPTS_SCHEMA_VERSION)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn migrate_acp_sessions_schema(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS acp_sessions (
            run_id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            workspace_id TEXT REFERENCES workspaces(id) ON DELETE SET NULL,
            checkout_id TEXT REFERENCES workspace_checkouts(id) ON DELETE SET NULL,
            workdir TEXT,
            agent_id TEXT NOT NULL,
            agent_command TEXT,
            task TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_acp_sessions_lookup
        ON acp_sessions(workspace_id, checkout_id, workdir, agent_id, updated_at DESC)
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO schema_migrations (version) VALUES (?)")
        .bind(ACP_SESSIONS_SCHEMA_VERSION)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn migrate_pull_request_review_drafts_schema(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS pull_request_review_drafts (
            id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
            checkout_id TEXT REFERENCES workspace_checkouts(id) ON DELETE SET NULL,
            pull_request_number INTEGER NOT NULL,
            run_id TEXT,
            summary TEXT NOT NULL,
            decision TEXT NOT NULL,
            comments_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_pull_request_review_drafts_lookup
        ON pull_request_review_drafts(workspace_id, pull_request_number, updated_at DESC)
        "#,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO schema_migrations (version) VALUES (?)")
        .bind(PULL_REQUEST_REVIEW_DRAFTS_SCHEMA_VERSION)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
