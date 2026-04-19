import { Octagon, Play, ShieldCheck, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { AcpSessionRecord } from "../../entities/acp-session";
import type { AgentDescriptor } from "../../entities/agent";
import type { RalphLoopSettings, ResumePolicy } from "../../entities/message";
import { clearAcpSession, listAcpSessions } from "../../features/agent-run";
import { cn } from "../../shared/lib";
import { Button, Card, CardContent, CardHeader, CardTitle, CardTitleBlock, Input, NativeSelect } from "../../shared/ui";

type RunPanelProps = {
  agents: AgentDescriptor[];
  selectedAgentId: string;
  onSelectAgent: (id: string) => void;
  selectedAgent?: AgentDescriptor;
  workspaceId: string | null;
  checkoutId: string | null;
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
  ralphLoop: RalphLoopSettings;
  onRalphLoopChange: (value: RalphLoopSettings) => void;
  idleTimeoutSec: number;
  onIdleTimeoutChange: (value: number) => void;
  idleRemainingSec: number | null;
  isRunning: boolean;
  activeRunId: string | null;
  onRun: () => void;
  onCancel: () => void;
};

export function RunPanel({
  agents,
  selectedAgentId,
  onSelectAgent,
  selectedAgent,
  workspaceId,
  checkoutId,
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
  ralphLoop,
  onRalphLoopChange,
  idleTimeoutSec,
  onIdleTimeoutChange,
  idleRemainingSec,
  isRunning,
  activeRunId,
  onRun,
  onCancel,
}: RunPanelProps) {
  const [sessions, setSessions] = useState<AcpSessionRecord[]>([]);
  const [sessionsLoading, setSessionsLoading] = useState(false);
  const [sessionsError, setSessionsError] = useState<string | null>(null);
  const [clearingSession, setClearingSession] = useState(false);

  const agentCommand = customCommand.trim() || selectedAgent?.command || null;
  const latestSession = sessions[0] ?? null;
  const sessionUpdatedAt = useMemo(
    () => (latestSession ? new Date(latestSession.updatedAt).toLocaleString() : null),
    [latestSession],
  );

  const refreshSessions = useCallback(async () => {
    if (resumePolicy === "fresh" || !selectedAgentId) {
      setSessions([]);
      setSessionsError(null);
      return;
    }
    setSessionsLoading(true);
    setSessionsError(null);
    try {
      const next = await listAcpSessions({
        workspaceId,
        checkoutId,
        agentId: selectedAgentId,
        agentCommand,
        limit: 1,
      });
      setSessions(next);
    } catch (err) {
      setSessionsError(String(err));
      setSessions([]);
    } finally {
      setSessionsLoading(false);
    }
  }, [agentCommand, checkoutId, resumePolicy, selectedAgentId, workspaceId]);

  useEffect(() => {
    void refreshSessions();
  }, [refreshSessions]);

  const clearLatestSession = useCallback(async () => {
    if (!latestSession || clearingSession) return;
    setClearingSession(true);
    setSessionsError(null);
    try {
      await clearAcpSession(latestSession.runId);
      await refreshSessions();
    } catch (err) {
      setSessionsError(String(err));
    } finally {
      setClearingSession(false);
    }
  }, [clearingSession, latestSession, refreshSessions]);

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
          <div className="grid gap-2 rounded-lg border border-border bg-muted/25 p-3 text-sm">
            {sessionsLoading ? (
              <p className="text-muted-foreground">Checking persisted sessions...</p>
            ) : latestSession ? (
              <div className="grid gap-2">
                <div className="grid gap-1">
                  <p className="font-medium text-foreground">Latest session {latestSession.sessionId.slice(0, 8)}</p>
                  <p className="truncate text-xs text-muted-foreground">{latestSession.task}</p>
                  <p className="text-xs text-muted-foreground">{sessionUpdatedAt}</p>
                </div>
                <Button
                  type="button"
                  variant="secondary"
                  icon={<Trash2 size={15} />}
                  disabled={isRunning || clearingSession}
                  onClick={clearLatestSession}
                >
                  Clear latest session
                </Button>
              </div>
            ) : (
              <p className="text-muted-foreground">No matching persisted session</p>
            )}
            {sessionsError ? <p className="text-xs font-medium text-destructive">{sessionsError}</p> : null}
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
