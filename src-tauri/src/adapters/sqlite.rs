use anyhow::Result;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{path::Path, time::Duration};

const WORKSPACE_SCHEMA_VERSION: i64 = 1;

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

    let applied: Option<(i64,)> =
        sqlx::query_as("SELECT version FROM schema_migrations WHERE version = ?")
            .bind(WORKSPACE_SCHEMA_VERSION)
            .fetch_optional(pool)
            .await?;
    if applied.is_some() {
        return Ok(());
    }

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
