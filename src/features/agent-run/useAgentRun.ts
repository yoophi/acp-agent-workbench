import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo } from "react";
import {
  cancelAgentRun,
  listWorkspaceCheckouts,
  listAgents,
  provisionWorkspaceTaskWorktree,
  startAgentRun,
} from "./api";
import type {
  AgentRunRequest,
  EventGroup,
  RalphLoopSettings,
  ResumePolicy,
  TimelineItem,
} from "../../entities/message";
import type { SavedPromptRunMode } from "../../entities/saved-prompt";
import {
  defaultRalphLoopSettings,
  selectTab,
  selectTabList,
  useWorkbenchStore,
  type TabState,
  type FollowUpQueueItem,
} from "./model";

const EMPTY_FOLLOW_UP_QUEUE: FollowUpQueueItem[] = [];
const EMPTY_ITEMS: TimelineItem[] = [];

export function useAgentRun(tabId: string) {
  const agentsQuery = useQuery({ queryKey: ["agents"], queryFn: listAgents });
  const agents = agentsQuery.data ?? [];

  const tab = useWorkbenchStore((state) => selectTab(state, tabId));

  const patch = useCallback(
    (update: Partial<TabState>) => useWorkbenchStore.getState().patchTab(tabId, update),
    [tabId],
  );

  useEffect(() => {
    const current = tab?.selectedAgentId;
    if (!current) return;
    if (agents.length > 0 && !agents.some((agent) => agent.id === current)) {
      patch({ selectedAgentId: agents[0].id });
    }
  }, [agents, tab?.selectedAgentId, patch]);

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
    const current = selectTab(useWorkbenchStore.getState(), tabId);
    if (!current) return;
    const trimmedGoal = current.goal.trim();
    if (!trimmedGoal) {
      patch({ error: "Goal is empty." });
      return;
    }
    const sameWorkdirRuns = selectTabList(useWorkbenchStore.getState()).filter(
      (entry) =>
        entry.id !== current.id &&
        entry.sessionActive &&
        entry.workspaceId === current.workspaceId &&
        entry.checkoutId === current.checkoutId &&
        entry.cwd === current.cwd,
    ).length;
    if (
      sameWorkdirRuns > 0 &&
      !current.workspaceId &&
      !window.confirm(
        `There ${sameWorkdirRuns === 1 ? "is" : "are"} ${sameWorkdirRuns} active run${
          sameWorkdirRuns === 1 ? "" : "s"
        } in this working directory. Start another run anyway?`,
      )
    ) {
      return;
    }
    const runId = crypto.randomUUID();
    useWorkbenchStore.getState().beginRun(tabId, runId);

    let checkoutId = current.checkoutId ?? undefined;
    let cwd = current.cwd.trim() || undefined;

    const request: AgentRunRequest = {
      runId,
      goal: trimmedGoal,
      agentId: current.selectedAgentId,
      workspaceId: current.workspaceId ?? undefined,
      checkoutId,
      cwd,
      agentCommand: current.customCommand.trim() || undefined,
      stdioBufferLimitMb: Math.min(512, Math.max(1, current.stdioBufferLimitMb || 50)),
      autoAllow: current.autoAllow,
      resumePolicy: current.resumePolicy === "fresh" ? undefined : current.resumePolicy,
      ralphLoop: current.ralphLoop.enabled ? current.ralphLoop : undefined,
    };

    try {
      if (current.workspaceId) {
        const worktree = await provisionWorkspaceTaskWorktree({
          workspaceId: current.workspaceId,
          checkoutId,
          taskSlug: trimmedGoal,
        });
        checkoutId = worktree.id;
        cwd = undefined;
        request.checkoutId = checkoutId;
        request.cwd = cwd;

        const store = useWorkbenchStore.getState();
        store.setTabWorkspace(tabId, current.workspaceId, checkoutId);
        store.patchTab(tabId, { cwd: "" });
        const checkouts = await listWorkspaceCheckouts(current.workspaceId);
        store.setWorkspaceCheckouts(current.workspaceId, checkouts);
      }
      await startAgentRun(request);
    } catch (err) {
      const error = String(err);
      const store = useWorkbenchStore.getState();
      store.patchTab(tabId, { error });
      store.endRun(tabId);
      store.patchTab(tabId, { activeRunId: null });
    }
  }, [tabId, patch]);

  const cancel = useCallback(async () => {
    const current = selectTab(useWorkbenchStore.getState(), tabId);
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
    const current = selectTab(store, tabId);
    if (!current?.sessionActive) return;
    const trimmed = current.followUpDraft.trim();
    if (!trimmed) return;
    store.enqueueFollowUp(tabId, trimmed);
    store.patchTab(tabId, { followUpDraft: "" });
  }, [tabId]);

  const applySavedPrompt = useCallback(
    (body: string, runMode: SavedPromptRunMode) => {
      const store = useWorkbenchStore.getState();
      const current = selectTab(store, tabId);
      const trimmed = body.trim();
      if (!current || !trimmed) return;
      if (!current.sessionActive) {
        store.patchTab(tabId, { goal: trimmed });
        return;
      }
      if (runMode === "insert") {
        store.patchTab(tabId, { followUpDraft: trimmed });
        return;
      }
      store.enqueueFollowUp(tabId, trimmed);
    },
    [tabId],
  );

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
  const setResumePolicy = useCallback((value: ResumePolicy) => patch({ resumePolicy: value }), [patch]);
  const setRalphLoop = useCallback(
    (value: RalphLoopSettings) => patch({ ralphLoop: value }),
    [patch],
  );
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
    workspaceId: tab?.workspaceId ?? null,
    checkoutId: tab?.checkoutId ?? null,
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
    resumePolicy: tab?.resumePolicy ?? "fresh",
    setResumePolicy,
    ralphLoop: tab?.ralphLoop ?? defaultRalphLoopSettings,
    setRalphLoop,
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
    applySavedPrompt,
    items,
    visibleItems,
    filter,
    setFilter,
  };
}
