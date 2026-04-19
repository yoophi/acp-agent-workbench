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
      const runLabel = request.activeRunCount === 1 ? "run" : "runs";
      if (
        window.confirm(
          `This window owns ${request.activeRunCount} active ${runLabel}. Close it and cancel those runs?`,
        )
      ) {
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
