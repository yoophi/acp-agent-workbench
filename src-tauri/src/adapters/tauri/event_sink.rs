use tauri::{AppHandle, Emitter};

use crate::{
    adapters::session_registry::AppState,
    domain::events::{RunEvent, RunEventEnvelope},
    ports::event_sink::RunEventSink,
};

pub const AGENT_RUN_EVENT: &str = "agent-run-event";

#[derive(Clone)]
pub struct TauriRunEventSink {
    app: AppHandle,
    state: AppState,
    fallback_owner: Option<String>,
}

impl TauriRunEventSink {
    pub fn new(app: AppHandle, state: AppState) -> Self {
        Self {
            app,
            state,
            fallback_owner: None,
        }
    }

    pub fn with_fallback_owner(app: AppHandle, state: AppState, owner: Option<String>) -> Self {
        Self {
            app,
            state,
            fallback_owner: owner,
        }
    }
}

impl RunEventSink for TauriRunEventSink {
    fn emit(&self, run_id: &str, event: RunEvent) {
        let envelope = RunEventEnvelope {
            run_id: run_id.to_string(),
            event,
        };
        if let Some(owner) = self
            .state
            .owner_of(run_id)
            .or_else(|| self.fallback_owner.clone())
        {
            if self.app.emit_to(&owner, AGENT_RUN_EVENT, &envelope).is_ok() {
                return;
            }
        }
        let _ = self.app.emit(AGENT_RUN_EVENT, envelope);
    }
}
