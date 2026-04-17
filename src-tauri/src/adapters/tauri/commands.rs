use tauri::{AppHandle, State};

use crate::{
    adapters::{
        acp::{client::lifecycle, runner::AcpAgentRunner},
        agent_catalog::ConfigurableAgentCatalog,
        fs::LocalGoalFileReader,
        tauri::{event_sink::TauriRunEventSink, session_state::AppState},
    },
    application::{
        list_agents::ListAgentsUseCase, load_goal_file::LoadGoalFileUseCase,
        respond_permission::RespondPermissionUseCase,
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
        let runner = AcpAgentRunner::new(ConfigurableAgentCatalog::from_env(), permissions);
        let setup = match runner
            .start_session(&request, run_for_task.id.clone(), sink.clone())
            .await
        {
            Ok(setup) => setup,
            Err(err) => {
                sink.emit(
                    &run_for_task.id,
                    RunEvent::Error {
                        message: err.to_string(),
                    },
                );
                state_for_task.finish_run(&run_for_task.id).await;
                return;
            }
        };
        let session = setup.session;
        let mut child = setup.child;
        let read_task = setup.read_task;
        let stderr_task = setup.stderr_task;

        if let Err(err) = state_for_task
            .attach_session(&run_for_task.id, session.clone())
            .await
        {
            sink.emit(
                &run_for_task.id,
                RunEvent::Diagnostic {
                    message: err.to_string(),
                },
            );
            let _ = child.start_kill();
            let _ = child.wait().await;
            read_task.abort();
            if let Some(task) = stderr_task {
                task.abort();
            }
            state_for_task.finish_run(&run_for_task.id).await;
            return;
        }

        let session_for_prompt = session.clone();
        let sink_for_prompt = sink.clone();
        let run_id_for_prompt = run_for_task.id.clone();
        tokio::spawn(async move {
            if let Err(err) = session_for_prompt
                .send_prompt(&sink_for_prompt, request.goal)
                .await
            {
                sink_for_prompt.emit(
                    &run_id_for_prompt,
                    RunEvent::Error {
                        message: err.to_string(),
                    },
                );
            }
        });

        match child.wait().await {
            Ok(status) => {
                if let Some(code) = status.code() {
                    if code != 0 {
                        sink.emit(
                            &run_for_task.id,
                            RunEvent::Diagnostic {
                                message: format!("agent process exited with code {code}"),
                            },
                        );
                    }
                }
            }
            Err(err) => {
                sink.emit(
                    &run_for_task.id,
                    RunEvent::Diagnostic {
                        message: format!("failed to wait for agent process: {err}"),
                    },
                );
            }
        }

        read_task.abort();
        let _ = read_task.await;
        if let Some(task) = stderr_task {
            task.abort();
        }

        sink.emit(
            &run_for_task.id,
            lifecycle(LifecycleStatus::Completed, "agent exited"),
        );
        state_for_task.finish_run(&run_for_task.id).await;
    });
    state_handle
        .attach_run_handle(&run.id, handle)
        .await
        .map_err(|err| err.to_string())?;

    Ok(run)
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
