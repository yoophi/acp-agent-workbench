import { Octagon, Play, ShieldCheck, Trash2 } from "lucide-react";
import type { AcpSessionRecord } from "../../entities/acp-session";
import type { AgentDescriptor } from "../../entities/agent";
import type { RalphLoopSettings, ResumePolicy } from "../../entities/message";
import { RUN_SCENARIOS, type LocalTaskRunSource, type RunScenarioId } from "../../features/agent-run";
import { cn } from "../../shared/lib";
import { Button, Card, CardContent, CardHeader, CardTitle, CardTitleBlock, Input, NativeSelect } from "../../shared/ui";

type RunPanelProps = {
  agents: AgentDescriptor[];
  selectedAgentId: string;
  onSelectAgent: (id: string) => void;
  scenario: RunScenarioId;
  onScenarioChange: (id: RunScenarioId) => void;
  selectedAgent?: AgentDescriptor;
  cwd: string;
  onCwdChange: (value: string) => void;
  customCommand: string;
  onCustomCommandChange: (value: string) => void;
  stdioBufferLimitMb: number;
  onStdioBufferLimitChange: (value: number) => void;
  autoAllow: boolean;
  onAutoAllowChange: (value: boolean) => void;
  resumePolicy: ResumePolicy;
  onResumePolicyChange: (value: ResumePolicy) => void;
  latestAcpSession: AcpSessionRecord | null;
  acpSessionLoading: boolean;
  onClearLatestAcpSession: () => void;
  ralphLoop: RalphLoopSettings;
  onRalphLoopChange: (value: RalphLoopSettings) => void;
  idleTimeoutSec: number;
  onIdleTimeoutChange: (value: number) => void;
  idleRemainingSec: number | null;
  isRunning: boolean;
  activeRunId: string | null;
  sourceTask: LocalTaskRunSource | null;
  onRun: () => void;
  onCancel: () => void;
};

export function RunPanel({
  agents,
  selectedAgentId,
  onSelectAgent,
  scenario,
  onScenarioChange,
  selectedAgent,
  cwd,
  onCwdChange,
  customCommand,
  onCustomCommandChange,
  stdioBufferLimitMb,
  onStdioBufferLimitChange,
  autoAllow,
  onAutoAllowChange,
  resumePolicy,
  onResumePolicyChange,
  latestAcpSession,
  acpSessionLoading,
  onClearLatestAcpSession,
  ralphLoop,
  onRalphLoopChange,
  idleTimeoutSec,
  onIdleTimeoutChange,
  idleRemainingSec,
  isRunning,
  activeRunId,
  sourceTask,
  onRun,
  onCancel,
}: RunPanelProps) {
  return (
    <Card as="section" aria-labelledby="run-heading">
      <CardHeader>
        <CardTitleBlock>
          <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">Agent</p>
          <CardTitle id="run-heading">Execution</CardTitle>
        </CardTitleBlock>
        <span
          className={cn(
            "h-2.5 w-2.5 rounded-full bg-muted-foreground/40",
            isRunning && "bg-primary shadow-status",
          )}
          aria-label={isRunning ? "Running" : "Idle"}
        />
      </CardHeader>

      <CardContent className="grid gap-4">
        <label className="grid gap-2">
          <span className="text-sm font-medium">Agent</span>
          <NativeSelect value={selectedAgentId} onChange={(event) => onSelectAgent(event.target.value)}>
            {agents.map((agent) => (
              <option key={agent.id} value={agent.id}>
                {agent.label}
              </option>
            ))}
          </NativeSelect>
        </label>

        <label className="grid gap-2">
          <span className="text-sm font-medium">Scenario</span>
          <NativeSelect
            value={scenario}
            onChange={(event) => onScenarioChange(event.target.value as RunScenarioId)}
            disabled={isRunning}
          >
            {RUN_SCENARIOS.map((option) => (
              <option key={option.id} value={option.id}>
                {option.label}
              </option>
            ))}
          </NativeSelect>
          <span className="text-xs leading-relaxed text-muted-foreground">
            {RUN_SCENARIOS.find((option) => option.id === scenario)?.description}
          </span>
        </label>

        <label className="grid gap-2">
          <span className="text-sm font-medium">Working directory</span>
          <Input value={cwd} onChange={(event) => onCwdChange(event.target.value)} />
        </label>

        <label className="grid gap-2">
          <span className="text-sm font-medium">Command override</span>
          <Input
            value={customCommand}
            onChange={(event) => onCustomCommandChange(event.target.value)}
            placeholder={selectedAgent?.command ?? "agent command"}
          />
        </label>

        <label className="grid gap-2">
          <span className="text-sm font-medium">Stdio buffer</span>
          <Input
            type="number"
            min={1}
            max={512}
            value={stdioBufferLimitMb}
            onChange={(event) => onStdioBufferLimitChange(Number(event.target.value))}
          />
        </label>

        <label className="grid gap-2">
          <span className="text-sm font-medium">Idle timeout (sec, 0 = off)</span>
          <Input
            type="number"
            min={0}
            max={3600}
            value={idleTimeoutSec}
            onChange={(event) => onIdleTimeoutChange(Math.max(0, Number(event.target.value) || 0))}
          />
        </label>

        <label className="flex items-center gap-2 text-sm font-medium">
          <input
            type="checkbox"
            className="h-4 w-4 accent-primary"
            checked={autoAllow}
            onChange={(event) => onAutoAllowChange(event.target.checked)}
          />
          <ShieldCheck size={16} className="shrink-0 text-muted-foreground" />
          <span>Auto-select allow permission</span>
        </label>

        <label className="grid gap-2">
          <span className="text-sm font-medium">ACP session</span>
          <NativeSelect
            value={resumePolicy}
            onChange={(event) => onResumePolicyChange(event.target.value as ResumePolicy)}
            disabled={isRunning}
          >
            <option value="fresh">Start new session</option>
            <option value="resumeIfAvailable">Resume latest if available</option>
            <option value="resumeRequired">Require latest session</option>
          </NativeSelect>
        </label>

        {resumePolicy !== "fresh" ? (
          <div className="grid gap-2 rounded-md border border-border bg-muted/20 p-3">
            <div className="flex items-center justify-between gap-2">
              <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Resume target
              </span>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                icon={<Trash2 size={14} />}
                disabled={isRunning || !latestAcpSession}
                onClick={onClearLatestAcpSession}
              >
                Clear
              </Button>
            </div>
            {latestAcpSession ? (
              <div className="grid gap-1 text-xs text-muted-foreground">
                <code className="truncate font-mono text-foreground">
                  {latestAcpSession.sessionId.slice(0, 12)}
                </code>
                <span className="truncate">{latestAcpSession.task}</span>
                <span>{formatSessionTime(latestAcpSession.updatedAt)}</span>
              </div>
            ) : (
              <span className="text-xs text-muted-foreground">
                {acpSessionLoading ? "Checking sessions..." : "No stored session"}
              </span>
            )}
          </div>
        ) : null}

        <div className="grid gap-3 rounded-lg border border-border bg-muted/25 p-3">
          <label className="flex items-center gap-2 text-sm font-medium">
            <input
              type="checkbox"
              className="h-4 w-4 accent-primary"
              checked={ralphLoop.enabled}
              disabled={isRunning}
              onChange={(event) => onRalphLoopChange({ ...ralphLoop, enabled: event.target.checked })}
            />
            <span>Ralph loop</span>
          </label>
          <label className="grid gap-2">
            <span className="text-xs font-medium text-muted-foreground">Max iterations</span>
            <Input
              type="number"
              min={1}
              max={50}
              value={ralphLoop.maxIterations}
              disabled={isRunning || !ralphLoop.enabled}
              onChange={(event) =>
                onRalphLoopChange({
                  ...ralphLoop,
                  maxIterations: Math.max(1, Math.min(50, Number(event.target.value) || 1)),
                })
              }
            />
          </label>
          <label className="grid gap-2">
            <span className="text-xs font-medium text-muted-foreground">Loop prompt</span>
            <Input
              value={ralphLoop.promptTemplate}
              disabled={isRunning || !ralphLoop.enabled}
              onChange={(event) => onRalphLoopChange({ ...ralphLoop, promptTemplate: event.target.value })}
            />
          </label>
          <label className="grid gap-2">
            <span className="text-xs font-medium text-muted-foreground">Delay (ms)</span>
            <Input
              type="number"
              min={0}
              max={60000}
              value={ralphLoop.delayMs}
              disabled={isRunning || !ralphLoop.enabled}
              onChange={(event) =>
                onRalphLoopChange({
                  ...ralphLoop,
                  delayMs: Math.max(0, Math.min(60000, Number(event.target.value) || 0)),
                })
              }
            />
          </label>
          <div className="grid gap-2">
            <label className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
              <input
                type="checkbox"
                className="h-4 w-4 accent-primary"
                checked={ralphLoop.stopOnError}
                disabled={isRunning || !ralphLoop.enabled}
                onChange={(event) => onRalphLoopChange({ ...ralphLoop, stopOnError: event.target.checked })}
              />
              <span>Stop on error</span>
            </label>
            <label className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
              <input
                type="checkbox"
                className="h-4 w-4 accent-primary"
                checked={ralphLoop.stopOnPermission}
                disabled={isRunning || !ralphLoop.enabled}
                onChange={(event) => onRalphLoopChange({ ...ralphLoop, stopOnPermission: event.target.checked })}
              />
              <span>Stop on permission request</span>
            </label>
          </div>
        </div>

        <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
          <Button type="button" variant="primary" icon={<Play size={17} />} disabled={isRunning} onClick={onRun}>
            {isRunning ? "Running" : "Run"}
          </Button>
          <Button type="button" variant="secondary" icon={<Octagon size={16} />} disabled={!isRunning} onClick={onCancel}>
            Stop
          </Button>
        </div>

        {sourceTask ? (
          <div className="grid gap-1 rounded-md border border-border bg-muted/25 px-3 py-2 text-sm">
            <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              Source task
            </span>
            <span className="truncate font-medium text-foreground">
              {sourceTask.id} · {sourceTask.title}
            </span>
            <span className="text-xs text-muted-foreground">
              {sourceTask.status ?? "no status"}
              {sourceTask.blocked ? " · blocked override" : ""}
            </span>
          </div>
        ) : null}

        {idleRemainingSec !== null ? (
          <p className="m-0 text-xs font-medium text-amber-700" role="status">
            idle {idleRemainingSec} sec. 종료 예정
          </p>
        ) : null}

        <div className="grid gap-2">
          <span className="text-sm font-medium">Run ID</span>
          <code className="min-h-8 overflow-hidden text-ellipsis whitespace-nowrap rounded-md bg-muted px-2.5 py-2 font-mono text-sm text-muted-foreground">
            {activeRunId ?? "not started"}
          </code>
        </div>
      </CardContent>
    </Card>
  );
}

function formatSessionTime(value: string) {
  const timestamp = Number(value);
  if (Number.isFinite(timestamp) && timestamp > 0) {
    return new Date(timestamp * 1000).toLocaleString();
  }
  return value;
}
