import { useWorkbenchStore, type TabState } from "./model";
import { closeWorkbenchTab } from "./tabActions";

export type WorkbenchTabListItem = Readonly<
  Pick<
    TabState,
    | "id"
    | "title"
    | "goal"
    | "sessionActive"
    | "awaitingResponse"
    | "idleRemainingSec"
    | "error"
    | "unreadCount"
    | "permissionPending"
    | "closing"
  >
>;

export function useActiveTabId() {
  return useWorkbenchStore((state) => state.activeTabId);
}

export function useTabList(): WorkbenchTabListItem[] {
  return useWorkbenchStore((state) => state.tabs);
}

export function createWorkbenchTab() {
  return useWorkbenchStore.getState().addTab();
}

export function activateWorkbenchTab(tabId: string) {
  useWorkbenchStore.getState().activateTab(tabId);
}

export { closeWorkbenchTab };
