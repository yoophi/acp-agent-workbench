import { describe, expect, it, vi } from "vitest";

vi.mock("../../shared/api", () => ({
  invokeCommand: vi.fn(),
  listenEvent: vi.fn(),
}));

import { invokeCommand, listenEvent } from "../../shared/api";
import {
  cancelAgentRun,
  createGitHubPullRequest,
  createWorkspaceCommit,
  getWorkspaceGitStatus,
  listenRunEvents,
  pushWorkspaceBranch,
  provisionWorkspaceTaskWorktree,
  sendPromptToRun,
  startAgentRun,
  summarizeWorkspaceDiff,
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

    await getWorkspaceGitStatus("ws-1", "co-1");
    await summarizeWorkspaceDiff("ws-1", "co-1");

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "get_workspace_git_status", {
      workspaceId: "ws-1",
      checkoutId: "co-1",
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "summarize_workspace_diff", {
      workspaceId: "ws-1",
      checkoutId: "co-1",
    });
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

    await createWorkspaceCommit(commitRequest);
    await pushWorkspaceBranch(pushRequest);
    await createGitHubPullRequest(pullRequestRequest);

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "create_workspace_commit", {
      request: commitRequest,
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "push_workspace_branch", {
      request: pushRequest,
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(3, "create_github_pull_request", {
      request: pullRequestRequest,
    });
  });

  it("provisionWorkspaceTaskWorktree forwards workspace task isolation args", async () => {
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
  });
});
