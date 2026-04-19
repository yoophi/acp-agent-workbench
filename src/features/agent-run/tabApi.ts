import type { Workspace, WorkspaceCheckout } from "../../entities/workspace";
import { useShallow } from "zustand/react/shallow";
import {
  isTabState,
  selectTab,
  selectTabList,
  useWorkbenchStore,
  type TabState,
} from "./model";
import { closeWorkbenchTab, detachWorkbenchTab } from "./tabActions";

const EMPTY_CHECKOUTS: WorkspaceCheckout[] = [];

export type WorkbenchTabListItem = Readonly<
  Pick<
    TabState,
    | "id"
    | "title"
    | "goal"
    | "workspaceId"
    | "checkoutId"
    | "cwd"
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
  return useWorkbenchStore((state) => state.activeWorkspaceViewId);
}

export function useTabList(): WorkbenchTabListItem[] {
  return useWorkbenchStore(useShallow(selectTabList));
}

export function createWorkbenchTab() {
  return useWorkbenchStore.getState().addTab();
}

export function activateWorkbenchTab(tabId: string) {
  useWorkbenchStore.getState().activateTab(tabId);
}

export { closeWorkbenchTab, detachWorkbenchTab };

export function hydrateDetachedWorkbenchTab(tab: unknown) {
  if (!isTabState(tab)) return false;
  useWorkbenchStore.getState().hydrateDetachedTab(tab);
  return true;
}

export function useWorkspaceState(tabId: string) {
  return useWorkbenchStore(useShallow((state) => {
    const tab = selectTab(state, tabId);
    const selectedWorkspace = state.workspaces.find((entry) => entry.id === tab?.workspaceId);
    const checkouts = tab?.workspaceId
      ? (state.checkoutsByWorkspaceId[tab.workspaceId] ?? EMPTY_CHECKOUTS)
      : EMPTY_CHECKOUTS;
    const selectedCheckout = checkouts.find((entry) => entry.id === tab?.checkoutId);
    const activeSameWorkdirCount = tab
      ? selectTabList(state).filter(
          (entry) =>
            entry.id !== tab.id &&
            entry.sessionActive &&
            entry.workspaceId === tab.workspaceId &&
            entry.checkoutId === tab.checkoutId &&
            entry.cwd === tab.cwd,
        ).length
      : 0;

    return {
      workspaces: state.workspaces,
      checkouts,
      selectedWorkspace,
      selectedCheckout,
      workspaceId: tab?.workspaceId ?? null,
      checkoutId: tab?.checkoutId ?? null,
      workdir: tab?.cwd ?? "",
      workspaceError: state.workspaceError,
      activeSameWorkdirCount,
    };
  }));
}

export function setWorkbenchWorkspaces(workspaces: Workspace[]) {
  useWorkbenchStore.getState().setWorkspaces(workspaces);
}

export function setWorkspaceCheckouts(workspaceId: string, checkouts: WorkspaceCheckout[]) {
  useWorkbenchStore.getState().setWorkspaceCheckouts(workspaceId, checkouts);
}

export function upsertWorkspaceRegistration(workspace: Workspace, checkout: WorkspaceCheckout) {
  useWorkbenchStore.getState().upsertWorkspaceRegistration(workspace, checkout);
}

export function setWorkspaceError(error: string | null) {
  useWorkbenchStore.getState().setWorkspaceError(error);
}

export function setTabWorkspace(
  tabId: string,
  workspaceId: string | null,
  checkoutId?: string | null,
) {
  useWorkbenchStore.getState().setTabWorkspace(tabId, workspaceId, checkoutId);
}

export function setTabWorkdir(tabId: string, workdir: string) {
  useWorkbenchStore.getState().setTabWorkdir(tabId, workdir);
}
