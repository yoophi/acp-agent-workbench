use anyhow::{Context, Result, anyhow, bail};
use serde_json::json;
use std::{fs, path::PathBuf, process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

use crate::{
    adapters::acp::{
        client::{AcpClient, RpcPeer, lifecycle, read_loop},
        util::{display_command, expand_tilde, normalize_path, rpc_to_anyhow},
    },
    domain::{
        events::{LifecycleStatus, RunEvent},
        run::AgentRunRequest,
    },
    ports::{
        agent_catalog::AgentCatalog, event_sink::RunEventSink, permission::PermissionDecisionPort,
        runner::AgentRunner,
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
}

impl<C, P> AgentRunner for AcpAgentRunner<C, P>
where
    C: AgentCatalog,
    P: PermissionDecisionPort,
{
    async fn run<S>(&self, request: AgentRunRequest, run_id: String, sink: S) -> Result<()>
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

        let session = peer
            .request(
                "session/new",
                json!({"cwd": workspace.to_string_lossy(), "mcpServers": []}),
            )
            .await
            .map_err(rpc_to_anyhow)?;
        let session_id = session
            .get("sessionId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| anyhow!("session/new response missing sessionId"))?
            .to_string();
        sink.emit(
            &run_id,
            lifecycle(LifecycleStatus::SessionCreated, session_id.clone()),
        );

        sink.emit(
            &run_id,
            lifecycle(LifecycleStatus::PromptSent, "goal submitted"),
        );
        let response = peer
            .request(
                "session/prompt",
                json!({"sessionId": session_id, "prompt": [{"type": "text", "text": request.goal}]}),
            )
            .await
            .map_err(rpc_to_anyhow)?;
        let stop_reason = response
            .get("stopReason")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");

        match peer
            .request("session/close", json!({"sessionId": session_id}))
            .await
        {
            Ok(_) => {}
            Err(err) if err.code == -32601 => {
                sink.emit(
                    &run_id,
                    RunEvent::Diagnostic {
                        message: "session/close is not supported by this agent".into(),
                    },
                );
            }
            Err(err) => return Err(rpc_to_anyhow(err)),
        }

        read_task.abort();
        let _ = read_task.await;
        peer.shutdown().await?;
        let status = child.wait().await?;
        if let Some(stderr_task) = stderr_task {
            stderr_task.abort();
        }
        if let Some(code) = status.code() {
            if code != 0 {
                bail!("agent process exited with {code}");
            }
        }
        sink.emit(
            &run_id,
            lifecycle(
                LifecycleStatus::Completed,
                format!("stopReason={stop_reason}"),
            ),
        );
        Ok(())
    }
}

fn normalize_workspace(path: &str) -> Result<PathBuf> {
    normalize_path(&expand_tilde(path))
}
