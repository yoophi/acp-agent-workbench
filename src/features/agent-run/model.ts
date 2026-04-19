import { create } from "zustand";
import {
  toTimelineItem,
  type EventGroup,
  type RalphLoopSettings,
  type ResumePolicy,
  type RunEvent,
  type TimelineItem,
} from "../../entities/message";
import type { Workspace, WorkspaceCheckout } from "../../entities/workspace";

const defaultGoal = "todo rest api 를 nodejs 로 작성해주세요. 데이터는 json 파일로 저장해주세요";
const EMPTY_FOLLOW_UP_QUEUE: FollowUpQueueItem[] = [];
const EMPTY_TIMELINE_ITEMS: TimelineItem[] = [];
export const defaultRalphLoopSettings: RalphLoopSettings = {
  enabled: false,
  maxIterations: 3,
  promptTemplate: "Continue from the previous result. If the task is complete, say so clearly.",
  stopOnError: true,
  stopOnPermission: true,
  delayMs: 0,
};

export type FollowUpQueueItem = {
  id: string;
  runId: string;
  text: string;
  createdAt: number;
};

export type AgentRunDraft = {
  selectedAgentId: string;
  goal: string;
  customCommand: string;
  stdioBufferLimitMb: number;
  autoAllow: boolean;
  resumePolicy: ResumePolicy;
  ralphLoop: RalphLoopSettings;
  idleTimeoutSec: number;
};

export type WorkspaceViewState = {
  id: string;
  title: string;
  workspaceId: string | null;
  checkoutId: string | null;
  cwd: string;
  activeRunId: string | null;
  draft: AgentRunDraft;
  followUpDraft: string;
  filter: EventGroup | "all";
  viewError: string | null;
  unreadCount: number;
  closing: boolean;
};

export type AgentRunState = {
  id: string;
  workspaceViewId: string;
  workspaceId: string | null;
  checkoutId: string | null;
  cwd: string;
  request: AgentRunDraft;
  sessionActive: boolean;
  awaitingResponse: boolean;
  idleRemainingSec: number | null;
  permissionPending: boolean;
  followUpQueue: FollowUpQueueItem[];
  items: TimelineItem[];
  runError: string | null;
  createdAt: number;
  completedAt: number | null;
};

export type TabState = {
  id: string;
  title: string;
  workspaceId: string | null;
  checkoutId: string | null;
  selectedAgentId: string;
  goal: string;
  cwd: string;
  customCommand: string;
  stdioBufferLimitMb: number;
  autoAllow: boolean;
  resumePolicy: ResumePolicy;
  ralphLoop: RalphLoopSettings;
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
  workspaces: Workspace[];
  checkoutsByWorkspaceId: Record<string, WorkspaceCheckout[]>;
  workspaceError: string | null;
  workspaceViews: WorkspaceViewState[];
  runsById: Record<string, AgentRunState>;
  activeWorkspaceViewId: string;
  tabs: TabState[];
  activeTabId: string;
  setWorkspaces: (workspaces: Workspace[]) => void;
  setWorkspaceCheckouts: (workspaceId: string, checkouts: WorkspaceCheckout[]) => void;
  upsertWorkspaceRegistration: (workspace: Workspace, checkout: WorkspaceCheckout) => void;
  setWorkspaceError: (error: string | null) => void;
  addTab: (preset?: Partial<TabState>) => string;
  closeTab: (tabId: string) => string | null;
  forceCloseTab: (tabId: string) => string | null;
  activateTab: (tabId: string) => void;
  renameTab: (tabId: string, title: string) => void;
  patchTab: (tabId: string, patch: Partial<TabState>) => void;
  setTabWorkspace: (tabId: string, workspaceId: string | null, checkoutId?: string | null) => void;
  setTabWorkdir: (tabId: string, workdir: string) => void;
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
    workspaceId: preset.workspaceId ?? null,
    checkoutId: preset.checkoutId ?? null,
    selectedAgentId: preset.selectedAgentId ?? "claude-code",
    goal: preset.goal ?? defaultGoal,
    cwd: preset.cwd ?? "~/tmp/acp-tauri-agent-workspace",
    customCommand: preset.customCommand ?? "",
    stdioBufferLimitMb: preset.stdioBufferLimitMb ?? 50,
    autoAllow: preset.autoAllow ?? true,
    resumePolicy: preset.resumePolicy ?? "fresh",
    ralphLoop: preset.ralphLoop ?? { ...defaultRalphLoopSettings },
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

function tabToDraft(tab: TabState): AgentRunDraft {
  return {
    selectedAgentId: tab.selectedAgentId,
    goal: tab.goal,
    customCommand: tab.customCommand,
    stdioBufferLimitMb: tab.stdioBufferLimitMb,
    autoAllow: tab.autoAllow,
    resumePolicy: tab.resumePolicy,
    ralphLoop: tab.ralphLoop,
    idleTimeoutSec: tab.idleTimeoutSec,
  };
}

export function createWorkspaceViewState(
  preset: Partial<TabState> = {},
  index = 0,
): WorkspaceViewState {
  return tabToWorkspaceView(createTabState(preset, index));
}

function tabToWorkspaceView(tab: TabState): WorkspaceViewState {
  return {
    id: tab.id,
    title: tab.title,
    workspaceId: tab.workspaceId,
    checkoutId: tab.checkoutId,
    cwd: tab.cwd,
    activeRunId: tab.activeRunId,
    draft: tabToDraft(tab),
    followUpDraft: tab.followUpDraft,
    filter: tab.filter,
    viewError: tab.error,
    unreadCount: tab.unreadCount,
    closing: tab.closing,
  };
}

function runStateFromTab(tab: TabState, runId: string): AgentRunState {
  return {
    id: runId,
    workspaceViewId: tab.id,
    workspaceId: tab.workspaceId,
    checkoutId: tab.checkoutId,
    cwd: tab.cwd,
    request: tabToDraft(tab),
    sessionActive: true,
    awaitingResponse: true,
    idleRemainingSec: null,
    permissionPending: false,
    followUpQueue: [],
    items: [],
    runError: null,
    createdAt: Date.now(),
    completedAt: null,
  };
}

function replaceTab(tabs: TabState[], tabId: string, updater: (tab: TabState) => TabState) {
  return tabs.map((tab) => (tab.id === tabId ? updater(tab) : tab));
}

function replaceWorkspaceView(
  views: WorkspaceViewState[],
  viewId: string,
  updater: (view: WorkspaceViewState) => WorkspaceViewState,
) {
  return views.map((view) => (view.id === viewId ? updater(view) : view));
}

function patchRunForTab(
  state: WorkbenchState,
  tabId: string,
  patch: Partial<TabState>,
): Record<string, AgentRunState> {
  const tab = state.tabs.find((entry) => entry.id === tabId);
  if (!tab?.activeRunId || patch.activeRunId === null) return state.runsById;
  const run = state.runsById[tab.activeRunId];
  if (!run) return state.runsById;
  const next: AgentRunState = { ...run };
  if (patch.sessionActive !== undefined) next.sessionActive = patch.sessionActive;
  if (patch.awaitingResponse !== undefined) next.awaitingResponse = patch.awaitingResponse;
  if (patch.idleRemainingSec !== undefined) next.idleRemainingSec = patch.idleRemainingSec;
  if (patch.permissionPending !== undefined) next.permissionPending = patch.permissionPending;
  if (patch.followUpQueue !== undefined) next.followUpQueue = patch.followUpQueue;
  if (patch.items !== undefined) next.items = patch.items;
  if (patch.error !== undefined) next.runError = patch.error;
  return { ...state.runsById, [run.id]: next };
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
const initialWorkspaceView = tabToWorkspaceView(initialTab);

export const useWorkbenchStore = create<WorkbenchState>((set, get) => ({
  workspaces: [],
  checkoutsByWorkspaceId: {},
  workspaceError: null,
  workspaceViews: [initialWorkspaceView],
  runsById: {},
  activeWorkspaceViewId: initialWorkspaceView.id,
  tabs: [initialTab],
  activeTabId: initialTab.id,

  setWorkspaces: (workspaces) =>
    set((state) => {
      const knownIds = new Set(workspaces.map((workspace) => workspace.id));
      return {
        workspaces,
        checkoutsByWorkspaceId: Object.fromEntries(
          Object.entries(state.checkoutsByWorkspaceId).filter(([workspaceId]) =>
            knownIds.has(workspaceId),
          ),
        ),
      };
    }),

  setWorkspaceCheckouts: (workspaceId, checkouts) =>
    set((state) => ({
      checkoutsByWorkspaceId: {
        ...state.checkoutsByWorkspaceId,
        [workspaceId]: checkouts,
      },
    })),

  upsertWorkspaceRegistration: (workspace, checkout) =>
    set((state) => {
      const workspaces = upsertItem(state.workspaces, workspace, (entry) => entry.id);
      const currentCheckouts = state.checkoutsByWorkspaceId[workspace.id] ?? [];
      return {
        workspaces,
        checkoutsByWorkspaceId: {
          ...state.checkoutsByWorkspaceId,
          [workspace.id]: upsertItem(currentCheckouts, checkout, (entry) => entry.id),
        },
        workspaceError: null,
      };
    }),

  setWorkspaceError: (workspaceError) => set({ workspaceError }),

  addTab: (preset) => {
    const state = get();
    const activeTab = state.tabs.find((entry) => entry.id === state.activeTabId);
    const tab = createTabState(
      {
        workspaceId: activeTab?.workspaceId ?? null,
        checkoutId: activeTab?.checkoutId ?? null,
        cwd: activeTab?.cwd,
        ...preset,
      },
      state.tabs.length,
    );
    const workspaceView = tabToWorkspaceView(tab);
    set({
      tabs: [...state.tabs, tab],
      activeTabId: tab.id,
      workspaceViews: [...state.workspaceViews, workspaceView],
      activeWorkspaceViewId: workspaceView.id,
    });
    return tab.id;
  },

  closeTab: (tabId) => {
    const state = get();
    const target = state.tabs.find((t) => t.id === tabId);
    if (!target) return state.activeTabId;

    if (target.sessionActive && target.activeRunId) {
      set({
        tabs: replaceTab(state.tabs, tabId, (tab) => ({ ...tab, closing: true })),
        workspaceViews: replaceWorkspaceView(state.workspaceViews, tabId, (view) => ({
          ...view,
          closing: true,
        })),
      });
      return state.activeTabId;
    }

    if (state.tabs.length <= 1) {
      const replacement = createTabState({}, 0);
      const replacementView = tabToWorkspaceView(replacement);
      set({
        tabs: [replacement],
        activeTabId: replacement.id,
        workspaceViews: [replacementView],
        activeWorkspaceViewId: replacementView.id,
      });
      return replacement.id;
    }
    const index = state.tabs.findIndex((t) => t.id === tabId);
    const remaining = state.tabs.filter((t) => t.id !== tabId);
    let nextActive = state.activeTabId;
    if (state.activeTabId === tabId) {
      const neighbor = remaining[index] ?? remaining[index - 1] ?? remaining[0];
      nextActive = neighbor.id;
    }
    set({
      tabs: remaining,
      activeTabId: nextActive,
      workspaceViews: state.workspaceViews.filter((view) => view.id !== tabId),
      activeWorkspaceViewId: nextActive,
    });
    return nextActive;
  },

  forceCloseTab: (tabId) => {
    const state = get();
    if (!state.tabs.some((t) => t.id === tabId)) return state.activeTabId;
    if (state.tabs.length <= 1) {
      const replacement = createTabState({}, 0);
      const replacementView = tabToWorkspaceView(replacement);
      set({
        tabs: [replacement],
        activeTabId: replacement.id,
        workspaceViews: [replacementView],
        activeWorkspaceViewId: replacementView.id,
      });
      return replacement.id;
    }
    const index = state.tabs.findIndex((t) => t.id === tabId);
    const remaining = state.tabs.filter((t) => t.id !== tabId);
    let nextActive = state.activeTabId;
    if (state.activeTabId === tabId) {
      const neighbor = remaining[index] ?? remaining[index - 1] ?? remaining[0];
      nextActive = neighbor.id;
    }
    set({
      tabs: remaining,
      activeTabId: nextActive,
      workspaceViews: state.workspaceViews.filter((view) => view.id !== tabId),
      activeWorkspaceViewId: nextActive,
    });
    return nextActive;
  },

  activateTab: (tabId) =>
    set((state) => {
      if (!state.tabs.some((t) => t.id === tabId)) return state;
      return {
        activeTabId: tabId,
        activeWorkspaceViewId: tabId,
        tabs: replaceTab(state.tabs, tabId, (tab) => ({
          ...tab,
          unreadCount: 0,
        })),
        workspaceViews: replaceWorkspaceView(state.workspaceViews, tabId, (view) => ({
          ...view,
          unreadCount: 0,
        })),
      };
    }),

  renameTab: (tabId, title) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({ ...tab, title })),
      workspaceViews: replaceWorkspaceView(state.workspaceViews, tabId, (view) => ({
        ...view,
        title,
      })),
    })),

  patchTab: (tabId, patch) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({ ...tab, ...patch })),
      workspaceViews: replaceWorkspaceView(state.workspaceViews, tabId, (view) => {
        const next = { ...view };
        if (patch.title !== undefined) next.title = patch.title;
        if (patch.workspaceId !== undefined) next.workspaceId = patch.workspaceId;
        if (patch.checkoutId !== undefined) next.checkoutId = patch.checkoutId;
        if (patch.cwd !== undefined) next.cwd = patch.cwd;
        if (patch.activeRunId !== undefined) next.activeRunId = patch.activeRunId;
        if (patch.followUpDraft !== undefined) next.followUpDraft = patch.followUpDraft;
        if (patch.filter !== undefined) next.filter = patch.filter;
        if (patch.error !== undefined) next.viewError = patch.error;
        if (patch.unreadCount !== undefined) next.unreadCount = patch.unreadCount;
        if (patch.closing !== undefined) next.closing = patch.closing;
        next.draft = {
          ...next.draft,
          selectedAgentId: patch.selectedAgentId ?? next.draft.selectedAgentId,
          goal: patch.goal ?? next.draft.goal,
          customCommand: patch.customCommand ?? next.draft.customCommand,
          stdioBufferLimitMb: patch.stdioBufferLimitMb ?? next.draft.stdioBufferLimitMb,
          autoAllow: patch.autoAllow ?? next.draft.autoAllow,
          resumePolicy: patch.resumePolicy ?? next.draft.resumePolicy,
          ralphLoop: patch.ralphLoop ?? next.draft.ralphLoop,
          idleTimeoutSec: patch.idleTimeoutSec ?? next.draft.idleTimeoutSec,
        };
        return next;
      }),
      runsById: patchRunForTab(state, tabId, patch),
    })),

  setTabWorkspace: (tabId, workspaceId, checkoutId) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => {
        const nextWorkspaceId = workspaceId;
        const checkouts = nextWorkspaceId
          ? (state.checkoutsByWorkspaceId[nextWorkspaceId] ?? [])
          : [];
        const selectedCheckout =
          checkoutId ??
          checkouts.find((entry) => entry.isDefault)?.id ??
          checkouts[0]?.id ??
          null;
        const checkout = checkouts.find((entry) => entry.id === selectedCheckout);
        const nextCwd = checkout?.path ?? tab.cwd;
        return {
          ...tab,
          workspaceId: nextWorkspaceId,
          checkoutId: selectedCheckout,
          cwd: nextCwd,
        };
      }),
      workspaceViews: replaceWorkspaceView(state.workspaceViews, tabId, (view) => {
        const nextWorkspaceId = workspaceId;
        const checkouts = nextWorkspaceId
          ? (state.checkoutsByWorkspaceId[nextWorkspaceId] ?? [])
          : [];
        const selectedCheckout =
          checkoutId ??
          checkouts.find((entry) => entry.isDefault)?.id ??
          checkouts[0]?.id ??
          null;
        const checkout = checkouts.find((entry) => entry.id === selectedCheckout);
        return {
          ...view,
          workspaceId: nextWorkspaceId,
          checkoutId: selectedCheckout,
          cwd: checkout?.path ?? view.cwd,
        };
      }),
    })),

  setTabWorkdir: (tabId, workdir) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({ ...tab, cwd: workdir })),
      workspaceViews: replaceWorkspaceView(state.workspaceViews, tabId, (view) => ({
        ...view,
        cwd: workdir,
      })),
    })),

  enqueueFollowUp: (tabId, text) =>
    set((state) => {
      const tab = state.tabs.find((entry) => entry.id === tabId);
      if (!tab?.activeRunId) return state;
      const item = {
        id: createId("q"),
        runId: tab.activeRunId,
        text,
        createdAt: Date.now(),
      };
      const run = state.runsById[tab.activeRunId];
      return {
        tabs: replaceTab(state.tabs, tabId, (tab) => {
          if (!tab.activeRunId) return tab;
          return {
            ...tab,
            followUpQueue: [...tab.followUpQueue, item],
          };
        }),
        runsById: run
          ? {
              ...state.runsById,
              [run.id]: {
                ...run,
                followUpQueue: [...run.followUpQueue, item],
              },
            }
          : state.runsById,
      };
    }),

  removeFollowUp: (tabId, id) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({
        ...tab,
        followUpQueue: tab.followUpQueue.filter((entry) => entry.id !== id),
      })),
      runsById: (() => {
        const tab = state.tabs.find((entry) => entry.id === tabId);
        if (!tab?.activeRunId) return state.runsById;
        const run = state.runsById[tab.activeRunId];
        if (!run) return state.runsById;
        return {
          ...state.runsById,
          [run.id]: {
            ...run,
            followUpQueue: run.followUpQueue.filter((entry) => entry.id !== id),
          },
        };
      })(),
    })),

  dequeueFollowUp: (tabId) => {
    const tab = get().tabs.find((t) => t.id === tabId);
    const [next, ...rest] = tab?.followUpQueue ?? [];
    if (!next) return undefined;
    set((state) => {
      const current = state.tabs.find((t) => t.id === tabId);
      const run = current?.activeRunId ? state.runsById[current.activeRunId] : undefined;
      return {
        tabs: replaceTab(state.tabs, tabId, (t) => ({ ...t, followUpQueue: rest })),
        runsById: run
          ? {
              ...state.runsById,
              [run.id]: {
                ...run,
                followUpQueue: rest,
              },
            }
          : state.runsById,
      };
    });
    return next;
  },

  appendItem: (tabId, item) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({
        ...tab,
        items: mergeStreamedText(tab.items, item),
      })),
      runsById: (() => {
        const tab = state.tabs.find((entry) => entry.id === tabId);
        if (!tab?.activeRunId) return state.runsById;
        const run = state.runsById[tab.activeRunId];
        if (!run) return state.runsById;
        return {
          ...state.runsById,
          [run.id]: {
            ...run,
            items: mergeStreamedText(run.items, item),
          },
        };
      })(),
    })),

  beginRun: (tabId, runId) =>
    set((state) => {
      const tab = state.tabs.find((entry) => entry.id === tabId);
      if (!tab) return state;
      return {
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
        workspaceViews: replaceWorkspaceView(state.workspaceViews, tabId, (view) => ({
          ...view,
          activeRunId: runId,
          followUpDraft: "",
          viewError: null,
          unreadCount: 0,
          closing: false,
        })),
        runsById: {
          ...state.runsById,
          [runId]: runStateFromTab(tab, runId),
        },
      };
    }),

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
      runsById: (() => {
        const tab = state.tabs.find((entry) => entry.id === tabId);
        if (!tab?.activeRunId) return state.runsById;
        const run = state.runsById[tab.activeRunId];
        if (!run) return state.runsById;
        return {
          ...state.runsById,
          [run.id]: {
            ...run,
            sessionActive: false,
            awaitingResponse: false,
            followUpQueue: [],
            idleRemainingSec: null,
            permissionPending: false,
            completedAt: run.completedAt ?? Date.now(),
          },
        };
      })(),
    })),

  markClosing: (tabId) =>
    set((state) => ({
      tabs: replaceTab(state.tabs, tabId, (tab) => ({ ...tab, closing: true })),
      workspaceViews: replaceWorkspaceView(state.workspaceViews, tabId, (view) => ({
        ...view,
        closing: true,
      })),
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
      workspaceViews: replaceWorkspaceView(current.workspaceViews, tab.id, (view) => ({
        ...view,
        viewError: event.type === "error" ? event.message : view.viewError,
        unreadCount: isActive ? view.unreadCount : view.unreadCount + 1,
      })),
      runsById: (() => {
        const run = current.runsById[runId];
        if (!run) return current.runsById;
        let awaitingResponse = run.awaitingResponse;
        let sessionActive = run.sessionActive;
        let runError = run.runError;
        let permissionPending = run.permissionPending;
        let idleRemainingSec = run.idleRemainingSec;
        let completedAt = run.completedAt;

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
            completedAt = completedAt ?? Date.now();
          }
        } else if (event.type === "error") {
          sessionActive = false;
          awaitingResponse = false;
          idleRemainingSec = null;
          runError = event.message;
          permissionPending = false;
          completedAt = completedAt ?? Date.now();
        } else if (event.type === "permission") {
          permissionPending = event.requiresResponse;
        }

        return {
          ...current.runsById,
          [runId]: {
            ...run,
            items: mergeStreamedText(run.items, item),
            awaitingResponse,
            sessionActive,
            runError,
            permissionPending,
            idleRemainingSec,
            completedAt,
          },
        };
      })(),
    }));

    if (terminal && tab.closing) {
      const after = get();
      const target = after.tabs.find((t) => t.id === tab.id);
      if (!target) return;
      if (after.tabs.length <= 1) {
        const replacement = createTabState({}, 0);
        const replacementView = tabToWorkspaceView(replacement);
        set({
          tabs: [replacement],
          activeTabId: replacement.id,
          workspaceViews: [replacementView],
          activeWorkspaceViewId: replacementView.id,
        });
        return;
      }
      const index = after.tabs.findIndex((t) => t.id === tab.id);
      const remaining = after.tabs.filter((t) => t.id !== tab.id);
      let nextActive = after.activeTabId;
      if (after.activeTabId === tab.id) {
        const neighbor = remaining[index] ?? remaining[index - 1] ?? remaining[0];
        nextActive = neighbor.id;
      }
      set({
        tabs: remaining,
        activeTabId: nextActive,
        workspaceViews: after.workspaceViews.filter((view) => view.id !== tab.id),
        activeWorkspaceViewId: nextActive,
      });
    }
  },
}));

export function selectWorkspaceView(
  state: WorkbenchState,
  workspaceViewId: string,
): WorkspaceViewState | undefined {
  return state.workspaceViews.find((view) => view.id === workspaceViewId);
}

export function selectActiveWorkspaceView(state: WorkbenchState): WorkspaceViewState | undefined {
  return selectWorkspaceView(state, state.activeWorkspaceViewId);
}

export function selectRun(state: WorkbenchState, runId: string): AgentRunState | undefined {
  return state.runsById[runId];
}

export function selectActiveRun(
  state: WorkbenchState,
  workspaceViewId: string,
): AgentRunState | undefined {
  const view = selectWorkspaceView(state, workspaceViewId);
  return view?.activeRunId ? selectRun(state, view.activeRunId) : undefined;
}

export function selectWorkspaceViewRuns(
  state: WorkbenchState,
  workspaceViewId: string,
): AgentRunState[] {
  return Object.values(state.runsById).filter((run) => run.workspaceViewId === workspaceViewId);
}

export function selectTab(state: WorkbenchState, tabId: string): TabState | undefined {
  const cached = selectTabCache.get(tabId);
  if (
    cached &&
    cached.workspaceViews === state.workspaceViews &&
    cached.runsById === state.runsById &&
    cached.tabs === state.tabs
  ) {
    return cached.result;
  }

  const view = selectWorkspaceView(state, tabId);
  if (!view) {
    const result = state.tabs.find((tab) => tab.id === tabId);
    selectTabCache.set(tabId, {
      workspaceViews: state.workspaceViews,
      runsById: state.runsById,
      tabs: state.tabs,
      result,
    });
    return result;
  }
  const activeRun = view.activeRunId ? selectRun(state, view.activeRunId) : undefined;
  const fallback = state.tabs.find((tab) => tab.id === tabId);
  const result = workspaceViewToTabState(view, activeRun, fallback);
  selectTabCache.set(tabId, {
    workspaceViews: state.workspaceViews,
    runsById: state.runsById,
    tabs: state.tabs,
    result,
  });
  return result;
}

export function selectTabList(state: WorkbenchState): TabState[] {
  if (
    selectTabListCache.workspaceViews === state.workspaceViews &&
    selectTabListCache.runsById === state.runsById &&
    selectTabListCache.tabs === state.tabs
  ) {
    return selectTabListCache.result;
  }

  if (state.workspaceViews.length === 0) {
    selectTabListCache = {
      workspaceViews: state.workspaceViews,
      runsById: state.runsById,
      tabs: state.tabs,
      result: state.tabs,
    };
    return state.tabs;
  }
  const result = state.workspaceViews.map((view) =>
    workspaceViewToTabState(
      view,
      view.activeRunId ? selectRun(state, view.activeRunId) : undefined,
      state.tabs.find((tab) => tab.id === view.id),
    ),
  );
  selectTabListCache = {
    workspaceViews: state.workspaceViews,
    runsById: state.runsById,
    tabs: state.tabs,
    result,
  };
  return result;
}

function workspaceViewToTabState(
  view: WorkspaceViewState,
  run: AgentRunState | undefined,
  fallback?: TabState,
): TabState {
  return {
    id: view.id,
    title: view.title,
    workspaceId: view.workspaceId,
    checkoutId: view.checkoutId,
    selectedAgentId: view.draft.selectedAgentId,
    goal: view.draft.goal,
    cwd: view.cwd,
    customCommand: view.draft.customCommand,
    stdioBufferLimitMb: view.draft.stdioBufferLimitMb,
    autoAllow: view.draft.autoAllow,
    resumePolicy: view.draft.resumePolicy,
    ralphLoop: view.draft.ralphLoop,
    idleTimeoutSec: view.draft.idleTimeoutSec,
    idleRemainingSec: run?.idleRemainingSec ?? fallback?.idleRemainingSec ?? null,
    activeRunId: view.activeRunId,
    sessionActive: run?.sessionActive ?? fallback?.sessionActive ?? false,
    awaitingResponse: run?.awaitingResponse ?? fallback?.awaitingResponse ?? false,
    followUpDraft: view.followUpDraft,
    followUpQueue: run?.followUpQueue ?? fallback?.followUpQueue ?? EMPTY_FOLLOW_UP_QUEUE,
    items: run?.items ?? fallback?.items ?? EMPTY_TIMELINE_ITEMS,
    filter: view.filter,
    error: run?.runError ?? view.viewError,
    unreadCount: view.unreadCount,
    permissionPending: run?.permissionPending ?? fallback?.permissionPending ?? false,
    closing: view.closing,
  };
}

type SelectorCacheRefs = Pick<WorkbenchState, "workspaceViews" | "runsById" | "tabs">;

type SelectTabCacheEntry = SelectorCacheRefs & {
  result: TabState | undefined;
};

let selectTabListCache: SelectorCacheRefs & { result: TabState[] } = {
  workspaceViews: [],
  runsById: {},
  tabs: [],
  result: [],
};
const selectTabCache = new Map<string, SelectTabCacheEntry>();

function upsertItem<T>(items: T[], item: T, getId: (item: T) => string): T[] {
  const id = getId(item);
  const index = items.findIndex((entry) => getId(entry) === id);
  if (index === -1) return [...items, item];
  return [...items.slice(0, index), item, ...items.slice(index + 1)];
}
