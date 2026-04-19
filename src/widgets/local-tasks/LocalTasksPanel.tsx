import { AlertTriangle, CheckCircle2, ListChecks, Play, RefreshCw, SendToBack } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { LocalTaskList, LocalTaskSummary } from "../../entities/workspace";
import { listLocalTasks } from "../../features/agent-run";
import { Badge, Button, NativeSelect, Textarea } from "../../shared/ui";

type LocalTasksPanelProps = {
  workspaceId: string | null;
  checkoutId: string | null;
  sessionActive?: boolean;
  onApplyTaskGoal: (goal: string, task: LocalTaskSummary) => void;
  onRunTask: (task: LocalTaskSummary, goal: string) => void;
  onError: (error: string | null) => void;
};

const EMPTY_TASKS: LocalTaskSummary[] = [];

export function LocalTasksPanel({
  workspaceId,
  checkoutId,
  sessionActive = false,
  onApplyTaskGoal,
  onRunTask,
  onError,
}: LocalTasksPanelProps) {
  const [result, setResult] = useState<LocalTaskList | null>(null);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [statusFilter, setStatusFilter] = useState("all");
  const [blockedFilter, setBlockedFilter] = useState("all");
  const [labelFilter, setLabelFilter] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!workspaceId) {
      setResult(null);
      setSelectedTaskId(null);
      setError(null);
      return;
    }
    setLoading(true);
    try {
      const nextResult = await listLocalTasks(workspaceId, checkoutId);
      setResult(nextResult);
      setSelectedTaskId((current) => {
        if (current && nextResult.tasks.some((task) => task.id === current)) return current;
        return nextResult.tasks[0]?.id ?? null;
      });
      setError(nextResult.available ? null : nextResult.error ?? "Local task data is unavailable.");
      onError(null);
    } catch (err) {
      const message = String(err);
      setError(message);
      onError(message);
    } finally {
      setLoading(false);
    }
  }, [checkoutId, onError, workspaceId]);

  useEffect(() => {
    void load();
  }, [load]);

  const tasks = result?.tasks ?? EMPTY_TASKS;
  const statusOptions = useMemo(
    () =>
      Array.from(new Set(tasks.map((task) => task.status).filter(Boolean) as string[])).sort(),
    [tasks],
  );
  const labelOptions = useMemo(
    () => Array.from(new Set(tasks.flatMap((task) => task.labels))).sort(),
    [tasks],
  );
  const visibleTasks = useMemo(() => {
    return tasks.filter((task) => {
      if (statusFilter !== "all" && task.status !== statusFilter) return false;
      if (blockedFilter === "blocked" && !task.blocked) return false;
      if (blockedFilter === "ready" && task.blocked) return false;
      if (labelFilter && !task.labels.includes(labelFilter)) return false;
      return true;
    });
  }, [blockedFilter, labelFilter, statusFilter, tasks]);
  const selectedTask =
    visibleTasks.find((task) => task.id === selectedTaskId) ??
    tasks.find((task) => task.id === selectedTaskId) ??
    visibleTasks[0] ??
    null;

  if (!workspaceId) return null;

  return (
    <section className="mb-4 grid gap-3 rounded-lg border bg-card/80 p-3 shadow-sm">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <ListChecks size={17} className="text-muted-foreground" />
          <span className="text-sm font-semibold text-foreground">Local tasks</span>
          <Badge variant={result?.available ? "default" : "secondary"}>
            {result?.available ? `${tasks.length} beads tasks` : "beads unavailable"}
          </Badge>
          {result?.workdir ? (
            <code className="max-w-[420px] truncate rounded bg-muted px-1.5 py-0.5 font-mono text-xs text-muted-foreground">
              {result.workdir}
            </code>
          ) : null}
        </div>
        <Button
          type="button"
          variant="outline"
          icon={<RefreshCw size={15} />}
          disabled={loading}
          onClick={() => void load()}
        >
          Task refresh
        </Button>
      </div>

      {error ? (
        <div className="flex items-start gap-2 rounded-md border border-border bg-muted/45 px-3 py-2 text-sm text-muted-foreground">
          <AlertTriangle size={16} className="mt-0.5 shrink-0 text-muted-foreground" />
          <span>{error}</span>
        </div>
      ) : null}

      {result?.available ? (
        <div className="grid gap-3">
          <div className="grid grid-cols-[minmax(120px,0.8fr)_minmax(120px,0.8fr)_minmax(140px,1fr)] gap-2 max-md:grid-cols-1">
            <NativeSelect value={statusFilter} onChange={(event) => setStatusFilter(event.target.value)}>
              <option value="all">All statuses</option>
              {statusOptions.map((status) => (
                <option key={status} value={status}>
                  {status}
                </option>
              ))}
            </NativeSelect>
            <NativeSelect value={blockedFilter} onChange={(event) => setBlockedFilter(event.target.value)}>
              <option value="all">All dependency states</option>
              <option value="ready">Ready only</option>
              <option value="blocked">Blocked only</option>
            </NativeSelect>
            <NativeSelect value={labelFilter} onChange={(event) => setLabelFilter(event.target.value)}>
              <option value="">All labels</option>
              {labelOptions.map((label) => (
                <option key={label} value={label}>
                  {label}
                </option>
              ))}
            </NativeSelect>
          </div>

          <div className="grid grid-cols-[minmax(220px,0.75fr)_minmax(260px,1fr)] gap-3 max-lg:grid-cols-1">
            <div className="grid max-h-80 gap-1.5 overflow-auto pr-1">
              {visibleTasks.map((task) => (
                <button
                  key={task.id}
                  type="button"
                  className="grid gap-1 rounded-md border bg-background px-3 py-2 text-left transition-colors hover:bg-accent hover:text-accent-foreground data-[selected=true]:border-primary data-[selected=true]:bg-primary/10"
                  data-selected={selectedTask?.id === task.id}
                  onClick={() => setSelectedTaskId(task.id)}
                >
                  <span className="flex min-w-0 items-center justify-between gap-2">
                    <span className="truncate text-sm font-semibold text-foreground">{task.title}</span>
                    <code className="shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-[11px] text-muted-foreground">
                      {task.id}
                    </code>
                  </span>
                  <span className="flex flex-wrap gap-1.5">
                    {task.status ? <Badge variant="outline">{task.status}</Badge> : null}
                    {task.priority ? (
                      <Badge variant="secondary">{priorityLabel(task.priority)}</Badge>
                    ) : null}
                    {task.blocked ? (
                      <Badge variant="destructive">blocked</Badge>
                    ) : (
                      <Badge variant="default">ready</Badge>
                    )}
                  </span>
                </button>
              ))}
              {visibleTasks.length === 0 ? (
                <p className="m-0 rounded-md border border-dashed px-3 py-4 text-sm text-muted-foreground">
                  No tasks match the current filters.
                </p>
              ) : null}
            </div>

            {selectedTask ? (
              <TaskDetails
                task={selectedTask}
                sessionActive={sessionActive}
                onApplyTaskGoal={onApplyTaskGoal}
                onRunTask={onRunTask}
              />
            ) : null}
          </div>
        </div>
      ) : null}
    </section>
  );
}

type TaskDetailsProps = {
  task: LocalTaskSummary;
  sessionActive: boolean;
  onApplyTaskGoal: (goal: string, task: LocalTaskSummary) => void;
  onRunTask: (task: LocalTaskSummary, goal: string) => void;
};

function TaskDetails({ task, sessionActive, onApplyTaskGoal, onRunTask }: TaskDetailsProps) {
  const goal = useMemo(() => composeTaskGoal(task), [task]);

  return (
    <div className="grid gap-3 rounded-md border bg-background p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="mb-1 flex flex-wrap items-center gap-1.5">
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs text-muted-foreground">
              {task.id}
            </code>
            {task.status ? <Badge variant="outline">{task.status}</Badge> : null}
            {task.blocked ? (
              <Badge variant="destructive">blocked</Badge>
            ) : (
              <Badge variant="default">
                <CheckCircle2 size={12} />
                ready
              </Badge>
            )}
          </div>
          <h2 className="m-0 text-base font-semibold leading-snug text-foreground">{task.title}</h2>
        </div>
        <Button
          type="button"
          variant="secondary"
          size="sm"
          icon={<SendToBack size={14} />}
          disabled={sessionActive}
          onClick={() => onApplyTaskGoal(goal, task)}
        >
          Use as goal
        </Button>
        <Button
          type="button"
          variant="primary"
          size="sm"
          icon={<Play size={14} />}
          disabled={sessionActive}
          onClick={() => onRunTask(task, goal)}
          title={task.blocked ? "Blocked tasks require confirmation before running" : undefined}
        >
          Run task
        </Button>
      </div>

      {task.description ? (
        <p className="m-0 whitespace-pre-wrap text-sm leading-relaxed text-muted-foreground">
          {task.description}
        </p>
      ) : null}

      <div className="grid grid-cols-2 gap-2 text-sm max-sm:grid-cols-1">
        <Metadata label="Priority" value={task.priority ? priorityLabel(task.priority) : "none"} />
        <Metadata label="Dependencies" value={task.dependencies.length ? task.dependencies.join(", ") : "none"} />
      </div>

      {task.labels.length > 0 ? (
        <div className="flex flex-wrap gap-1.5">
          {task.labels.map((label) => (
            <Badge key={label} variant="secondary">
              {label}
            </Badge>
          ))}
        </div>
      ) : null}

      {task.acceptanceCriteria ? (
        <div className="grid gap-1.5 rounded-md bg-muted/60 p-2.5">
          <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Acceptance criteria
          </span>
          <p className="m-0 whitespace-pre-wrap text-sm leading-relaxed text-foreground">
            {task.acceptanceCriteria}
          </p>
        </div>
      ) : null}

      <Textarea value={goal} readOnly aria-label="Generated task goal preview" rows={5} />
    </div>
  );
}

function Metadata({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid gap-1 rounded-md bg-muted/50 px-2.5 py-2">
      <span className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">{label}</span>
      <span className="break-words text-sm text-foreground">{value}</span>
    </div>
  );
}

function composeTaskGoal(task: LocalTaskSummary) {
  const sections = [`Work on local task ${task.id}: ${task.title}`];
  if (task.description) sections.push(`Description:\n${task.description}`);
  if (task.acceptanceCriteria) {
    sections.push(`Acceptance criteria:\n${task.acceptanceCriteria}`);
  }
  if (task.dependencies.length > 0) {
    sections.push(`Dependencies:\n${task.dependencies.map((dependency) => `- ${dependency}`).join("\n")}`);
  }
  if (task.labels.length > 0) sections.push(`Labels: ${task.labels.join(", ")}`);
  return sections.join("\n\n");
}

function priorityLabel(priority: string) {
  return priority.toUpperCase().startsWith("P") ? priority : `P${priority}`;
}
