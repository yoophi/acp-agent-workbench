import { create } from "zustand";
import type { EventGroup, TimelineItem } from "../../entities/message/model";

const defaultGoal = "todo rest api 를 nodejs 로 작성해주세요. 데이터는 json 파일로 저장해주세요";

export type FollowUpQueueItem = {
  id: string;
  text: string;
  createdAt: number;
};

type AgentRunState = {
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
  setSelectedAgentId: (value: string) => void;
  setGoal: (value: string) => void;
  setCwd: (value: string) => void;
  setCustomCommand: (value: string) => void;
  setStdioBufferLimitMb: (value: number) => void;
  setAutoAllow: (value: boolean) => void;
  setIdleTimeoutSec: (value: number) => void;
  setIdleRemainingSec: (value: number | null) => void;
  setActiveRunId: (value: string | null) => void;
  setSessionActive: (value: boolean) => void;
  setAwaitingResponse: (value: boolean) => void;
  setFollowUpDraft: (value: string) => void;
  enqueueFollowUp: (text: string) => void;
  removeFollowUp: (id: string) => void;
  dequeueFollowUp: () => FollowUpQueueItem | undefined;
  clearFollowUpQueue: () => void;
  setError: (value: string | null) => void;
  setFilter: (value: EventGroup | "all") => void;
  beginRun: () => void;
  endRun: () => void;
  resetTimeline: () => void;
  appendItem: (item: TimelineItem) => void;
};

function createQueueId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2, 8)}`;
}

export const useAgentRunStore = create<AgentRunState>((set, get) => ({
  selectedAgentId: "claude-code",
  goal: defaultGoal,
  cwd: "~/tmp/acp-tauri-agent-workspace",
  customCommand: "",
  stdioBufferLimitMb: 50,
  autoAllow: true,
  idleTimeoutSec: 60,
  idleRemainingSec: null,
  activeRunId: null,
  sessionActive: false,
  awaitingResponse: false,
  followUpDraft: "",
  followUpQueue: [],
  items: [],
  filter: "all",
  error: null,
  setSelectedAgentId: (selectedAgentId) => set({ selectedAgentId }),
  setGoal: (goal) => set({ goal }),
  setCwd: (cwd) => set({ cwd }),
  setCustomCommand: (customCommand) => set({ customCommand }),
  setStdioBufferLimitMb: (stdioBufferLimitMb) => set({ stdioBufferLimitMb }),
  setAutoAllow: (autoAllow) => set({ autoAllow }),
  setIdleTimeoutSec: (idleTimeoutSec) => set({ idleTimeoutSec }),
  setIdleRemainingSec: (idleRemainingSec) => set({ idleRemainingSec }),
  setActiveRunId: (activeRunId) => set({ activeRunId }),
  setSessionActive: (sessionActive) => set({ sessionActive }),
  setAwaitingResponse: (awaitingResponse) => set({ awaitingResponse }),
  setFollowUpDraft: (followUpDraft) => set({ followUpDraft }),
  enqueueFollowUp: (text) =>
    set((state) => ({
      followUpQueue: [
        ...state.followUpQueue,
        { id: createQueueId(), text, createdAt: Date.now() },
      ],
    })),
  removeFollowUp: (id) =>
    set((state) => ({
      followUpQueue: state.followUpQueue.filter((item) => item.id !== id),
    })),
  dequeueFollowUp: () => {
    const [next, ...rest] = get().followUpQueue;
    if (!next) return undefined;
    set({ followUpQueue: rest });
    return next;
  },
  clearFollowUpQueue: () => set({ followUpQueue: [] }),
  setError: (error) => set({ error }),
  setFilter: (filter) => set({ filter }),
  beginRun: () =>
    set({
      activeRunId: null,
      sessionActive: true,
      awaitingResponse: true,
      followUpDraft: "",
      followUpQueue: [],
      idleRemainingSec: null,
      items: [],
      error: null,
    }),
  endRun: () =>
    set({
      sessionActive: false,
      awaitingResponse: false,
      followUpQueue: [],
      idleRemainingSec: null,
    }),
  resetTimeline: () => set({ items: [] }),
  appendItem: (item) =>
    set((state) => {
      const previous = state.items[state.items.length - 1];
      if (
        (previous?.event.type === "agentMessage" && item.event.type === "agentMessage") ||
        (previous?.event.type === "thought" && item.event.type === "thought")
      ) {
        const mergedText = `${previous.event.text}${item.event.text}`;
        return {
          items: [
            ...state.items.slice(0, -1),
            {
              ...previous,
              body: `${previous.body}${item.body}`,
              event: { ...previous.event, text: mergedText },
            },
          ],
        };
      }
      return { items: [...state.items, item] };
    }),
}));
