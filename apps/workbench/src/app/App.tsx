import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { AgentWorkbenchPage } from "../pages/agent-workbench";
import { SettingsPage } from "../pages/settings";
import { hydrateDetachedWorkbenchTab, installAgentRuntime } from "../features/agent-run";
import {
  closeWorkbenchWindow,
  getWindowBootstrap,
  listenWorkbenchWindowCloseRequests,
} from "../features/workbench-window";

const queryClient = new QueryClient();

export function App() {
  const [view, setView] = useState<"workbench" | "settings">("workbench");

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

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.metaKey && event.key === ",") {
        event.preventDefault();
        setView("settings");
      }
      if (event.key === "Escape") {
        setView("workbench");
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      {view === "settings" ? (
        <SettingsPage onBack={() => setView("workbench")} />
      ) : (
        <AgentWorkbenchPage />
      )}
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
