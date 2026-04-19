import type { AgentDescriptor } from "../../entities/agent";
import type { AcpSessionListQuery, AcpSessionRecord } from "../../entities/acp-session";
import type { AgentRun, AgentRunRequest, RunEventEnvelope } from "../../entities/message";
import type { CreateSavedPromptInput, SavedPrompt, UpdateSavedPromptPatch } from "../../entities/saved-prompt";
import type { WorkbenchWindowInfo } from "../../entities/workbench-window";
import type {
  CreatePullRequestReviewDraftInput,
  GitHubPullRequestCreateRequest,
  GitHubPullRequestContext,
  GitHubPullRequestContextRequest,
  GitHubPullRequestReviewRequest,
  GitHubPullRequestReviewResult,
  GitHubPullRequestSummary,
  PullRequestReviewDraft,
  RegisteredWorkspace,
  UpdatePullRequestReviewDraftPatch,
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
import type { TabState } from "./model";

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

export function detachAgentRunTab(tab: TabState) {
  return invokeCommand<WorkbenchWindowInfo>("detach_tab", {
    tab,
    runId: tab.sessionActive ? tab.activeRunId : null,
  });
}

export function listenRunEvents(callback: (event: RunEventEnvelope) => void) {
  return listenEvent<RunEventEnvelope>("agent-run-event", callback);
}

export function listAcpSessions(query: AcpSessionListQuery) {
  return invokeCommand<AcpSessionRecord[]>("list_acp_sessions", { query });
}

export function clearAcpSession(runId: string) {
  return invokeCommand<boolean>("clear_acp_session", { runId });
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

export function getGitHubPullRequestContext(request: GitHubPullRequestContextRequest) {
  return invokeCommand<GitHubPullRequestContext>("get_github_pull_request_context", { request });
}

export function submitGitHubPullRequestReview(request: GitHubPullRequestReviewRequest) {
  return invokeCommand<GitHubPullRequestReviewResult>("submit_github_pull_request_review", { request });
}

export function listPullRequestReviewDrafts(workspaceId: string, pullRequestNumber?: number | null) {
  return invokeCommand<PullRequestReviewDraft[]>("list_pull_request_review_drafts", {
    workspaceId,
    pullRequestNumber,
  });
}

export function createPullRequestReviewDraft(input: CreatePullRequestReviewDraftInput) {
  return invokeCommand<PullRequestReviewDraft>("create_pull_request_review_draft", { input });
}

export function updatePullRequestReviewDraft(id: string, patch: UpdatePullRequestReviewDraftPatch) {
  return invokeCommand<PullRequestReviewDraft | null>("update_pull_request_review_draft", { id, patch });
}

export function deletePullRequestReviewDraft(id: string) {
  return invokeCommand<void>("delete_pull_request_review_draft", { id });
}

export function provisionWorkspaceTaskWorktree(args: {
  workspaceId: string;
  checkoutId?: string | null;
  taskSlug?: string | null;
}) {
  return invokeCommand<WorkspaceCheckout>("provision_workspace_task_worktree", args);
}

export function cleanupWorkspaceTaskWorktree(checkoutId: string) {
  return invokeCommand<boolean>("cleanup_workspace_task_worktree", { checkoutId });
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
