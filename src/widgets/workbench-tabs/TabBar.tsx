import { useCallback } from "react";
import { cancelAgentRun } from "../../shared/api/tauri";
import { useWorkbenchStore, type TabState } from "../../features/agent-run/model";

type TabStatus =
  | "idle"
  | "running"
  | "awaiting"
  | "error"
  | "idle-countdown"
  | "closing";

function resolveStatus(tab: TabState): TabStatus {
  if (tab.closing) return "closing";
  if (tab.error) return "error";
  if (!tab.sessionActive) return "idle";
  if (tab.awaitingResponse) return "awaiting";
  if (tab.idleRemainingSec !== null) return "idle-countdown";
  return "running";
}

function statusLabel(status: TabStatus) {
  switch (status) {
    case "running":
      return "활성";
    case "awaiting":
      return "응답 대기";
    case "error":
      return "오류";
    case "idle-countdown":
      return "idle 카운트다운";
    case "closing":
      return "종료 중";
    default:
      return "대기";
  }
}

function tabDisplayTitle(tab: TabState) {
  if (tab.title && tab.title.trim().length > 0) return tab.title;
  const goalPreview = tab.goal.trim().split(/\s+/).slice(0, 5).join(" ");
  return goalPreview || "빈 탭";
}

export function TabBar() {
  const tabs = useWorkbenchStore((state) => state.tabs);
  const activeTabId = useWorkbenchStore((state) => state.activeTabId);

  const handleActivate = useCallback((tabId: string) => {
    useWorkbenchStore.getState().activateTab(tabId);
  }, []);

  const handleAdd = useCallback(() => {
    useWorkbenchStore.getState().addTab();
  }, []);

  const handleClose = useCallback(async (tabId: string) => {
    const store = useWorkbenchStore.getState();
    const tab = store.tabs.find((t) => t.id === tabId);
    if (!tab) return;
    if (tab.closing) return;
    if (tab.activeRunId && tab.sessionActive) {
      store.closeTab(tabId);
      try {
        await cancelAgentRun(tab.activeRunId);
      } catch (err) {
        useWorkbenchStore.getState().patchTab(tabId, {
          error: `탭 종료 실패: ${String(err)}`,
        });
      }
      return;
    }
    store.closeTab(tabId);
  }, []);

  return (
    <div className="tab-bar" role="tablist">
      {tabs.map((tab) => {
        const status = resolveStatus(tab);
        const isActive = tab.id === activeTabId;
        return (
          <div
            key={tab.id}
            role="tab"
            aria-selected={isActive}
            className={`tab-item ${isActive ? "tab-item--active" : ""}`}
            onClick={() => handleActivate(tab.id)}
          >
            <span
              className={`tab-status tab-status--${status}`}
              title={statusLabel(status)}
              aria-label={statusLabel(status)}
            />
            <span className="tab-title">{tabDisplayTitle(tab)}</span>
            {tab.permissionPending ? (
              <span className="tab-permission" title="권한 요청 대기">
                ⚠
              </span>
            ) : null}
            {!isActive && tab.unreadCount > 0 ? (
              <span className="tab-unread" aria-label={`${tab.unreadCount}개 새 이벤트`}>
                {tab.unreadCount > 99 ? "99+" : tab.unreadCount}
              </span>
            ) : null}
            <button
              type="button"
              className="tab-close"
              aria-label={tab.closing ? "종료 중" : "탭 닫기"}
              disabled={tab.closing}
              onClick={(event) => {
                event.stopPropagation();
                void handleClose(tab.id);
              }}
            >
              {tab.closing ? "…" : "×"}
            </button>
          </div>
        );
      })}
      <button type="button" className="tab-add" onClick={handleAdd} aria-label="새 탭">
        +
      </button>
    </div>
  );
}
