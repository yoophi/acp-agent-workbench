import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo } from "react";
import {
  cancelAgentRun,
  listAgents,
  startAgentRun,
} from "../../shared/api/tauri";
import type { AgentRunRequest, EventGroup, TimelineItem } from "../../entities/message/model";
import { useWorkbenchStore, type TabState, type FollowUpQueueItem } from "./model";

const EMPTY_FOLLOW_UP_QUEUE: FollowUpQueueItem[] = [];
const EMPTY_ITEMS: TimelineItem[] = [];

export function useAgentRun(tabId: string) {
  const agentsQuery = useQuery({ queryKey: ["agents"], queryFn: listAgents });
  const agents = agentsQuery.data ?? [];

  const tab = useWorkbenchStore(
    (state) => state.tabs.find((t) => t.id === tabId),
  );

  const patch = useCallback(
    (update: Partial<TabState>) => useWorkbenchStore.getState().patchTab(tabId, update),
    [tabId],
  );

  useEffect(() => {
    if (!tab) return;
    if (agents.length > 0 && !agents.some((agent) => agent.id === tab.selectedAgentId)) {
      patch({ selectedAgentId: agents[0].id });
    }
  }, [agents, tab?.selectedAgentId, patch, tab]);

  const selectedAgent = useMemo(
    () => agents.find((agent) => agent.id === tab?.selectedAgentId),
    [agents, tab?.selectedAgentId],
  );

  const items = tab?.items ?? EMPTY_ITEMS;
  const filter: EventGroup | "all" = tab?.filter ?? "all";
  const visibleItems = useMemo(
    () => (filter === "all" ? items : items.filter((item) => item.group === filter)),
    [filter, items],
  );

  const run = useCallback(async () => {
    const current = useWorkbenchStore.getState().tabs.find((t) => t.id === tabId);
    if (!current) return;
    const trimmedGoal = current.goal.trim();
    if (!trimmedGoal) {
      patch({ error: "Goal is empty." });
      return;
    }
    const runId = crypto.randomUUID();
    useWorkbenchStore.getState().beginRun(tabId, runId);

    const request: AgentRunRequest = {
      runId,
      goal: trimmedGoal,
      agentId: current.selectedAgentId,
      cwd: current.cwd.trim() || undefined,
      agentCommand: current.customCommand.trim() || undefined,
      stdioBufferLimitMb: Math.min(512, Math.max(1, current.stdioBufferLimitMb || 50)),
      autoAllow: current.autoAllow,
    };

    try {
      await startAgentRun(request);
    } catch (err) {
      useWorkbenchStore.getState().patchTab(tabId, { activeRunId: null });
      useWorkbenchStore.getState().endRun(tabId);
      patch({ error: String(err) });
    }
  }, [tabId, patch]);

  const cancel = useCallback(async () => {
    const current = useWorkbenchStore.getState().tabs.find((t) => t.id === tabId);
    if (!current?.activeRunId) return;
    try {
      await cancelAgentRun(current.activeRunId);
      useWorkbenchStore.getState().endRun(tabId);
      patch({ error: null });
    } catch (err) {
      patch({ error: String(err) });
    }
  }, [tabId, patch]);

  const send = useCallback(() => {
    const store = useWorkbenchStore.getState();
    const current = store.tabs.find((t) => t.id === tabId);
    if (!current?.sessionActive) return;
    const trimmed = current.followUpDraft.trim();
    if (!trimmed) return;
    store.enqueueFollowUp(tabId, trimmed);
    store.patchTab(tabId, { followUpDraft: "" });
  }, [tabId]);

  const cancelFollowUp = useCallback(
    (id: string) => useWorkbenchStore.getState().removeFollowUp(tabId, id),
    [tabId],
  );

  const setSelectedAgentId = useCallback(
    (value: string) => patch({ selectedAgentId: value }),
    [patch],
  );
  const setGoal = useCallback((value: string) => patch({ goal: value }), [patch]);
  const setCwd = useCallback((value: string) => patch({ cwd: value }), [patch]);
  const setCustomCommand = useCallback(
    (value: string) => patch({ customCommand: value }),
    [patch],
  );
  const setStdioBufferLimitMb = useCallback(
    (value: number) => patch({ stdioBufferLimitMb: value }),
    [patch],
  );
  const setAutoAllow = useCallback((value: boolean) => patch({ autoAllow: value }), [patch]);
  const setIdleTimeoutSec = useCallback(
    (value: number) => patch({ idleTimeoutSec: value }),
    [patch],
  );
  const setFollowUpDraft = useCallback(
    (value: string) => patch({ followUpDraft: value }),
    [patch],
  );
  const setFilter = useCallback(
    (value: EventGroup | "all") => patch({ filter: value }),
    [patch],
  );
  const setError = useCallback((value: string | null) => patch({ error: value }), [patch]);

  return {
    agents,
    agentsLoading: agentsQuery.isLoading,
    selectedAgent,
    selectedAgentId: tab?.selectedAgentId ?? "",
    setSelectedAgentId,
    goal: tab?.goal ?? "",
    setGoal,
    cwd: tab?.cwd ?? "",
    setCwd,
    customCommand: tab?.customCommand ?? "",
    setCustomCommand,
    stdioBufferLimitMb: tab?.stdioBufferLimitMb ?? 50,
    setStdioBufferLimitMb,
    autoAllow: tab?.autoAllow ?? true,
    setAutoAllow,
    idleTimeoutSec: tab?.idleTimeoutSec ?? 0,
    setIdleTimeoutSec,
    idleRemainingSec: tab?.idleRemainingSec ?? null,
    activeRunId: tab?.activeRunId ?? null,
    sessionActive: tab?.sessionActive ?? false,
    awaitingResponse: tab?.awaitingResponse ?? false,
    isRunning: tab?.sessionActive ?? false,
    followUpDraft: tab?.followUpDraft ?? "",
    setFollowUpDraft,
    followUpQueue: tab?.followUpQueue ?? EMPTY_FOLLOW_UP_QUEUE,
    cancelFollowUp,
    error: tab?.error ?? (agentsQuery.error ? String(agentsQuery.error) : null),
    setError,
    run,
    cancel,
    send,
    items,
    visibleItems,
    filter,
    setFilter,
  };
}
