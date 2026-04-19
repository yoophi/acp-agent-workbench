export type ResumePolicy = "fresh" | "resumeIfAvailable" | "resumeRequired";

export type RalphLoopSettings = {
  enabled: boolean;
  maxIterations: number;
  promptTemplate: string;
  stopOnError: boolean;
  stopOnPermission: boolean;
  delayMs: number;
};

export type AgentRunRequest = {
  runId?: string;
  goal: string;
  agentId: string;
  workspaceId?: string;
  checkoutId?: string;
  cwd?: string;
  agentCommand?: string;
  stdioBufferLimitMb?: number;
  autoAllow?: boolean;
  resumeSessionId?: string;
  resumePolicy?: ResumePolicy;
  ralphLoop?: RalphLoopSettings;
};

export type AgentRun = {
  id: string;
  goal: string;
  agentId: string;
};

export type RunEventEnvelope = {
  runId: string;
  event: RunEvent;
};

export type RunEvent =
  | { type: "lifecycle"; status: LifecycleStatus; message: string }
  | { type: "agentMessage"; text: string }
  | { type: "thought"; text: string }
  | { type: "plan"; entries: PlanEntry[] }
  | { type: "tool"; status: string; title: string; locations: string[] }
  | { type: "usage"; used: number; size: number }
  | {
      type: "permission";
      permissionId?: string;
      title: string;
      input?: unknown;
      options: PermissionOption[];
      selected?: string;
      requiresResponse: boolean;
    }
  | { type: "fileSystem"; operation: string; path: string }
  | { type: "terminal"; operation: string; terminalId?: string; message: string }
  | { type: "diagnostic"; message: string }
  | { type: "raw"; method: string; payload: unknown }
  | { type: "error"; message: string };

export type LifecycleStatus =
  | "started"
  | "initialized"
  | "sessionCreated"
  | "promptSent"
  | "promptCompleted"
  | "completed"
  | "cancelled";

export type PlanEntry = {
  status: string;
  content: string;
};

export type PermissionOption = {
  name: string;
  kind: string;
  optionId: string;
};

export type EventGroup =
  | "assistant/message"
  | "thought"
  | "tool_call/tool_result"
  | "usage"
  | "permission"
  | "terminal"
  | "lifecycle"
  | "error"
  | "raw";

export type TimelineItem = {
  id: string;
  runId: string;
  group: EventGroup;
  title: string;
  body: string;
  tone?: "info" | "success" | "warning" | "danger";
  createdAt: number;
  event: RunEvent;
};
