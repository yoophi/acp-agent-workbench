use anyhow::{Context, Result, anyhow, bail};
use serde_json::json;
use std::{fs, path::PathBuf, process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
    task::JoinHandle,
};

use crate::{
    adapters::acp::{
        client::{AcpClient, lifecycle},
        transport::{RpcPeer, read_loop},
        util::{RpcError, display_command, expand_tilde, normalize_path, rpc_to_anyhow},
    },
    application::start_agent_run::{
        AbortFuture, DriverFuture, LaunchedSession, RunCommander,
    },
    domain::{
        events::{LifecycleStatus, RunEvent},
        run::AgentRunRequest,
    },
    ports::{
        agent_catalog::AgentCatalog, event_sink::RunEventSink, permission::PermissionDecisionPort,
        session_handle::SessionHandle,
    },
};

const DEFAULT_WORKDIR: &str = "~/tmp/acp-tauri-agent-workspace";
const DEFAULT_STDIO_BUFFER_LIMIT_MB: usize = 50;

pub struct AcpAgentRunner<C, P>
where
    C: AgentCatalog,
    P: PermissionDecisionPort,
{
    catalog: C,
    permissions: P,
}

impl<C, P> AcpAgentRunner<C, P>
where
    C: AgentCatalog,
    P: PermissionDecisionPort,
{
    pub fn new(catalog: C, permissions: P) -> Self {
        Self {
            catalog,
            permissions,
        }
    }

    pub async fn start_session<S>(
        &self,
        request: &AgentRunRequest,
        run_id: String,
        sink: S,
    ) -> Result<AcpSessionSetup>
    where
        S: RunEventSink,
    {
        let workspace = normalize_workspace(request.cwd.as_deref().unwrap_or(DEFAULT_WORKDIR))?;
        fs::create_dir_all(&workspace)?;

        let agent_command = request
            .agent_command
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .or_else(|| self.catalog.command_for_agent(&request.agent_id))
            .ok_or_else(|| anyhow!("unknown agent: {}", request.agent_id))?;
        let agent_argv =
            shell_words::split(&agent_command).context("agent command cannot be parsed")?;
        if agent_argv.is_empty() {
            bail!("agent command cannot be empty");
        }

        sink.emit(
            &run_id,
            lifecycle(
                LifecycleStatus::Started,
                format!(
                    "{} in {}",
                    display_command(&agent_argv[0], &agent_argv[1..]),
                    workspace.display()
                ),
            ),
        );

        let mut child = Command::new(&agent_argv[0])
            .args(&agent_argv[1..])
            .current_dir(&workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("spawning ACP agent {}", agent_argv[0]))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("agent stdin is unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("agent stdout is unavailable"))?;
        let stderr = child.stderr.take();

        let peer = RpcPeer::new(stdin);
        let client = Arc::new(AcpClient::new(
            run_id.clone(),
            workspace.clone(),
            request.auto_allow.unwrap_or(true),
            self.permissions.clone(),
            sink.clone(),
        ));
        let read_task = tokio::spawn(read_loop(
            BufReader::new(stdout),
            peer.clone(),
            Arc::clone(&client),
            request
                .stdio_buffer_limit_mb
                .unwrap_or(DEFAULT_STDIO_BUFFER_LIMIT_MB)
                * 1024
                * 1024,
        ));

        let stderr_task = stderr.map(|stderr| {
            let sink = sink.clone();
            let run_id = run_id.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    sink.emit(&run_id, RunEvent::Diagnostic { message: line });
                }
            })
        });

        let init = peer
            .request(
                "initialize",
                json!({
                    "protocolVersion": 1,
                    "clientCapabilities": {
                        "fs": {"readTextFile": true, "writeTextFile": true},
                        "terminal": true
                    },
                    "clientInfo": {
                        "name": "tauri-acp-agent-workbench",
                        "title": "Tauri ACP Agent Workbench",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            )
            .await
            .map_err(rpc_to_anyhow)?;
        let agent_name = init
            .pointer("/agentInfo/name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let agent_version = init
            .pointer("/agentInfo/version")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        sink.emit(
            &run_id,
            lifecycle(
                LifecycleStatus::Initialized,
                format!("{agent_name} {agent_version}").trim().to_string(),
            ),
        );

        let session_id = create_agent_session(&peer, &workspace).await?;
        sink.emit(
            &run_id,
            lifecycle(LifecycleStatus::SessionCreated, session_id.clone()),
        );

        let session = Arc::new(AcpSession {
            run_id,
            session_id: Mutex::new(session_id),
            workspace,
            peer,
            in_flight: Mutex::new(()),
        });

        Ok(AcpSessionSetup {
            session,
            child,
            read_task,
            stderr_task,
        })
    }
}

pub struct AcpSessionSetup {
    pub session: Arc<AcpSession>,
    pub child: Child,
    pub read_task: JoinHandle<Result<()>>,
    pub stderr_task: Option<JoinHandle<()>>,
}

pub async fn launch_agent_run<C, P, S>(
    runner: Arc<AcpAgentRunner<C, P>>,
    request: AgentRunRequest,
    run_id: String,
    sink: S,
) -> Result<LaunchedSession<AcpSession>>
where
    C: AgentCatalog,
    P: PermissionDecisionPort,
    S: RunEventSink,
{
    let setup = runner
        .start_session(&request, run_id.clone(), sink.clone())
        .await?;
    let AcpSessionSetup {
        session,
        child,
        read_task,
        stderr_task,
    } = setup;

    let commander = AcpRunCommander {
        child,
        read_task,
        stderr_task,
        session: session.clone(),
        sink,
        run_id,
        initial_goal: request.goal,
    };

    Ok(LaunchedSession {
        session,
        commander: Box::new(commander),
    })
}

struct AcpRunCommander<S>
where
    S: RunEventSink,
{
    child: Child,
    read_task: JoinHandle<Result<()>>,
    stderr_task: Option<JoinHandle<()>>,
    session: Arc<AcpSession>,
    sink: S,
    run_id: String,
    initial_goal: String,
}

impl<S> RunCommander for AcpRunCommander<S>
where
    S: RunEventSink,
{
    fn run_to_completion(self: Box<Self>) -> DriverFuture {
        Box::pin(async move {
            let Self {
                mut child,
                read_task,
                stderr_task,
                session,
                sink,
                run_id,
                initial_goal,
            } = *self;

            let session_for_prompt = session.clone();
            let sink_for_prompt = sink.clone();
            let run_id_for_prompt = run_id.clone();
            tokio::spawn(async move {
                if let Err(err) = session_for_prompt
                    .send_prompt(sink_for_prompt.clone(), initial_goal)
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
                                &run_id,
                                RunEvent::Diagnostic {
                                    message: format!("agent process exited with code {code}"),
                                },
                            );
                        }
                    }
                }
                Err(err) => {
                    sink.emit(
                        &run_id,
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
                &run_id,
                lifecycle(LifecycleStatus::Completed, "agent exited"),
            );
        })
    }

    fn abort(self: Box<Self>) -> AbortFuture {
        Box::pin(async move {
            let Self {
                mut child,
                read_task,
                stderr_task,
                ..
            } = *self;
            let _ = child.start_kill();
            let _ = child.wait().await;
            read_task.abort();
            if let Some(task) = stderr_task {
                task.abort();
            }
        })
    }
}

pub struct AcpSession {
    pub run_id: String,
    pub peer: RpcPeer,
    session_id: Mutex<String>,
    workspace: PathBuf,
    in_flight: Mutex<()>,
}

impl AcpSession {
    pub async fn session_id(&self) -> String {
        self.session_id.lock().await.clone()
    }
}

impl SessionHandle for AcpSession {
    async fn send_prompt<S>(&self, sink: S, text: String) -> Result<String>
    where
        S: RunEventSink,
    {
        let _guard = self
            .in_flight
            .try_lock()
            .map_err(|_| anyhow!("agent is still responding to the previous prompt"))?;

        sink.emit(
            &self.run_id,
            lifecycle(LifecycleStatus::PromptSent, "prompt submitted"),
        );

        let mut reissued = false;
        let stop_reason = loop {
            let current_id = self.session_id().await;
            let outcome = self
                .peer
                .request(
                    "session/prompt",
                    json!({
                        "sessionId": current_id,
                        "prompt": [{"type": "text", "text": text.clone()}],
                    }),
                )
                .await;
            match outcome {
                Ok(response) => {
                    break response
                        .get("stopReason")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("unknown")
                        .to_string();
                }
                Err(err) if !reissued && is_session_not_found(&err) => {
                    reissued = true;
                    sink.emit(
                        &self.run_id,
                        RunEvent::Diagnostic {
                            message: "agent dropped the previous session; creating a new one"
                                .into(),
                        },
                    );
                    let new_id = create_agent_session(&self.peer, &self.workspace).await?;
                    sink.emit(
                        &self.run_id,
                        lifecycle(LifecycleStatus::SessionCreated, new_id.clone()),
                    );
                    *self.session_id.lock().await = new_id;
                    continue;
                }
                Err(err) => return Err(rpc_to_anyhow(err)),
            }
        };

        sink.emit(
            &self.run_id,
            lifecycle(
                LifecycleStatus::PromptCompleted,
                format!("stopReason={stop_reason}"),
            ),
        );
        Ok(stop_reason)
    }

}

fn normalize_workspace(path: &str) -> Result<PathBuf> {
    normalize_path(&expand_tilde(path))
}

async fn create_agent_session(peer: &RpcPeer, workspace: &PathBuf) -> Result<String> {
    let response = peer
        .request(
            "session/new",
            json!({"cwd": workspace.to_string_lossy(), "mcpServers": []}),
        )
        .await
        .map_err(rpc_to_anyhow)?;
    response
        .get("sessionId")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("session/new response missing sessionId"))
}

fn is_session_not_found(err: &RpcError) -> bool {
    const MARKER: &str = "Session not found";
    if err.message.contains(MARKER) {
        return true;
    }
    let Some(data) = &err.data else {
        return false;
    };
    if let Some(text) = data.as_str() {
        if text.contains(MARKER) {
            return true;
        }
    }
    if let Some(details) = data.get("details").and_then(serde_json::Value::as_str) {
        if details.contains(MARKER) {
            return true;
        }
    }
    false
}
