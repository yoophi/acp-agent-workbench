import { create } from "zustand";
import type { EventGroup, RunEvent, TimelineItem } from "../../entities/message/model";
import { toTimelineItem } from "../../entities/message/format";

const defaultGoal = "todo rest api 를 nodejs 로 작성해주세요. 데이터는 json 파일로 저장해주세요";

export type FollowUpQueueItem = {
  id: string;
  runId: string;
  text: string;
  createdAt: number;
};

export type TabState = {
  id: string;
  title: string;
  selectedAgentId: string;
  goal: string;
  cwd: string;
  customCommand: string;
  stdioBufferLimitMb: number;
  autoAllow: boolean;
  idleTimeoutSec: number;
  idleRemainingSec: number | null;
  activeRunId: string | null;
  sessionActive: boolean;
  awaitingResponse: boolean;
  followUpDraft: string;
  followUpQueue: FollowUpQueueItem[];
  items: TimelineItem[];
  filter: EventGroup | "all";
  error: string | null;
  unreadCount: number;
  permissionPending: boolean;
  closing: boolean;
};

type WorkbenchState = {
  tabs: TabState[];
  activeTabId: string;
  addTab: (preset?: Partial<TabState>) => string;
  closeTab: (tabId: string) => string | null;
  forceCloseTab: (tabId: string) => string | null;
  activateTab: (tabId: string) => void;
  renameTab: (tabId: string, title: string) => void;
  patchTab: (tabId: string, patch: Partial<TabState>) => void;
  enqueueFollowUp: (tabId: string, text: string) => void;
  removeFollowUp: (tabId: string, id: string) => void;
  dequeueFollowUp: (tabId: string) => FollowUpQueueItem | undefined;
  appendItem: (tabId: string, item: TimelineItem) => void;
  beginRun: (tabId: string, runId: string) => void;
  endRun: (tabId: string) => void;
  markClosing: (tabId: string) => void;
  dispatchRunEvent: (runId: string, event: RunEvent) => void;
};

function createId(prefix: string) {
  return `${prefix}-${crypto.randomUUID()}`;
}

function defaultTabTitle(index: number) {
  return `Tab ${index + 1}`;
}

export function createTabState(preset: Partial<TabState> = {}, index = 0): TabState {
  return {
    id: preset.id ?? createId("tab"),
    title: preset.title ?? defaultTabTitle(index),
    selectedAgentId: preset.selectedAgentId ?? "claude-code",
    goal: preset.goal ?? defaultGoal,
    cwd: preset.cwd ?? "~/tmp/acp-tauri-agent-workspace",
    customCommand: preset.customCommand ?? "",
    stdioBufferLimitMb: preset.stdioBufferLimitMb ?? 50,
    autoAllow: preset.autoAllow ?? true,
    idleTimeoutSec: preset.idleTimeoutSec ?? 60,
    idleRemainingSec: preset.idleRemainingSec ?? null,
    activeRunId: preset.activeRunId ?? null,
    sessionActive: preset.sessionActive ?? false,
    awaitingResponse: preset.awaitingResponse ?? false,
    followUpDraft: preset.followUpDraft ?? "",
    followUpQueue: preset.followUpQueue ?? [],
    items: preset.items ?? [],
    filter: preset.filter ?? "all",
    error: preset.error ?? null,
    unreadCount: preset.unreadCount ?? 0,
    permissionPending: preset.permissionPending ?? false,
    closing: preset.closing ?? false,
  };
}

function replaceTab(tabs: TabState[], tabId: string, updater: (tab: TabState) => TabState) {
  return tabs.map((tab) => (tab.id === tabId ? updater(tab) : tab));
}

function mergeStreamedText(items: TimelineItem[], item: TimelineItem): TimelineItem[] {
  const previous = items[items.length - 1];
  const canMerge =
    (previous?.event.type === "agentMessage" && item.event.type === "agentMessage") ||
    (previous?.event.type === "thought" && item.event.type === "thought");
  if (previous && canMerge) {
    const previousText = (previous.event as { text: string }).text;
    const incomingText = (item.event as { text: string }).text;
    const mergedText = `${previousText}${incomingText}`;
    return [
      ...items.slice(0, -1),
      {
        ...previous,
        body: `${previous.body}${item.body}`,
        event: { ...previous.event, text: mergedText } as typeof previous.event,
      },
    ];
  }
  return [...items, item];
}

const initialTab = createTabState({}, 0);

export const useWorkbenchStore = create<WorkbenchState>((set, get) => ({
  tabs: [initialTab],
  activeTabId: initialTab.id,

  addTab: (preset) => {
    const state = get();
    const tab = createTabState(preset, state.tabs.length);
    set({ tabs: [...state.tabs, tab], activeTabId: tab.id });
    return tab.id;
  },

  closeTab: (tabId) => {
    const state = get();
    const target = state.tabs.find((t) => t.id === tabId);
    if (!target) return state.activeTabId;

    if (target.sessionActive && target.activeRunId) {
      set({
        tabs: replaceTab(state.tabs, tabId, (tab) => ({ ...tab, closing: true })),
      });
      return state.activeTabId;
    }

    if (state.tabs.length <= 1) {
      const replacement = createTabState({}, 0);
      set({ tabs: [replacement], activeTabId: replacement.id });
      return replacement.id;
    }
    const index = state.tabs.findIndex((t) => t.id === tabId);
    const remaining = state.tabs.filter((t) => t.id !== tabId);
    let nextActive = state.activeTabId;
    if (state.activeTabId === tabId) {
      const neighbor = remaining[index] ?? remaining[index - 1] ?? remaining[0];
      nextActive = neighbor.id;
    }
    set({ tabs: remaining, activeTabId: nextActive });
    return nextActive;
  },

  forceCloseTab: (tabId) => {
    const state = get();
    if (!state.tabs.some((t) => t.id === tabId)) return state.activeTabId;
    if (state.tabs.length <= 1) {
      const replacement = createTabState({}, 0);
      set({ tabs: [replacement], activeTabId: replacement.id });
      return replacement.id;
    }
    const index = state.tabs.findIndex((t) => t.id === tabId);
    const remaining = state.tabs.filter((t) => t.id !== tabId);
    let nextActive = state.activeTabId;
    if (state.activeTabId === tabId) {
      const neighbor = remaining[index] ?? remaining[index - 1] ?? remaining[0];
      nextActive = neighbor.id;
    }
    set({ tabs: remaining, activeTabId: nextActive });
    return nextActive;
  },

  activateTab: (tabId) =>
    set((state) => {
      if (!state.tabs.some((t) => t.id === tabId)) return state;
      return {
        activeTabId: tabId,
        tabs: replaceTab(state.tabs, tabId, (tab) => ({
          ...tab,
          unreadCount: 0,
        })),
      };
    }),

  renameTab: (tabId, title) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({ ...tab, title })),
    })),

  patchTab: (tabId, patch) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({ ...tab, ...patch })),
    })),

  enqueueFollowUp: (tabId, text) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => {
        if (!tab.activeRunId) return tab;
        return {
          ...tab,
          followUpQueue: [
            ...tab.followUpQueue,
            {
              id: createId("q"),
              runId: tab.activeRunId,
              text,
              createdAt: Date.now(),
            },
          ],
        };
      }),
    })),

  removeFollowUp: (tabId, id) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({
        ...tab,
        followUpQueue: tab.followUpQueue.filter((entry) => entry.id !== id),
      })),
    })),

  dequeueFollowUp: (tabId) => {
    const tab = get().tabs.find((t) => t.id === tabId);
    const [next, ...rest] = tab?.followUpQueue ?? [];
    if (!next) return undefined;
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (t) => ({ ...t, followUpQueue: rest })),
    }));
    return next;
  },

  appendItem: (tabId, item) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({
        ...tab,
        items: mergeStreamedText(tab.items, item),
      })),
    })),

  beginRun: (tabId, runId) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({
        ...tab,
        activeRunId: runId,
        sessionActive: true,
        awaitingResponse: true,
        followUpDraft: "",
        followUpQueue: [],
        idleRemainingSec: null,
        items: [],
        error: null,
        unreadCount: 0,
        permissionPending: false,
        closing: false,
      })),
    })),

  endRun: (tabId) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({
        ...tab,
        sessionActive: false,
        awaitingResponse: false,
        followUpQueue: [],
        idleRemainingSec: null,
        permissionPending: false,
      })),
    })),

  markClosing: (tabId) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({ ...tab, closing: true })),
    })),

  dispatchRunEvent: (runId, event) => {
    const state = get();
    const tab = state.tabs.find((t) => t.activeRunId === runId);
    if (!tab) return;
    const item = toTimelineItem(runId, event);
    const isActive = state.activeTabId === tab.id;
    const terminal =
      (event.type === "lifecycle" &&
        (event.status === "completed" || event.status === "cancelled")) ||
      event.type === "error";

    set((current) => ({
      tabs: replaceTab(current.tabs, tab.id, (t) => {
        const nextItems = mergeStreamedText(t.items, item);
        let awaitingResponse = t.awaitingResponse;
        let sessionActive = t.sessionActive;
        let error = t.error;
        let permissionPending = t.permissionPending;
        let idleRemainingSec = t.idleRemainingSec;

        if (event.type === "lifecycle") {
          if (event.status === "promptSent") {
            awaitingResponse = true;
            idleRemainingSec = null;
          } else if (event.status === "promptCompleted") {
            awaitingResponse = false;
          } else if (event.status === "completed" || event.status === "cancelled") {
            sessionActive = false;
            awaitingResponse = false;
            idleRemainingSec = null;
            permissionPending = false;
          }
        } else if (event.type === "error") {
          sessionActive = false;
          awaitingResponse = false;
          idleRemainingSec = null;
          error = event.message;
          permissionPending = false;
        } else if (event.type === "permission") {
          permissionPending = event.requiresResponse;
        }

        const nextUnread = isActive ? t.unreadCount : t.unreadCount + 1;
        return {
          ...t,
          items: nextItems,
          awaitingResponse,
          sessionActive,
          error,
          permissionPending,
          idleRemainingSec,
          unreadCount: nextUnread,
        };
      }),
    }));

    if (terminal && tab.closing) {
      const after = get();
      const target = after.tabs.find((t) => t.id === tab.id);
      if (!target) return;
      if (after.tabs.length <= 1) {
        const replacement = createTabState({}, 0);
        set({ tabs: [replacement], activeTabId: replacement.id });
        return;
      }
      const index = after.tabs.findIndex((t) => t.id === tab.id);
      const remaining = after.tabs.filter((t) => t.id !== tab.id);
      let nextActive = after.activeTabId;
      if (after.activeTabId === tab.id) {
        const neighbor = remaining[index] ?? remaining[index - 1] ?? remaining[0];
        nextActive = neighbor.id;
      }
      set({ tabs: remaining, activeTabId: nextActive });
    }
  },
}));

export function selectTab(state: WorkbenchState, tabId: string): TabState | undefined {
  return state.tabs.find((t) => t.id === tabId);
}
