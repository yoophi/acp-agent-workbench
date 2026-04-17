import { describe, expect, it, vi } from "vitest";

vi.mock("../../shared/api", () => ({
  invokeCommand: vi.fn(),
  listenEvent: vi.fn(),
}));

import { invokeCommand, listenEvent } from "../../shared/api";
import {
  cancelAgentRun,
  listenRunEvents,
  sendPromptToRun,
  startAgentRun,
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
});
