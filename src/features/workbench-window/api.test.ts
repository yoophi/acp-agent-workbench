import { describe, expect, it, vi } from "vitest";

vi.mock("../../shared/api", () => ({
  invokeCommand: vi.fn(),
  listenEvent: vi.fn(),
}));

import { invokeCommand, listenEvent } from "../../shared/api";
import {
  closeWorkbenchWindow,
  getWindowBootstrap,
  listenWorkbenchWindowCloseRequests,
  listWorkbenchWindows,
  openWorkbenchWindow,
} from "./api";

const mockedInvoke = vi.mocked(invokeCommand);
const mockedListen = vi.mocked(listenEvent);

describe("workbench-window api", () => {
  it("forwards window bootstrap and registry commands", async () => {
    mockedInvoke
      .mockResolvedValueOnce({ label: "main", isMain: true })
      .mockResolvedValueOnce([{ label: "main", isMain: true, title: "ACP Agent Workbench" }])
      .mockResolvedValueOnce({ label: "workbench-1", isMain: false, title: "ACP Agent Workbench" })
      .mockResolvedValueOnce(undefined);

    await expect(getWindowBootstrap()).resolves.toEqual({ label: "main", isMain: true });
    await expect(listWorkbenchWindows()).resolves.toHaveLength(1);
    await expect(openWorkbenchWindow()).resolves.toMatchObject({ label: "workbench-1" });
    await expect(closeWorkbenchWindow()).resolves.toBeUndefined();

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "get_window_bootstrap");
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "list_workbench_windows");
    expect(mockedInvoke).toHaveBeenNthCalledWith(3, "open_workbench_window");
    expect(mockedInvoke).toHaveBeenNthCalledWith(4, "close_workbench_window");
  });

  it("subscribes to close request events", async () => {
    const dispose = vi.fn();
    const callback = vi.fn();
    mockedListen.mockResolvedValueOnce(dispose);

    await expect(listenWorkbenchWindowCloseRequests(callback)).resolves.toBe(dispose);

    expect(mockedListen).toHaveBeenCalledWith("workbench-window-close-requested", callback);
  });
});
