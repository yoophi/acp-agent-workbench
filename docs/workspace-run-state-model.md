# Workspace-scoped Run State Model

## 1. Purpose

Issue #13 should move the workbench from flat agent-run tabs to workspace-scoped work areas. This document defines the frontend state model before implementation so the change can be split into small PRs.

The current app already has persistent `Workspace` and `WorkspaceCheckout` entities, plus `TabState.workspaceId`, `checkoutId`, and `cwd`. The remaining problem is ownership: a tab currently mixes workspace selection, run configuration, live run state, timeline data, and UI-only presentation state.

## 2. Current `TabState` Classification

| Current field | Proposed owner | Notes |
| --- | --- | --- |
| `id` | Workspace view | Today this is a tab id. In #13 it becomes the workspace view id, not a run id. |
| `title` | Workspace view | User-facing label for the workspace view. Default can come from workspace name or checkout path. |
| `workspaceId` | Workspace view | Selects the repo-origin workspace. |
| `checkoutId` | Workspace view | Selects the active checkout/worktree inside the workspace. |
| `cwd` | Workspace view | Workspace-level working directory baseline. Runs may snapshot it at start. |
| `selectedAgentId` | Run draft | Part of the next run request. |
| `goal` | Run draft | Part of the next run request. |
| `customCommand` | Run draft | Part of the next run request. |
| `stdioBufferLimitMb` | Run draft | Part of the next run request. |
| `autoAllow` | Run draft | Part of the next run request. |
| `idleTimeoutSec` | Run draft or workspace default | Keep on draft first; later promote to workspace default if needed. |
| `activeRunId` | Workspace view selection | Points at the selected live or recent run within a workspace view. |
| `sessionActive` | Agent run | Derived from the selected run. |
| `awaitingResponse` | Agent run | Derived from the selected run. |
| `idleRemainingSec` | Agent run | Runtime countdown for a specific run. |
| `followUpDraft` | Workspace view UI | Draft text belongs to the visible composer, not the historical run. |
| `followUpQueue` | Agent run | Queue entries target a concrete `runId`. |
| `items` | Agent run | Timeline is run-owned. |
| `filter` | Workspace view UI | Event filter is a view preference. |
| `error` | Agent run or workspace view | Start/cancel errors can be view-owned; runtime errors are run-owned. Store both separately. |
| `unreadCount` | Workspace view UI | Counts events for non-active workspace views. |
| `permissionPending` | Agent run | Runtime state from permission events. |
| `closing` | Workspace view UI | View-close lifecycle, independent from a run after cancellation starts. |

## 3. Proposed Store Shape

```ts
type WorkspaceViewState = {
  id: string;
  title: string;
  workspaceId: string | null;
  checkoutId: string | null;
  cwd: string;
  activeRunId: string | null;
  draft: AgentRunDraft;
  followUpDraft: string;
  filter: EventGroup | "all";
  viewError: string | null;
  unreadCount: number;
  closing: boolean;
};

type AgentRunDraft = {
  selectedAgentId: string;
  goal: string;
  customCommand: string;
  stdioBufferLimitMb: number;
  autoAllow: boolean;
  idleTimeoutSec: number;
};

type AgentRunState = {
  id: string;
  workspaceViewId: string;
  workspaceId: string | null;
  checkoutId: string | null;
  cwd: string;
  request: AgentRunDraft;
  sessionActive: boolean;
  awaitingResponse: boolean;
  idleRemainingSec: number | null;
  permissionPending: boolean;
  followUpQueue: FollowUpQueueItem[];
  items: TimelineItem[];
  runError: string | null;
  createdAt: number;
  completedAt: number | null;
};

type WorkbenchState = {
  workspaces: Workspace[];
  checkoutsByWorkspaceId: Record<string, WorkspaceCheckout[]>;
  workspaceError: string | null;
  workspaceViews: WorkspaceViewState[];
  runsById: Record<string, AgentRunState>;
  activeWorkspaceViewId: string;
};
```

Key decisions:

- `cwd` is owned by the workspace view and copied into `AgentRunState` when a run starts. Changing the workspace view directory later does not rewrite historical runs.
- The draft prompt/agent settings live on the workspace view until start. The run stores an immutable request snapshot for replay, diagnostics, and future persistence.
- Timeline, permission pending state, follow-up queue, and lifecycle flags are run-owned.
- The active run is selected per workspace view, so a workspace can keep more than one historical run without changing the top-level view structure.

## 4. Selector and Action API

Replace tab-centric selectors with workspace-view selectors first, then migrate call sites.

```ts
selectActiveWorkspaceView(state): WorkspaceViewState
selectWorkspaceView(state, workspaceViewId): WorkspaceViewState | undefined
selectActiveRun(state, workspaceViewId): AgentRunState | undefined
selectRun(state, runId): AgentRunState | undefined
selectWorkspaceViewRuns(state, workspaceViewId): AgentRunState[]
```

Actions:

```ts
addWorkspaceView(preset?: Partial<WorkspaceViewState>): string
closeWorkspaceView(viewId: string): string | null
forceCloseWorkspaceView(viewId: string): string | null
activateWorkspaceView(viewId: string): void
patchWorkspaceView(viewId: string, patch: Partial<WorkspaceViewState>): void
patchRun(runId: string, patch: Partial<AgentRunState>): void
beginRun(viewId: string, run: AgentRun): void
dispatchRunEvent(runId: string, event: RunEvent): void
selectRunForWorkspaceView(viewId: string, runId: string): void
```

Existing widgets should keep receiving plain props. `useAgentRun(viewId)` becomes the compatibility layer that combines `WorkspaceViewState`, selected `AgentRunState`, and actions into the current widget-facing shape.

## 5. Migration Path

The first implementation PR can migrate in memory without persistence changes.

1. Rename UI language from tab to workspace view in store internals while keeping component names temporarily.
2. Convert each current `TabState` into one `WorkspaceViewState`.
3. If `TabState.activeRunId` is present, create one `AgentRunState` from the run-owned fields and set `activeRunId`.
4. If there is no active run, keep only the workspace view plus draft.
5. Preserve the existing last-view behavior: at least one workspace view always exists.

Compatibility mapping:

```text
TabState.id             -> WorkspaceViewState.id
TabState.goal           -> WorkspaceViewState.draft.goal
TabState.cwd            -> WorkspaceViewState.cwd
TabState.items          -> AgentRunState.items, only when activeRunId exists
TabState.followUpQueue  -> AgentRunState.followUpQueue, only when activeRunId exists
```

## 6. Backend Impact

No backend schema or command changes are required for #13.

The backend already accepts `workspace_id`, `checkout_id`, and `cwd` in `AgentRunRequest`, validates workspace workdirs, stores workspaces/checkouts in SQLite WAL mode, and routes events by `runId`. #13 is therefore a frontend state and UI restructuring task.

Future backend work may be useful but should stay out of #13:

- Persist `AgentRunState` and timeline events.
- Add explicit workspace-view or window ownership for #8.
- Add directory leases or conflict metadata for same-directory parallel runs.

## 7. Multi-window Compatibility

The model does not assume one global owner for a workspace. For #8, add an owner field without changing run ownership:

```ts
type WorkspaceViewState = {
  id: string;
  ownerWindowId: string | null;
  // ...
};
```

`AgentRunState.workspaceViewId` remains a logical owner. A view can move between windows, and the run continues to route by `runId`.

## 8. Implementation PR Split for #13

```text
[#28 design]
   |
   v
[PR A] Store shape migration
   - introduce WorkspaceViewState / AgentRunState
   - add selectors and compatibility helpers
   - keep UI visually unchanged
   |
   v
[PR B] Event and run action migration
   - route dispatchRunEvent into runsById
   - move follow-up queue and lifecycle flags to AgentRunState
   - add focused store tests
   |
   v
[PR C] Workspace-scoped UI language
   - rename tab-facing labels/actions where user-visible
   - expose active run selection inside a workspace view
   - keep existing close/cancel behavior
   |
   v
[PR D] Historical run list within a workspace
   - show current/recent runs for the selected workspace view
   - allow switching selected run timeline
   - no persistence yet
```

Parallelizable follow-ups after PR A:

```text
                 +--> [tests for selectors/actions]
[PR A complete] -+
                 +--> [UI copy/name cleanup]
                 +--> [run list visual prototype]
```

## 9. Review Checklist

- Workspace-level `cwd` and checkout selection are separate from run-level request snapshots.
- A workspace view can have zero runs, one active run, or multiple historical runs.
- Existing single-view behavior remains representable.
- Active run selection is per workspace view.
- The model allows future `ownerWindowId` without changing backend run routing.
