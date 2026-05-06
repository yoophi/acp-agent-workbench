use anyhow::Result;
use sqlx::SqlitePool;
use std::path::Path;

use crate::{
    adapters::{
        workspace_store::LocalWorkspaceStore, workspace_store_sqlite::SqliteWorkspaceStore,
    },
    ports::workspace_store::WorkspaceStore,
};

const LEGACY_WORKSPACES_FILE: &str = "workspaces.json";
const MIGRATED_WORKSPACES_FILE: &str = "workspaces.json.migrated";

pub async fn migrate_json_workspace_store(pool: &SqlitePool, app_data_dir: &Path) -> Result<()> {
    let legacy_path = app_data_dir.join(LEGACY_WORKSPACES_FILE);
    if !legacy_path.exists() {
        return Ok(());
    }

    let sqlite_store = SqliteWorkspaceStore::new(pool.clone());
    if !sqlite_store.list_workspaces().await?.is_empty() {
        return Ok(());
    }

    let json_store = LocalWorkspaceStore::new(app_data_dir.to_path_buf());
    let workspaces = json_store.list_workspaces().await?;
    if workspaces.is_empty() {
        mark_legacy_file_migrated(&legacy_path, app_data_dir).await?;
        return Ok(());
    }

    for workspace in workspaces {
        let checkouts = json_store.list_checkouts(&workspace.id).await?;
        sqlite_store
            .import_workspace_with_checkouts(workspace, checkouts)
            .await?;
    }
    mark_legacy_file_migrated(&legacy_path, app_data_dir).await
}

async fn mark_legacy_file_migrated(legacy_path: &Path, app_data_dir: &Path) -> Result<()> {
    let migrated_path = app_data_dir.join(MIGRATED_WORKSPACES_FILE);
    if migrated_path.exists() {
        tokio::fs::remove_file(&migrated_path).await?;
    }
    tokio::fs::rename(legacy_path, migrated_path).await?;
    Ok(())
}
