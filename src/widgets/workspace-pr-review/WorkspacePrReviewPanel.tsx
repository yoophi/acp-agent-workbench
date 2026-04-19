import { ClipboardCopy, FileText, GitPullRequest, RefreshCw, Send } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { getGitHubPullRequestContext, submitGitHubPullRequestReview } from "../../features/agent-run";
import type {
  GitHubPullRequestContext,
  GitHubPullRequestReviewComment,
  GitHubPullRequestReviewDecision,
  GitHubPullRequestReviewResult,
} from "../../entities/workspace";
import { Badge, Button, Input, NativeSelect, Textarea } from "../../shared/ui";

type WorkspacePrReviewPanelProps = {
  workspaceId: string | null;
  checkoutId: string | null;
  onApplyReviewGoal: (goal: string) => void;
};

const decisionLabels: Record<GitHubPullRequestReviewDecision, string> = {
  comment: "Comment",
  approve: "Approve",
  requestChanges: "Request changes",
};

export function WorkspacePrReviewPanel({
  workspaceId,
  checkoutId,
  onApplyReviewGoal,
}: WorkspacePrReviewPanelProps) {
  const [numberInput, setNumberInput] = useState("");
  const [context, setContext] = useState<GitHubPullRequestContext | null>(null);
  const [reviewBody, setReviewBody] = useState("");
  const [commentDraft, setCommentDraft] = useState("");
  const [decision, setDecision] = useState<GitHubPullRequestReviewDecision>("comment");
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<GitHubPullRequestReviewResult | null>(null);

  const prNumber = Number.parseInt(numberInput.trim(), 10);
  const parsedComments = useMemo(() => parseCommentDraft(commentDraft), [commentDraft]);
  const reviewGoal = useMemo(() => buildReviewGoal(context), [context]);

  const loadContext = useCallback(async () => {
    if (!workspaceId || !Number.isInteger(prNumber) || prNumber <= 0) {
      setError("Valid PR number is required.");
      return;
    }
    setBusy("load");
    try {
      const next = await getGitHubPullRequestContext({ workspaceId, checkoutId, number: prNumber });
      setContext(next);
      setReviewBody((current) => current || `Review for ${next.title}\n\n`);
      setResult(null);
      setError(null);
    } catch (err) {
      setContext(null);
      setError(String(err));
    } finally {
      setBusy(null);
    }
  }, [checkoutId, prNumber, workspaceId]);

  const applyGoal = useCallback(() => {
    if (!reviewGoal) return;
    onApplyReviewGoal(reviewGoal);
  }, [onApplyReviewGoal, reviewGoal]);

  const copyGoal = useCallback(async () => {
    if (!reviewGoal) return;
    setBusy("copy");
    try {
      await navigator.clipboard.writeText(reviewGoal);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  }, [reviewGoal]);

  const submitReview = useCallback(async () => {
    if (!workspaceId || !context) return;
    const body = reviewBody.trim();
    const comments = parsedComments.validComments;
    if (!body && comments.length === 0) {
      setError("Review body or file comments are required.");
      return;
    }
    if (parsedComments.invalidLines.length > 0) {
      setError(`Invalid file comment lines: ${parsedComments.invalidLines.join(", ")}`);
      return;
    }
    if (!window.confirm(`Publish ${decisionLabels[decision]} review on PR #${context.number}?`)) {
      return;
    }
    setBusy("submit");
    try {
      const next = await submitGitHubPullRequestReview({
        workspaceId,
        checkoutId,
        number: context.number,
        body,
        decision,
        comments,
        confirmed: true,
      });
      setResult(next);
      setError(null);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  }, [checkoutId, context, decision, parsedComments, reviewBody, workspaceId]);

  if (!workspaceId) return null;

  return (
    <section className="mb-4 grid gap-3 rounded-lg border bg-card/80 p-3 shadow-sm">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <GitPullRequest size={17} className="text-muted-foreground" />
          <span className="text-sm font-semibold text-foreground">Review GitHub PR</span>
          {context ? <Badge variant="secondary">{context.changedFiles.length} files</Badge> : null}
          {context?.headRef ? (
            <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs text-muted-foreground">
              {context.headRef}
            </code>
          ) : null}
        </div>
        <div className="flex min-w-[220px] flex-1 justify-end gap-2">
          <Input
            className="max-w-36"
            inputMode="numeric"
            value={numberInput}
            onChange={(event) => setNumberInput(event.target.value)}
            placeholder="PR #"
          />
          <Button
            type="button"
            variant="outline"
            icon={<RefreshCw size={15} />}
            disabled={Boolean(busy)}
            onClick={() => void loadContext()}
          >
            Load
          </Button>
        </div>
      </div>

      {context ? (
        <div className="grid gap-3">
          <div className="grid gap-1.5">
            <a
              className="truncate text-sm font-semibold text-primary underline-offset-4 hover:underline"
              href={context.url}
              target="_blank"
              rel="noreferrer"
            >
              #{context.number} {context.title}
            </a>
            <p className="m-0 text-xs text-muted-foreground">
              {context.baseRef} {"<-"} {context.headRef} - {context.headSha.slice(0, 8)}
              {context.author ? ` - ${context.author}` : ""}
            </p>
          </div>

          <div className="grid gap-1.5">
            {context.changedFiles.slice(0, 8).map((file) => (
              <div key={file} className="flex items-center gap-2 rounded-md bg-muted/50 px-2.5 py-1.5 text-sm">
                <FileText size={14} className="shrink-0 text-muted-foreground" />
                <span className="truncate text-foreground">{file}</span>
              </div>
            ))}
            {context.changedFiles.length > 8 ? (
              <p className="m-0 text-xs text-muted-foreground">+{context.changedFiles.length - 8} more files</p>
            ) : null}
          </div>

          <pre className="m-0 max-h-40 overflow-auto rounded-md bg-muted p-2.5 text-xs leading-relaxed text-muted-foreground">
            {context.diff}
          </pre>

          <div className="grid gap-2">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="text-sm font-medium">Agent review goal</span>
              <div className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  variant="outline"
                  icon={<ClipboardCopy size={15} />}
                  disabled={Boolean(busy)}
                  onClick={() => void copyGoal()}
                >
                  Copy
                </Button>
                <Button type="button" variant="secondary" icon={<GitPullRequest size={16} />} onClick={applyGoal}>
                  Use as goal
                </Button>
              </div>
            </div>
            <Textarea value={reviewGoal} readOnly rows={6} />
          </div>
        </div>
      ) : null}

      <div className="grid gap-3 md:grid-cols-[minmax(160px,0.35fr)_minmax(0,1fr)]">
        <label className="grid gap-2">
          <span className="text-sm font-medium">Review decision</span>
          <NativeSelect
            value={decision}
            onChange={(event) => setDecision(event.target.value as GitHubPullRequestReviewDecision)}
          >
            <option value="comment">Comment</option>
            <option value="approve">Approve</option>
            <option value="requestChanges">Request changes</option>
          </NativeSelect>
        </label>
        <label className="grid gap-2">
          <span className="text-sm font-medium">File comment draft</span>
          <Textarea
            value={commentDraft}
            onChange={(event) => setCommentDraft(event.target.value)}
            placeholder="src/file.ts:42 | Comment text"
            rows={4}
          />
        </label>
      </div>

      <label className="grid gap-2">
        <span className="text-sm font-medium">Review body draft</span>
        <Textarea
          value={reviewBody}
          onChange={(event) => setReviewBody(event.target.value)}
          placeholder="Summary, findings, questions, and recommendation"
          rows={6}
        />
      </label>

      <Button
        type="button"
        variant="primary"
        icon={<Send size={16} />}
        disabled={Boolean(busy) || !context}
        onClick={() => void submitReview()}
      >
        Publish review
      </Button>

      {error ? <p className="m-0 text-sm font-medium text-destructive">{error}</p> : null}
      {result?.submitted ? (
        <p className="m-0 text-sm font-medium text-primary">
          Published {decisionLabels[result.decision]} review on PR #{result.number}.
        </p>
      ) : null}
    </section>
  );
}

function buildReviewGoal(context: GitHubPullRequestContext | null) {
  if (!context) return "";
  const files = context.changedFiles.map((file) => `- ${file}`).join("\n");
  return [
    `Review GitHub PR #${context.number}: ${context.title}`,
    "",
    `URL: ${context.url}`,
    `Base: ${context.baseRef}`,
    `Head: ${context.headRef} (${context.headSha})`,
    `Author: ${context.author ?? "unknown"}`,
    context.body ? `\nPR body:\n${context.body}` : "",
    `\nChanged files:\n${files || "- none"}`,
    "",
    "Diff:",
    context.diff,
    "",
    "Focus on correctness, regressions, missing tests, and actionable feedback. Return blocking findings first, then non-blocking suggestions.",
  ]
    .filter(Boolean)
    .join("\n");
}

function parseCommentDraft(value: string): {
  validComments: GitHubPullRequestReviewComment[];
  invalidLines: number[];
} {
  const validComments: GitHubPullRequestReviewComment[] = [];
  const invalidLines: number[] = [];

  value.split(/\r?\n/).forEach((rawLine, index) => {
    const line = rawLine.trim();
    if (!line) return;
    const match = /^(?<path>[^:|]+?)(?::(?<line>\d+))?\s*\|\s*(?<body>.+)$/.exec(line);
    if (!match?.groups) {
      invalidLines.push(index + 1);
      return;
    }
    validComments.push({
      path: match.groups.path.trim(),
      line: match.groups.line ? Number.parseInt(match.groups.line, 10) : null,
      body: match.groups.body.trim(),
    });
  });

  return { validComments, invalidLines };
}
