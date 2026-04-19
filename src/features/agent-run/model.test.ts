import { beforeEach, describe, expect, it } from "vitest";
import {
  selectActiveRun,
  selectWorkspaceView,
  selectWorkspaceViewRuns,
  useWorkbenchStore,
} from "./model";

describe("workspace-scoped run state", () => {
  beforeEach(() => {
    useWorkbenchStore.setState(useWorkbenchStore.getInitialState(), true);
  });

  it("creates and activates a workspace view alongside a tab", () => {
    const tabId = useWorkbenchStore.getState().addTab({
      id: "tab-2",
      title: "Workbench",
      workspaceId: "workspace-1",
      checkoutId: "checkout-1",
      cwd: "/repo/workbench",
      goal: "ship the feature",
      selectedAgentId: "codex",
    });

    const state = useWorkbenchStore.getState();
    const view = selectWorkspaceView(state, tabId);

    expect(state.activeTabId).toBe(tabId);
    expect(state.activeWorkspaceViewId).toBe(tabId);
    expect(view).toMatchObject({
      id: tabId,
      title: "Workbench",
      workspaceId: "workspace-1",
      checkoutId: "checkout-1",
      cwd: "/repo/workbench",
      activeRunId: null,
    });
    expect(view?.draft).toMatchObject({
      goal: "ship the feature",
      selectedAgentId: "codex",
    });
  });

  it("snapshots run-specific request state when a run begins", () => {
    const tabId = useWorkbenchStore.getState().activeTabId;
    useWorkbenchStore.getState().patchTab(tabId, {
      workspaceId: "workspace-1",
      checkoutId: "checkout-1",
      cwd: "/repo/workbench",
      goal: "implement workspace tabs",
      selectedAgentId: "claude-code",
    });

    useWorkbenchStore.getState().beginRun(tabId, "run-1");
    useWorkbenchStore.getState().patchTab(tabId, { goal: "draft the next run" });

    const state = useWorkbenchStore.getState();
    const run = selectActiveRun(state, tabId);
    const view = selectWorkspaceView(state, tabId);

    expect(view?.activeRunId).toBe("run-1");
    expect(view?.draft.goal).toBe("draft the next run");
    expect(run).toMatchObject({
      id: "run-1",
      workspaceViewId: tabId,
      workspaceId: "workspace-1",
      checkoutId: "checkout-1",
      cwd: "/repo/workbench",
      sessionActive: true,
      awaitingResponse: true,
    });
    expect(run?.request).toMatchObject({
      goal: "implement workspace tabs",
      selectedAgentId: "claude-code",
    });
  });

  it("keeps run timeline and lifecycle state in the workspace run map", () => {
    const tabId = useWorkbenchStore.getState().activeTabId;
    useWorkbenchStore.getState().beginRun(tabId, "run-1");

    useWorkbenchStore.getState().dispatchRunEvent("run-1", {
      type: "agentMessage",
      text: "first ",
    });
    useWorkbenchStore.getState().dispatchRunEvent("run-1", {
      type: "agentMessage",
      text: "reply",
    });
    useWorkbenchStore.getState().dispatchRunEvent("run-1", {
      type: "lifecycle",
      status: "completed",
      message: "done",
    });

    const state = useWorkbenchStore.getState();
    const [run] = selectWorkspaceViewRuns(state, tabId);

    expect(run.id).toBe("run-1");
    expect(run.sessionActive).toBe(false);
    expect(run.awaitingResponse).toBe(false);
    expect(run.completedAt).toEqual(expect.any(Number));
    expect(run.items.map((item) => item.body)).toEqual(["first reply", "done"]);
  });

  it("keeps failed startup errors on the run before clearing the active run", () => {
    const tabId = useWorkbenchStore.getState().activeTabId;
    const store = useWorkbenchStore.getState();
    store.beginRun(tabId, "run-1");
    store.patchTab(tabId, { error: "failed to start" });
    store.endRun(tabId);
    store.patchTab(tabId, { activeRunId: null });

    const state = useWorkbenchStore.getState();
    const [run] = selectWorkspaceViewRuns(state, tabId);
    const view = selectWorkspaceView(state, tabId);

    expect(view?.activeRunId).toBeNull();
    expect(view?.viewError).toBe("failed to start");
    expect(run).toMatchObject({
      id: "run-1",
      sessionActive: false,
      awaitingResponse: false,
      runError: "failed to start",
    });
    expect(run.completedAt).toEqual(expect.any(Number));
  });
});
