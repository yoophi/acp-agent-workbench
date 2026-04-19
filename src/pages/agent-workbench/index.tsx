import { useActiveTabId, useAgentRun } from "../../features/agent-run";
import { GoalEditor } from "../../features/goal-input";
import { EventStream } from "../../widgets/event-stream";
import { FollowUpComposer } from "../../widgets/follow-up-composer";
import { FollowUpQueue } from "../../widgets/follow-up-queue";
import { RunPanel } from "../../widgets/run-panel";
import { TabBar } from "../../widgets/workbench-tabs";
import { WorkspaceBar } from "../../widgets/workspace-bar";

export function AgentWorkbenchPage() {
  const activeTabId = useActiveTabId();
  const state = useAgentRun(activeTabId);

  return (
    <main className="mx-auto flex h-dvh min-h-screen w-full max-w-[1480px] flex-col overflow-hidden p-6 max-lg:h-auto max-lg:min-h-dvh max-lg:overflow-visible max-lg:p-4 max-sm:p-3">
      <header className="mb-6 flex items-end justify-between gap-6 max-lg:flex-col max-lg:items-start">
        <div>
          <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Agent Client Protocol
          </p>
          <h1 className="m-0 text-3xl font-semibold leading-tight tracking-tight text-foreground max-sm:text-2xl">
            ACP Agent Workbench
          </h1>
        </div>
        {state.error ? (
          <div className="max-w-[560px] rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2.5 text-sm font-medium text-destructive">
            {state.error}
          </div>
        ) : null}
      </header>

      <WorkspaceBar tabId={activeTabId} disabled={state.sessionActive} />
      <TabBar />

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(360px,0.43fr)_minmax(520px,1fr)] items-stretch gap-4 max-lg:grid-cols-1">
        <div className="grid min-h-0 grid-rows-[minmax(220px,1fr)_auto] gap-4 overflow-y-auto max-lg:min-h-0">
          <GoalEditor
            value={state.goal}
            onChange={state.setGoal}
            onError={state.setError}
            readOnly={state.sessionActive}
          />
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
            idleTimeoutSec={state.idleTimeoutSec}
            onIdleTimeoutChange={state.setIdleTimeoutSec}
            idleRemainingSec={state.idleRemainingSec}
            isRunning={state.isRunning}
            activeRunId={state.activeRunId}
            onRun={state.run}
            onCancel={state.cancel}
          />
          <FollowUpComposer
            value={state.followUpDraft}
            onChange={state.setFollowUpDraft}
            onSend={state.send}
            sessionActive={state.sessionActive}
            awaitingResponse={state.awaitingResponse}
            queueLength={state.followUpQueue.length}
          />
          <FollowUpQueue
            items={state.followUpQueue}
            awaitingResponse={state.awaitingResponse}
            onCancel={state.cancelFollowUp}
          />
        </div>
        <EventStream
          items={state.visibleItems}
          filter={state.filter}
          onFilterChange={state.setFilter}
          onError={state.setError}
        />
      </div>
    </main>
  );
}
