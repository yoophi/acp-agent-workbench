import { cancelAgentRun, listenRunEvents, sendPromptToRun } from "./api";
import { useWorkbenchStore } from "./model";

let installed = false;
let disposers: Array<() => void> = [];

async function drainTabQueue(tabId: string) {
  const store = useWorkbenchStore.getState();
  const tab = store.tabs.find((t) => t.id === tabId);
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
    const current = useWorkbenchStore.getState().tabs.find((t) => t.id === tabId);
    if (current?.activeRunId === runId) {
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
    for (const tab of store.tabs) {
      const shouldCount =
        tab.sessionActive &&
        !tab.awaitingResponse &&
        tab.followUpQueue.length === 0 &&
        tab.idleTimeoutSec > 0;

      if (!shouldCount) {
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
    const signature = state.tabs
      .map(
        (t) =>
          `${t.id}:${t.sessionActive ? 1 : 0}:${t.awaitingResponse ? 1 : 0}:${t.followUpQueue.length}`,
      )
      .join("|");
    if (signature === previousSignature) return;
    previousSignature = signature;
    for (const tab of state.tabs) {
      if (
        tab.sessionActive &&
        !tab.awaitingResponse &&
        tab.followUpQueue.length > 0 &&
        tab.activeRunId
      ) {
        void drainTabQueue(tab.id);
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

export { drainTabQueue };
