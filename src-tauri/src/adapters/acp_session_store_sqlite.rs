use anyhow::Result;
use sqlx::{Row, SqlitePool};
use std::{future::Future, pin::Pin};

use crate::{
    domain::{
        acp_session::{AcpSessionListQuery, AcpSessionLookup, AcpSessionRecord},
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
                  AND agent_command IS ?
                ORDER BY updated_at DESC
                LIMIT 1
                "#,
            )
            .bind(&lookup.agent_id)
            .bind(&lookup.workspace_id)
            .bind(&lookup.checkout_id)
            .bind(&lookup.workdir)
            .bind(&lookup.agent_command)
            .fetch_optional(&self.pool)
            .await?;
            row.map(session_from_row).transpose()
        })
    }

    fn list_sessions<'a>(
        &'a self,
        query: AcpSessionListQuery,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<AcpSessionRecord>>> + Send + 'a>> {
        Box::pin(async move { list_sessions(&self.pool, query).await })
    }

    fn clear_session<'a>(
        &'a self,
        run_id: String,
    ) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + 'a>> {
        Box::pin(async move {
            let result = sqlx::query("DELETE FROM acp_sessions WHERE run_id = ?")
                .bind(run_id)
                .execute(&self.pool)
                .await?;
            Ok(result.rows_affected() > 0)
        })
    }
}

async fn list_sessions(
    pool: &SqlitePool,
    query: AcpSessionListQuery,
) -> Result<Vec<AcpSessionRecord>> {
    let limit = i64::from(query.limit.unwrap_or(20).clamp(1, 100));
    let rows = sqlx::query(
        r#"
        SELECT run_id, session_id, workspace_id, checkout_id, workdir,
               agent_id, agent_command, task, created_at, updated_at
        FROM acp_sessions
        WHERE (? IS NULL OR workspace_id = ?)
          AND (? IS NULL OR checkout_id = ?)
          AND (? IS NULL OR workdir = ?)
          AND (? IS NULL OR agent_id = ?)
          AND (? IS NULL OR agent_command = ?)
        ORDER BY updated_at DESC, run_id DESC
        LIMIT ?
        "#,
    )
    .bind(&query.workspace_id)
    .bind(&query.workspace_id)
    .bind(&query.checkout_id)
    .bind(&query.checkout_id)
    .bind(&query.workdir)
    .bind(&query.workdir)
    .bind(&query.agent_id)
    .bind(&query.agent_id)
    .bind(&query.agent_command)
    .bind(&query.agent_command)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(session_from_row).collect()
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
            acp_session::{AcpSessionListQuery, AcpSessionLookup, AcpSessionRecord},
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
                agent_command: Some("agent --stdio".into()),
            })
            .await
            .unwrap()
            .expect("session should exist");

        assert_eq!(found.run_id, "run-1");
        assert_eq!(found.session_id, "session-2");
    }

    #[tokio::test]
    async fn latest_session_requires_matching_agent_command() {
        let store = temp_store().await;
        insert_workspace_fixture(&store).await;
        store
            .record_session(record("run-1", "session-default"))
            .await
            .unwrap();

        let found = store
            .latest_session(AcpSessionLookup {
                workspace_id: Some("ws-1".into()),
                checkout_id: Some("co-1".into()),
                workdir: Some("/tmp/work".into()),
                agent_id: "agent".into(),
                agent_command: Some("custom-agent --stdio".into()),
            })
            .await
            .unwrap();

        assert_eq!(found, None);
    }

    #[tokio::test]
    async fn lists_sessions_by_context_newest_first() {
        let store = temp_store().await;
        insert_workspace_fixture(&store).await;
        store
            .record_session(record("run-1", "session-1"))
            .await
            .unwrap();
        store
            .record_session(record("run-2", "session-2"))
            .await
            .unwrap();

        let sessions = store
            .list_sessions(AcpSessionListQuery {
                workspace_id: Some("ws-1".into()),
                checkout_id: Some("co-1".into()),
                agent_id: Some("agent".into()),
                limit: Some(10),
                ..AcpSessionListQuery::default()
            })
            .await
            .unwrap();

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].run_id, "run-2");
        assert_eq!(sessions[1].run_id, "run-1");
    }

    #[tokio::test]
    async fn clears_session_by_run_id() {
        let store = temp_store().await;
        insert_workspace_fixture(&store).await;
        store
            .record_session(record("run-1", "session-1"))
            .await
            .unwrap();

        assert!(store.clear_session("run-1".into()).await.unwrap());
        assert!(!store.clear_session("run-1".into()).await.unwrap());

        let sessions = store
            .list_sessions(AcpSessionListQuery {
                workspace_id: Some("ws-1".into()),
                ..AcpSessionListQuery::default()
            })
            .await
            .unwrap();
        assert!(sessions.is_empty());
    }
}
