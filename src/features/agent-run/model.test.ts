import { beforeEach, describe, expect, it } from "vitest";
import {
  createTabState,
  createWorkspaceViewState,
  selectActiveRun,
  selectActiveWorkspaceView,
  selectRun,
  useWorkbenchStore,
} from "./model";

function resetWorkbench(preset = {}) {
  const tab = createTabState({ id: "view-1", ...preset }, 0);
  const view = createWorkspaceViewState({ id: tab.id, ...preset }, 0);
  useWorkbenchStore.setState({
    workspaces: [],
    checkoutsByWorkspaceId: {},
    workspaceError: null,
    workspaceViews: [view],
    runsById: {},
    activeWorkspaceViewId: view.id,
    tabs: [tab],
    activeTabId: tab.id,
  });
  return tab.id;
}

describe("workspace-scoped run model", () => {
  beforeEach(() => {
    resetWorkbench({
      workspaceId: "workspace-1",
      checkoutId: "checkout-1",
      cwd: "/repo",
      selectedAgentId: "codex",
      goal: "implement feature",
      customCommand: "codex",
    });
  });

  it("creates a workspace view and run snapshot when a run begins", () => {
    useWorkbenchStore.getState().beginRun("view-1", "run-1");

    const state = useWorkbenchStore.getState();
    const view = selectActiveWorkspaceView(state);
    const run = selectActiveRun(state, "view-1");

    expect(view?.activeRunId).toBe("run-1");
    expect(run).toMatchObject({
      id: "run-1",
      workspaceViewId: "view-1",
      workspaceId: "workspace-1",
      checkoutId: "checkout-1",
      cwd: "/repo",
      sessionActive: true,
      awaitingResponse: true,
      request: {
        selectedAgentId: "codex",
        goal: "implement feature",
        customCommand: "codex",
      },
    });
  });

  it("keeps compatibility tab state and run state in sync for queued follow-ups", () => {
    const store = useWorkbenchStore.getState();
    store.beginRun("view-1", "run-1");
    store.enqueueFollowUp("view-1", "next prompt");

    const state = useWorkbenchStore.getState();
    const tabItem = state.tabs[0].followUpQueue[0];
    const runItem = selectRun(state, "run-1")?.followUpQueue[0];

    expect(runItem).toEqual(tabItem);
    expect(runItem?.text).toBe("next prompt");

    const dequeued = useWorkbenchStore.getState().dequeueFollowUp("view-1");

    expect(dequeued).toEqual(tabItem);
    expect(useWorkbenchStore.getState().tabs[0].followUpQueue).toEqual([]);
    expect(selectRun(useWorkbenchStore.getState(), "run-1")?.followUpQueue).toEqual([]);
  });

  it("routes run events into both the compatibility tab and AgentRunState", () => {
    const store = useWorkbenchStore.getState();
    store.beginRun("view-1", "run-1");
    store.dispatchRunEvent("run-1", {
      type: "lifecycle",
      status: "promptCompleted",
      message: "done",
    });

    const state = useWorkbenchStore.getState();

    expect(state.tabs[0].awaitingResponse).toBe(false);
    expect(selectRun(state, "run-1")?.awaitingResponse).toBe(false);
    expect(state.tabs[0].items).toHaveLength(1);
    expect(selectRun(state, "run-1")?.items).toHaveLength(1);
  });
});
