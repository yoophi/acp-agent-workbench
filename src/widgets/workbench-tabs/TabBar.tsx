import { useCallback } from "react";
import {
  activateWorkbenchTab,
  closeWorkbenchTab,
  createWorkbenchTab,
  detachWorkbenchTab,
  useActiveTabId,
  useTabList,
  type WorkbenchTabListItem,
} from "../../features/agent-run";
import { cn } from "../../shared/lib";
import { Badge, Button } from "../../shared/ui";

type TabStatus =
  | "idle"
  | "running"
  | "awaiting"
  | "error"
  | "idle-countdown"
  | "closing";

function resolveStatus(tab: WorkbenchTabListItem): TabStatus {
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

function tabDisplayTitle(tab: WorkbenchTabListItem) {
  if (tab.title && tab.title.trim().length > 0) return tab.title;
  const goalPreview = tab.goal.trim().split(/\s+/).slice(0, 5).join(" ");
  return goalPreview || "빈 탭";
}

function statusClassName(status: TabStatus) {
  switch (status) {
    case "running":
      return "bg-primary";
    case "awaiting":
      return "animate-pulse bg-info";
    case "error":
      return "bg-destructive";
    case "idle-countdown":
      return "bg-warning";
    case "closing":
      return "animate-pulse bg-muted-foreground";
    default:
      return "bg-muted-foreground/50";
  }
}

export function TabBar() {
  const tabs = useTabList();
  const activeTabId = useActiveTabId();

  const handleActivate = useCallback((tabId: string) => {
    activateWorkbenchTab(tabId);
  }, []);

  const handleAdd = useCallback(() => {
    createWorkbenchTab();
  }, []);

  const handleClose = useCallback((tabId: string) => {
    void closeWorkbenchTab(tabId);
  }, []);

  const handleDetach = useCallback((tabId: string) => {
    void detachWorkbenchTab(tabId);
  }, []);

  return (
    <div
      className="mb-4 flex items-center gap-1.5 overflow-x-auto rounded-lg border bg-card/80 p-1 shadow-sm"
      role="tablist"
    >
      {tabs.map((tab) => {
        const status = resolveStatus(tab);
        const isActive = tab.id === activeTabId;
        return (
          <div
            key={tab.id}
            role="tab"
            aria-selected={isActive}
            className={cn(
              "flex max-w-[220px] cursor-pointer select-none items-center gap-2 whitespace-nowrap rounded-md border border-transparent px-2.5 py-1.5 text-sm transition-colors",
              isActive
                ? "border-border bg-background shadow-sm"
                : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
            )}
            onClick={() => handleActivate(tab.id)}
          >
            <span
              className={cn("h-2 w-2 shrink-0 rounded-full", statusClassName(status))}
              title={statusLabel(status)}
              aria-label={statusLabel(status)}
            />
            <span className="min-w-0 flex-1 overflow-hidden text-ellipsis">
              {tabDisplayTitle(tab)}
            </span>
            {tab.permissionPending ? (
              <Badge variant="secondary" title="권한 요청 대기">
                권한
              </Badge>
            ) : null}
            {!isActive && tab.unreadCount > 0 ? (
              <Badge aria-label={`${tab.unreadCount}개 새 이벤트`}>
                {tab.unreadCount > 99 ? "99+" : tab.unreadCount}
              </Badge>
            ) : null}
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-6 w-6 shrink-0 text-muted-foreground hover:text-foreground"
              aria-label="탭을 새 창으로 분리"
              disabled={tab.closing}
              onClick={(event) => {
                event.stopPropagation();
                void handleDetach(tab.id);
              }}
              title="탭을 새 창으로 분리"
            >
              ↗
            </Button>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="h-6 w-6 shrink-0 text-muted-foreground hover:text-foreground"
              aria-label={
                tab.closing && tab.error
                  ? "강제 닫기"
                  : tab.closing
                    ? "종료 중"
                    : "탭 닫기"
              }
              disabled={tab.closing && !tab.error}
              onClick={(event) => {
                event.stopPropagation();
                void handleClose(tab.id);
              }}
              title={
                tab.closing && tab.error
                  ? "닫기를 재시도하면 강제로 제거합니다"
                  : undefined
              }
            >
              {tab.closing && tab.error ? "!" : tab.closing ? "…" : "×"}
            </Button>
          </div>
        );
      })}
      <Button type="button" variant="outline" size="icon" onClick={handleAdd} aria-label="새 탭">
        +
      </Button>
    </div>
  );
}
