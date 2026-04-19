import type { AgentDescriptor } from "../../entities/agent";
import type { AgentRun, AgentRunRequest, RunEventEnvelope } from "../../entities/message";
import type { CreateSavedPromptInput, SavedPrompt, UpdateSavedPromptPatch } from "../../entities/saved-prompt";
import type {
  GitHubPullRequestCreateRequest,
  GitHubPullRequestSummary,
  RegisteredWorkspace,
  Workspace,
  WorkspaceCheckout,
  WorkspaceCommitRequest,
  WorkspaceCommitResult,
  WorkspaceDiffSummary,
  WorkspaceGitStatus,
  WorkspacePushRequest,
  WorkspacePushResult,
} from "../../entities/workspace";
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

export function getWorkspaceGitStatus(workspaceId: string, checkoutId?: string | null) {
  return invokeCommand<WorkspaceGitStatus>("get_workspace_git_status", { workspaceId, checkoutId });
}

export function summarizeWorkspaceDiff(workspaceId: string, checkoutId?: string | null) {
  return invokeCommand<WorkspaceDiffSummary>("summarize_workspace_diff", { workspaceId, checkoutId });
}

export function createWorkspaceCommit(request: WorkspaceCommitRequest) {
  return invokeCommand<WorkspaceCommitResult>("create_workspace_commit", { request });
}

export function pushWorkspaceBranch(request: WorkspacePushRequest) {
  return invokeCommand<WorkspacePushResult>("push_workspace_branch", { request });
}

export function createGitHubPullRequest(request: GitHubPullRequestCreateRequest) {
  return invokeCommand<GitHubPullRequestSummary>("create_github_pull_request", { request });
}

export function listSavedPrompts(workspaceId?: string | null) {
  return invokeCommand<SavedPrompt[]>("list_saved_prompts", { workspaceId });
}

export function createSavedPrompt(input: CreateSavedPromptInput) {
  return invokeCommand<SavedPrompt>("create_saved_prompt", { input });
}

export function updateSavedPrompt(id: string, patch: UpdateSavedPromptPatch) {
  return invokeCommand<SavedPrompt | null>("update_saved_prompt", { id, patch });
}

export function deleteSavedPrompt(id: string) {
  return invokeCommand<void>("delete_saved_prompt", { id });
}

export function recordSavedPromptUsed(id: string) {
  return invokeCommand<SavedPrompt | null>("record_saved_prompt_used", { id });
}
