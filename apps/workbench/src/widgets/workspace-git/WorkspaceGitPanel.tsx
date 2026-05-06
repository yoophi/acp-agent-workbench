import { GitPullRequestDraft, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  getWorkspaceGitStatus,
  summarizeWorkspaceDiff,
} from "../../features/agent-run";
import type { WorkspaceDiffSummary, WorkspaceGitStatus } from "../../entities/workspace";
import { Badge, Button } from "../../shared/ui";

type WorkspaceGitPanelProps = {
  workspaceId: string | null;
  checkoutId: string | null;
};

export function WorkspaceGitPanel({ workspaceId, checkoutId }: WorkspaceGitPanelProps) {
  const [status, setStatus] = useState<WorkspaceGitStatus | null>(null);
  const [summary, setSummary] = useState<WorkspaceDiffSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!workspaceId) {
      setStatus(null);
      setSummary(null);
      setError(null);
      return;
    }
    setLoading(true);
    try {
      const [nextStatus, nextSummary] = await Promise.all([
        getWorkspaceGitStatus(workspaceId, checkoutId),
        summarizeWorkspaceDiff(workspaceId, checkoutId),
      ]);
      setStatus(nextStatus);
      setSummary(nextSummary);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [checkoutId, workspaceId]);

  useEffect(() => {
    void load();
  }, [load]);

  const visibleFiles = useMemo(() => status?.files.slice(0, 8) ?? [], [status?.files]);

  if (!workspaceId) return null;

  return (
    <section className="mb-4 grid gap-3 rounded-lg border bg-card/80 p-3 shadow-sm">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <GitPullRequestDraft size={17} className="text-muted-foreground" />
          <span className="text-sm font-semibold text-foreground">
            {status?.branch || "detached"}
          </span>
          {status?.headSha ? (
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs text-muted-foreground">
              {status.headSha.slice(0, 8)}
            </code>
          ) : null}
          <Badge variant={status?.isDirty ? "secondary" : "default"}>
            {status?.isDirty ? `${status.files.length} changed` : "clean"}
          </Badge>
        </div>
        <Button
          type="button"
          variant="outline"
          icon={<RefreshCw size={15} />}
          disabled={loading}
          onClick={() => void load()}
        >
          Git refresh
        </Button>
      </div>

      {error ? <p className="m-0 text-sm font-medium text-destructive">{error}</p> : null}

      {visibleFiles.length > 0 ? (
        <div className="grid gap-1.5">
          {visibleFiles.map((file) => (
            <div
              key={`${file.path}:${file.statusCode}`}
              className="grid grid-cols-[72px_minmax(0,1fr)] gap-2 rounded-md bg-muted/50 px-2.5 py-1.5 text-sm"
            >
              <span className="font-mono text-xs text-muted-foreground">{file.statusCode}</span>
              <span className="truncate text-foreground">{file.path}</span>
            </div>
          ))}
          {status && status.files.length > visibleFiles.length ? (
            <p className="m-0 text-xs text-muted-foreground">
              +{status.files.length - visibleFiles.length} more files
            </p>
          ) : null}
        </div>
      ) : null}

      {summary?.diffStat ? (
        <pre className="m-0 max-h-32 overflow-auto rounded-md bg-muted p-2.5 text-xs leading-relaxed text-muted-foreground">
          {summary.diffStat}
        </pre>
      ) : null}
    </section>
  );
}
