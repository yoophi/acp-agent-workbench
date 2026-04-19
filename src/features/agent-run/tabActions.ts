import { cancelAgentRun, detachAgentRunTab } from "./api";
import { selectTab, useWorkbenchStore } from "./model";

const CLOSE_FALLBACK_MS = 5000;

export async function closeWorkbenchTab(tabId: string) {
  const store = useWorkbenchStore.getState();
  const tab = selectTab(store, tabId);
  if (!tab) return;

  if (tab.closing && tab.error) {
    store.forceCloseTab(tabId);
    return;
  }
  if (tab.closing) return;

  if (tab.activeRunId && tab.sessionActive) {
    store.closeTab(tabId);
    try {
      await cancelAgentRun(tab.activeRunId);
    } catch (err) {
      useWorkbenchStore.getState().patchTab(tabId, {
        error: `탭 종료 실패: ${String(err)}`,
      });
      return;
    }
    setTimeout(() => {
      const current = selectTab(useWorkbenchStore.getState(), tabId);
      if (current && current.closing && !current.error) {
        useWorkbenchStore.getState().patchTab(tabId, {
          error: "backend lifecycle 이벤트를 받지 못했습니다. 강제로 닫을 수 있습니다.",
        });
      }
    }, CLOSE_FALLBACK_MS);
    return;
  }

  store.closeTab(tabId);
}

export async function detachWorkbenchTab(tabId: string) {
  const store = useWorkbenchStore.getState();
  const tab = selectTab(store, tabId);
  if (!tab || tab.closing) return;

  try {
    await detachAgentRunTab(tab);
    useWorkbenchStore.getState().forceCloseTab(tabId);
  } catch (err) {
    useWorkbenchStore.getState().patchTab(tabId, {
      error: `탭 분리 실패: ${String(err)}`,
    });
  }
}
