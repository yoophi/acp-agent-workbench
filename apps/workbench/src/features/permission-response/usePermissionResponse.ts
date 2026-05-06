import { useCallback, useEffect, useState } from "react";
import type { TimelineItem } from "../../entities/message";
import { respondAgentPermission } from "./api";

type PermissionMode = "allow" | "reject";

export function usePermissionResponse(
  items: TimelineItem[],
  onError: (message: string | null) => void,
) {
  const [pendingPermissionIds, setPendingPermissionIds] = useState<Set<string>>(() => new Set());

  useEffect(() => {
    setPendingPermissionIds((current) => {
      const next = new Set(current);
      for (const item of items) {
        if (item.event.type === "permission" && item.event.permissionId && !item.event.requiresResponse) {
          next.delete(item.event.permissionId);
        }
      }
      return next;
    });
  }, [items]);

  const isPending = useCallback(
    (permissionId: string) => pendingPermissionIds.has(permissionId),
    [pendingPermissionIds],
  );

  const hasOption = useCallback(
    (item: TimelineItem, mode: PermissionMode) => Boolean(findPermissionOption(item, mode)),
    [],
  );

  const respond = useCallback(
    async (item: TimelineItem, mode: PermissionMode) => {
      if (item.event.type !== "permission" || !item.event.permissionId) {
        return;
      }
      const option = findPermissionOption(item, mode);
      if (!option) {
        return;
      }
      const permissionId = item.event.permissionId;
      setPendingPermissionIds((current) => new Set(current).add(permissionId));
      try {
        await respondAgentPermission(permissionId, option.optionId);
        onError(null);
      } catch (err) {
        setPendingPermissionIds((current) => {
          const next = new Set(current);
          next.delete(permissionId);
          return next;
        });
        onError(String(err));
      }
    },
    [onError],
  );

  return { hasOption, isPending, respond };
}

function findPermissionOption(item: TimelineItem, mode: PermissionMode) {
  if (item.event.type !== "permission") {
    return undefined;
  }
  return item.event.options.find((option) => {
    const kind = option.kind.toLowerCase();
    if (mode === "allow") {
      return kind.startsWith("allow");
    }
    return kind.startsWith("reject") || kind.startsWith("deny");
  });
}
