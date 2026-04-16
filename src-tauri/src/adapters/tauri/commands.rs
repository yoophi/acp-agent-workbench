use tauri::{AppHandle, State};

use crate::{
    adapters::{
        agent_catalog::ConfigurableAgentCatalog,
        fs::LocalGoalFileReader,
        tauri::{event_sink::TauriRunEventSink, session_state::AppState},
    },
    application::{
        list_agents::ListAgentsUseCase, load_goal_file::LoadGoalFileUseCase,
        respond_permission::RespondPermissionUseCase, run_agent::RunAgentUseCase,
    },
    domain::{
        agent::AgentDescriptor,
        events::{LifecycleStatus, RunEvent},
        run::{AgentRun, AgentRunRequest},
    },
    ports::event_sink::RunEventSink,
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
    let run = AgentRun::new(request.goal.clone(), request.agent_id.clone());
    let run_for_task = run.clone();
    let sink = TauriRunEventSink::new(app);
    let permissions = state.permissions();
    let state_handle = state.inner().clone();
    let state_for_task = state_handle.clone();

    state_handle
        .reserve_run_if_idle(run.id.clone())
        .await
        .map_err(|err| err.to_string())?;

    let handle = tokio::spawn(async move {
        let use_case = RunAgentUseCase::new(ConfigurableAgentCatalog::from_env(), permissions);
        if let Err(err) = use_case
            .execute_with_run_id(request, run_for_task.id.clone(), sink.clone())
            .await
        {
            sink.emit(
                &run_for_task.id,
                RunEvent::Error {
                    message: err.to_string(),
                },
            );
        }
        state_for_task.finish_run(&run_for_task.id).await;
    });
    state_handle
        .attach_run_handle(&run.id, handle)
        .await
        .map_err(|err| err.to_string())?;

    Ok(run)
}

#[tauri::command]
pub async fn cancel_agent_run(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), String> {
    let cancelled = state.cancel_run(&run_id).await;
    if cancelled {
        TauriRunEventSink::new(app).emit(
            &run_id,
            RunEvent::Lifecycle {
                status: LifecycleStatus::Cancelled,
                message: "run cancelled".into(),
            },
        );
    }
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
