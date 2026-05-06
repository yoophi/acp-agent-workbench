use anyhow::Result;
use sqlx::SqlitePool;
use std::path::PathBuf;

use crate::adapters::{
    acp_session_store_sqlite::SqliteAcpSessionStore,
    pull_request_review_store_sqlite::SqlitePullRequestReviewDraftStore,
    saved_prompt_store_sqlite::SqliteSavedPromptStore, sqlite::open_database,
    workspace_store_migration::migrate_json_workspace_store,
    workspace_store_sqlite::SqliteWorkspaceStore,
};

#[derive(Clone)]
pub struct StorageState {
    pool: SqlitePool,
    #[allow(dead_code)]
    app_data_dir: PathBuf,
}

impl StorageState {
    pub async fn open(app_data_dir: PathBuf) -> Result<Self> {
        let pool = open_database(&app_data_dir).await?;
        migrate_json_workspace_store(&pool, &app_data_dir).await?;
        Ok(Self { pool, app_data_dir })
    }

    pub fn pool(&self) -> SqlitePool {
        self.pool.clone()
    }

    #[allow(dead_code)]
    pub fn app_data_dir(&self) -> PathBuf {
        self.app_data_dir.clone()
    }

    pub fn workspace_store(&self) -> SqliteWorkspaceStore {
        SqliteWorkspaceStore::new(self.pool())
    }

    pub fn saved_prompt_store(&self) -> SqliteSavedPromptStore {
        SqliteSavedPromptStore::new(self.pool())
    }

    pub fn acp_session_store(&self) -> SqliteAcpSessionStore {
        SqliteAcpSessionStore::new(self.pool())
    }

    pub fn pull_request_review_draft_store(&self) -> SqlitePullRequestReviewDraftStore {
        SqlitePullRequestReviewDraftStore::new(self.pool())
    }
}
