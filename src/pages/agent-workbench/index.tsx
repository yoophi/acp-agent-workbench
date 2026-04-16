import { GoalEditor } from "../../features/goal-input/GoalEditor";
import { useAgentRun } from "../../features/agent-run/useAgentRun";
import { EventStream } from "../../widgets/event-stream/EventStream";
import { RunPanel } from "../../widgets/run-panel/RunPanel";

export function AgentWorkbenchPage() {
  const state = useAgentRun();

  return (
    <main className="shell min-h-screen">
      <header className="app-header">
        <div>
          <p className="eyebrow">Agent Client Protocol</p>
          <h1>ACP Agent Workbench</h1>
        </div>
        {state.error ? <div className="error-banner">{state.error}</div> : null}
      </header>

      <div className="workspace-grid">
        <div className="left-column">
          <GoalEditor value={state.goal} onChange={state.setGoal} onError={state.setError} />
          <RunPanel
            agents={state.agents}
            selectedAgentId={state.selectedAgentId}
            onSelectAgent={state.setSelectedAgentId}
            selectedAgent={state.selectedAgent}
            cwd={state.cwd}
            onCwdChange={state.setCwd}
            customCommand={state.customCommand}
            onCustomCommandChange={state.setCustomCommand}
            stdioBufferLimitMb={state.stdioBufferLimitMb}
            onStdioBufferLimitChange={state.setStdioBufferLimitMb}
            autoAllow={state.autoAllow}
            onAutoAllowChange={state.setAutoAllow}
            isRunning={state.isRunning}
            activeRunId={state.activeRunId}
            onRun={state.run}
            onCancel={state.cancel}
          />
        </div>
        <EventStream items={state.visibleItems} filter={state.filter} onFilterChange={state.setFilter} />
      </div>
    </main>
  );
}
