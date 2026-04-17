use serde_json::Value;

use crate::domain::events::{PlanEntry, RunEvent};

/// Outcome of mapping a `session/update` JSON-RPC payload.
///
/// The ACP protocol mixes stateless updates (message chunks, plans,
/// usage) with stateful ones (tool call transitions that depend on
/// earlier tool events). The stateless mapping is captured here so it
/// can be unit-tested without the rest of the ACP client; stateful
/// tool updates are forwarded back to the caller as a raw `Value` for
/// session-aware handling.
pub enum MappedSessionUpdate {
    /// A ready-to-emit `RunEvent`.
    Event(RunEvent),
    /// A `tool_call` or `tool_call_update` payload; the caller tracks
    /// tool identity / locations / dedupe signature state.
    Tool(Value),
    /// Update contained no actionable data (e.g. an agent message
    /// chunk without text). The caller should emit nothing.
    Ignored,
}

/// Map a `session/update` params JSON into either a direct run event,
/// a tool payload that requires stateful handling, or a no-op.
pub fn map_session_update(params: &Value) -> MappedSessionUpdate {
    let Some(update) = params.get("update") else {
        return MappedSessionUpdate::Event(RunEvent::Raw {
            method: "session/update".into(),
            payload: params.clone(),
        });
    };

    let kind = update
        .get("sessionUpdate")
        .and_then(Value::as_str)
        .unwrap_or("session/update");

    match kind {
        "agent_message_chunk" => update
            .pointer("/content/text")
            .and_then(Value::as_str)
            .map(|text| {
                MappedSessionUpdate::Event(RunEvent::AgentMessage { text: text.into() })
            })
            .unwrap_or(MappedSessionUpdate::Ignored),
        "agent_thought_chunk" => update
            .pointer("/content/text")
            .and_then(Value::as_str)
            .map(|text| MappedSessionUpdate::Event(RunEvent::Thought { text: text.into() }))
            .unwrap_or(MappedSessionUpdate::Ignored),
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
            MappedSessionUpdate::Event(RunEvent::Plan { entries })
        }
        "tool_call" | "tool_call_update" => MappedSessionUpdate::Tool(update.clone()),
        "usage_update" => {
            let used = update
                .get("used")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            let size = update
                .get("size")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            MappedSessionUpdate::Event(RunEvent::Usage { used, size })
        }
        other => MappedSessionUpdate::Event(RunEvent::Raw {
            method: other.to_string(),
            payload: update.clone(),
        }),
    }
}
