use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::{collections::HashMap, fs, path::PathBuf, process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, Command},
    sync::{Mutex, oneshot},
};
use uuid::Uuid;

use crate::{
    adapters::acp::util::{
        clean_tool_title, display_command, expand_tilde, extract_locations, normalize_path,
        select_lines, string_param,
    },
    domain::events::{LifecycleStatus, PermissionOption, PlanEntry, RunEvent},
    ports::{event_sink::RunEventSink, permission::PermissionDecisionPort},
};

use super::util::RpcError;

type PendingMap = HashMap<u64, oneshot::Sender<std::result::Result<Value, RpcError>>>;

#[derive(Clone)]
pub struct RpcPeer {
    writer: Arc<Mutex<ChildStdin>>,
    pending: Arc<Mutex<PendingMap>>,
    next_id: Arc<Mutex<u64>>,
}

impl RpcPeer {
    pub fn new(stdin: ChildStdin) -> Self {
        Self {
            writer: Arc::new(Mutex::new(stdin)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn request(
        &self,
        method: &str,
        params: Value,
    ) -> std::result::Result<Value, RpcError> {
        let id = {
            let mut next_id = self.next_id.lock().await;
            let id = *next_id;
            *next_id += 1;
            id
        };
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        let payload = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        if let Err(err) = self.send_value(&payload).await {
            let _ = self.pending.lock().await.remove(&id);
            return Err(RpcError {
                code: -32603,
                message: err.to_string(),
                data: None,
            });
        }
        match rx.await {
            Ok(result) => result,
            Err(err) => Err(RpcError {
                code: -32603,
                message: format!("connection closed while waiting for {method}: {err}"),
                data: None,
            }),
        }
    }

    async fn respond_ok(&self, id: Value, result: Value) -> Result<()> {
        self.send_value(&json!({"jsonrpc": "2.0", "id": id, "result": result}))
            .await
    }

    async fn respond_error(
        &self,
        id: Value,
        code: i64,
        message: &str,
        data: Option<Value>,
    ) -> Result<()> {
        let mut error = json!({"code": code, "message": message});
        if let Some(data) = data {
            error["data"] = data;
        }
        self.send_value(&json!({"jsonrpc": "2.0", "id": id, "error": error}))
            .await
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.writer.lock().await.shutdown().await?;
        Ok(())
    }

    async fn fail_pending(&self, message: impl Into<String>) {
        let message = message.into();
        let pending = {
            let mut pending = self.pending.lock().await;
            std::mem::take(&mut *pending)
        };
        for (_, tx) in pending {
            let _ = tx.send(Err(RpcError {
                code: -32603,
                message: message.clone(),
                data: None,
            }));
        }
    }

    async fn send_value(&self, value: &Value) -> Result<()> {
        let mut writer = self.writer.lock().await;
        let mut bytes = serde_json::to_vec(value)?;
        bytes.push(b'\n');
        writer.write_all(&bytes).await?;
        writer.flush().await?;
        Ok(())
    }
}

struct TerminalState {
    child: Arc<Mutex<Child>>,
    output: Arc<Mutex<Vec<u8>>>,
    output_limit: usize,
    reader_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

pub struct AcpClient<S, P>
where
    S: RunEventSink,
    P: PermissionDecisionPort,
{
    run_id: String,
    workspace: PathBuf,
    auto_allow: bool,
    permission_decisions: P,
    terminals: Mutex<HashMap<String, TerminalState>>,
    last_tool_signature: Mutex<Option<(String, String, Vec<String>)>>,
    pending_tool_locations: Mutex<HashMap<String, Vec<String>>>,
    sink: S,
}

impl<S, P> AcpClient<S, P>
where
    S: RunEventSink,
    P: PermissionDecisionPort,
{
    pub fn new(
        run_id: String,
        workspace: PathBuf,
        auto_allow: bool,
        permission_decisions: P,
        sink: S,
    ) -> Self {
        Self {
            run_id,
            workspace,
            auto_allow,
            permission_decisions,
            terminals: Mutex::new(HashMap::new()),
            last_tool_signature: Mutex::new(None),
            pending_tool_locations: Mutex::new(HashMap::new()),
            sink,
        }
    }

    fn emit(&self, event: RunEvent) {
        self.sink.emit(&self.run_id, event);
    }

    pub async fn handle_request(
        self: Arc<Self>,
        peer: RpcPeer,
        id: Value,
        method: String,
        params: Value,
    ) {
        let result = match method.as_str() {
            "session/request_permission" => self.request_permission(params).await,
            "fs/read_text_file" => self.read_text_file(params).await,
            "fs/write_text_file" => self.write_text_file(params).await,
            "terminal/create" => self.create_terminal(params).await,
            "terminal/output" => self.terminal_output(params).await,
            "terminal/wait_for_exit" => self.wait_for_terminal_exit(params).await,
            "terminal/kill" => self.kill_terminal(params).await,
            "terminal/release" => self.release_terminal(params).await,
            method if method.starts_with("ext/") => {
                self.emit(RunEvent::Raw {
                    method: method.to_string(),
                    payload: params,
                });
                Ok(json!({}))
            }
            _ => Err(anyhow!("unsupported client method: {method}")),
        };

        match result {
            Ok(result) => {
                let _ = peer.respond_ok(id, result).await;
            }
            Err(err) => {
                let _ = peer
                    .respond_error(
                        id,
                        -32603,
                        "Internal error",
                        Some(json!({"details": err.to_string()})),
                    )
                    .await;
            }
        }
    }

    pub async fn handle_notification(&self, method: &str, params: Value) {
        if method == "session/update" {
            self.session_update(params).await;
        } else if method.starts_with("ext/") {
            self.emit(RunEvent::Raw {
                method: method.to_string(),
                payload: params,
            });
        } else {
            self.emit(RunEvent::Raw {
                method: method.to_string(),
                payload: params,
            });
        }
    }

    async fn request_permission(&self, params: Value) -> Result<Value> {
        let options = params
            .get("options")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("permission request missing options"))?;
        let tool_call = params.get("toolCall").cloned().unwrap_or(Value::Null);
        let title = clean_tool_title(tool_call.get("title").and_then(Value::as_str));
        let mapped_options = options
            .iter()
            .map(|option| PermissionOption {
                name: option
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                kind: option
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                option_id: option
                    .get("optionId")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
            .collect::<Vec<_>>();
        let permission_id = Uuid::new_v4().to_string();
        let selected = if self.auto_allow {
            select_permission_option(options, true)
                .ok_or_else(|| anyhow!("No allow permission option was offered by the agent."))?
                .to_owned()
        } else {
            let receiver = self
                .permission_decisions
                .create_waiter(self.run_id.clone(), permission_id.clone())
                .await;
            self.emit(RunEvent::Permission {
                permission_id: Some(permission_id.clone()),
                title: title.clone(),
                input: tool_call.get("rawInput").cloned(),
                options: mapped_options.clone(),
                selected: None,
                requires_response: true,
            });
            let decision = receiver
                .await
                .map_err(|_| anyhow!("permission response channel closed"))?;
            let selected = options
                .iter()
                .find(|option| {
                    option.get("optionId").and_then(Value::as_str)
                        == Some(decision.option_id.as_str())
                })
                .ok_or_else(|| anyhow!("permission response selected an unknown option"))?;
            selected.to_owned()
        };
        let selected_name = selected
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let option_id = selected
            .get("optionId")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("selected permission option is missing optionId"))?;
        self.emit(RunEvent::Permission {
            permission_id: if self.auto_allow {
                None
            } else {
                Some(permission_id)
            },
            title,
            input: tool_call.get("rawInput").cloned(),
            options: mapped_options,
            selected: Some(selected_name),
            requires_response: false,
        });
        Ok(json!({"outcome": {"outcome": "selected", "optionId": option_id}}))
    }

    async fn session_update(&self, params: Value) {
        let Some(update) = params.get("update") else {
            self.emit(RunEvent::Raw {
                method: "session/update".into(),
                payload: params,
            });
            return;
        };
        let kind = update
            .get("sessionUpdate")
            .and_then(Value::as_str)
            .unwrap_or("session/update");
        match kind {
            "agent_message_chunk" => {
                if let Some(text) = update.pointer("/content/text").and_then(Value::as_str) {
                    self.emit(RunEvent::AgentMessage { text: text.into() });
                }
            }
            "agent_thought_chunk" => {
                if let Some(text) = update.pointer("/content/text").and_then(Value::as_str) {
                    self.emit(RunEvent::Thought { text: text.into() });
                }
            }
            "plan" => {
                let entries = update
                    .get("entries")
                    .and_then(Value::as_array)
                    .map(|entries| {
                        entries
                            .iter()
                            .map(|entry| PlanEntry {
                                status: entry
                                    .get("status")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                                content: entry
                                    .get("content")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                self.emit(RunEvent::Plan { entries });
            }
            "tool_call" | "tool_call_update" => self.tool_update(update).await,
            "usage_update" => {
                let used = update
                    .get("used")
                    .and_then(Value::as_i64)
                    .unwrap_or_default();
                let size = update
                    .get("size")
                    .and_then(Value::as_i64)
                    .unwrap_or_default();
                self.emit(RunEvent::Usage { used, size });
            }
            other => self.emit(RunEvent::Raw {
                method: other.to_string(),
                payload: update.clone(),
            }),
        }
    }

    async fn tool_update(&self, update: &Value) {
        let status = update
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let title = clean_tool_title(update.get("title").and_then(Value::as_str));
        let tool_call_id = update
            .get("toolCallId")
            .and_then(Value::as_str)
            .map(str::to_string);
        let mut locations = extract_locations(update);

        if status == "pending" && title.is_empty() {
            if let Some(tool_call_id) = &tool_call_id {
                self.pending_tool_locations
                    .lock()
                    .await
                    .insert(tool_call_id.clone(), locations);
                return;
            }
        }

        if locations.is_empty() {
            if let Some(tool_call_id) = &tool_call_id {
                if let Some(cached) = self
                    .pending_tool_locations
                    .lock()
                    .await
                    .get(tool_call_id)
                    .cloned()
                {
                    locations = cached;
                }
            }
        }

        if matches!(status.as_str(), "completed" | "failed") {
            if let Some(tool_call_id) = &tool_call_id {
                self.pending_tool_locations
                    .lock()
                    .await
                    .remove(tool_call_id);
            }
        }

        let label = if !title.is_empty() {
            title
        } else if let Some(tool_call_id) = &tool_call_id {
            format!("id={tool_call_id}")
        } else {
            String::new()
        };
        let signature = (status.clone(), label.clone(), locations.clone());
        {
            let mut last = self.last_tool_signature.lock().await;
            if last.as_ref() == Some(&signature) {
                return;
            }
            *last = Some(signature);
        }

        self.emit(RunEvent::Tool {
            status,
            title: label,
            locations,
        });
    }

    async fn read_text_file(&self, params: Value) -> Result<Value> {
        let path = string_param(&params, "path")?;
        let target = self.resolve_inside_workspace(path)?;
        let content =
            fs::read_to_string(&target).with_context(|| format!("reading {}", target.display()))?;
        let start = params
            .get("line")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .saturating_sub(1) as usize;
        let limit = params
            .get("limit")
            .and_then(Value::as_u64)
            .map(|v| v as usize);
        let selected = select_lines(&content, start, limit);
        Ok(json!({"content": selected}))
    }

    async fn write_text_file(&self, params: Value) -> Result<Value> {
        let path = string_param(&params, "path")?;
        let content = string_param(&params, "content")?;
        let target = self.resolve_inside_workspace(path)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, content)?;
        self.emit(RunEvent::FileSystem {
            operation: "write".into(),
            path: target.display().to_string(),
        });
        Ok(json!({}))
    }

    async fn create_terminal(&self, params: Value) -> Result<Value> {
        let command = string_param(&params, "command")?;
        let args = params
            .get("args")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let working_dir = if let Some(cwd) = params.get("cwd").and_then(Value::as_str) {
            self.resolve_inside_workspace(cwd)?
        } else {
            self.workspace.clone()
        };
        let mut cmd = Command::new(command);
        cmd.args(&args)
            .current_dir(&working_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(env_items) = params.get("env").and_then(Value::as_array) {
            for item in env_items {
                if let (Some(name), Some(value)) = (
                    item.get("name").and_then(Value::as_str),
                    item.get("value").and_then(Value::as_str),
                ) {
                    cmd.env(name, value);
                }
            }
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("spawning terminal command {command}"))?;
        let terminal_id = Uuid::new_v4().to_string();
        let output_limit = params
            .get("outputByteLimit")
            .and_then(Value::as_u64)
            .unwrap_or(256_000) as usize;

        self.emit(RunEvent::Terminal {
            operation: "create".into(),
            terminal_id: Some(terminal_id.clone()),
            message: format!(
                "{} (cwd={})",
                display_command(command, &args),
                working_dir.display()
            ),
        });

        let output = Arc::new(Mutex::new(Vec::new()));
        let mut stdout = child.stdout.take();
        let mut stderr = child.stderr.take();
        let reader_output = Arc::clone(&output);
        let reader_task = tokio::spawn(async move {
            let out_task = async {
                if let Some(stdout) = stdout.as_mut() {
                    capture_output(stdout, Arc::clone(&reader_output), output_limit).await;
                }
            };
            let err_task = async {
                if let Some(stderr) = stderr.as_mut() {
                    capture_output(stderr, Arc::clone(&reader_output), output_limit).await;
                }
            };
            tokio::join!(out_task, err_task);
        });

        self.terminals.lock().await.insert(
            terminal_id.clone(),
            TerminalState {
                child: Arc::new(Mutex::new(child)),
                output,
                output_limit,
                reader_task: Arc::new(Mutex::new(Some(reader_task))),
            },
        );
        Ok(json!({"terminalId": terminal_id}))
    }

    async fn terminal_output(&self, params: Value) -> Result<Value> {
        let terminal_id = string_param(&params, "terminalId")?;
        let terminals = self.terminals.lock().await;
        let state = terminals
            .get(terminal_id)
            .ok_or_else(|| anyhow!("unknown terminal id: {terminal_id}"))?;
        let output = terminal_text(state).await;
        let truncated = state.output.lock().await.len() >= state.output_limit;
        let exit_status = terminal_exit_status(state).await?;
        Ok(json!({"output": output, "truncated": truncated, "exitStatus": exit_status}))
    }

    async fn wait_for_terminal_exit(&self, params: Value) -> Result<Value> {
        let terminal_id = string_param(&params, "terminalId")?;
        let (child, reader_task) = {
            let terminals = self.terminals.lock().await;
            let state = terminals
                .get(terminal_id)
                .ok_or_else(|| anyhow!("unknown terminal id: {terminal_id}"))?;
            (Arc::clone(&state.child), Arc::clone(&state.reader_task))
        };
        let status = child.lock().await.wait().await?;
        if let Some(reader_task) = reader_task.lock().await.take() {
            let _ = reader_task.await;
        }
        self.emit(RunEvent::Terminal {
            operation: "exit".into(),
            terminal_id: Some(terminal_id.to_string()),
            message: format!("{:?}", status.code()),
        });
        Ok(exit_status_json(
            status.code(),
            unix_signal_from_status(&status),
        ))
    }

    async fn kill_terminal(&self, params: Value) -> Result<Value> {
        let terminal_id = string_param(&params, "terminalId")?;
        let terminals = self.terminals.lock().await;
        let state = terminals
            .get(terminal_id)
            .ok_or_else(|| anyhow!("unknown terminal id: {terminal_id}"))?;
        let child = state.child.lock().await;
        if let Some(pid) = child.id() {
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
        self.emit(RunEvent::Terminal {
            operation: "kill".into(),
            terminal_id: Some(terminal_id.to_string()),
            message: "SIGTERM sent".into(),
        });
        Ok(json!({}))
    }

    async fn release_terminal(&self, params: Value) -> Result<Value> {
        let terminal_id = string_param(&params, "terminalId")?;
        if let Some(state) = self.terminals.lock().await.remove(terminal_id) {
            if let Some(reader_task) = state.reader_task.lock().await.take() {
                reader_task.abort();
            }
        }
        Ok(json!({}))
    }

    fn resolve_inside_workspace(&self, path: &str) -> Result<PathBuf> {
        let mut target = expand_tilde(path);
        if !target.is_absolute() {
            target = self.workspace.join(target);
        }
        let resolved = normalize_path(&target)?;
        if resolved != self.workspace && !resolved.starts_with(&self.workspace) {
            bail!("Path escapes workspace: {}", resolved.display());
        }
        Ok(resolved)
    }
}

pub async fn read_loop<R, S, P>(
    mut reader: BufReader<R>,
    peer: RpcPeer,
    client: Arc<AcpClient<S, P>>,
    limit: usize,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    S: RunEventSink,
    P: PermissionDecisionPort,
{
    loop {
        let mut bytes = Vec::new();
        let read = reader.read_until(b'\n', &mut bytes).await?;
        if read == 0 {
            peer.fail_pending("ACP connection closed").await;
            break;
        }
        if bytes.len() > limit {
            peer.fail_pending("ACP message exceeded stdio buffer limit")
                .await;
            bail!("ACP message exceeded stdio buffer limit of {limit} bytes");
        }
        let message: Value = match serde_json::from_slice(&bytes) {
            Ok(message) => message,
            Err(err) => {
                client.emit(RunEvent::Diagnostic {
                    message: format!("failed to parse JSON-RPC message: {err}"),
                });
                continue;
            }
        };
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .map(str::to_string);
        let id = message.get("id").cloned();
        match (method, id) {
            (Some(method), Some(id)) => {
                let params = message.get("params").cloned().unwrap_or(Value::Null);
                let peer = peer.clone();
                let client = Arc::clone(&client);
                tokio::spawn(async move {
                    client.handle_request(peer, id, method, params).await;
                });
            }
            (Some(method), None) => {
                let params = message.get("params").cloned().unwrap_or(Value::Null);
                client.handle_notification(&method, params).await;
            }
            (None, Some(id)) => {
                if let Some(request_id) = id.as_u64() {
                    if let Some(tx) = peer.pending.lock().await.remove(&request_id) {
                        if let Some(result) = message.get("result") {
                            let _ = tx.send(Ok(result.clone()));
                        } else {
                            let error = message.get("error").cloned().unwrap_or(Value::Null);
                            let _ = tx.send(Err(RpcError {
                                code: error.get("code").and_then(Value::as_i64).unwrap_or(-32603),
                                message: error
                                    .get("message")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Error")
                                    .to_string(),
                                data: error.get("data").cloned(),
                            }));
                        }
                    }
                }
            }
            (None, None) => {}
        }
    }
    Ok(())
}

fn select_permission_option<'a>(options: &'a [Value], auto_allow: bool) -> Option<&'a Value> {
    if auto_allow {
        for desired in ["allow_once", "allow_always"] {
            if let Some(option) = options
                .iter()
                .find(|option| option.get("kind").and_then(Value::as_str) == Some(desired))
            {
                return Some(option);
            }
        }
    }
    options.iter().find(|option| {
        option
            .get("kind")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind.starts_with("allow"))
    })
}

async fn capture_output<R>(reader: &mut R, output: Arc<Mutex<Vec<u8>>>, limit: usize)
where
    R: AsyncReadExt + Unpin,
{
    let mut buf = [0_u8; 4096];
    loop {
        let Ok(read) = reader.read(&mut buf).await else {
            return;
        };
        if read == 0 {
            return;
        }
        let mut output = output.lock().await;
        output.extend_from_slice(&buf[..read]);
        if output.len() > limit {
            let excess = output.len() - limit;
            output.drain(..excess);
        }
    }
}

async fn terminal_text(state: &TerminalState) -> String {
    String::from_utf8_lossy(&state.output.lock().await).into_owned()
}

async fn terminal_exit_status(state: &TerminalState) -> Result<Value> {
    let mut child = state.child.lock().await;
    if let Some(status) = child.try_wait()? {
        Ok(exit_status_json(
            status.code(),
            unix_signal_from_status(&status),
        ))
    } else {
        Ok(Value::Null)
    }
}

fn exit_status_json(code: Option<i32>, signal: Option<i32>) -> Value {
    if let Some(code) = code {
        json!({"exitCode": code})
    } else if let Some(signal) = signal {
        json!({"signal": signal.to_string()})
    } else {
        Value::Null
    }
}

#[cfg(unix)]
fn unix_signal_from_status(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn unix_signal_from_status(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

pub fn lifecycle(status: LifecycleStatus, message: impl Into<String>) -> RunEvent {
    RunEvent::Lifecycle {
        status,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::{RpcPeer, select_permission_option};
    use serde_json::json;
    use std::process::Stdio;
    use tokio::process::Command;

    #[test]
    fn auto_allow_prefers_allow_once_before_allow_always() {
        let options = vec![
            json!({"kind": "allow_always", "optionId": "always"}),
            json!({"kind": "allow_once", "optionId": "once"}),
        ];

        let selected = select_permission_option(&options, true).expect("permission option");

        assert_eq!(
            selected.get("optionId").and_then(|value| value.as_str()),
            Some("once")
        );
    }

    #[test]
    fn manual_mode_still_selects_first_allow_option_as_fallback() {
        let options = vec![
            json!({"kind": "reject_once", "optionId": "reject"}),
            json!({"kind": "allow_always", "optionId": "always"}),
        ];

        let selected = select_permission_option(&options, false).expect("permission option");

        assert_eq!(
            selected.get("optionId").and_then(|value| value.as_str()),
            Some("always")
        );
    }

    #[tokio::test]
    async fn fail_pending_releases_waiting_requests() {
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .expect("spawn cat");
        let stdin = child.stdin.take().expect("stdin");
        let peer = RpcPeer::new(stdin);
        let (tx, pending) = tokio::sync::oneshot::channel();
        peer.pending.lock().await.insert(1, tx);

        peer.fail_pending("closed for test").await;

        let result = pending.await.expect("pending response");
        assert!(result.is_err());
        assert_eq!(result.err().expect("error").message, "closed for test");
        let _ = child.kill().await;
    }
}
