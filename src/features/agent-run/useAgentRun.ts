import { useQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo } from "react";
import {
  cancelAgentRun,
  listAgents,
  listenRunEvents,
  sendPromptToRun,
  startAgentRun,
} from "../../shared/api/tauri";
import type { AgentRunRequest } from "../../entities/message/model";
import { toTimelineItem } from "../../entities/message/format";
import { useAgentRunStore } from "./model";

async function drainQueue() {
  const store = useAgentRunStore.getState();
  if (!store.sessionActive || !store.activeRunId || store.awaitingResponse) {
    return;
  }
  const next = store.dequeueFollowUp();
  if (!next) {
    return;
  }
  store.setAwaitingResponse(true);
  try {
    await sendPromptToRun(store.activeRunId, next.text);
  } catch (err) {
    useAgentRunStore.setState({ awaitingResponse: false });
    useAgentRunStore.getState().setError(String(err));
  }
}

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
      if (envelope.event.type === "lifecycle") {
        const status = envelope.event.status;
        if (status === "promptSent") {
          store.setAwaitingResponse(true);
        } else if (status === "promptCompleted") {
          store.setAwaitingResponse(false);
          void drainQueue();
        } else if (status === "completed" || status === "cancelled") {
          store.endRun();
        }
      }
      if (envelope.event.type === "error") {
        store.endRun();
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

  useEffect(() => {
    const shouldCountdown =
      state.sessionActive &&
      !state.awaitingResponse &&
      state.followUpQueue.length === 0 &&
      state.idleTimeoutSec > 0;

    if (!shouldCountdown) {
      if (useAgentRunStore.getState().idleRemainingSec !== null) {
        useAgentRunStore.getState().setIdleRemainingSec(null);
      }
      return;
    }

    useAgentRunStore.getState().setIdleRemainingSec(state.idleTimeoutSec);
    const interval = setInterval(() => {
      const current = useAgentRunStore.getState();
      if (
        !current.sessionActive ||
        current.awaitingResponse ||
        current.followUpQueue.length > 0
      ) {
        current.setIdleRemainingSec(null);
        clearInterval(interval);
        return;
      }
      const next = (current.idleRemainingSec ?? 0) - 1;
      if (next <= 0) {
        current.setIdleRemainingSec(null);
        clearInterval(interval);
        const runId = current.activeRunId;
        if (runId) {
          cancelAgentRun(runId).catch(() => undefined);
          current.endRun();
        }
      } else {
        current.setIdleRemainingSec(next);
      }
    }, 1000);
    return () => clearInterval(interval);
  }, [
    state.sessionActive,
    state.awaitingResponse,
    state.followUpQueue.length,
    state.idleTimeoutSec,
  ]);

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
      state.endRun();
      state.setError(String(err));
    }
  }, [state]);

  const cancel = useCallback(async () => {
    if (!state.activeRunId) {
      return;
    }
    try {
      await cancelAgentRun(state.activeRunId);
      state.endRun();
      state.setError(null);
    } catch (err) {
      state.setError(String(err));
    }
  }, [state]);

  const send = useCallback(() => {
    const store = useAgentRunStore.getState();
    if (!store.sessionActive) {
      return;
    }
    const trimmed = store.followUpDraft.trim();
    if (!trimmed) {
      return;
    }
    store.enqueueFollowUp(trimmed);
    store.setFollowUpDraft("");
    void drainQueue();
  }, []);

  const cancelFollowUp = useCallback((id: string) => {
    useAgentRunStore.getState().removeFollowUp(id);
  }, []);

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
    idleTimeoutSec: state.idleTimeoutSec,
    setIdleTimeoutSec: state.setIdleTimeoutSec,
    idleRemainingSec: state.idleRemainingSec,
    activeRunId: state.activeRunId,
    sessionActive: state.sessionActive,
    awaitingResponse: state.awaitingResponse,
    isRunning: state.sessionActive,
    followUpDraft: state.followUpDraft,
    setFollowUpDraft: state.setFollowUpDraft,
    followUpQueue: state.followUpQueue,
    cancelFollowUp,
    error: state.error ?? (agentsQuery.error ? String(agentsQuery.error) : null),
    setError: state.setError,
    run,
    cancel,
    send,
    items: state.items,
    visibleItems,
    filter: state.filter,
    setFilter: state.setFilter,
  };
}
