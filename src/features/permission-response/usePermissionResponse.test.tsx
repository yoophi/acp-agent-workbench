import { act, useEffect } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { TimelineItem } from "../../entities/message";

vi.mock("../../shared/api", () => ({
  invokeCommand: vi.fn(),
  listenEvent: vi.fn(),
}));

import { invokeCommand } from "../../shared/api";
import { usePermissionResponse } from "./usePermissionResponse";

const mockedInvoke = vi.mocked(invokeCommand);

type HookResult = ReturnType<typeof usePermissionResponse>;

let root: Root | undefined;
let container: HTMLDivElement | undefined;

function permissionItem(
  permissionId: string,
  requiresResponse = true,
): TimelineItem {
  return {
    id: `${permissionId}-item`,
    runId: "run-1",
    group: "permission",
    title: "permission",
    body: "",
    createdAt: 1,
    event: {
      type: "permission",
      permissionId,
      title: "Allow command?",
      requiresResponse,
      options: [
        { name: "Allow", kind: "allow_once", optionId: "allow-1" },
        { name: "Deny", kind: "reject_once", optionId: "deny-1" },
      ],
    },
  };
}

function renderHook(items: TimelineItem[], onError = vi.fn()) {
  const latest: { current?: HookResult } = {};

  function Probe(props: { currentItems: TimelineItem[] }) {
    const result = usePermissionResponse(props.currentItems, onError);
    useEffect(() => {
      latest.current = result;
    }, [result]);
    return null;
  }

  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);

  act(() => {
    root?.render(<Probe currentItems={items} />);
  });

  return {
    latest,
    onError,
    rerender(nextItems: TimelineItem[]) {
      act(() => {
        root?.render(<Probe currentItems={nextItems} />);
      });
    },
  };
}

afterEach(() => {
  act(() => {
    root?.unmount();
  });
  container?.remove();
  root = undefined;
  container = undefined;
});

describe("usePermissionResponse", () => {
  it("tracks pending permission responses and sends the selected allow option", () => {
    mockedInvoke.mockReturnValueOnce(new Promise(() => undefined));
    const item = permissionItem("perm-1");
    const { latest } = renderHook([item]);

    act(() => {
      void latest.current?.respond(item, "allow");
    });

    expect(latest.current?.isPending("perm-1")).toBe(true);
    expect(latest.current?.hasOption(item, "allow")).toBe(true);
    expect(mockedInvoke).toHaveBeenCalledWith("respond_agent_permission", {
      permissionId: "perm-1",
      optionId: "allow-1",
    });
  });

  it("clears pending state when a terminal permission event arrives", () => {
    mockedInvoke.mockReturnValueOnce(new Promise(() => undefined));
    const item = permissionItem("perm-1");
    const done = permissionItem("perm-1", false);
    const { latest, rerender } = renderHook([item]);

    act(() => {
      void latest.current?.respond(item, "reject");
    });
    expect(latest.current?.isPending("perm-1")).toBe(true);

    rerender([item, done]);

    expect(latest.current?.isPending("perm-1")).toBe(false);
    expect(mockedInvoke).toHaveBeenCalledWith("respond_agent_permission", {
      permissionId: "perm-1",
      optionId: "deny-1",
    });
  });

  it("resets pending state and reports an error when responding fails", async () => {
    mockedInvoke.mockRejectedValueOnce(new Error("permission failed"));
    const item = permissionItem("perm-1");
    const onError = vi.fn();
    const { latest } = renderHook([item], onError);

    await act(async () => {
      await latest.current?.respond(item, "allow");
    });

    expect(latest.current?.isPending("perm-1")).toBe(false);
    expect(onError).toHaveBeenCalledWith("Error: permission failed");
  });
});
