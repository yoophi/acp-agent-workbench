use anyhow::Result;
use sqlx::{Row, SqlitePool};
use std::{future::Future, pin::Pin};

use crate::{
    domain::{
        acp_session::{AcpSessionLookup, AcpSessionRecord},
        workspace::timestamp,
    },
    ports::acp_session_store::AcpSessionStore,
};

#[derive(Clone)]
pub struct SqliteAcpSessionStore {
    pool: SqlitePool,
}

impl SqliteAcpSessionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl AcpSessionStore for SqliteAcpSessionStore {
    fn record_session<'a>(
        &'a self,
        mut record: AcpSessionRecord,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let now = timestamp();
            record.updated_at = now;
            upsert_session(&self.pool, &record).await
        })
    }

    fn latest_session<'a>(
        &'a self,
        lookup: AcpSessionLookup,
    ) -> Pin<Box<dyn Future<Output = Result<Option<AcpSessionRecord>>> + Send + 'a>> {
        Box::pin(async move {
            let row = sqlx::query(
                r#"
                SELECT run_id, session_id, workspace_id, checkout_id, workdir,
                       agent_id, agent_command, task, created_at, updated_at
                FROM acp_sessions
                WHERE agent_id = ?
                  AND workspace_id IS ?
                  AND checkout_id IS ?
                  AND workdir IS ?
                ORDER BY updated_at DESC
                LIMIT 1
                "#,
            )
            .bind(&lookup.agent_id)
            .bind(&lookup.workspace_id)
            .bind(&lookup.checkout_id)
            .bind(&lookup.workdir)
            .fetch_optional(&self.pool)
            .await?;
            row.map(session_from_row).transpose()
        })
    }
}

async fn upsert_session(pool: &SqlitePool, record: &AcpSessionRecord) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO acp_sessions (
            run_id, session_id, workspace_id, checkout_id, workdir,
            agent_id, agent_command, task, created_at, updated_at
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(run_id) DO UPDATE SET
            session_id = excluded.session_id,
            workspace_id = excluded.workspace_id,
            checkout_id = excluded.checkout_id,
            workdir = excluded.workdir,
            agent_id = excluded.agent_id,
            agent_command = excluded.agent_command,
            task = excluded.task,
            updated_at = excluded.updated_at
        "#,
    )
    .bind(&record.run_id)
    .bind(&record.session_id)
    .bind(&record.workspace_id)
    .bind(&record.checkout_id)
    .bind(&record.workdir)
    .bind(&record.agent_id)
    .bind(&record.agent_command)
    .bind(&record.task)
    .bind(&record.created_at)
    .bind(&record.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

fn session_from_row(row: sqlx::sqlite::SqliteRow) -> Result<AcpSessionRecord> {
    Ok(AcpSessionRecord {
        run_id: row.try_get("run_id")?,
        session_id: row.try_get("session_id")?,
        workspace_id: row.try_get("workspace_id")?,
        checkout_id: row.try_get("checkout_id")?,
        workdir: row.try_get("workdir")?,
        agent_id: row.try_get("agent_id")?,
        agent_command: row.try_get("agent_command")?,
        task: row.try_get("task")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::SqliteAcpSessionStore;
    use crate::{
        adapters::sqlite::open_database,
        domain::{
            acp_session::{AcpSessionLookup, AcpSessionRecord},
            workspace::timestamp,
        },
        ports::acp_session_store::AcpSessionStore,
    };

    async fn temp_store() -> SqliteAcpSessionStore {
        let dir = std::env::temp_dir().join(format!("acp-sessions-{}", uuid::Uuid::new_v4()));
        let pool = open_database(&dir).await.unwrap();
        SqliteAcpSessionStore::new(pool)
    }

    async fn insert_workspace_fixture(store: &SqliteAcpSessionStore) {
        sqlx::query(
            r#"
            INSERT INTO workspaces (
                id, name, raw_origin_url, canonical_origin_url, host, owner, repo, created_at, updated_at
            )
            VALUES ('ws-1', 'repo', 'git@github.com:owner/repo.git', 'github.com/owner/repo', 'github.com', 'owner', 'repo', '1', '1')
            "#,
        )
        .execute(&store.pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            INSERT INTO workspace_checkouts (
                id, workspace_id, path, kind, is_default, created_at, updated_at
            )
            VALUES ('co-1', 'ws-1', '/tmp/work', 'clone', 1, '1', '1')
            "#,
        )
        .execute(&store.pool)
        .await
        .unwrap();
    }

    fn record(run_id: &str, session_id: &str) -> AcpSessionRecord {
        let now = timestamp();
        AcpSessionRecord {
            run_id: run_id.into(),
            session_id: session_id.into(),
            workspace_id: Some("ws-1".into()),
            checkout_id: Some("co-1".into()),
            workdir: Some("/tmp/work".into()),
            agent_id: "agent".into(),
            agent_command: Some("agent --stdio".into()),
            task: "task".into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn records_and_replaces_session_for_run() {
        let store = temp_store().await;
        insert_workspace_fixture(&store).await;
        store
            .record_session(record("run-1", "session-1"))
            .await
            .unwrap();
        store
            .record_session(record("run-1", "session-2"))
            .await
            .unwrap();

        let found = store
            .latest_session(AcpSessionLookup {
                workspace_id: Some("ws-1".into()),
                checkout_id: Some("co-1".into()),
                workdir: Some("/tmp/work".into()),
                agent_id: "agent".into(),
            })
            .await
            .unwrap()
            .expect("session should exist");

        assert_eq!(found.run_id, "run-1");
        assert_eq!(found.session_id, "session-2");
    }
}
