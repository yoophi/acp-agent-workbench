import { create } from "zustand";
import type { EventGroup, TimelineItem } from "../../entities/message/model";

const defaultGoal = "todo rest api 를 nodejs 로 작성해주세요. 데이터는 json 파일로 저장해주세요";

type AgentRunState = {
  selectedAgentId: string;
  goal: string;
  cwd: string;
  customCommand: string;
  stdioBufferLimitMb: number;
  autoAllow: boolean;
  activeRunId: string | null;
  isRunning: boolean;
  items: TimelineItem[];
  filter: EventGroup | "all";
  error: string | null;
  setSelectedAgentId: (value: string) => void;
  setGoal: (value: string) => void;
  setCwd: (value: string) => void;
  setCustomCommand: (value: string) => void;
  setStdioBufferLimitMb: (value: number) => void;
  setAutoAllow: (value: boolean) => void;
  setActiveRunId: (value: string | null) => void;
  setIsRunning: (value: boolean) => void;
  setError: (value: string | null) => void;
  setFilter: (value: EventGroup | "all") => void;
  beginRun: () => void;
  resetTimeline: () => void;
  appendItem: (item: TimelineItem) => void;
};

export const useAgentRunStore = create<AgentRunState>((set) => ({
  selectedAgentId: "claude-code",
  goal: defaultGoal,
  cwd: "~/tmp/acp-tauri-agent-workspace",
  customCommand: "",
  stdioBufferLimitMb: 50,
  autoAllow: true,
  activeRunId: null,
  isRunning: false,
  items: [],
  filter: "all",
  error: null,
  setSelectedAgentId: (selectedAgentId) => set({ selectedAgentId }),
  setGoal: (goal) => set({ goal }),
  setCwd: (cwd) => set({ cwd }),
  setCustomCommand: (customCommand) => set({ customCommand }),
  setStdioBufferLimitMb: (stdioBufferLimitMb) => set({ stdioBufferLimitMb }),
  setAutoAllow: (autoAllow) => set({ autoAllow }),
  setActiveRunId: (activeRunId) => set({ activeRunId }),
  setIsRunning: (isRunning) => set({ isRunning }),
  setError: (error) => set({ error }),
  setFilter: (filter) => set({ filter }),
  beginRun: () => set({ activeRunId: null, isRunning: true, items: [], error: null }),
  resetTimeline: () => set({ items: [] }),
  appendItem: (item) =>
    set((state) => {
      const previous = state.items[state.items.length - 1];
      if (previous?.event.type === "agentMessage" && item.event.type === "agentMessage") {
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
