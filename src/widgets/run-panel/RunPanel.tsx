import { Octagon, Play, ShieldCheck } from "lucide-react";
import type { AgentDescriptor } from "../../entities/agent/model";
import { Button } from "../../shared/ui/Button";
import { Card, CardContent, CardHeader, CardTitle, CardTitleBlock } from "../../shared/ui/Card";
import { Input } from "../../shared/ui/Input";
import { NativeSelect } from "../../shared/ui/NativeSelect";
import { cn } from "../../shared/lib/utils";

type RunPanelProps = {
  agents: AgentDescriptor[];
  selectedAgentId: string;
  onSelectAgent: (id: string) => void;
  selectedAgent?: AgentDescriptor;
  cwd: string;
  onCwdChange: (value: string) => void;
  customCommand: string;
  onCustomCommandChange: (value: string) => void;
  stdioBufferLimitMb: number;
  onStdioBufferLimitChange: (value: number) => void;
  autoAllow: boolean;
  onAutoAllowChange: (value: boolean) => void;
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
  cwd,
  onCwdChange,
  customCommand,
  onCustomCommandChange,
  stdioBufferLimitMb,
  onStdioBufferLimitChange,
  autoAllow,
  onAutoAllowChange,
  idleTimeoutSec,
  onIdleTimeoutChange,
  idleRemainingSec,
  isRunning,
  activeRunId,
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
          <span className="text-sm font-medium">Workspace</span>
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
