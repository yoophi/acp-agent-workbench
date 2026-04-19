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
}

impl TauriRunEventSink {
    pub fn new(app: AppHandle, state: AppState) -> Self {
        Self { app, state }
    }
}

impl RunEventSink for TauriRunEventSink {
    fn emit(&self, run_id: &str, event: RunEvent) {
        let envelope = RunEventEnvelope {
            run_id: run_id.to_string(),
            event,
        };
        let app = self.app.clone();
        let state = self.state.clone();
        let run_id = run_id.to_string();
        tauri::async_runtime::spawn(async move {
            if let Some(owner) = state.owner_of(&run_id).await {
                if app.emit_to(&owner, AGENT_RUN_EVENT, &envelope).is_ok() {
                    return;
                }
            }
            let _ = app.emit(AGENT_RUN_EVENT, envelope);
        });
    }
}
