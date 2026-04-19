use std::sync::Arc;

use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use uuid::Uuid;

use crate::{
    adapters::{
        acp::runner::AcpAgentRunner, acp_session_store_sqlite::SqliteAcpSessionStore,
        agent_catalog::ConfigurableAgentCatalog, fs::LocalGoalFileReader, git::LocalGitRepository,
        github::GhCliPullRequestClient, session_registry::AppState, storage_state::StorageState,
        tauri::event_sink::TauriRunEventSink,
    },
    application::{
        cancel_agent_run::CancelAgentRunUseCase, list_agents::ListAgentsUseCase,
        load_goal_file::LoadGoalFileUseCase, resolve_workdir::ResolveWorkdirUseCase,
        respond_permission::RespondPermissionUseCase, send_prompt::SendPromptUseCase,
        start_agent_run::StartAgentRunUseCase, workspace_git::WorkspaceGitUseCase,
        workspace_worktree::WorkspaceTaskWorktreeUseCase,
    },
    domain::{
        acp_session::{
            AcpSessionListQuery, AcpSessionLookup, AcpSessionRecord, normalize_agent_command,
        },
        agent::AgentDescriptor,
        git::{
            GitHubPullRequestContext, GitHubPullRequestContextRequest,
            GitHubPullRequestCreateRequest, GitHubPullRequestReviewRequest,
            GitHubPullRequestReviewResult, GitHubPullRequestSummary, WorkspaceCommitRequest,
            WorkspaceCommitResult, WorkspaceDiffSummary, WorkspaceGitStatus, WorkspacePushRequest,
            WorkspacePushResult,
        },
        pull_request_review::{
            CreatePullRequestReviewDraftInput, PullRequestReviewDraft, PullRequestReviewDraftId,
            UpdatePullRequestReviewDraftPatch,
        },
        run::{AgentRun, AgentRunRequest, ResumePolicy},
        saved_prompt::{
            CreateSavedPromptInput, SavedPrompt, SavedPromptId, UpdateSavedPromptPatch,
        },
        workbench_window::{WorkbenchWindowBootstrap, WorkbenchWindowInfo},
        workspace::{RegisteredWorkspace, Workspace, WorkspaceCheckout},
    },
    ports::{
        acp_session_store::AcpSessionStore, agent_catalog::AgentCatalog,
        pull_request_review_store::PullRequestReviewDraftStore,
        saved_prompt_store::SavedPromptStore, workspace_store::WorkspaceStore,
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
pub fn get_window_bootstrap(window: WebviewWindow) -> WorkbenchWindowBootstrap {
    WorkbenchWindowBootstrap::new(window.label())
}

#[tauri::command]
pub fn list_workbench_windows(app: AppHandle) -> Vec<WorkbenchWindowInfo> {
    let mut windows: Vec<_> = app
        .webview_windows()
        .into_values()
        .map(|window| {
            let title = window
                .title()
                .unwrap_or_else(|_| window.label().to_string());
            WorkbenchWindowInfo::new(window.label(), title)
        })
        .collect();
    windows.sort_by(|a, b| a.label.cmp(&b.label));
    windows
}

#[tauri::command]
pub fn open_workbench_window(app: AppHandle) -> Result<WorkbenchWindowInfo, String> {
    let label = next_workbench_window_label(&app);
    let title = "ACP Agent Workbench".to_string();

    let window =
        WebviewWindowBuilder::new(&app, label.clone(), WebviewUrl::App("index.html".into()))
            .title(&title)
            .build()
            .map_err(|err| err.to_string())?;
    window.set_focus().map_err(|err| err.to_string())?;

    Ok(WorkbenchWindowInfo::new(label, title))
}

fn next_workbench_window_label(app: &AppHandle) -> String {
    loop {
        let suffix = Uuid::new_v4().simple().to_string();
        let label = format!("workbench-{suffix}");
        if app.get_webview_window(&label).is_none() {
            return label;
        }
    }
}

#[tauri::command]
pub async fn start_agent_run(
    app: AppHandle,
    window: WebviewWindow,
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

    let owner = window.label().to_string();
    let registry = state.inner().clone();
    let sink = TauriRunEventSink::new(app, registry.clone());
    let permissions = state.permissions();
    let runner = AcpAgentRunner::new(
        ConfigurableAgentCatalog::from_env(),
        permissions,
        Arc::new(session_store),
    );

    StartAgentRunUseCase::new(registry)
        .execute(runner, sink, request, Some(owner))
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

    let agent_command = resolve_agent_command(&ConfigurableAgentCatalog::from_env(), request)?;
    let mut lookup = AcpSessionLookup::from_request(request);
    lookup.agent_command = Some(agent_command);
    let latest = session_store.latest_session(lookup).await?;
    if let Some(record) = latest {
        request.resume_session_id = Some(record.session_id);
        return Ok(());
    }

    if resume_policy == ResumePolicy::ResumeRequired {
        anyhow::bail!("resume session not found for requested workspace context");
    }
    Ok(())
}

fn resolve_agent_command<C: AgentCatalog>(
    catalog: &C,
    request: &AgentRunRequest,
) -> anyhow::Result<String> {
    if let Some(command) = request.agent_command.as_deref() {
        if let Some(command) = normalize_agent_command(command)? {
            return Ok(command);
        }
    }

    let command = catalog
        .command_for_agent(&request.agent_id)
        .ok_or_else(|| anyhow::anyhow!("unknown agent: {}", request.agent_id))?;
    normalize_agent_command(&command)?
        .ok_or_else(|| anyhow::anyhow!("agent command cannot be empty"))
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
    let registry = state.inner().clone();
    let sink = TauriRunEventSink::new(app, registry.clone());
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
    let registry = state.inner().clone();
    let owner = registry.owner_of(&run_id);
    let sink = TauriRunEventSink::with_fallback_owner(app, registry.clone(), owner);
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
pub async fn list_acp_sessions(
    storage: State<'_, StorageState>,
    mut query: AcpSessionListQuery,
) -> Result<Vec<AcpSessionRecord>, String> {
    query.agent_command = query
        .agent_command
        .as_deref()
        .map(normalize_agent_command)
        .transpose()
        .map_err(|err| err.to_string())?
        .flatten();
    storage
        .acp_session_store()
        .list_sessions(query)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn clear_acp_session(
    storage: State<'_, StorageState>,
    run_id: String,
) -> Result<bool, String> {
    storage
        .acp_session_store()
        .clear_session(run_id)
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
    WorkspaceGitUseCase::new(storage.workspace_store(), LocalGitRepository)
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
    WorkspaceGitUseCase::new(storage.workspace_store(), LocalGitRepository)
        .diff_summary(&workspace_id, checkout_id.as_deref())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn create_workspace_commit(
    storage: State<'_, StorageState>,
    request: WorkspaceCommitRequest,
) -> Result<WorkspaceCommitResult, String> {
    WorkspaceGitUseCase::new(storage.workspace_store(), LocalGitRepository)
        .commit(request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn push_workspace_branch(
    storage: State<'_, StorageState>,
    request: WorkspacePushRequest,
) -> Result<WorkspacePushResult, String> {
    WorkspaceGitUseCase::new(storage.workspace_store(), LocalGitRepository)
        .push(request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn provision_workspace_task_worktree(
    storage: State<'_, StorageState>,
    workspace_id: String,
    checkout_id: Option<String>,
    task_slug: Option<String>,
) -> Result<WorkspaceCheckout, String> {
    WorkspaceTaskWorktreeUseCase::new(storage.workspace_store(), LocalGitRepository)
        .provision(&workspace_id, checkout_id.as_deref(), task_slug.as_deref())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn cleanup_workspace_task_worktree(
    storage: State<'_, StorageState>,
    checkout_id: String,
) -> Result<bool, String> {
    WorkspaceTaskWorktreeUseCase::new(storage.workspace_store(), LocalGitRepository)
        .cleanup(&checkout_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn create_github_pull_request(
    storage: State<'_, StorageState>,
    request: GitHubPullRequestCreateRequest,
) -> Result<GitHubPullRequestSummary, String> {
    WorkspaceGitUseCase::new(storage.workspace_store(), LocalGitRepository)
        .create_pull_request(GhCliPullRequestClient, request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_github_pull_request_context(
    storage: State<'_, StorageState>,
    request: GitHubPullRequestContextRequest,
) -> Result<GitHubPullRequestContext, String> {
    WorkspaceGitUseCase::new(storage.workspace_store(), LocalGitRepository)
        .pull_request_context(GhCliPullRequestClient, request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn submit_github_pull_request_review(
    storage: State<'_, StorageState>,
    request: GitHubPullRequestReviewRequest,
) -> Result<GitHubPullRequestReviewResult, String> {
    WorkspaceGitUseCase::new(storage.workspace_store(), LocalGitRepository)
        .submit_pull_request_review(GhCliPullRequestClient, request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_pull_request_review_drafts(
    storage: State<'_, StorageState>,
    workspace_id: String,
    pull_request_number: Option<u64>,
) -> Result<Vec<PullRequestReviewDraft>, String> {
    storage
        .pull_request_review_draft_store()
        .list_pull_request_review_drafts(&workspace_id, pull_request_number)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn create_pull_request_review_draft(
    storage: State<'_, StorageState>,
    input: CreatePullRequestReviewDraftInput,
) -> Result<PullRequestReviewDraft, String> {
    storage
        .pull_request_review_draft_store()
        .create_pull_request_review_draft(input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn update_pull_request_review_draft(
    storage: State<'_, StorageState>,
    id: PullRequestReviewDraftId,
    patch: UpdatePullRequestReviewDraftPatch,
) -> Result<Option<PullRequestReviewDraft>, String> {
    storage
        .pull_request_review_draft_store()
        .update_pull_request_review_draft(&id, patch)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn delete_pull_request_review_draft(
    storage: State<'_, StorageState>,
    id: PullRequestReviewDraftId,
) -> Result<(), String> {
    storage
        .pull_request_review_draft_store()
        .delete_pull_request_review_draft(&id)
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

#[cfg(test)]
mod tests {
    use super::resolve_agent_command;
    use crate::{adapters::agent_catalog::StaticAgentCatalog, domain::run::AgentRunRequest};

    fn request(agent_command: Option<&str>) -> AgentRunRequest {
        AgentRunRequest {
            goal: "task".into(),
            agent_id: "codex".into(),
            workspace_id: Some("ws-1".into()),
            checkout_id: Some("co-1".into()),
            cwd: Some("/tmp/work".into()),
            agent_command: agent_command.map(str::to_string),
            stdio_buffer_limit_mb: None,
            auto_allow: None,
            run_id: None,
            resume_session_id: None,
            resume_policy: None,
            ralph_loop: None,
        }
    }

    #[test]
    fn resolves_catalog_command_for_resume_lookup() {
        let command = resolve_agent_command(&StaticAgentCatalog, &request(None)).unwrap();

        assert_eq!(command, "npx -y @zed-industries/codex-acp");
    }

    #[test]
    fn normalizes_explicit_command_for_resume_lookup() {
        let command = resolve_agent_command(
            &StaticAgentCatalog,
            &request(Some(" npx   -y   @zed-industries/codex-acp ")),
        )
        .unwrap();

        assert_eq!(command, "npx -y @zed-industries/codex-acp");
    }
}
