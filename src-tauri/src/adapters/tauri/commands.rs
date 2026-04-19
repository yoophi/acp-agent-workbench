use tauri::{AppHandle, Manager, State};

use crate::{
    adapters::{
        acp::runner::AcpAgentRunner, agent_catalog::ConfigurableAgentCatalog,
        fs::LocalGoalFileReader, session_registry::AppState, tauri::event_sink::TauriRunEventSink,
        workspace_store::LocalWorkspaceStore,
    },
    application::{
        cancel_agent_run::CancelAgentRunUseCase, list_agents::ListAgentsUseCase,
        load_goal_file::LoadGoalFileUseCase, resolve_workdir::ResolveWorkdirUseCase,
        respond_permission::RespondPermissionUseCase, send_prompt::SendPromptUseCase,
        start_agent_run::StartAgentRunUseCase,
    },
    domain::{
        agent::AgentDescriptor,
        run::{AgentRun, AgentRunRequest},
        workspace::{RegisteredWorkspace, Workspace, WorkspaceCheckout},
    },
    ports::workspace_store::WorkspaceStore,
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
    mut request: AgentRunRequest,
) -> Result<AgentRun, String> {
    let workspace_store = workspace_store(&app)?;
    let resolved_cwd = ResolveWorkdirUseCase::new(workspace_store)
        .execute(
            request.workspace_id.as_deref(),
            request.checkout_id.as_deref(),
            request.cwd.as_deref(),
        )
        .await
        .map_err(|err| err.to_string())?;
    if let Some(cwd) = resolved_cwd {
        request.cwd = Some(cwd.to_string_lossy().to_string());
    }

    let sink = TauriRunEventSink::new(app);
    let permissions = state.permissions();
    let registry = state.inner().clone();
    let runner = AcpAgentRunner::new(ConfigurableAgentCatalog::from_env(), permissions);

    StartAgentRunUseCase::new(registry)
        .execute(runner, sink, request)
        .await
        .map_err(String::from)
}

#[tauri::command]
pub async fn send_prompt_to_run(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    let sink = TauriRunEventSink::new(app);
    let registry = state.inner().clone();
    SendPromptUseCase::new(registry)
        .execute(sink, run_id, prompt)
        .await
        .map_err(String::from)
}

#[tauri::command]
pub async fn cancel_agent_run(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), String> {
    let sink = TauriRunEventSink::new(app);
    let registry = state.inner().clone();
    CancelAgentRunUseCase::new(registry)
        .execute(sink, run_id)
        .await;
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

#[tauri::command]
pub async fn list_workspaces(app: AppHandle) -> Result<Vec<Workspace>, String> {
    workspace_store(&app)?
        .list_workspaces()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn register_workspace_from_path(
    app: AppHandle,
    path: String,
) -> Result<RegisteredWorkspace, String> {
    workspace_store(&app)?
        .register_from_path(&path)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn remove_workspace(app: AppHandle, workspace_id: String) -> Result<(), String> {
    workspace_store(&app)?
        .remove_workspace(&workspace_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_workspace_checkouts(
    app: AppHandle,
    workspace_id: String,
) -> Result<Vec<WorkspaceCheckout>, String> {
    workspace_store(&app)?
        .list_checkouts(&workspace_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn refresh_workspace_checkout(
    app: AppHandle,
    checkout_id: String,
) -> Result<Option<WorkspaceCheckout>, String> {
    workspace_store(&app)?
        .refresh_checkout(&checkout_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn resolve_workspace_workdir(
    app: AppHandle,
    workspace_id: Option<String>,
    checkout_id: Option<String>,
    cwd: Option<String>,
) -> Result<Option<String>, String> {
    ResolveWorkdirUseCase::new(workspace_store(&app)?)
        .execute(
            workspace_id.as_deref(),
            checkout_id.as_deref(),
            cwd.as_deref(),
        )
        .await
        .map(|path| path.map(|value| value.to_string_lossy().to_string()))
        .map_err(|err| err.to_string())
}

fn workspace_store(app: &AppHandle) -> Result<LocalWorkspaceStore, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
    Ok(LocalWorkspaceStore::new(app_data_dir))
}
