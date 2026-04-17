import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect } from "react";
import { AgentWorkbenchPage } from "../pages/agent-workbench";
import { installAgentRuntime } from "../features/agent-run/runtime";

const queryClient = new QueryClient();

export function App() {
  useEffect(() => {
    void installAgentRuntime();
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <AgentWorkbenchPage />
    </QueryClientProvider>
  );
}
