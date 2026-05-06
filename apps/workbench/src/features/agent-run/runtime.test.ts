import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("./api", () => ({
  cancelAgentRun: vi.fn(),
  listenRunEvents: vi.fn(),
  sendPromptToRun: vi.fn(),
}));

import { sendPromptToRun } from "./api";
import { defaultRalphLoopSettings, useWorkbenchStore } from "./model";
import { drainRalphLoop, resetRalphLoopStateForTests } from "./runtime";

const mockedSendPrompt = vi.mocked(sendPromptToRun);

function startRunWithLoop(overrides: Partial<typeof defaultRalphLoopSettings> = {}) {
  const tabId = useWorkbenchStore.getState().activeTabId;
  useWorkbenchStore.getState().patchTab(tabId, {
    ralphLoop: {
      ...defaultRalphLoopSettings,
      enabled: true,
      ...overrides,
    },
  });
  useWorkbenchStore.getState().beginRun(tabId, "run-1");
  useWorkbenchStore.getState().dispatchRunEvent("run-1", {
    type: "lifecycle",
    status: "promptCompleted",
    message: "stopReason=end_turn",
  });
  return tabId;
}

describe("Ralph loop runtime", () => {
  beforeEach(() => {
    useWorkbenchStore.setState(useWorkbenchStore.getInitialState(), true);
    resetRalphLoopStateForTests();
    mockedSendPrompt.mockReset();
  });

  it("does nothing when Ralph loop is disabled", async () => {
    const tabId = useWorkbenchStore.getState().activeTabId;
    useWorkbenchStore.getState().beginRun(tabId, "run-1");
    useWorkbenchStore.getState().dispatchRunEvent("run-1", {
      type: "lifecycle",
      status: "promptCompleted",
      message: "stopReason=end_turn",
    });

    await drainRalphLoop(tabId);

    expect(mockedSendPrompt).not.toHaveBeenCalled();
  });

  it("sends loop prompts until max iterations is reached", async () => {
    mockedSendPrompt.mockResolvedValue(undefined);
    const tabId = startRunWithLoop({
      maxIterations: 2,
      promptTemplate: "continue",
    });

    await drainRalphLoop(tabId);
    expect(mockedSendPrompt).toHaveBeenCalledTimes(1);
    expect(mockedSendPrompt).toHaveBeenLastCalledWith("run-1", "continue");

    useWorkbenchStore.getState().dispatchRunEvent("run-1", {
      type: "lifecycle",
      status: "promptCompleted",
      message: "stopReason=end_turn",
    });
    await drainRalphLoop(tabId);
    expect(mockedSendPrompt).toHaveBeenCalledTimes(2);

    useWorkbenchStore.getState().dispatchRunEvent("run-1", {
      type: "lifecycle",
      status: "promptCompleted",
      message: "stopReason=end_turn",
    });
    await drainRalphLoop(tabId);
    expect(mockedSendPrompt).toHaveBeenCalledTimes(2);
  });

  it("stops when permission is pending and stopOnPermission is enabled", async () => {
    mockedSendPrompt.mockResolvedValue(undefined);
    const tabId = startRunWithLoop({ stopOnPermission: true });
    useWorkbenchStore.getState().patchTab(tabId, { permissionPending: true });

    await drainRalphLoop(tabId);

    expect(mockedSendPrompt).not.toHaveBeenCalled();
  });

  it("records an error when a loop prompt fails", async () => {
    mockedSendPrompt.mockRejectedValue(new Error("send failed"));
    const tabId = startRunWithLoop({ promptTemplate: "continue" });

    await drainRalphLoop(tabId);

    const tab = useWorkbenchStore.getState().tabs.find((entry) => entry.id === tabId);
    expect(tab?.awaitingResponse).toBe(false);
    expect(tab?.error).toBe("Error: send failed");
  });

  it("does not send when the run has been cancelled", async () => {
    mockedSendPrompt.mockResolvedValue(undefined);
    const tabId = startRunWithLoop({ promptTemplate: "continue" });
    useWorkbenchStore.getState().endRun(tabId);

    await drainRalphLoop(tabId);

    expect(mockedSendPrompt).not.toHaveBeenCalled();
  });
});
