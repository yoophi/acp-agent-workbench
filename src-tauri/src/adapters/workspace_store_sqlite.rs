use anyhow::{Result, anyhow};
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;

use crate::{
    adapters::git::GitRepository,
    domain::workspace::{
        CheckoutId, CheckoutKind, GitOrigin, RegisteredWorkspace, Workspace, WorkspaceCheckout,
        WorkspaceId, timestamp,
    },
    ports::workspace_store::WorkspaceStore,
};

#[derive(Clone)]
pub struct SqliteWorkspaceStore {
    pool: SqlitePool,
}

impl SqliteWorkspaceStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
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

        if let Some(existing) = self.get_workspace(&workspace.id).await? {
            workspace.created_at = existing.created_at;
            workspace.default_checkout_id = existing
                .default_checkout_id
                .or_else(|| Some(checkout.id.clone()));
        } else {
            workspace.default_checkout_id = Some(checkout.id.clone());
        }
        workspace.updated_at = timestamp();

        let mut tx = self.pool.begin().await?;
        upsert_workspace(&mut tx, &workspace).await?;
        upsert_checkout(&mut tx, &checkout).await?;
        tx.commit().await?;

        Ok(RegisteredWorkspace {
            workspace,
            checkout,
        })
    }

    pub async fn import_workspace_with_checkouts(
        &self,
        workspace: Workspace,
        checkouts: Vec<WorkspaceCheckout>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        upsert_workspace(&mut tx, &workspace).await?;
        for checkout in checkouts {
            upsert_checkout(&mut tx, &checkout).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

impl WorkspaceStore for SqliteWorkspaceStore {
    async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, raw_origin_url, canonical_origin_url, host, owner, repo,
                   default_checkout_id, created_at, updated_at
            FROM workspaces
            ORDER BY updated_at DESC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(workspace_from_row).collect()
    }

    async fn get_workspace(&self, id: &str) -> Result<Option<Workspace>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, raw_origin_url, canonical_origin_url, host, owner, repo,
                   default_checkout_id, created_at, updated_at
            FROM workspaces
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(workspace_from_row).transpose()
    }

    async fn list_checkouts(&self, workspace_id: &str) -> Result<Vec<WorkspaceCheckout>> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, path, kind, branch, head_sha, is_default
            FROM workspace_checkouts
            WHERE workspace_id = ?
            ORDER BY is_default DESC, path ASC
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(checkout_from_row).collect()
    }

    async fn get_checkout(&self, id: &str) -> Result<Option<WorkspaceCheckout>> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, path, kind, branch, head_sha, is_default
            FROM workspace_checkouts
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(checkout_from_row).transpose()
    }

    async fn remove_workspace(&self, workspace_id: &WorkspaceId) -> Result<()> {
        sqlx::query("DELETE FROM workspaces WHERE id = ?")
            .bind(workspace_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_checkout(&self, checkout_id: &CheckoutId) -> Result<()> {
        sqlx::query("DELETE FROM workspace_checkouts WHERE id = ?")
            .bind(checkout_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn save_checkout(&self, checkout: WorkspaceCheckout) -> Result<WorkspaceCheckout> {
        let mut tx = self.pool.begin().await?;
        upsert_checkout(&mut tx, &checkout).await?;
        tx.commit().await?;
        Ok(checkout)
    }

    async fn refresh_checkout(
        &self,
        checkout_id: &CheckoutId,
    ) -> Result<Option<WorkspaceCheckout>> {
        let Some(mut checkout) = self.get_checkout(checkout_id).await? else {
            return Ok(None);
        };
        let repo = GitRepository::from_path(&checkout.path.to_string_lossy())?;
        checkout.branch = repo.branch;
        checkout.head_sha = repo.head_sha;
        let mut tx = self.pool.begin().await?;
        upsert_checkout(&mut tx, &checkout).await?;
        tx.commit().await?;
        Ok(Some(checkout))
    }
}

async fn upsert_workspace(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    workspace: &Workspace,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO workspaces (
            id, name, raw_origin_url, canonical_origin_url, host, owner, repo,
            default_checkout_id, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            raw_origin_url = excluded.raw_origin_url,
            canonical_origin_url = excluded.canonical_origin_url,
            host = excluded.host,
            owner = excluded.owner,
            repo = excluded.repo,
            default_checkout_id = excluded.default_checkout_id,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&workspace.id)
    .bind(&workspace.name)
    .bind(&workspace.origin.raw_url)
    .bind(&workspace.origin.canonical_url)
    .bind(&workspace.origin.host)
    .bind(&workspace.origin.owner)
    .bind(&workspace.origin.repo)
    .bind(&workspace.default_checkout_id)
    .bind(&workspace.created_at)
    .bind(&workspace.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_checkout(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    checkout: &WorkspaceCheckout,
) -> Result<()> {
    let now = timestamp();
    sqlx::query(
        r#"
        INSERT INTO workspace_checkouts (
            id, workspace_id, path, kind, branch, head_sha, is_default, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            workspace_id = excluded.workspace_id,
            path = excluded.path,
            kind = excluded.kind,
            branch = excluded.branch,
            head_sha = excluded.head_sha,
            is_default = excluded.is_default,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&checkout.id)
    .bind(&checkout.workspace_id)
    .bind(checkout.path.to_string_lossy().to_string())
    .bind(checkout_kind_to_db(&checkout.kind))
    .bind(&checkout.branch)
    .bind(&checkout.head_sha)
    .bind(checkout.is_default)
    .bind(&now)
    .bind(&now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn workspace_from_row(row: sqlx::sqlite::SqliteRow) -> Result<Workspace> {
    Ok(Workspace {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        origin: GitOrigin {
            raw_url: row.try_get("raw_origin_url")?,
            canonical_url: row.try_get("canonical_origin_url")?,
            host: row.try_get("host")?,
            owner: row.try_get("owner")?,
            repo: row.try_get("repo")?,
        },
        default_checkout_id: row.try_get("default_checkout_id")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn checkout_from_row(row: sqlx::sqlite::SqliteRow) -> Result<WorkspaceCheckout> {
    let kind: String = row.try_get("kind")?;
    let path: String = row.try_get("path")?;
    Ok(WorkspaceCheckout {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        path: PathBuf::from(path),
        kind: checkout_kind_from_db(&kind)?,
        branch: row.try_get("branch")?,
        head_sha: row.try_get("head_sha")?,
        is_default: row.try_get("is_default")?,
    })
}

fn checkout_kind_to_db(kind: &CheckoutKind) -> &'static str {
    match kind {
        CheckoutKind::Clone => "clone",
        CheckoutKind::Worktree => "worktree",
    }
}

fn checkout_kind_from_db(value: &str) -> Result<CheckoutKind> {
    match value {
        "clone" => Ok(CheckoutKind::Clone),
        "worktree" => Ok(CheckoutKind::Worktree),
        other => Err(anyhow!("unknown checkout kind: {other}")),
    }
}
