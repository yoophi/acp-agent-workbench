import { describe, expect, it, vi } from "vitest";

vi.mock("../../shared/api", () => ({
  invokeCommand: vi.fn(),
  listenEvent: vi.fn(),
}));

import { invokeCommand, listenEvent } from "../../shared/api";
import {
  cancelAgentRun,
  cleanupWorkspaceTaskWorktree,
  clearAcpSession,
  createGitHubPullRequest,
  createPullRequestReviewDraft,
  createWorkspaceCommit,
  deletePullRequestReviewDraft,
  detachAgentRunTab,
  getGitHubPullRequestContext,
  getWorkspaceGitStatus,
  listAcpSessions,
  listLocalTasks,
  listPullRequestReviewDrafts,
  listenRunEvents,
  pushWorkspaceBranch,
  provisionWorkspaceTaskWorktree,
  sendPromptToRun,
  startAgentRun,
  submitGitHubPullRequestReview,
  summarizeWorkspaceDiff,
  updateLocalTaskStatus,
  updatePullRequestReviewDraft,
} from "./api";
import { setupTauriListeners } from "../../test/tauri";

const mockedInvoke = vi.mocked(invokeCommand);
const mockedListen = vi.mocked(listenEvent);

// Mocks are reset between tests via vitest config (`mockReset: true`),
// so individual cases only need to set up the behavior they rely on.

describe("agent-run api", () => {
  it("startAgentRun forwards the request under an args object", async () => {
    mockedInvoke.mockResolvedValueOnce({ id: "run-1", goal: "hi", agentId: "claude" });

    const run = await startAgentRun({ goal: "hi", agentId: "claude" });

    expect(run.id).toBe("run-1");
    expect(mockedInvoke).toHaveBeenCalledWith("start_agent_run", {
      request: { goal: "hi", agentId: "claude" },
    });
  });

  it("cancelAgentRun and sendPromptToRun pass the run id alongside their payloads", async () => {
    mockedInvoke.mockResolvedValue(undefined);

    await cancelAgentRun("run-1");
    await sendPromptToRun("run-1", "follow up");

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "cancel_agent_run", { runId: "run-1" });
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "send_prompt_to_run", {
      runId: "run-1",
      prompt: "follow up",
    });
  });

  it("detachAgentRunTab forwards the tab snapshot and active run id", async () => {
    mockedInvoke.mockResolvedValueOnce({
      label: "workbench-1",
      isMain: false,
      title: "ACP Agent Workbench",
    });

    await detachAgentRunTab({
      id: "tab-1",
      title: "Review",
      workspaceId: "workspace-1",
      checkoutId: "checkout-1",
      selectedAgentId: "codex",
      scenario: "default",
      goal: "review the diff",
      cwd: "/repo",
      customCommand: "",
      stdioBufferLimitMb: 50,
      autoAllow: true,
      resumePolicy: "fresh",
      ralphLoop: {
        enabled: false,
        maxIterations: 3,
        promptTemplate: "Continue",
        stopOnError: true,
        stopOnPermission: true,
        delayMs: 0,
      },
      idleTimeoutSec: 60,
      idleRemainingSec: null,
      activeRunId: "run-1",
      sessionActive: true,
      awaitingResponse: false,
      followUpDraft: "",
      followUpQueue: [],
      items: [],
      filter: "all",
      error: null,
      sourceTask: null,
      unreadCount: 0,
      permissionPending: false,
      closing: false,
    });

    expect(mockedInvoke).toHaveBeenCalledWith("detach_tab", {
      tab: expect.objectContaining({ id: "tab-1" }),
      runId: "run-1",
    });
  });

  it("listenRunEvents subscribes to agent-run-event and delivers emitted payloads", async () => {
    const events = setupTauriListeners(mockedListen);
    const received: Array<{ runId: string }> = [];

    const dispose = await listenRunEvents((envelope) => {
      received.push({ runId: envelope.runId });
    });

    events.emit("agent-run-event", { runId: "run-1", event: { type: "lifecycle" } });
    events.emit("agent-run-event", { runId: "run-2", event: { type: "lifecycle" } });

    expect(received).toEqual([{ runId: "run-1" }, { runId: "run-2" }]);

    dispose();
    expect(events.count("agent-run-event")).toBe(0);
  });

  it("workspace git helpers pass the workspace and checkout ids", async () => {
    mockedInvoke.mockResolvedValue(undefined);

    await listLocalTasks("ws-1", "co-1");
    await updateLocalTaskStatus({
      workspaceId: "ws-1",
      checkoutId: "co-1",
      taskId: "bd-1",
      status: "in_progress",
    });
    await getWorkspaceGitStatus("ws-1", "co-1");
    await summarizeWorkspaceDiff("ws-1", "co-1");

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "list_local_tasks", {
      workspaceId: "ws-1",
      checkoutId: "co-1",
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "update_local_task_status", {
      workspaceId: "ws-1",
      checkoutId: "co-1",
      taskId: "bd-1",
      status: "in_progress",
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(3, "get_workspace_git_status", {
      workspaceId: "ws-1",
      checkoutId: "co-1",
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(4, "summarize_workspace_diff", {
      workspaceId: "ws-1",
      checkoutId: "co-1",
    });
  });

  it("ACP session helpers pass query and run id payloads", async () => {
    mockedInvoke.mockResolvedValue(undefined);
    const query = {
      workspaceId: "ws-1",
      checkoutId: "co-1",
      agentId: "codex",
      limit: 5,
    };

    await listAcpSessions(query);
    await clearAcpSession("run-1");

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "list_acp_sessions", { query });
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "clear_acp_session", { runId: "run-1" });
  });

  it("workspace publishing helpers pass request payloads under request", async () => {
    mockedInvoke.mockResolvedValue(undefined);

    const commitRequest = {
      workspaceId: "ws-1",
      checkoutId: "co-1",
      message: "Add feature",
      files: ["src/index.ts"],
      confirmed: true,
    };
    const pushRequest = {
      workspaceId: "ws-1",
      checkoutId: "co-1",
      remote: "origin",
      branch: "feature/test",
      setUpstream: true,
      confirmed: true,
    };
    const pullRequestRequest = {
      workspaceId: "ws-1",
      checkoutId: "co-1",
      base: "main",
      head: "feature/test",
      title: "Add feature",
      body: "Summary",
      draft: true,
      confirmed: true,
    };
    const reviewRequest = {
      workspaceId: "ws-1",
      checkoutId: "co-1",
      number: 42,
      body: "Looks good.",
      decision: "comment" as const,
      comments: [{ path: "src/index.ts", line: 10, body: "Consider a narrower name." }],
      confirmed: true,
    };

    await createWorkspaceCommit(commitRequest);
    await pushWorkspaceBranch(pushRequest);
    await createGitHubPullRequest(pullRequestRequest);
    await getGitHubPullRequestContext({
      workspaceId: "ws-1",
      checkoutId: "co-1",
      number: 42,
    });
    await submitGitHubPullRequestReview(reviewRequest);

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "create_workspace_commit", {
      request: commitRequest,
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "push_workspace_branch", {
      request: pushRequest,
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(3, "create_github_pull_request", {
      request: pullRequestRequest,
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(4, "get_github_pull_request_context", {
      request: { workspaceId: "ws-1", checkoutId: "co-1", number: 42 },
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(5, "submit_github_pull_request_review", {
      request: reviewRequest,
    });
  });

  it("workspace task worktree helpers forward task isolation args", async () => {
    mockedInvoke.mockResolvedValueOnce({
      id: "checkout-worktree",
      workspaceId: "workspace-1",
      path: "/repo/acp-agent-workbench-issue-63",
      branch: "worktree/issue-63",
      headSha: "abc123",
      kind: "worktree",
      isDefault: false,
    });

    const checkout = await provisionWorkspaceTaskWorktree({
      workspaceId: "workspace-1",
      checkoutId: "checkout-main",
      taskSlug: "Issue #63",
    });

    expect(checkout.kind).toBe("worktree");
    expect(mockedInvoke).toHaveBeenCalledWith("provision_workspace_task_worktree", {
      workspaceId: "workspace-1",
      checkoutId: "checkout-main",
      taskSlug: "Issue #63",
    });

    await cleanupWorkspaceTaskWorktree("checkout-worktree");

    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "cleanup_workspace_task_worktree", {
      checkoutId: "checkout-worktree",
    });
  });

  it("pull request review draft helpers pass command payloads", async () => {
    mockedInvoke.mockResolvedValue(undefined);
    const input = {
      workspaceId: "ws-1",
      checkoutId: "co-1",
      pullRequestNumber: 42,
      runId: "run-1",
      summary: "Looks good overall",
      decision: "comment" as const,
      comments: [{ path: "src/lib.rs", line: 12, side: "RIGHT" as const, body: "Add a regression test." }],
    };
    const patch = { decision: "request_changes" as const, comments: [] };

    await listPullRequestReviewDrafts("ws-1", 42);
    await createPullRequestReviewDraft(input);
    await updatePullRequestReviewDraft("draft-1", patch);
    await deletePullRequestReviewDraft("draft-1");

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "list_pull_request_review_drafts", {
      workspaceId: "ws-1",
      pullRequestNumber: 42,
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "create_pull_request_review_draft", { input });
    expect(mockedInvoke).toHaveBeenNthCalledWith(3, "update_pull_request_review_draft", {
      id: "draft-1",
      patch,
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(4, "delete_pull_request_review_draft", { id: "draft-1" });
  });
});
