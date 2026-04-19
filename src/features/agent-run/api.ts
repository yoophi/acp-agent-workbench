import type { AgentDescriptor } from "../../entities/agent";
import type { AgentRun, AgentRunRequest, RunEventEnvelope } from "../../entities/message";
import type { RegisteredWorkspace, Workspace, WorkspaceCheckout } from "../../entities/workspace";
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

export function listWorkspaces() {
  return invokeCommand<Workspace[]>("list_workspaces");
}

export function registerWorkspaceFromPath(path: string) {
  return invokeCommand<RegisteredWorkspace>("register_workspace_from_path", { path });
}

export function listWorkspaceCheckouts(workspaceId: string) {
  return invokeCommand<WorkspaceCheckout[]>("list_workspace_checkouts", { workspaceId });
}

export function refreshWorkspaceCheckout(checkoutId: string) {
  return invokeCommand<WorkspaceCheckout | null>("refresh_workspace_checkout", { checkoutId });
}

export function resolveWorkspaceWorkdir(args: {
  workspaceId?: string | null;
  checkoutId?: string | null;
  cwd?: string | null;
}) {
  return invokeCommand<string | null>("resolve_workspace_workdir", args);
}
