use anyhow::{Result, anyhow};
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex as StdMutex},
};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::{
    adapters::{acp::runner::AcpSession, permission_broker::PermissionBroker},
    ports::session_registry::{ReserveRunError, SessionRegistry},
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
    run_owners: Arc<StdMutex<HashMap<String, String>>>,
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
            run_owners: Arc::default(),
            permissions: PermissionBroker::default(),
            max_concurrent_runs,
        }
    }

    pub fn permissions(&self) -> PermissionBroker {
        self.permissions.clone()
    }

    pub fn owner_of(&self, run_id: &str) -> Option<String> {
        self.run_owners
            .lock()
            .expect("run owner mutex poisoned")
            .get(run_id)
            .cloned()
    }

    #[allow(dead_code)]
    pub async fn transfer_run(&self, run_id: &str, new_owner: String) -> Result<()> {
        if !self.runs.lock().await.contains_key(run_id) {
            return Err(anyhow!("agent run not found: {run_id}"));
        }
        self.run_owners
            .lock()
            .expect("run owner mutex poisoned")
            .insert(run_id.to_string(), new_owner);
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn runs_owned_by(&self, owner: &str) -> Vec<String> {
        let mut runs: Vec<_> = self
            .run_owners
            .lock()
            .expect("run owner mutex poisoned")
            .iter()
            .filter_map(|(run_id, label)| (label == owner).then(|| run_id.clone()))
            .collect();
        runs.sort();
        runs
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

impl SessionRegistry for AppState {
    type Session = AcpSession;

    async fn reserve_run(
        &self,
        run_id: String,
        owner: Option<String>,
    ) -> Result<(), ReserveRunError> {
        let mut runs = self.runs.lock().await;
        if runs.contains_key(&run_id) {
            return Err(ReserveRunError::DuplicateRunId { run_id });
        }
        if let Some(limit) = self.max_concurrent_runs {
            if runs.len() >= limit {
                return Err(ReserveRunError::ConcurrentLimit { limit });
            }
        }
        runs.insert(run_id.clone(), RunSlot::Reserved);
        drop(runs);
        if let Some(owner) = owner
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            self.run_owners
                .lock()
                .expect("run owner mutex poisoned")
                .insert(run_id, owner);
        }
        Ok(())
    }

    async fn attach_run_handle(&self, run_id: &str, handle: JoinHandle<()>) -> Result<()> {
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

    async fn attach_session(&self, run_id: &str, session: Arc<AcpSession>) -> Result<()> {
        let mut runs = self.runs.lock().await;
        match runs.get_mut(run_id) {
            Some(RunSlot::Running(ctx)) => {
                ctx.session = Some(session);
                Ok(())
            }
            _ => Err(anyhow!(
                "agent run was cancelled before session was attached"
            )),
        }
    }

    async fn active_session(&self, run_id: &str) -> Option<Arc<AcpSession>> {
        let runs = self.runs.lock().await;
        match runs.get(run_id) {
            Some(RunSlot::Running(ctx)) => ctx.session.clone(),
            _ => None,
        }
    }

    async fn finish_run(&self, run_id: &str) {
        self.runs.lock().await.remove(run_id);
        self.run_owners
            .lock()
            .expect("run owner mutex poisoned")
            .remove(run_id);
        self.permissions.clear_run(run_id).await;
    }

    async fn cancel_run(&self, run_id: &str) -> bool {
        let cancelled = match self.runs.lock().await.remove(run_id) {
            Some(RunSlot::Running(ctx)) => {
                ctx.join_handle.abort();
                true
            }
            Some(RunSlot::Reserved) => true,
            None => false,
        };
        if cancelled {
            self.run_owners
                .lock()
                .expect("run owner mutex poisoned")
                .remove(run_id);
        }
        if cancelled {
            self.permissions.clear_run(run_id).await;
        }
        cancelled
    }
}

#[cfg(test)]
impl AppState {
    pub async fn active_run_count(&self) -> usize {
        self.runs.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use crate::ports::session_registry::{ReserveRunError, SessionRegistry};

    #[tokio::test]
    async fn reserve_run_allows_multiple_distinct_run_ids() {
        let state = AppState::default();
        state
            .reserve_run("run-a".into(), None)
            .await
            .expect("first run should reserve");
        state
            .reserve_run("run-b".into(), None)
            .await
            .expect("second concurrent run should reserve");
        assert_eq!(state.active_run_count().await, 2);
    }

    #[tokio::test]
    async fn reserve_run_rejects_duplicate_run_id() {
        let state = AppState::default();
        state.reserve_run("run-a".into(), None).await.unwrap();
        let err = state
            .reserve_run("run-a".into(), None)
            .await
            .expect_err("duplicate reservation must fail");
        assert_eq!(
            err,
            ReserveRunError::DuplicateRunId {
                run_id: "run-a".into()
            }
        );
    }

    #[tokio::test]
    async fn cancel_run_does_not_affect_other_runs() {
        let state = AppState::default();
        state.reserve_run("run-a".into(), None).await.unwrap();
        state.reserve_run("run-b".into(), None).await.unwrap();
        assert!(state.cancel_run("run-a").await);
        assert_eq!(state.active_run_count().await, 1);
        assert!(state.cancel_run("run-b").await);
        assert_eq!(state.active_run_count().await, 0);
    }

    #[tokio::test]
    async fn reserve_run_respects_injected_concurrent_limit() {
        let state = AppState::with_max_concurrent_runs(Some(1));
        state.reserve_run("run-a".into(), None).await.unwrap();
        let err = state
            .reserve_run("run-b".into(), None)
            .await
            .expect_err("second run should be rejected by the limit");
        assert_eq!(err, ReserveRunError::ConcurrentLimit { limit: 1 });
        assert!(state.cancel_run("run-a").await);
        state
            .reserve_run("run-b".into(), None)
            .await
            .expect("limit should free up after cancel");
    }

    #[tokio::test]
    async fn reserve_run_duplicate_id_is_rejected_before_limit_check() {
        let state = AppState::with_max_concurrent_runs(Some(2));
        state.reserve_run("run-a".into(), None).await.unwrap();
        let err = state
            .reserve_run("run-a".into(), None)
            .await
            .expect_err("duplicate id must fail");
        assert_eq!(
            err,
            ReserveRunError::DuplicateRunId {
                run_id: "run-a".into()
            }
        );
    }

    #[tokio::test]
    async fn tracks_and_transfers_run_owners() {
        let state = AppState::default();
        state
            .reserve_run("run-a".into(), Some("workbench-a".into()))
            .await
            .unwrap();

        assert_eq!(state.owner_of("run-a").as_deref(), Some("workbench-a"));
        assert_eq!(
            state.runs_owned_by("workbench-a").await,
            vec!["run-a".to_string()]
        );

        state
            .transfer_run("run-a", "workbench-b".into())
            .await
            .unwrap();
        assert_eq!(state.owner_of("run-a").as_deref(), Some("workbench-b"));
        assert!(state.runs_owned_by("workbench-a").await.is_empty());

        state.finish_run("run-a").await;
        assert_eq!(state.owner_of("run-a"), None);
    }
}
