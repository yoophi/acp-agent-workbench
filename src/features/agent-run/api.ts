import type { AgentDescriptor } from "../../entities/agent";
import type { AgentRun, AgentRunRequest, RunEventEnvelope } from "../../entities/message";
import { invokeCommand, listenEvent } from "../../shared/api";

export function listAgents() {
  return invokeCommand<AgentDescriptor[]>("list_agents");
}

export function startAgentRun(request: AgentRunRequest) {
  return invokeCommand<AgentRun>("start_agent_run", { request });
}

export function cancelAgentRun(runId: string) {
  return invokeCommand<void>("cancel_agent_run", { runId });
}

export function sendPromptToRun(runId: string, prompt: string) {
  return invokeCommand<void>("send_prompt_to_run", { runId, prompt });
}

export function listenRunEvents(callback: (event: RunEventEnvelope) => void) {
  return listenEvent<RunEventEnvelope>("agent-run-event", callback);
}
