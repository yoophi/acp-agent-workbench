import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../shared/api", () => ({
  invokeCommand: vi.fn(),
  listenEvent: vi.fn(),
}));

import { invokeCommand } from "../../shared/api";
import { closeWorkbenchTab, detachWorkbenchTab } from "./tabActions";
import { createTabState, createWorkspaceViewState, useWorkbenchStore } from "./model";

const mockedInvoke = vi.mocked(invokeCommand);

function resetWorkbench(tabs = [createTabState({ id: "tab-1" }, 0)], activeTabId = tabs[0].id) {
  useWorkbenchStore.setState({
    workspaces: [],
    checkoutsByWorkspaceId: {},
    workspaceError: null,
    tabs,
    activeTabId,
    workspaceViews: tabs.map((tab, index) => createWorkspaceViewState(tab, index)),
    activeWorkspaceViewId: activeTabId,
    runsById: {},
  });
}

describe("closeWorkbenchTab", () => {
  beforeEach(() => {
    resetWorkbench();
  });

  it("force closes a tab that is already closing with an error", async () => {
    resetWorkbench([
      createTabState({ id: "tab-1", closing: true, error: "cancel failed" }, 0),
      createTabState({ id: "tab-2" }, 1),
    ]);

    await closeWorkbenchTab("tab-1");

    expect(useWorkbenchStore.getState().tabs.map((tab) => tab.id)).toEqual(["tab-2"]);
    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it("ignores a tab that is already closing without an error", async () => {
    resetWorkbench([
      createTabState(
        {
          id: "tab-1",
          activeRunId: "run-1",
          sessionActive: true,
          closing: true,
        },
        0,
      ),
      createTabState({ id: "tab-2" }, 1),
    ]);

    await closeWorkbenchTab("tab-1");

    const tab = useWorkbenchStore.getState().tabs.find((entry) => entry.id === "tab-1");
    expect(tab?.closing).toBe(true);
    expect(mockedInvoke).not.toHaveBeenCalled();
  });

  it("marks an active tab as closing and cancels its run", async () => {
    mockedInvoke.mockResolvedValueOnce(undefined);
    resetWorkbench([
      createTabState(
        {
          id: "tab-1",
          activeRunId: "run-1",
          sessionActive: true,
        },
        0,
      ),
      createTabState({ id: "tab-2" }, 1),
    ]);

    await closeWorkbenchTab("tab-1");

    const tab = useWorkbenchStore.getState().tabs.find((entry) => entry.id === "tab-1");
    expect(tab?.closing).toBe(true);
    expect(mockedInvoke).toHaveBeenCalledWith("cancel_agent_run", { runId: "run-1" });
  });

  it("closes an idle tab immediately", async () => {
    resetWorkbench([
      createTabState({ id: "tab-1" }, 0),
      createTabState({ id: "tab-2" }, 1),
    ]);

    await closeWorkbenchTab("tab-1");

    expect(useWorkbenchStore.getState().tabs.map((tab) => tab.id)).toEqual(["tab-2"]);
    expect(mockedInvoke).not.toHaveBeenCalled();
  });
});

describe("detachWorkbenchTab", () => {
  beforeEach(() => {
    resetWorkbench();
  });

  it("opens a detached window and removes the local tab without cancelling the run", async () => {
    mockedInvoke.mockResolvedValueOnce({
      label: "workbench-1",
      isMain: false,
      title: "ACP Agent Workbench",
    });
    resetWorkbench([
      createTabState(
        {
          id: "tab-1",
          activeRunId: "run-1",
          sessionActive: true,
        },
        0,
      ),
      createTabState({ id: "tab-2" }, 1),
    ]);
    useWorkbenchStore.getState().beginRun("tab-1", "run-1");

    await detachWorkbenchTab("tab-1");

    expect(mockedInvoke).toHaveBeenCalledWith("detach_tab", {
      tab: expect.objectContaining({ id: "tab-1", activeRunId: "run-1" }),
      runId: "run-1",
    });
    expect(useWorkbenchStore.getState().tabs.map((tab) => tab.id)).toEqual(["tab-2"]);
    expect(useWorkbenchStore.getState().runsById["run-1"]).toBeUndefined();
  });

  it("keeps the tab and records an error when detach fails", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("window failed"));
    resetWorkbench([
      createTabState({ id: "tab-1" }, 0),
      createTabState({ id: "tab-2" }, 1),
    ]);

    await detachWorkbenchTab("tab-1");

    const tab = useWorkbenchStore.getState().tabs.find((entry) => entry.id === "tab-1");
    expect(tab?.error).toContain("탭 분리 실패");
    expect(useWorkbenchStore.getState().tabs.map((entry) => entry.id)).toEqual(["tab-1", "tab-2"]);
  });
});
