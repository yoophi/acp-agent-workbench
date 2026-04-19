import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect } from "react";
import { AgentWorkbenchPage } from "../pages/agent-workbench";
import { hydrateDetachedWorkbenchTab, installAgentRuntime } from "../features/agent-run";
import {
  closeWorkbenchWindow,
  getWindowBootstrap,
  listenWorkbenchWindowCloseRequests,
} from "../features/workbench-window";

const queryClient = new QueryClient();

export function App() {
  useEffect(() => {
    void (async () => {
      const bootstrap = await getWindowBootstrap();
      if (bootstrap.detachedTab) {
        hydrateDetachedWorkbenchTab(bootstrap.detachedTab);
      }
      await installAgentRuntime();
    })();
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let mounted = true;

    void listenWorkbenchWindowCloseRequests((request) => {
      if (window.confirm(closeRequestMessage(request))) {
        void closeWorkbenchWindow();
      }
    }).then((dispose) => {
      if (mounted) {
        unlisten = dispose;
      } else {
        dispose();
      }
    });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <AgentWorkbenchPage />
    </QueryClientProvider>
  );
}

function closeRequestMessage(request: { activeRunCount: number; lastWindow: boolean }) {
  const runLabel = request.activeRunCount === 1 ? "run" : "runs";
  if (request.activeRunCount > 0 && request.lastWindow) {
    return `This is the last workbench window and it owns ${request.activeRunCount} active ${runLabel}. Close it and cancel those runs?`;
  }
  if (request.activeRunCount > 0) {
    return `This window owns ${request.activeRunCount} active ${runLabel}. Close it and cancel those runs?`;
  }
  return "This is the last workbench window. Close it and quit the app?";
}
