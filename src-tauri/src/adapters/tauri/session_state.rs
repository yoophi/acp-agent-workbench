use anyhow::{Result, anyhow};
use std::{collections::HashMap, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

use crate::ports::permission::{PermissionDecision, PermissionDecisionPort};

#[derive(Clone, Default)]
pub struct AppState {
    runs: Arc<Mutex<HashMap<String, RunSlot>>>,
    permissions: PermissionBroker,
}

enum RunSlot {
    Reserved,
    Running(JoinHandle<()>),
}

impl AppState {
    pub fn permissions(&self) -> PermissionBroker {
        self.permissions.clone()
    }

    pub async fn reserve_run_if_idle(&self, run_id: String) -> Result<()> {
        let mut runs = self.runs.lock().await;
        if !runs.is_empty() {
            return Err(anyhow!("another agent run is already in progress"));
        }
        runs.insert(run_id, RunSlot::Reserved);
        Ok(())
    }

    pub async fn attach_run_handle(&self, run_id: &str, handle: JoinHandle<()>) -> Result<()> {
        let mut runs = self.runs.lock().await;
        let Some(slot) = runs.get_mut(run_id) else {
            handle.abort();
            return Err(anyhow!("agent run was cancelled before it started"));
        };
        *slot = RunSlot::Running(handle);
        Ok(())
    }

    pub async fn finish_run(&self, run_id: &str) {
        self.runs.lock().await.remove(run_id);
        self.permissions.clear_all().await;
    }

    pub async fn cancel_run(&self, run_id: &str) -> bool {
        let cancelled = match self.runs.lock().await.remove(run_id) {
            Some(RunSlot::Running(handle)) => {
                handle.abort();
                true
            }
            Some(RunSlot::Reserved) => true,
            None => false,
        };
        if cancelled {
            self.permissions.clear_all().await;
        }
        cancelled
    }
}

#[derive(Clone, Default)]
pub struct PermissionBroker {
    pending: Arc<Mutex<HashMap<String, tokio::sync::oneshot::Sender<PermissionDecision>>>>,
}

impl PermissionDecisionPort for PermissionBroker {
    async fn create_waiter(
        &self,
        permission_id: String,
    ) -> tokio::sync::oneshot::Receiver<PermissionDecision> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.pending.lock().await.insert(permission_id, sender);
        receiver
    }

    async fn respond(&self, permission_id: &str, decision: PermissionDecision) -> Result<()> {
        let Some(sender) = self.pending.lock().await.remove(permission_id) else {
            return Err(anyhow!(
                "unknown or already answered permission: {permission_id}"
            ));
        };
        sender
            .send(decision)
            .map_err(|_| anyhow!("permission waiter is no longer active"))
    }
}

impl PermissionBroker {
    pub async fn clear_all(&self) {
        self.pending.lock().await.clear();
    }
}
