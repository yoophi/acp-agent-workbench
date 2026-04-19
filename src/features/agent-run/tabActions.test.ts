import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../shared/api", () => ({
  invokeCommand: vi.fn(),
  listenEvent: vi.fn(),
}));

import { invokeCommand } from "../../shared/api";
import { closeWorkbenchTab } from "./tabActions";
import { createTabState, useWorkbenchStore } from "./model";

const mockedInvoke = vi.mocked(invokeCommand);

function resetWorkbench(tabs = [createTabState({ id: "tab-1" }, 0)], activeTabId = tabs[0].id) {
  useWorkbenchStore.setState({
    workspaces: [],
    checkoutsByWorkspaceId: {},
    workspaceError: null,
    tabs,
    activeTabId,
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
