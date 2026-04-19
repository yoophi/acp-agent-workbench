use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::{
    adapters::{
        acp::runner::AcpAgentRunner, acp_session_store_sqlite::SqliteAcpSessionStore,
        agent_catalog::ConfigurableAgentCatalog, fs::LocalGoalFileReader,
        git::GitWorkspaceInspector, session_registry::AppState, storage_state::StorageState,
        tauri::event_sink::TauriRunEventSink,
    },
    application::{
        cancel_agent_run::CancelAgentRunUseCase, list_agents::ListAgentsUseCase,
        load_goal_file::LoadGoalFileUseCase, resolve_workdir::ResolveWorkdirUseCase,
        respond_permission::RespondPermissionUseCase, send_prompt::SendPromptUseCase,
        start_agent_run::StartAgentRunUseCase, workspace_git::WorkspaceGitUseCase,
    },
    domain::{
        acp_session::AcpSessionLookup,
        agent::AgentDescriptor,
        run::{AgentRun, AgentRunRequest, ResumePolicy},
        saved_prompt::{
            CreateSavedPromptInput, SavedPrompt, SavedPromptId, UpdateSavedPromptPatch,
        },
        workspace::{RegisteredWorkspace, Workspace, WorkspaceCheckout},
        workspace_git::{WorkspaceDiffSummary, WorkspaceGitStatus},
    },
    ports::{
        acp_session_store::AcpSessionStore, saved_prompt_store::SavedPromptStore,
        workspace_store::WorkspaceStore,
    },
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
    storage: State<'_, StorageState>,
    mut request: AgentRunRequest,
) -> Result<AgentRun, String> {
    let workspace_store = storage.workspace_store();
    let resolved_cwd = ResolveWorkdirUseCase::new(workspace_store.clone())
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
    if request.checkout_id.as_deref().is_none_or(str::is_empty) {
        if let Some(workspace_id) = request.workspace_id.as_deref() {
            let workspace = workspace_store
                .get_workspace(workspace_id)
                .await
                .map_err(|err| err.to_string())?;
            request.checkout_id = workspace.and_then(|value| value.default_checkout_id);
        }
    }

    let session_store = storage.acp_session_store();
    hydrate_resume_session(&mut request, &session_store)
        .await
        .map_err(|err| err.to_string())?;

    let sink = TauriRunEventSink::new(app);
    let permissions = state.permissions();
    let registry = state.inner().clone();
    let runner = AcpAgentRunner::new(
        ConfigurableAgentCatalog::from_env(),
        permissions,
        Arc::new(session_store),
    );

    StartAgentRunUseCase::new(registry)
        .execute(runner, sink, request)
        .await
        .map_err(String::from)
}

async fn hydrate_resume_session(
    request: &mut AgentRunRequest,
    session_store: &SqliteAcpSessionStore,
) -> anyhow::Result<()> {
    let resume_policy = request.resume_policy.unwrap_or_default();
    if resume_policy == ResumePolicy::Fresh || has_resume_session_id(request) {
        return Ok(());
    }

    let latest = session_store
        .latest_session(AcpSessionLookup::from_request(request))
        .await?;
    if let Some(record) = latest {
        request.resume_session_id = Some(record.session_id);
        return Ok(());
    }

    if resume_policy == ResumePolicy::ResumeRequired {
        anyhow::bail!("resume session not found for requested workspace context");
    }
    Ok(())
}

fn has_resume_session_id(request: &AgentRunRequest) -> bool {
    request
        .resume_session_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
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
pub async fn list_workspaces(storage: State<'_, StorageState>) -> Result<Vec<Workspace>, String> {
    storage
        .workspace_store()
        .list_workspaces()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn register_workspace_from_path(
    storage: State<'_, StorageState>,
    path: String,
) -> Result<RegisteredWorkspace, String> {
    storage
        .workspace_store()
        .register_from_path(&path)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn remove_workspace(
    storage: State<'_, StorageState>,
    workspace_id: String,
) -> Result<(), String> {
    storage
        .workspace_store()
        .remove_workspace(&workspace_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_workspace_checkouts(
    storage: State<'_, StorageState>,
    workspace_id: String,
) -> Result<Vec<WorkspaceCheckout>, String> {
    storage
        .workspace_store()
        .list_checkouts(&workspace_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn refresh_workspace_checkout(
    storage: State<'_, StorageState>,
    checkout_id: String,
) -> Result<Option<WorkspaceCheckout>, String> {
    storage
        .workspace_store()
        .refresh_checkout(&checkout_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn resolve_workspace_workdir(
    storage: State<'_, StorageState>,
    workspace_id: Option<String>,
    checkout_id: Option<String>,
    cwd: Option<String>,
) -> Result<Option<String>, String> {
    ResolveWorkdirUseCase::new(storage.workspace_store())
        .execute(
            workspace_id.as_deref(),
            checkout_id.as_deref(),
            cwd.as_deref(),
        )
        .await
        .map(|path| path.map(|value| value.to_string_lossy().to_string()))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_workspace_git_status(
    storage: State<'_, StorageState>,
    workspace_id: String,
    checkout_id: Option<String>,
) -> Result<WorkspaceGitStatus, String> {
    WorkspaceGitUseCase::new(storage.workspace_store(), GitWorkspaceInspector)
        .status(&workspace_id, checkout_id.as_deref())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn summarize_workspace_diff(
    storage: State<'_, StorageState>,
    workspace_id: String,
    checkout_id: Option<String>,
) -> Result<WorkspaceDiffSummary, String> {
    WorkspaceGitUseCase::new(storage.workspace_store(), GitWorkspaceInspector)
        .diff_summary(&workspace_id, checkout_id.as_deref())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_saved_prompts(
    storage: State<'_, StorageState>,
    workspace_id: Option<String>,
) -> Result<Vec<SavedPrompt>, String> {
    storage
        .saved_prompt_store()
        .list_saved_prompts(workspace_id.as_deref())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn create_saved_prompt(
    storage: State<'_, StorageState>,
    input: CreateSavedPromptInput,
) -> Result<SavedPrompt, String> {
    storage
        .saved_prompt_store()
        .create_saved_prompt(input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn update_saved_prompt(
    storage: State<'_, StorageState>,
    id: SavedPromptId,
    patch: UpdateSavedPromptPatch,
) -> Result<Option<SavedPrompt>, String> {
    storage
        .saved_prompt_store()
        .update_saved_prompt(&id, patch)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn delete_saved_prompt(
    storage: State<'_, StorageState>,
    id: SavedPromptId,
) -> Result<(), String> {
    storage
        .saved_prompt_store()
        .delete_saved_prompt(&id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn record_saved_prompt_used(
    storage: State<'_, StorageState>,
    id: SavedPromptId,
) -> Result<Option<SavedPrompt>, String> {
    storage
        .saved_prompt_store()
        .record_saved_prompt_used(&id)
        .await
        .map_err(|err| err.to_string())
}
