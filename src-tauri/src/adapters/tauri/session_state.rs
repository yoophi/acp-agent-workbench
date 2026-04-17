use anyhow::{Result, anyhow};
use std::{collections::HashMap, env, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{
    adapters::acp::runner::AcpSession,
    ports::permission::{PermissionDecision, PermissionDecisionPort},
};

const MAX_RUNS_ENV: &str = "ACP_WORKBENCH_MAX_RUNS";

fn read_max_runs_from_env() -> Option<usize> {
    match env::var(MAX_RUNS_ENV) {
        Ok(raw) => raw.trim().parse::<usize>().ok().filter(|n| *n > 0),
        Err(_) => None,
    }
}

#[derive(Clone)]
pub struct AppState {
    runs: Arc<Mutex<HashMap<String, RunSlot>>>,
    permissions: PermissionBroker,
    max_concurrent_runs: Option<usize>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::with_max_concurrent_runs(read_max_runs_from_env())
    }
}

impl AppState {
    pub fn with_max_concurrent_runs(max_concurrent_runs: Option<usize>) -> Self {
        Self {
            runs: Arc::default(),
            permissions: PermissionBroker::default(),
            max_concurrent_runs,
        }
    }
}

enum RunSlot {
    Reserved,
    Running(RunContext),
}

struct RunContext {
    join_handle: JoinHandle<()>,
    session: Option<Arc<AcpSession>>,
}

impl AppState {
    pub fn permissions(&self) -> PermissionBroker {
        self.permissions.clone()
    }

    pub async fn reserve_run(&self, run_id: String) -> Result<()> {
        let mut runs = self.runs.lock().await;
        if runs.contains_key(&run_id) {
            return Err(anyhow!("duplicate run id: {run_id}"));
        }
        if let Some(limit) = self.max_concurrent_runs {
            if runs.len() >= limit {
                return Err(anyhow!(
                    "concurrent run limit ({limit}) reached; cancel an existing run before starting a new one"
                ));
            }
        }
        runs.insert(run_id, RunSlot::Reserved);
        Ok(())
    }

    pub async fn active_run_count(&self) -> usize {
        self.runs.lock().await.len()
    }

    pub async fn attach_run_handle(&self, run_id: &str, handle: JoinHandle<()>) -> Result<()> {
        let mut runs = self.runs.lock().await;
        let Some(slot) = runs.get_mut(run_id) else {
            handle.abort();
            return Err(anyhow!("agent run was cancelled before it started"));
        };
        *slot = RunSlot::Running(RunContext {
            join_handle: handle,
            session: None,
        });
        Ok(())
    }

    pub async fn attach_session(&self, run_id: &str, session: Arc<AcpSession>) -> Result<()> {
        let mut runs = self.runs.lock().await;
        match runs.get_mut(run_id) {
            Some(RunSlot::Running(ctx)) => {
                ctx.session = Some(session);
                Ok(())
            }
            _ => Err(anyhow!("agent run was cancelled before session was attached")),
        }
    }

    pub async fn active_session(&self, run_id: &str) -> Option<Arc<AcpSession>> {
        let runs = self.runs.lock().await;
        match runs.get(run_id) {
            Some(RunSlot::Running(ctx)) => ctx.session.clone(),
            _ => None,
        }
    }

    pub async fn finish_run(&self, run_id: &str) {
        self.runs.lock().await.remove(run_id);
        self.permissions.clear_run(run_id).await;
    }

    pub async fn cancel_run(&self, run_id: &str) -> bool {
        let cancelled = match self.runs.lock().await.remove(run_id) {
            Some(RunSlot::Running(ctx)) => {
                ctx.join_handle.abort();
                true
            }
            Some(RunSlot::Reserved) => true,
            None => false,
        };
        if cancelled {
            self.permissions.clear_run(run_id).await;
        }
        cancelled
    }
}

struct PendingPermission {
    run_id: String,
    sender: tokio::sync::oneshot::Sender<PermissionDecision>,
}

#[derive(Clone, Default)]
pub struct PermissionBroker {
    pending: Arc<Mutex<HashMap<String, PendingPermission>>>,
}

impl PermissionDecisionPort for PermissionBroker {
    async fn create_waiter(
        &self,
        run_id: String,
        permission_id: String,
    ) -> tokio::sync::oneshot::Receiver<PermissionDecision> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.pending
            .lock()
            .await
            .insert(permission_id, PendingPermission { run_id, sender });
        receiver
    }

    async fn respond(&self, permission_id: &str, decision: PermissionDecision) -> Result<()> {
        let Some(pending) = self.pending.lock().await.remove(permission_id) else {
            return Err(anyhow!(
                "unknown or already answered permission: {permission_id}"
            ));
        };
        pending
            .sender
            .send(decision)
            .map_err(|_| anyhow!("permission waiter is no longer active"))
    }
}

impl PermissionBroker {
    pub async fn clear_run(&self, run_id: &str) {
        self.pending
            .lock()
            .await
            .retain(|_, pending| pending.run_id != run_id);
    }
}

#[cfg(test)]
mod tests {
    use super::{AppState, PermissionBroker};
    use crate::ports::permission::{PermissionDecision, PermissionDecisionPort};

    #[tokio::test]
    async fn reserve_run_allows_multiple_distinct_run_ids() {
        let state = AppState::default();
        state
            .reserve_run("run-a".into())
            .await
            .expect("first run should reserve");
        state
            .reserve_run("run-b".into())
            .await
            .expect("second concurrent run should reserve");
        assert_eq!(state.active_run_count().await, 2);
    }

    #[tokio::test]
    async fn reserve_run_rejects_duplicate_run_id() {
        let state = AppState::default();
        state.reserve_run("run-a".into()).await.unwrap();
        let err = state
            .reserve_run("run-a".into())
            .await
            .expect_err("duplicate reservation must fail");
        assert!(err.to_string().contains("duplicate run id"));
    }

    #[tokio::test]
    async fn cancel_run_does_not_affect_other_runs() {
        let state = AppState::default();
        state.reserve_run("run-a".into()).await.unwrap();
        state.reserve_run("run-b".into()).await.unwrap();
        assert!(state.cancel_run("run-a").await);
        assert_eq!(state.active_run_count().await, 1);
        assert!(state.cancel_run("run-b").await);
        assert_eq!(state.active_run_count().await, 0);
    }

    #[tokio::test]
    async fn reserve_run_respects_injected_concurrent_limit() {
        let state = AppState::with_max_concurrent_runs(Some(1));
        state.reserve_run("run-a".into()).await.unwrap();
        let err = state
            .reserve_run("run-b".into())
            .await
            .expect_err("second run should be rejected by the limit");
        assert!(err.to_string().contains("concurrent run limit"));
        assert!(state.cancel_run("run-a").await);
        state
            .reserve_run("run-b".into())
            .await
            .expect("limit should free up after cancel");
    }

    #[tokio::test]
    async fn reserve_run_duplicate_id_is_rejected_before_limit_check() {
        let state = AppState::with_max_concurrent_runs(Some(2));
        state.reserve_run("run-a".into()).await.unwrap();
        let err = state
            .reserve_run("run-a".into())
            .await
            .expect_err("duplicate id must fail");
        assert!(err.to_string().contains("duplicate run id"));
    }

    #[tokio::test]
    async fn clearing_one_run_keeps_other_run_permission_waiters() {
        let broker = PermissionBroker::default();
        let first = broker
            .create_waiter("run-a".to_string(), "permission-a".to_string())
            .await;
        let second = broker
            .create_waiter("run-b".to_string(), "permission-b".to_string())
            .await;

        broker.clear_run("run-a").await;

        assert!(first.await.is_err());
        broker
            .respond(
                "permission-b",
                PermissionDecision {
                    option_id: "allow".to_string(),
                },
            )
            .await
            .expect("second run permission should remain active");
        assert_eq!(second.await.expect("decision").option_id, "allow");
    }
}
