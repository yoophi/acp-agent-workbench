export type AcpSessionRecord = {
  runId: string;
  sessionId: string;
  workspaceId?: string | null;
  checkoutId?: string | null;
  workdir?: string | null;
  agentId: string;
  agentCommand?: string | null;
  task: string;
  createdAt: string;
  updatedAt: string;
};

export type AcpSessionListQuery = {
  workspaceId?: string | null;
  checkoutId?: string | null;
  workdir?: string | null;
  agentId?: string | null;
  agentCommand?: string | null;
  limit?: number | null;
};
