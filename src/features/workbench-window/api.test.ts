import { describe, expect, it, vi } from "vitest";

vi.mock("../../shared/api", () => ({
  invokeCommand: vi.fn(),
}));

import { invokeCommand } from "../../shared/api";
import { getWindowBootstrap, listWorkbenchWindows, openWorkbenchWindow } from "./api";

const mockedInvoke = vi.mocked(invokeCommand);

describe("workbench-window api", () => {
  it("forwards window bootstrap and registry commands", async () => {
    mockedInvoke
      .mockResolvedValueOnce({ label: "main", isMain: true })
      .mockResolvedValueOnce([{ label: "main", isMain: true, title: "ACP Agent Workbench" }])
      .mockResolvedValueOnce({ label: "workbench-1", isMain: false, title: "ACP Agent Workbench" });

    await expect(getWindowBootstrap()).resolves.toEqual({ label: "main", isMain: true });
    await expect(listWorkbenchWindows()).resolves.toHaveLength(1);
    await expect(openWorkbenchWindow()).resolves.toMatchObject({ label: "workbench-1" });

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "get_window_bootstrap");
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "list_workbench_windows");
    expect(mockedInvoke).toHaveBeenNthCalledWith(3, "open_workbench_window");
  });
});
