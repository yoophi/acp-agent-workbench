import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo } from "react";
import { cancelAgentRun, listAgents, listenRunEvents, startAgentRun } from "../../shared/api/tauri";
import type { AgentRunRequest } from "../../entities/message/model";
import { toTimelineItem } from "../../entities/message/format";
import { useAgentRunStore } from "./model";

export function useAgentRun() {
  const state = useAgentRunStore();
  const agentsQuery = useQuery({ queryKey: ["agents"], queryFn: listAgents });
  const agents = agentsQuery.data ?? [];

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    listenRunEvents((envelope) => {
      const store = useAgentRunStore.getState();
      store.appendItem(toTimelineItem(envelope.runId, envelope.event));
      if (
        envelope.event.type === "lifecycle" &&
        (envelope.event.status === "completed" || envelope.event.status === "cancelled")
      ) {
        store.setIsRunning(false);
      }
      if (envelope.event.type === "error") {
        store.setIsRunning(false);
        store.setError(envelope.event.message);
      }
    }).then((dispose) => {
      if (disposed) {
        dispose();
        return;
      }
      unlisten = dispose;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (agents.length > 0 && !agents.some((agent) => agent.id === state.selectedAgentId)) {
      state.setSelectedAgentId(agents[0].id);
    }
  }, [agents, state]);

  const selectedAgent = useMemo(
    () => agents.find((agent) => agent.id === state.selectedAgentId),
    [agents, state.selectedAgentId],
  );

  const visibleItems = useMemo(
    () => (state.filter === "all" ? state.items : state.items.filter((item) => item.group === state.filter)),
    [state.filter, state.items],
  );

  const run = useCallback(async () => {
    const trimmedGoal = state.goal.trim();
    if (!trimmedGoal) {
      state.setError("Goal is empty.");
      return;
    }
    // Default policy: each new run starts with a clean timeline.
    state.beginRun();

    const request: AgentRunRequest = {
      goal: trimmedGoal,
      agentId: state.selectedAgentId,
      cwd: state.cwd.trim() || undefined,
      agentCommand: state.customCommand.trim() || undefined,
      stdioBufferLimitMb: Math.min(512, Math.max(1, state.stdioBufferLimitMb || 50)),
      autoAllow: state.autoAllow,
    };

    try {
      const started = await startAgentRun(request);
      state.setActiveRunId(started.id);
    } catch (err) {
      state.setIsRunning(false);
      state.setError(String(err));
    }
  }, [state]);

  const cancel = useCallback(async () => {
    if (!state.activeRunId) {
      return;
    }
    await cancelAgentRun(state.activeRunId);
    state.setIsRunning(false);
  }, [state]);

  return {
    agents,
    agentsLoading: agentsQuery.isLoading,
    selectedAgent,
    selectedAgentId: state.selectedAgentId,
    setSelectedAgentId: state.setSelectedAgentId,
    goal: state.goal,
    setGoal: state.setGoal,
    cwd: state.cwd,
    setCwd: state.setCwd,
    customCommand: state.customCommand,
    setCustomCommand: state.setCustomCommand,
    stdioBufferLimitMb: state.stdioBufferLimitMb,
    setStdioBufferLimitMb: state.setStdioBufferLimitMb,
    autoAllow: state.autoAllow,
    setAutoAllow: state.setAutoAllow,
    activeRunId: state.activeRunId,
    isRunning: state.isRunning,
    error: state.error ?? (agentsQuery.error ? String(agentsQuery.error) : null),
    setError: state.setError,
    run,
    cancel,
    items: state.items,
    visibleItems,
    filter: state.filter,
    setFilter: state.setFilter,
  };
}
