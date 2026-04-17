import { Octagon, Play, ShieldCheck } from "lucide-react";
import type { AgentDescriptor } from "../../entities/agent/model";
import { Button } from "../../shared/ui/Button";

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
    <section className="panel run-panel" aria-labelledby="run-heading">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">Agent</p>
          <h2 id="run-heading">Execution</h2>
        </div>
        <span className={isRunning ? "status-dot status-active" : "status-dot"} aria-label={isRunning ? "Running" : "Idle"} />
      </div>

      <label className="field">
        <span>Agent</span>
        <select value={selectedAgentId} onChange={(event) => onSelectAgent(event.target.value)}>
          {agents.map((agent) => (
            <option key={agent.id} value={agent.id}>
              {agent.label}
            </option>
          ))}
        </select>
      </label>

      <label className="field">
        <span>Workspace</span>
        <input value={cwd} onChange={(event) => onCwdChange(event.target.value)} />
      </label>

      <label className="field">
        <span>Command override</span>
        <input
          value={customCommand}
          onChange={(event) => onCustomCommandChange(event.target.value)}
          placeholder={selectedAgent?.command ?? "agent command"}
        />
      </label>

      <label className="field">
        <span>Stdio buffer</span>
        <input
          type="number"
          min={1}
          max={512}
          value={stdioBufferLimitMb}
          onChange={(event) => onStdioBufferLimitChange(Number(event.target.value))}
        />
      </label>

      <label className="field">
        <span>Idle timeout (sec, 0 = off)</span>
        <input
          type="number"
          min={0}
          max={3600}
          value={idleTimeoutSec}
          onChange={(event) => onIdleTimeoutChange(Math.max(0, Number(event.target.value) || 0))}
        />
      </label>

      <label className="toggle-row">
        <input type="checkbox" checked={autoAllow} onChange={(event) => onAutoAllowChange(event.target.checked)} />
        <ShieldCheck size={16} />
        <span>Auto-select allow permission</span>
      </label>

      <div className="run-actions">
        <Button type="button" variant="primary" icon={<Play size={17} />} disabled={isRunning} onClick={onRun}>
          {isRunning ? "Running" : "Run"}
        </Button>
        <Button type="button" variant="secondary" icon={<Octagon size={16} />} disabled={!isRunning} onClick={onCancel}>
          Stop
        </Button>
      </div>

      {idleRemainingSec !== null ? (
        <p className="run-idle-hint" role="status">
          idle {idleRemainingSec} sec. 종료 예정
        </p>
      ) : null}

      <div className="run-meta">
        <span>Run ID</span>
        <code>{activeRunId ?? "not started"}</code>
      </div>
    </section>
  );
}
