import { useActiveTabId, useAgentRun } from "../../features/agent-run";
import { GoalEditor } from "../../features/goal-input";
import { EventStream } from "../../widgets/event-stream";
import { FollowUpComposer } from "../../widgets/follow-up-composer";
import { FollowUpQueue } from "../../widgets/follow-up-queue";
import { LocalTasksPanel } from "../../widgets/local-tasks";
import { RunPanel } from "../../widgets/run-panel";
import { SavedPromptsPanel } from "../../widgets/saved-prompts";
import { TabBar } from "../../widgets/workbench-tabs";
import { WorkspaceBar } from "../../widgets/workspace-bar";
import { WorkspaceGitPanel } from "../../widgets/workspace-git";
import { WorkspacePrPublishPanel } from "../../widgets/workspace-pr-publish";
import { WorkspacePrReviewPanel } from "../../widgets/workspace-pr-review";

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
      <WorkspaceGitPanel workspaceId={state.workspaceId} checkoutId={state.checkoutId} />
      <LocalTasksPanel
        workspaceId={state.workspaceId}
        checkoutId={state.checkoutId}
        sessionActive={state.sessionActive}
        onApplyTaskGoal={state.setGoalFromTask}
        onRunTaskGoal={(goal, task, allowBlockedTask) =>
          void state.run({ goal, sourceTask: task, allowBlockedTask })
        }
        onError={state.setError}
      />
      <WorkspacePrPublishPanel workspaceId={state.workspaceId} checkoutId={state.checkoutId} />
      <WorkspacePrReviewPanel
        workspaceId={state.workspaceId}
        checkoutId={state.checkoutId}
        onApplyReviewGoal={state.setGoal}
      />
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
            scenario={state.scenario}
            onScenarioChange={state.setScenario}
            selectedAgent={state.selectedAgent}
            cwd={state.cwd}
            onCwdChange={state.setCwd}
            customCommand={state.customCommand}
            onCustomCommandChange={state.setCustomCommand}
            stdioBufferLimitMb={state.stdioBufferLimitMb}
            onStdioBufferLimitChange={state.setStdioBufferLimitMb}
            autoAllow={state.autoAllow}
            onAutoAllowChange={state.setAutoAllow}
            resumePolicy={state.resumePolicy}
            onResumePolicyChange={state.setResumePolicy}
            latestAcpSession={state.latestAcpSession}
            acpSessionLoading={state.acpSessionLoading}
            onClearLatestAcpSession={state.clearLatestAcpSession}
            ralphLoop={state.ralphLoop}
            onRalphLoopChange={state.setRalphLoop}
            idleTimeoutSec={state.idleTimeoutSec}
            onIdleTimeoutChange={state.setIdleTimeoutSec}
            idleRemainingSec={state.idleRemainingSec}
            isRunning={state.isRunning}
            activeRunId={state.activeRunId}
            sourceTask={state.sourceTask}
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
          <SavedPromptsPanel
            workspaceId={state.workspaceId}
            sessionActive={state.sessionActive}
            onApply={state.applySavedPrompt}
            onError={state.setError}
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
