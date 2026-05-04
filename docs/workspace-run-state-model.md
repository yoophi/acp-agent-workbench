# Workspace-scoped Run State Model

> 용어 주석: 이 문서는 기존 구현의 `Workspace` 명칭을 기준으로 작성되었다. 새 taxonomy에서는 repo-origin 기반 `Workspace`가 `GitHubRepository`에 대응한다. 향후 umbrella term은 `WorkContext`를 사용한다.

## 1. 목적

Issue #13은 workbench를 flat agent-run tab 구조에서 workspace-scoped work area 구조로 옮기는 작업이다. 이 문서는 구현 전에 frontend state model을 정의해 변경을 작은 PR로 나눌 수 있게 한다.

현재 앱은 이미 persistent `Workspace`, `WorkspaceCheckout` 엔티티와 `TabState.workspaceId`, `checkoutId`, `cwd`를 갖고 있다. 남은 문제는 ownership이다. 현재 tab은 workspace selection, run configuration, live run state, timeline data, UI-only presentation state를 한 구조 안에 섞어 갖고 있다.

## 2. 현재 `TabState` 분류

| 현재 field | 제안 owner | 비고 |
| --- | --- | --- |
| `id` | Workspace view | 현재는 tab id다. #13 이후에는 run id가 아니라 workspace view id가 된다. |
| `title` | Workspace view | 사용자에게 보이는 workspace view label. 기본값은 workspace name 또는 checkout path에서 만들 수 있다. |
| `workspaceId` | Workspace view | repo-origin workspace를 선택한다. 향후 `contextId`로 일반화 가능하다. |
| `checkoutId` | Workspace view | workspace 안의 active checkout/worktree를 선택한다. |
| `cwd` | Workspace view | workspace-level working directory baseline. run 시작 시 snapshot된다. |
| `selectedAgentId` | Run draft | 다음 run request의 일부다. |
| `goal` | Run draft | 다음 run request의 일부다. |
| `customCommand` | Run draft | 다음 run request의 일부다. |
| `stdioBufferLimitMb` | Run draft | 다음 run request의 일부다. |
| `autoAllow` | Run draft | 다음 run request의 일부다. |
| `idleTimeoutSec` | Run draft 또는 workspace default | 우선 draft에 둔다. 필요하면 나중에 workspace default로 승격한다. |
| `activeRunId` | Workspace view selection | workspace view 안에서 선택된 live/recent run을 가리킨다. |
| `sessionActive` | Agent run | 선택된 run에서 파생한다. |
| `awaitingResponse` | Agent run | 선택된 run에서 파생한다. |
| `idleRemainingSec` | Agent run | 특정 run의 runtime countdown이다. |
| `followUpDraft` | Workspace view UI | composer의 draft text다. historical run 소유가 아니다. |
| `followUpQueue` | Agent run | queue entry는 구체적인 `runId`를 target한다. |
| `items` | Agent run | timeline은 run-owned다. |
| `filter` | Workspace view UI | event filter는 view preference다. |
| `error` | Agent run 또는 workspace view | start/cancel error는 view-owned 가능. runtime error는 run-owned로 둔다. |
| `unreadCount` | Workspace view UI | 비활성 workspace view의 event count다. |
| `permissionPending` | Agent run | permission event에서 파생되는 runtime state다. |
| `closing` | Workspace view UI | close lifecycle은 cancellation 시작 후 run과 독립적이다. |

## 3. 제안 Store Shape

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

핵심 원칙:

- `cwd`는 workspace view가 소유하고, run 시작 시 `AgentRunState`에 복사한다. 이후 workspace view directory를 바꿔도 historical run은 바꾸지 않는다.
- prompt/agent setting draft는 시작 전까지 workspace view에 둔다. run은 replay, diagnostics, persistence를 위해 immutable request snapshot을 저장한다.
- timeline, permission, follow-up queue는 run-owned다.
- active run 선택은 workspace view마다 독립적이다.

## 4. Selector와 Action

tab-centric selector를 workspace-view selector로 교체한 뒤 call site를 migration한다.

```ts
selectActiveWorkspaceView(state): WorkspaceViewState
selectWorkspaceView(state, workspaceViewId): WorkspaceViewState | undefined
selectActiveRun(state, workspaceViewId): AgentRunState | undefined
selectRun(state, runId): AgentRunState | undefined
selectWorkspaceViewRuns(state, workspaceViewId): AgentRunState[]
```

action은 다음 형태를 기준으로 한다.

```ts
addWorkspaceView(preset?: Partial<WorkspaceViewState>): string
closeWorkspaceView(viewId: string): string | null
forceCloseWorkspaceView(viewId: string): string | null
activateWorkspaceView(viewId: string): void
patchWorkspaceView(viewId: string, patch: Partial<WorkspaceViewState>): void
beginRun(viewId: string, runId: string): void
endRun(runId: string): void
dispatchRunEvent(runId: string, event: RunEvent): void
selectRunForWorkspaceView(viewId: string, runId: string): void
```

기존 widget은 plain props를 계속 받는다. `useAgentRun(viewId)`는 `WorkspaceViewState`, 선택된 `AgentRunState`, action을 현재 widget-facing shape로 합치는 compatibility layer가 된다.

## 5. Migration Strategy

첫 구현 PR은 persistence 변경 없이 memory state만 migration할 수 있다.

1. store 내부 language를 tab 중심에서 workspace view 중심으로 변경한다. component 이름은 임시로 유지해도 된다.
2. 현재 `TabState` 각각을 `WorkspaceViewState` 하나로 변환한다.
3. `activeRunId`가 있으면 해당 tab runtime fields에서 `AgentRunState`를 만든다.
4. active run이 없으면 workspace view와 draft만 유지한다.
5. 기존 last-view behavior를 유지한다. workspace view는 항상 하나 이상 존재해야 한다.

field mapping:

```text
TabState.id             -> WorkspaceViewState.id
TabState.goal           -> WorkspaceViewState.draft.goal
TabState.cwd            -> WorkspaceViewState.cwd
TabState.items          -> AgentRunState.items
TabState.followUpQueue  -> AgentRunState.followUpQueue
TabState.error          -> WorkspaceViewState.viewError or AgentRunState.runError
```

## 6. Backend 영향

backend는 이미 `AgentRunRequest`에서 `workspace_id`, `checkout_id`, `cwd`를 받고, workspace workdir을 검증하고, workspace/checkouts를 SQLite WAL mode로 저장하며, event를 `runId`로 route한다. 따라서 #13은 주로 frontend state/UI restructuring 작업이다.

향후 backend 작업은 가능하지만 #13 범위 밖에 둔다.

- run history persistence 추가
- `WorkspaceViewState` persistence 추가
- #8을 위한 workspace-view 또는 window ownership 추가

## 7. Multi-window 준비

이 모델은 workspace의 전역 owner 하나를 가정하지 않는다. #8에서는 run ownership을 바꾸지 않고 owner field만 추가할 수 있다.

```ts
type WorkspaceViewState = {
  id: string;
  ownerWindowId?: string;
  ...
};
```

`AgentRunState.workspaceViewId`는 logical owner로 유지된다. view가 window 사이를 이동해도 run은 계속 `runId`로 route된다.

## 8. PR Split

```text
[PR A] Store shape migration
   - WorkspaceViewState / AgentRunState 도입
   - 기존 TabState compatibility 유지

[PR B] Event and run action migration
   - dispatchRunEvent를 runsById로 이동
   - begin/end run action 정리

[PR C] Workspace-scoped UI language
   - TabBar copy와 selector 이름 정리
   - workspace view 안에서 active run 선택 노출

[PR D] Historical run list within a workspace
   - 선택된 workspace view의 current/recent runs 표시
```

PR A 이후에는 다음 작업을 병렬화할 수 있다.

```text
                  +--> [PR B]
[PR A complete] -+
                  +--> [PR C]
                  +--> [PR D prototype]
```

## 9. Acceptance Criteria

- 기존 single-view behavior가 그대로 표현된다.
- workspace-level `cwd`와 checkout selection은 run-level request snapshot과 분리된다.
- workspace view는 run이 없거나, active run이 하나 있거나, 여러 historical run을 가질 수 있다.
- timeline과 follow-up queue는 run-owned다.
- active run selection은 workspace view별이다.
- 이 모델은 backend run routing을 바꾸지 않고도 향후 `ownerWindowId`를 허용한다.
