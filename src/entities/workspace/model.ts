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

export type WorkspaceGitFileStatus = {
  path: string;
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
