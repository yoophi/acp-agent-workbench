import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect } from "react";
import { AgentWorkbenchPage } from "../pages/agent-workbench";
import { hydrateDetachedWorkbenchTab, installAgentRuntime } from "../features/agent-run";
import { getWindowBootstrap } from "../features/workbench-window";

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

  return (
    <QueryClientProvider client={queryClient}>
      <AgentWorkbenchPage />
    </QueryClientProvider>
  );
}
