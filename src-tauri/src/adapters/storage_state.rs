use anyhow::Result;
use sqlx::SqlitePool;
use std::path::PathBuf;

use crate::adapters::sqlite::open_database;

#[derive(Clone)]
#[allow(dead_code)]
pub struct StorageState {
    pool: SqlitePool,
    app_data_dir: PathBuf,
}

impl StorageState {
    pub async fn open(app_data_dir: PathBuf) -> Result<Self> {
        let pool = open_database(&app_data_dir).await?;
        Ok(Self { pool, app_data_dir })
    }

    #[allow(dead_code)]
    pub fn pool(&self) -> SqlitePool {
        self.pool.clone()
    }

    #[allow(dead_code)]
    pub fn app_data_dir(&self) -> PathBuf {
        self.app_data_dir.clone()
    }
}
