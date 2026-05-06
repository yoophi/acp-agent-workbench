export type GitOrigin = {
  rawUrl: string;
  canonicalUrl: string;
  host: string;
  owner: string;
  repo: string;
};

export type Workspace = {
  id: string;
  name: string;
  origin: GitOrigin;
  defaultCheckoutId?: string | null;
  createdAt: string;
  updatedAt: string;
};

export type WorkspaceCheckout = {
  id: string;
  workspaceId: string;
  path: string;
  kind: "clone" | "worktree";
  branch?: string | null;
  headSha?: string | null;
  isDefault: boolean;
};

export type RegisteredWorkspace = {
  workspace: Workspace;
  checkout: WorkspaceCheckout;
};

export type LocalTaskSummary = {
  id: string;
  title: string;
  description?: string | null;
  status?: string | null;
  priority?: string | null;
  labels: string[];
  dependencies: string[];
  blocked: boolean;
  acceptanceCriteria?: string | null;
};

export type LocalTaskStatus = "open" | "in_progress" | "closed";

export type LocalTaskList = {
  workspaceId: string;
  checkoutId: string;
  workdir: string;
  source: "beads";
  detected: boolean;
  available: boolean;
  tasks: LocalTaskSummary[];
  error?: string | null;
};

export type WorkspaceGitFileStatus = {
  path: string;
  previousPath?: string | null;
  statusCode: string;
  statusLabel: string;
};

export type WorkspaceGitStatus = {
  root: string;
  branch?: string | null;
  headSha?: string | null;
  isDirty: boolean;
  files: WorkspaceGitFileStatus[];
};

export type WorkspaceDiffSummary = {
  status: WorkspaceGitStatus;
  diffStat: string;
};

export type WorkspaceCommitRequest = {
  workspaceId: string;
  checkoutId?: string | null;
  message: string;
  files: string[];
  confirmed: boolean;
};

export type WorkspaceCommitResult = {
  commitSha: string;
  status: WorkspaceGitStatus;
};

export type WorkspacePushRequest = {
  workspaceId: string;
  checkoutId?: string | null;
  remote?: string | null;
  branch?: string | null;
  setUpstream: boolean;
  confirmed: boolean;
};

export type WorkspacePushResult = {
  remote: string;
  branch: string;
};

export type GitHubPullRequestCreateRequest = {
  workspaceId: string;
  checkoutId?: string | null;
  base: string;
  head?: string | null;
  title: string;
  body: string;
  draft: boolean;
  confirmed: boolean;
};

export type GitHubPullRequestSummary = {
  number?: number | null;
  url: string;
  title: string;
  baseRef: string;
  headRef: string;
};

export type GitHubPullRequestContextRequest = {
  workspaceId: string;
  checkoutId?: string | null;
  number: number;
};

export type GitHubPullRequestContext = {
  number: number;
  url: string;
  title: string;
  body?: string | null;
  author?: string | null;
  baseRef: string;
  headRef: string;
  headSha: string;
  changedFiles: string[];
  diff: string;
};

export type GitHubPullRequestReviewComment = {
  path: string;
  line?: number | null;
  body: string;
};

export type GitHubPullRequestReviewDecision = "comment" | "approve" | "requestChanges";

export type GitHubPullRequestReviewRequest = {
  workspaceId: string;
  checkoutId?: string | null;
  number: number;
  body: string;
  decision: GitHubPullRequestReviewDecision;
  comments: GitHubPullRequestReviewComment[];
  confirmed: boolean;
};

export type GitHubPullRequestReviewResult = {
  number: number;
  decision: GitHubPullRequestReviewDecision;
  submitted: boolean;
};

export type PullRequestReviewDecision = "comment" | "approve" | "request_changes";
export type PullRequestReviewCommentSide = "LEFT" | "RIGHT";

export type PullRequestReviewComment = {
  path: string;
  line?: number | null;
  side?: PullRequestReviewCommentSide | null;
  body: string;
};

export type PullRequestReviewDraft = {
  id: string;
  workspaceId: string;
  checkoutId?: string | null;
  pullRequestNumber: number;
  runId?: string | null;
  summary: string;
  decision: PullRequestReviewDecision;
  comments: PullRequestReviewComment[];
  createdAt: string;
  updatedAt: string;
};

export type CreatePullRequestReviewDraftInput = {
  workspaceId: string;
  checkoutId?: string | null;
  pullRequestNumber: number;
  runId?: string | null;
  summary: string;
  decision: PullRequestReviewDecision;
  comments: PullRequestReviewComment[];
};

export type UpdatePullRequestReviewDraftPatch = Partial<
  Pick<CreatePullRequestReviewDraftInput, "checkoutId" | "runId" | "summary" | "decision" | "comments">
>;
