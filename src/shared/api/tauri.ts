import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AgentDescriptor } from "../../entities/agent/model";
import type { AgentRun, AgentRunRequest, RunEventEnvelope } from "../../entities/message/model";

export function listAgents() {
  return invoke<AgentDescriptor[]>("list_agents");
}

export function startAgentRun(request: AgentRunRequest) {
  return invoke<AgentRun>("start_agent_run", { request });
}

export function cancelAgentRun(runId: string) {
  return invoke<void>("cancel_agent_run", { runId });
}

export function sendPromptToRun(runId: string, prompt: string) {
  return invoke<void>("send_prompt_to_run", { runId, prompt });
}

export function loadGoalFile(path: string) {
  return invoke<string>("load_goal_file", { path });
}

export function respondAgentPermission(permissionId: string, optionId: string) {
  return invoke<void>("respond_agent_permission", { permissionId, optionId });
}

export function listenRunEvents(callback: (event: RunEventEnvelope) => void) {
  return listen<RunEventEnvelope>("agent-run-event", (event) => callback(event.payload));
}
