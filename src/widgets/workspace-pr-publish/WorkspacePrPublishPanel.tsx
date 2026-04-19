import { GitCommitHorizontal, GitPullRequestArrow, RefreshCw, UploadCloud } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  createGitHubPullRequest,
  createWorkspaceCommit,
  getWorkspaceGitStatus,
  pushWorkspaceBranch,
  summarizeWorkspaceDiff,
} from "../../features/agent-run";
import type { GitHubPullRequestSummary, WorkspaceDiffSummary, WorkspaceGitStatus } from "../../entities/workspace";
import { Badge, Button, Input, Textarea } from "../../shared/ui";

type WorkspacePrPublishPanelProps = {
  workspaceId: string | null;
  checkoutId: string | null;
};

export function WorkspacePrPublishPanel({ workspaceId, checkoutId }: WorkspacePrPublishPanelProps) {
  const [status, setStatus] = useState<WorkspaceGitStatus | null>(null);
  const [summary, setSummary] = useState<WorkspaceDiffSummary | null>(null);
  const [commitMessage, setCommitMessage] = useState("");
  const [baseBranch, setBaseBranch] = useState("main");
  const [prTitle, setPrTitle] = useState("");
  const [prBody, setPrBody] = useState("");
  const [draft, setDraft] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<GitHubPullRequestSummary | null>(null);

  const changedFiles = useMemo(() => status?.files.map((file) => file.path) ?? [], [status?.files]);
  const branch = status?.branch?.trim() || "";
  const canPublish = Boolean(workspaceId && branch);

  const load = useCallback(async () => {
    if (!workspaceId) {
      setStatus(null);
      setSummary(null);
      setError(null);
      return;
    }
    setBusy("refresh");
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
      setBusy(null);
    }
  }, [checkoutId, workspaceId]);

  useEffect(() => {
    void load();
  }, [load]);

  const commit = useCallback(async () => {
    if (!workspaceId || changedFiles.length === 0) return;
    const message = commitMessage.trim();
    if (!message) {
      setError("Commit message is required.");
      return;
    }
    if (!window.confirm(`Commit ${changedFiles.length} changed file(s) in ${branch || "this checkout"}?`)) {
      return;
    }
    setBusy("commit");
    try {
      await createWorkspaceCommit({
        workspaceId,
        checkoutId,
        message,
        files: changedFiles,
        confirmed: true,
      });
      setError(null);
      await load();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  }, [branch, changedFiles, checkoutId, commitMessage, load, workspaceId]);

  const push = useCallback(async () => {
    if (!workspaceId || !branch) return;
    if (!window.confirm(`Push branch ${branch} to origin?`)) return;
    setBusy("push");
    try {
      await pushWorkspaceBranch({
        workspaceId,
        checkoutId,
        remote: "origin",
        branch,
        setUpstream: true,
        confirmed: true,
      });
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  }, [branch, checkoutId, workspaceId]);

  const createPullRequest = useCallback(async () => {
    if (!workspaceId || !branch) return;
    const title = prTitle.trim() || commitMessage.trim();
    if (!title) {
      setError("PR title is required.");
      return;
    }
    if (!baseBranch.trim()) {
      setError("Base branch is required.");
      return;
    }
    if (!window.confirm(`Create ${draft ? "draft " : ""}PR from ${branch} into ${baseBranch.trim()}?`)) {
      return;
    }
    setBusy("pr");
    try {
      const next = await createGitHubPullRequest({
        workspaceId,
        checkoutId,
        base: baseBranch.trim(),
        head: branch,
        title,
        body: prBody,
        draft,
        confirmed: true,
      });
      setResult(next);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  }, [baseBranch, branch, checkoutId, commitMessage, draft, prBody, prTitle, workspaceId]);

  if (!workspaceId) return null;

  return (
    <section className="mb-4 grid gap-3 rounded-lg border bg-card/80 p-3 shadow-sm">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <GitPullRequestArrow size={17} className="text-muted-foreground" />
          <span className="text-sm font-semibold text-foreground">Publish workspace changes</span>
          <Badge variant={status?.isDirty ? "secondary" : "default"}>
            {status?.isDirty ? `${changedFiles.length} files` : "clean"}
          </Badge>
          {branch ? (
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs text-muted-foreground">{branch}</code>
          ) : null}
        </div>
        <Button
          type="button"
          variant="outline"
          icon={<RefreshCw size={15} />}
          disabled={Boolean(busy)}
          onClick={() => void load()}
        >
          Refresh
        </Button>
      </div>

      {summary?.diffStat ? (
        <pre className="m-0 max-h-28 overflow-auto rounded-md bg-muted p-2.5 text-xs leading-relaxed text-muted-foreground">
          {summary.diffStat}
        </pre>
      ) : null}

      <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_minmax(220px,0.45fr)]">
        <label className="grid gap-2">
          <span className="text-sm font-medium">Commit message</span>
          <Input
            value={commitMessage}
            onChange={(event) => setCommitMessage(event.target.value)}
            placeholder="Describe the workspace changes"
          />
        </label>
        <label className="grid gap-2">
          <span className="text-sm font-medium">Base branch</span>
          <Input value={baseBranch} onChange={(event) => setBaseBranch(event.target.value)} />
        </label>
      </div>

      <label className="grid gap-2">
        <span className="text-sm font-medium">PR title</span>
        <Input
          value={prTitle}
          onChange={(event) => setPrTitle(event.target.value)}
          placeholder={commitMessage || "Pull request title"}
        />
      </label>

      <label className="grid gap-2">
        <span className="text-sm font-medium">PR body</span>
        <Textarea
          value={prBody}
          onChange={(event) => setPrBody(event.target.value)}
          placeholder="Summary, validation, and review notes"
          rows={5}
        />
      </label>

      <label className="flex items-center gap-2 text-sm font-medium">
        <input
          type="checkbox"
          className="h-4 w-4 accent-primary"
          checked={draft}
          onChange={(event) => setDraft(event.target.checked)}
        />
        <span>Create as draft PR</span>
      </label>

      <div className="grid gap-2 sm:grid-cols-3">
        <Button
          type="button"
          variant="secondary"
          icon={<GitCommitHorizontal size={16} />}
          disabled={Boolean(busy) || changedFiles.length === 0}
          onClick={() => void commit()}
        >
          Commit
        </Button>
        <Button
          type="button"
          variant="secondary"
          icon={<UploadCloud size={16} />}
          disabled={Boolean(busy) || !canPublish}
          onClick={() => void push()}
        >
          Push
        </Button>
        <Button
          type="button"
          variant="primary"
          icon={<GitPullRequestArrow size={16} />}
          disabled={Boolean(busy) || !canPublish}
          onClick={() => void createPullRequest()}
        >
          Create PR
        </Button>
      </div>

      {error ? <p className="m-0 text-sm font-medium text-destructive">{error}</p> : null}
      {result ? (
        <p className="m-0 text-sm font-medium text-primary">
          PR #{result.number ?? "created"}: {result.url}
        </p>
      ) : null}
    </section>
  );
}
