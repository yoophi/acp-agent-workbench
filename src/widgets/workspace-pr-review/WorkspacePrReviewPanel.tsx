import { FileText, GitPullRequest, RefreshCw } from "lucide-react";
import { useCallback, useState } from "react";
import { getGitHubPullRequestContext } from "../../features/agent-run";
import type { GitHubPullRequestContext } from "../../entities/workspace";
import { Badge, Button, Input } from "../../shared/ui";

type WorkspacePrReviewPanelProps = {
  workspaceId: string | null;
  checkoutId: string | null;
  onApplyReviewGoal: (goal: string) => void;
};

export function WorkspacePrReviewPanel({
  workspaceId,
  checkoutId,
  onApplyReviewGoal,
}: WorkspacePrReviewPanelProps) {
  const [numberText, setNumberText] = useState("");
  const [context, setContext] = useState<GitHubPullRequestContext | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!workspaceId) return;
    const number = Number(numberText);
    if (!Number.isInteger(number) || number <= 0) {
      setError("PR number is required.");
      return;
    }
    setLoading(true);
    try {
      const next = await getGitHubPullRequestContext({
        workspaceId,
        checkoutId,
        number,
      });
      setContext(next);
      setError(null);
    } catch (err) {
      setContext(null);
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [checkoutId, numberText, workspaceId]);

  const applyGoal = useCallback(() => {
    if (!context) return;
    onApplyReviewGoal(reviewGoalFromContext(context));
  }, [context, onApplyReviewGoal]);

  if (!workspaceId) return null;

  return (
    <section className="mb-4 grid gap-3 rounded-lg border bg-card/80 p-3 shadow-sm">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <GitPullRequest size={17} className="text-muted-foreground" />
          <span className="text-sm font-semibold text-foreground">Review GitHub PR</span>
          {context ? <Badge variant="secondary">{context.changedFiles.length} files</Badge> : null}
        </div>
        <div className="flex min-w-[220px] flex-1 justify-end gap-2">
          <Input
            className="max-w-36"
            inputMode="numeric"
            value={numberText}
            onChange={(event) => setNumberText(event.target.value)}
            placeholder="PR #"
          />
          <Button
            type="button"
            variant="outline"
            icon={<RefreshCw size={15} />}
            disabled={loading}
            onClick={() => void load()}
          >
            Load
          </Button>
        </div>
      </div>

      {error ? <p className="m-0 text-sm font-medium text-destructive">{error}</p> : null}

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

          <Button type="button" variant="secondary" icon={<GitPullRequest size={16} />} onClick={applyGoal}>
            Use as review goal
          </Button>
        </div>
      ) : null}
    </section>
  );
}

function reviewGoalFromContext(context: GitHubPullRequestContext) {
  const files = context.changedFiles.map((file) => `- ${file}`).join("\n");
  return [
    `Review GitHub PR #${context.number}: ${context.title}`,
    "",
    `URL: ${context.url}`,
    `Base: ${context.baseRef}`,
    `Head: ${context.headRef} (${context.headSha})`,
    context.body ? `\nPR body:\n${context.body}` : "",
    `\nChanged files:\n${files || "- none"}`,
    "",
    "Focus on correctness, regressions, missing tests, and actionable feedback. Return blocking findings first, then non-blocking suggestions.",
  ]
    .filter(Boolean)
    .join("\n");
}
