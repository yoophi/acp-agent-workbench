import { cancelAgentRun, listenRunEvents, sendPromptToRun } from "./api";
import { selectTab, selectTabList, useWorkbenchStore } from "./model";

let installed = false;
let disposers: Array<() => void> = [];
const ralphLoopStateByRunId = new Map<string, { sent: number; pending: boolean }>();

function resetRalphLoopStateForTests() {
  ralphLoopStateByRunId.clear();
}

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function drainTabQueue(tabId: string) {
  const store = useWorkbenchStore.getState();
  const tab = selectTab(store, tabId);
  if (
    !tab ||
    !tab.sessionActive ||
    !tab.activeRunId ||
    tab.awaitingResponse ||
    tab.closing
  ) {
    return;
  }
  const runId = tab.activeRunId;
  const head = tab.followUpQueue[0];
  if (!head) return;
  if (head.runId !== runId) {
    store.removeFollowUp(tabId, head.id);
    return;
  }
  const next = store.dequeueFollowUp(tabId);
  if (!next) return;
  store.patchTab(tabId, { awaitingResponse: true });
  try {
    await sendPromptToRun(runId, next.text);
  } catch (err) {
    const current = selectTab(useWorkbenchStore.getState(), tabId);
    if (current?.activeRunId === runId) {
      useWorkbenchStore.getState().patchTab(tabId, {
        awaitingResponse: false,
        error: String(err),
      });
    }
  }
}

async function drainRalphLoop(tabId: string) {
  const store = useWorkbenchStore.getState();
  const tab = selectTab(store, tabId);
  if (
    !tab ||
    !tab.sessionActive ||
    !tab.activeRunId ||
    tab.awaitingResponse ||
    tab.closing ||
    tab.followUpQueue.length > 0 ||
    !tab.ralphLoop.enabled
  ) {
    return;
  }
  if (tab.permissionPending && tab.ralphLoop.stopOnPermission) return;
  if (tab.error && tab.ralphLoop.stopOnError) return;

  const runId = tab.activeRunId;
  const loop = tab.ralphLoop;
  const loopState = ralphLoopStateByRunId.get(runId) ?? { sent: 0, pending: false };
  const maxIterations = Math.max(1, loop.maxIterations);
  const prompt = loop.promptTemplate.trim();
  if (loopState.pending || loopState.sent >= maxIterations || !prompt) return;

  const iteration = loopState.sent + 1;
  ralphLoopStateByRunId.set(runId, { ...loopState, pending: true });
  store.dispatchRunEvent(runId, {
    type: "diagnostic",
    message: `Ralph loop iteration ${iteration}/${maxIterations} started`,
  });
  store.patchTab(tabId, { awaitingResponse: true });

  if (loop.delayMs > 0) {
    await sleep(loop.delayMs);
    const latest = selectTab(useWorkbenchStore.getState(), tabId);
    if (!latest?.sessionActive || latest.activeRunId !== runId || latest.closing) {
      ralphLoopStateByRunId.set(runId, { ...loopState, pending: false });
      return;
    }
  }

  try {
    await sendPromptToRun(runId, prompt);
    ralphLoopStateByRunId.set(runId, { sent: iteration, pending: false });
    void drainRalphLoop(tabId);
  } catch (err) {
    ralphLoopStateByRunId.set(runId, { ...loopState, pending: false });
    const latest = selectTab(useWorkbenchStore.getState(), tabId);
    if (latest?.activeRunId === runId) {
      useWorkbenchStore.getState().patchTab(tabId, {
        awaitingResponse: false,
        error: String(err),
      });
    }
  }
}

function startIdleTicker() {
  const interval = setInterval(() => {
    const store = useWorkbenchStore.getState();
    for (const tab of selectTabList(store)) {
      const shouldCount =
        tab.sessionActive &&
        !tab.awaitingResponse &&
        tab.followUpQueue.length === 0 &&
        tab.idleTimeoutSec > 0;

      if (!shouldCount) {
        if (!tab.sessionActive && tab.activeRunId) {
          ralphLoopStateByRunId.delete(tab.activeRunId);
        }
        if (tab.idleRemainingSec !== null) {
          store.patchTab(tab.id, { idleRemainingSec: null });
        }
        continue;
      }

      const current = tab.idleRemainingSec ?? tab.idleTimeoutSec;
      const next = current - 1;
      if (next <= 0) {
        store.patchTab(tab.id, { idleRemainingSec: null });
        if (tab.activeRunId) {
          cancelAgentRun(tab.activeRunId).catch(() => undefined);
        }
        store.endRun(tab.id);
      } else {
        store.patchTab(tab.id, { idleRemainingSec: next });
      }
    }
  }, 1000);
  return () => clearInterval(interval);
}

function subscribeForDrain() {
  let previousSignature = "";
  return useWorkbenchStore.subscribe((state) => {
    const tabs = selectTabList(state);
    const signature = tabs
      .map(
        (t) =>
          `${t.id}:${t.activeRunId ?? ""}:${t.sessionActive ? 1 : 0}:${
            t.awaitingResponse ? 1 : 0
          }:${t.followUpQueue.length}:${t.permissionPending ? 1 : 0}:${
            t.ralphLoop.enabled ? 1 : 0
          }`,
      )
      .join("|");
    if (signature === previousSignature) return;
    previousSignature = signature;
    for (const tab of tabs) {
      if (
        tab.sessionActive &&
        !tab.awaitingResponse &&
        tab.followUpQueue.length > 0 &&
        tab.activeRunId
      ) {
        void drainTabQueue(tab.id);
      } else if (
        tab.sessionActive &&
        !tab.awaitingResponse &&
        tab.followUpQueue.length === 0 &&
        tab.activeRunId &&
        tab.ralphLoop.enabled
      ) {
        void drainRalphLoop(tab.id);
      }
    }
  });
}

export async function installAgentRuntime() {
  if (installed) return;
  installed = true;

  const unlistenEvents = await listenRunEvents((envelope) => {
    useWorkbenchStore.getState().dispatchRunEvent(envelope.runId, envelope.event);
  });
  disposers.push(unlistenEvents);

  disposers.push(subscribeForDrain());
  disposers.push(startIdleTicker());

  const meta = import.meta as ImportMeta & {
    hot?: { dispose: (cb: () => void) => void };
  };
  meta.hot?.dispose(() => {
    disposers.forEach((dispose) => dispose());
    disposers = [];
    installed = false;
  });
}

export { drainRalphLoop, drainTabQueue, resetRalphLoopStateForTests };
