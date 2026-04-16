import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AgentWorkbenchPage } from "../pages/agent-workbench";

const queryClient = new QueryClient();

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AgentWorkbenchPage />
    </QueryClientProvider>
  );
}
