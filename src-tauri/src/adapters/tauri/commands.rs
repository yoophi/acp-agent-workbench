use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    adapters::{
        acp::runner::{AcpAgentRunner, launch_agent_run},
        agent_catalog::ConfigurableAgentCatalog,
        fs::LocalGoalFileReader,
        session_registry::AppState,
        tauri::event_sink::TauriRunEventSink,
    },
    application::{
        list_agents::ListAgentsUseCase, load_goal_file::LoadGoalFileUseCase,
        respond_permission::RespondPermissionUseCase, start_agent_run::StartAgentRunUseCase,
    },
    domain::{
        agent::AgentDescriptor,
        events::{LifecycleStatus, RunEvent},
        run::{AgentRun, AgentRunRequest},
    },
    ports::{event_sink::RunEventSink, session_registry::SessionRegistry},
};

#[tauri::command]
pub fn list_agents() -> Vec<AgentDescriptor> {
    ListAgentsUseCase::new(ConfigurableAgentCatalog::from_env()).execute()
}

#[tauri::command]
pub fn load_goal_file(path: String) -> Result<String, String> {
    LoadGoalFileUseCase::new(LocalGoalFileReader)
        .execute(&path)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn start_agent_run(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AgentRunRequest,
) -> Result<AgentRun, String> {
    let sink = TauriRunEventSink::new(app);
    let permissions = state.permissions();
    let registry = state.inner().clone();
    let runner = Arc::new(AcpAgentRunner::new(
        ConfigurableAgentCatalog::from_env(),
        permissions,
    ));

    StartAgentRunUseCase::new(registry)
        .execute(sink, request, move |request, run_id, sink_for_launch| {
            let runner = runner.clone();
            async move { launch_agent_run(runner, request, run_id, sink_for_launch).await }
        })
        .await
}

#[tauri::command]
pub async fn send_prompt_to_run(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return Err("prompt is empty".into());
    }
    let session = state
        .active_session(&run_id)
        .await
        .ok_or_else(|| "agent run is not active".to_string())?;
    let sink = TauriRunEventSink::new(app);
    let prompt_text = trimmed.to_string();
    tokio::spawn(async move {
        if let Err(err) = session.send_prompt(&sink, prompt_text).await {
            sink.emit(
                &session.run_id,
                RunEvent::Error {
                    message: err.to_string(),
                },
            );
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn cancel_agent_run(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), String> {
    let cancelled = state.cancel_run(&run_id).await;
    TauriRunEventSink::new(app).emit(
        &run_id,
        RunEvent::Lifecycle {
            status: LifecycleStatus::Cancelled,
            message: if cancelled {
                "run cancelled".into()
            } else {
                "run was already terminated".into()
            },
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn respond_agent_permission(
    state: State<'_, AppState>,
    permission_id: String,
    option_id: String,
) -> Result<(), String> {
    RespondPermissionUseCase::new(state.permissions())
        .execute(&permission_id, option_id)
        .await
        .map_err(|err| err.to_string())
}
