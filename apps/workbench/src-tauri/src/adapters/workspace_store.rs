use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};
use tokio::sync::Mutex;

use crate::{
    adapters::git::GitRepository,
    domain::workspace::{
        CheckoutId, RegisteredWorkspace, Workspace, WorkspaceCheckout, WorkspaceId, timestamp,
    },
    ports::workspace_store::WorkspaceStore,
};

const SCHEMA_VERSION: u32 = 1;

#[derive(Clone)]
pub struct LocalWorkspaceStore {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceStoreFile {
    schema_version: u32,
    workspaces: Vec<Workspace>,
    checkouts: Vec<WorkspaceCheckout>,
}

impl LocalWorkspaceStore {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            path: app_data_dir.join("workspaces.json"),
            lock: shared_store_lock(),
        }
    }

    pub async fn register_from_path(&self, path: &str) -> Result<RegisteredWorkspace> {
        let repo = GitRepository::from_path(path)?;
        let mut workspace = Workspace::from_origin(repo.origin.clone());
        let checkout = WorkspaceCheckout::new(
            workspace.id.clone(),
            &repo.origin.canonical_url,
            repo.root,
            repo.branch,
            repo.head_sha,
            true,
        );

        let _guard = self.lock.lock().await;
        let mut file = self.read_file_unlocked()?;
        if let Some(existing) = file
            .workspaces
            .iter()
            .find(|entry| entry.id == workspace.id)
        {
            workspace.created_at = existing.created_at.clone();
            workspace.default_checkout_id = existing
                .default_checkout_id
                .clone()
                .or_else(|| Some(checkout.id.clone()));
        } else {
            workspace.default_checkout_id = Some(checkout.id.clone());
        }
        workspace.updated_at = timestamp();

        upsert_by_id(&mut file.workspaces, workspace.clone(), |entry| {
            entry.id.clone()
        });
        upsert_by_id(&mut file.checkouts, checkout.clone(), |entry| {
            entry.id.clone()
        });
        self.write_file_unlocked(&file)?;

        Ok(RegisteredWorkspace {
            workspace,
            checkout,
        })
    }

    async fn update_checkout_from_git(
        &self,
        checkout_id: &CheckoutId,
    ) -> Result<Option<WorkspaceCheckout>> {
        let _guard = self.lock.lock().await;
        let mut file = self.read_file_unlocked()?;
        let Some(index) = file
            .checkouts
            .iter()
            .position(|checkout| checkout.id == *checkout_id)
        else {
            return Ok(None);
        };
        let path = file.checkouts[index].path.to_string_lossy().to_string();
        let repo = GitRepository::from_path(&path)?;
        file.checkouts[index].branch = repo.branch;
        file.checkouts[index].head_sha = repo.head_sha;
        let checkout = file.checkouts[index].clone();
        self.write_file_unlocked(&file)?;
        Ok(Some(checkout))
    }

    fn read_file_unlocked(&self) -> Result<WorkspaceStoreFile> {
        if !self.path.exists() {
            return Ok(WorkspaceStoreFile {
                schema_version: SCHEMA_VERSION,
                ..WorkspaceStoreFile::default()
            });
        }
        let raw = fs::read_to_string(&self.path)?;
        let mut file: WorkspaceStoreFile = serde_json::from_str(&raw)?;
        if file.schema_version == 0 {
            file.schema_version = SCHEMA_VERSION;
        }
        Ok(file)
    }

    fn write_file_unlocked(&self, file: &WorkspaceStoreFile) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(file)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, raw)?;
        fs::rename(tmp, &self.path)?;
        Ok(())
    }
}

fn shared_store_lock() -> Arc<Mutex<()>> {
    static LOCK: OnceLock<Arc<Mutex<()>>> = OnceLock::new();
    LOCK.get_or_init(Arc::default).clone()
}

impl WorkspaceStore for LocalWorkspaceStore {
    async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let _guard = self.lock.lock().await;
        Ok(self.read_file_unlocked()?.workspaces)
    }

    async fn get_workspace(&self, id: &str) -> Result<Option<Workspace>> {
        let _guard = self.lock.lock().await;
        Ok(self
            .read_file_unlocked()?
            .workspaces
            .into_iter()
            .find(|workspace| workspace.id == id))
    }

    async fn list_checkouts(&self, workspace_id: &str) -> Result<Vec<WorkspaceCheckout>> {
        let _guard = self.lock.lock().await;
        Ok(self
            .read_file_unlocked()?
            .checkouts
            .into_iter()
            .filter(|checkout| checkout.workspace_id == workspace_id)
            .collect())
    }

    async fn get_checkout(&self, id: &str) -> Result<Option<WorkspaceCheckout>> {
        let _guard = self.lock.lock().await;
        Ok(self
            .read_file_unlocked()?
            .checkouts
            .into_iter()
            .find(|checkout| checkout.id == id))
    }

    async fn remove_workspace(&self, workspace_id: &WorkspaceId) -> Result<()> {
        let _guard = self.lock.lock().await;
        let mut file = self.read_file_unlocked()?;
        file.workspaces
            .retain(|workspace| workspace.id != *workspace_id);
        file.checkouts
            .retain(|checkout| checkout.workspace_id != *workspace_id);
        self.write_file_unlocked(&file)
    }

    async fn remove_checkout(&self, checkout_id: &CheckoutId) -> Result<()> {
        let _guard = self.lock.lock().await;
        let mut file = self.read_file_unlocked()?;
        file.checkouts
            .retain(|checkout| checkout.id != *checkout_id);
        self.write_file_unlocked(&file)
    }

    async fn save_checkout(&self, checkout: WorkspaceCheckout) -> Result<WorkspaceCheckout> {
        let _guard = self.lock.lock().await;
        let mut file = self.read_file_unlocked()?;
        upsert_by_id(&mut file.checkouts, checkout.clone(), |entry| {
            entry.id.clone()
        });
        self.write_file_unlocked(&file)?;
        Ok(checkout)
    }

    async fn refresh_checkout(
        &self,
        checkout_id: &CheckoutId,
    ) -> Result<Option<WorkspaceCheckout>> {
        self.update_checkout_from_git(checkout_id).await
    }
}

fn upsert_by_id<T, F>(items: &mut Vec<T>, item: T, id: F)
where
    F: Fn(&T) -> String,
{
    let item_id = id(&item);
    match items.iter().position(|entry| id(entry) == item_id) {
        Some(index) => items[index] = item,
        None => items.push(item),
    }
}

#[allow(dead_code)]
fn store_path(base: &Path) -> PathBuf {
    base.join("workspaces.json")
}
