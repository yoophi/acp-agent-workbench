use anyhow::{Result, anyhow, bail};
use std::{
    env,
    path::{Path, PathBuf},
};

use crate::ports::workspace_store::WorkspaceStore;

#[derive(Clone)]
pub struct ResolveWorkdirUseCase<S>
where
    S: WorkspaceStore,
{
    store: S,
}

impl<S> ResolveWorkdirUseCase<S>
where
    S: WorkspaceStore,
{
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub async fn execute(
        self,
        workspace_id: Option<&str>,
        checkout_id: Option<&str>,
        cwd: Option<&str>,
    ) -> Result<Option<PathBuf>> {
        let Some(workspace_id) = workspace_id.filter(|value| !value.trim().is_empty()) else {
            return Ok(cwd.map(|value| expand_tilde(value)));
        };
        let workspace = self
            .store
            .get_workspace(workspace_id)
            .await?
            .ok_or_else(|| anyhow!("workspace not found: {workspace_id}"))?;

        let checkout_id = checkout_id
            .filter(|value| !value.trim().is_empty())
            .or(workspace.default_checkout_id.as_deref())
            .ok_or_else(|| anyhow!("workspace has no checkout: {workspace_id}"))?;

        let checkout = self
            .store
            .get_checkout(checkout_id)
            .await?
            .ok_or_else(|| anyhow!("checkout not found: {checkout_id}"))?;

        if checkout.workspace_id != workspace.id {
            bail!("checkout {checkout_id} does not belong to workspace {workspace_id}");
        }

        let root = checkout.path.canonicalize()?;
        let requested = cwd
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(expand_tilde)
            .unwrap_or_else(|| root.clone());
        let target = if requested.is_absolute() {
            requested
        } else {
            root.join(requested)
        };
        if !target.exists() {
            bail!("working directory does not exist: {}", target.display());
        }
        let resolved = normalize_path(Path::new(&target))?;
        if resolved != root && !resolved.starts_with(&root) {
            bail!(
                "working directory must be inside checkout {}: {}",
                root.display(),
                resolved.display()
            );
        }
        Ok(Some(resolved))
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

fn normalize_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else if let Some(parent) = path.parent() {
        let parent = if parent.as_os_str().is_empty() {
            env::current_dir()?
        } else if parent.exists() {
            parent.canonicalize()?
        } else {
            normalize_path(parent)?
        };
        Ok(parent.join(path.file_name().unwrap_or_default()))
    } else {
        Ok(env::current_dir()?.join(path))
    }
}
