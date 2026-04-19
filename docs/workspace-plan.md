# Workspace 도입 계획

## 1. 목표

ACP Agent Workbench에 **workspace** 개념을 도입한다.

workspace는 하나의 GitHub repository origin에 대응하는 상위 작업 컨텍스트다. 사용자는 workspace를 선택한 뒤, 그 아래에서 여러 작업을 동시에 실행할 수 있다.

지원해야 하는 사용 시나리오:

- 하나의 GitHub repo를 workspace로 등록한다.
- 같은 workspace 아래에서 여러 작업을 서로 다른 디렉토리에서 실행한다.
  - 예: `frontend/` 작업과 `src-tauri/` 작업을 동시에 진행
- 같은 workspace 아래에서 같은 디렉토리를 대상으로 여러 작업을 병렬 실행한다.
  - 예: 동일 repo root에서 `codex`는 구현, `claude-code`는 테스트 보강
- 작업별 실행 이력, 권한 요청, follow-up queue, agent 설정은 분리된다.
- workspace 단위로 repo origin, 로컬 checkout/worktree, 최근 작업 목록, 기본 agent 설정을 관리한다.

## 2. 용어 정리

현재 코드와 README에서는 실행 디렉토리 입력을 `Workspace`라고 부르고 있지만, 실제 의미는 `AgentRunRequest.cwd`다. 새 모델에서는 이 용어를 분리한다.

| 용어 | 의미 | 예 |
| --- | --- | --- |
| Workspace | 하나의 GitHub repo origin에 대응하는 장기 컨텍스트 | `git@github.com:org/project.git` |
| Repository origin | workspace를 식별하는 원격 저장소 URL. canonical form으로 정규화 | `https://github.com/org/project.git` |
| Checkout | workspace의 로컬 clone 또는 worktree root | `/Users/me/work/project` |
| Work directory | 특정 작업/run이 실행되는 디렉토리. checkout 내부 경로 | `/Users/me/work/project/src-tauri` |
| Task | 사용자가 해결하려는 작업 단위. 탭 UI와 1:1로 매핑 가능 | "권한 브로커 테스트 추가" |
| Run | agent 프로세스의 한 실행 세션. task 아래에 여러 번 생길 수 있음 | `runId` |

결론:

- UI의 현재 `Workspace` 입력은 `Working directory` 또는 `Run directory`로 이름을 바꾼다.
- 새 `Workspace`는 repo origin을 기준으로 하는 별도 엔티티로 둔다.
- `TabState`는 task/run UI 상태를 계속 소유하되, `workspaceId`와 `workdir`을 참조한다.

## 3. 현재 구조 요약

### 3.1 백엔드

Rust/Tauri 백엔드는 hexagonal 구조다.

| 영역 | 현재 상태 |
| --- | --- |
| `domain/run.rs` | `AgentRunRequest`가 `cwd: Option<String>`을 포함 |
| `adapters/acp/runner.rs` | `cwd`를 workspace path처럼 정규화하고 ACP session `cwd`로 전달 |
| `adapters/acp/client.rs` | ACP tool 요청의 path 접근을 runner workspace 내부로 제한 |
| `adapters/session_registry.rs` | `runId` 기준으로 동시 run을 관리 |
| `ports/session_registry.rs` | run 생명주기만 다루며 repo/workspace 개념 없음 |

### 3.2 프런트엔드

| 영역 | 현재 상태 |
| --- | --- |
| `features/agent-run/model.ts` | `TabState`가 `cwd`, `goal`, agent 설정, run 상태를 모두 보유 |
| `features/agent-run/useAgentRun.ts` | 탭의 `cwd`를 `AgentRunRequest.cwd`로 전달 |
| `widgets/workbench-tabs/TabBar.tsx` | task/run을 탭으로 표현 |
| `widgets/run-panel/RunPanel.tsx` | `cwd` 입력을 실행 설정으로 노출 |

현재 구현은 이미 "여러 탭, 여러 run"을 감당하는 방향으로 정리되어 있으므로, workspace 도입은 **탭 위의 repo 컨텍스트 추가**로 접근한다.

## 4. 핵심 설계 이슈

| # | 이슈 | 해결 방향 |
| --- | --- | --- |
| A | 기존 `Workspace` UI 용어와 새 workspace 개념 충돌 | 기존 입력은 `Working directory`로 변경. 새 workspace는 origin 기반 엔티티로 분리 |
| B | 같은 repo를 URL 형식만 다르게 등록할 수 있음 | origin canonicalization 도입. `git@github.com:org/repo.git`, `https://github.com/org/repo`를 같은 키로 정규화 |
| C | workdir이 workspace checkout 밖을 가리킬 수 있음 | 백엔드에서 `workdir`이 선택한 checkout 내부인지 검증 |
| D | 같은 디렉토리 병렬 작업은 파일 충돌 가능 | 허용하되 UI에 "shared directory" 상태를 표시하고, 선택적으로 directory lease/경고 제공 |
| E | 서로 다른 디렉토리 작업은 checkout 하나로 충분할 수 있음 | 기본은 같은 checkout 내부의 하위 디렉토리 사용 |
| F | 강한 격리가 필요한 병렬 작업은 별도 worktree가 필요 | workspace 아래에 여러 checkout/worktree를 등록할 수 있게 확장 |
| G | run 이벤트 라우팅은 runId 기준으로 이미 가능 | task/tab에 `workspaceId`, `workdir`만 추가하고 기존 run 라우팅 유지 |
| H | workspace 목록/최근 상태를 앱 재시작 후 복원해야 함 | local app data에 workspace/task metadata 저장 |
| I | GitHub origin이 없는 로컬 디렉토리도 있을 수 있음 | 1차 범위에서는 "GitHub origin 필수". 후속으로 local-only workspace 고려 |

## 5. 제안 도메인 모델

### 5.1 Workspace

```rust
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub origin: GitOrigin,
    pub default_checkout_id: Option<CheckoutId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct GitOrigin {
    pub raw_url: String,
    pub canonical_url: String,
    pub host: String,
    pub owner: String,
    pub repo: String,
}
```

`Workspace.id`는 canonical origin 기반으로 안정적으로 생성한다.

예:

```text
workspaceId = sha256("github.com/org/repo").prefix(16)
```

이 방식이면 같은 origin을 다른 로컬 경로에서 열어도 같은 workspace로 인식할 수 있다.

### 5.2 Checkout

```rust
pub struct WorkspaceCheckout {
    pub id: CheckoutId,
    pub workspace_id: WorkspaceId,
    pub path: PathBuf,
    pub kind: CheckoutKind,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub is_default: bool,
}

pub enum CheckoutKind {
    Clone,
    Worktree,
}
```

초기 구현은 사용자가 이미 clone한 디렉토리를 등록하는 방식으로 충분하다. 자동 clone/worktree 생성은 후속 단계로 둔다.

### 5.3 Task

프런트의 `TabState`를 task UI 상태로 계속 사용하되, workspace 참조를 추가한다.

```ts
export type TabState = {
  id: string;
  title: string;
  workspaceId: string | null;
  checkoutId: string | null;
  workdir: string;

  selectedAgentId: string;
  goal: string;
  customCommand: string;
  stdioBufferLimitMb: number;
  autoAllow: boolean;
  idleTimeoutSec: number;

  activeRunId: string | null;
  sessionActive: boolean;
  awaitingResponse: boolean;
  followUpQueue: FollowUpQueueItem[];
  items: TimelineItem[];
};
```

기존 `cwd`는 `workdir`로 이름을 바꾸거나, 호환성을 위해 API 경계에서는 `cwd`로만 변환한다.

### 5.4 Run

`AgentRunRequest`에 workspace 식별자를 추가한다.

```rust
pub struct AgentRunRequest {
    pub goal: String,
    pub agent_id: String,
    pub workspace_id: Option<String>,
    pub checkout_id: Option<String>,
    pub cwd: Option<String>,
    pub agent_command: Option<String>,
    pub stdio_buffer_limit_mb: Option<usize>,
    pub auto_allow: Option<bool>,
    pub run_id: Option<String>,
}
```

호환성을 위해 `workspace_id`가 없으면 기존처럼 `cwd`만으로 실행할 수 있게 둔다. 다만 새 UI 경로에서는 항상 workspace를 선택해 전달한다.

## 6. 실행 디렉토리 정책

workspace 아래 작업은 세 가지 모드로 나눈다.

| 모드 | 설명 | 기본 정책 |
| --- | --- | --- |
| Same checkout, different directories | 같은 checkout 안의 서로 다른 하위 디렉토리에서 작업 | 기본 허용 |
| Same checkout, same directory | 같은 디렉토리에서 병렬 작업 | 허용 + 충돌 경고 |
| Separate worktrees | 같은 origin의 별도 worktree에서 작업 | 후속 구현. 충돌 위험 낮음 |

### 6.1 path 검증

백엔드는 run 시작 전에 다음을 검증한다.

1. `workspaceId`가 존재한다.
2. `checkoutId`가 workspace에 속한다.
3. `cwd`를 canonicalize한 경로가 checkout root 내부다.
4. 경로가 존재하지 않으면 생성 여부를 정책으로 결정한다.
   - 1차: 존재하지 않으면 에러
   - 후속: checkout 내부 하위 경로는 생성 허용 옵션 제공

### 6.2 같은 디렉토리 병렬 실행

같은 디렉토리 병렬 작업은 실제로 유용하지만, 두 agent가 같은 파일을 동시에 수정할 수 있다. 1차 구현에서는 강제 차단하지 않는다.

대신 프런트에서 다음 정보를 보여준다.

- 같은 workspace/workdir에서 실행 중인 task 개수
- 실행 중 agent 목록
- "shared directory" badge
- 선택적 확인: 이미 실행 중인 작업이 있는 디렉토리에서 run 시작 시 confirm

후속 단계에서 `DirectoryLease`를 추가할 수 있다.

```rust
pub struct DirectoryLease {
    pub workspace_id: WorkspaceId,
    pub checkout_id: CheckoutId,
    pub workdir: PathBuf,
    pub run_id: String,
    pub mode: LeaseMode,
}

pub enum LeaseMode {
    Shared,
    Exclusive,
}
```

초기값은 `Shared`다. `Exclusive`는 위험한 작업, release 작업, 대규모 refactor에서 선택할 수 있게 한다.

## 7. 백엔드 변경 계획

### 7.1 새 도메인/포트

추가 파일:

- `src-tauri/src/domain/workspace.rs`
- `src-tauri/src/ports/workspace_store.rs`
- `src-tauri/src/application/list_workspaces.rs`
- `src-tauri/src/application/register_workspace.rs`
- `src-tauri/src/application/resolve_workdir.rs`

`WorkspaceStore` 포트:

```rust
pub trait WorkspaceStore: Clone + Send + Sync + 'static {
    fn list_workspaces(&self) -> impl Future<Output = Result<Vec<Workspace>>> + Send;
    fn get_workspace(&self, id: &str) -> impl Future<Output = Result<Option<Workspace>>> + Send;
    fn upsert_workspace(&self, workspace: Workspace) -> impl Future<Output = Result<()>> + Send;
    fn list_checkouts(&self, workspace_id: &str) -> impl Future<Output = Result<Vec<WorkspaceCheckout>>> + Send;
    fn upsert_checkout(&self, checkout: WorkspaceCheckout) -> impl Future<Output = Result<()>> + Send;
}
```

1차 adapter는 JSON 파일 저장으로 충분하다.

예상 저장 위치:

```text
~/Library/Application Support/acp-agent-workbench/workspaces.json
```

플랫폼별 app data path는 Tauri path API 또는 Rust crate를 통해 얻는다.

### 7.2 Git origin resolver

추가 adapter:

- `src-tauri/src/adapters/git.rs`

책임:

- 선택한 디렉토리에서 `git rev-parse --show-toplevel` 실행
- `git remote get-url origin` 실행
- GitHub URL canonicalization
- 현재 branch/head sha 조회

origin 정규화 예:

| 입력 | canonical |
| --- | --- |
| `git@github.com:org/repo.git` | `github.com/org/repo` |
| `https://github.com/org/repo.git` | `github.com/org/repo` |
| `https://github.com/org/repo` | `github.com/org/repo` |

GitHub 외 host는 1차에서는 에러 처리하거나 `host/owner/repo` 구조가 확인되는 경우만 허용한다. 요구사항이 "github repo origin"이므로 GitHub만 먼저 지원하는 편이 명확하다.

### 7.3 Run 시작 시 workspace 검증

`StartAgentRunUseCase` 앞단에서 `AgentRunRequest`의 `workspace_id`, `checkout_id`, `cwd`를 검증하고 실제 실행 path를 확정한다.

선택지는 두 가지다.

1. `StartAgentRunUseCase` 안에 `WorkdirResolver` 포트를 주입
2. Tauri command에서 먼저 `ResolveWorkdirUseCase`를 호출한 뒤 request의 `cwd`를 확정

권장: 1번. 실행 path 검증은 run 시작의 도메인 규칙에 가깝고, Tauri command를 얇게 유지할 수 있다.

```rust
pub trait WorkdirResolver: Clone + Send + Sync + 'static {
    fn resolve(
        &self,
        workspace_id: Option<&str>,
        checkout_id: Option<&str>,
        cwd: Option<&str>,
    ) -> impl Future<Output = Result<PathBuf>> + Send;
}
```

호환성:

- `workspace_id == None`이면 기존 `cwd` 직접 실행 경로를 유지
- `workspace_id != None`이면 workspace checkout 내부 검증을 강제

### 7.4 Tauri command

추가 command:

| Command | 역할 |
| --- | --- |
| `list_workspaces` | 등록된 workspace 목록 반환 |
| `register_workspace_from_path(path)` | 로컬 repo path를 검사해 workspace/checkouts 등록 |
| `remove_workspace(workspace_id)` | workspace metadata 제거. 파일 삭제는 하지 않음 |
| `list_workspace_checkouts(workspace_id)` | checkout/worktree 목록 반환 |
| `refresh_workspace_checkout(checkout_id)` | branch/head/origin 정보 갱신 |
| `resolve_workspace_workdir(workspace_id, checkout_id, relative_path)` | UI 검증/preview용 |

후속 command:

| Command | 역할 |
| --- | --- |
| `clone_workspace(origin, target_path)` | origin clone 후 등록 |
| `create_workspace_worktree(workspace_id, branch, path)` | 별도 worktree 생성 |
| `delete_workspace_worktree(checkout_id)` | worktree 제거. destructive라 별도 확인 필요 |

## 8. 프런트엔드 변경 계획

### 8.1 FSD 배치

추가/변경 영역:

| 경로 | 책임 |
| --- | --- |
| `src/entities/workspace/` | Workspace, Checkout 타입 |
| `src/features/workspace-select/` | workspace 선택/등록 UI |
| `src/features/workdir-select/` | checkout 및 하위 경로 선택 UI |
| `src/features/agent-run/model.ts` | `TabState`에 `workspaceId`, `checkoutId`, `workdir` 추가 |
| `src/widgets/workspace-sidebar/` | workspace 목록과 최근 task 표시 (선택) |
| `src/widgets/run-panel/RunPanel.tsx` | `cwd` 입력을 workspace-aware workdir 선택으로 변경 |

### 8.2 UI 구조

1차 UI는 현재 화면 구조를 크게 바꾸지 않는다.

```text
AgentWorkbenchPage
├── WorkspaceBar
│   ├── workspace selector
│   ├── checkout selector
│   └── workdir selector
├── TabBar
└── ActiveTabPanel
    ├── GoalEditor
    ├── RunPanel
    ├── FollowUpComposer
    ├── FollowUpQueue
    └── EventStream
```

`TabBar`는 task list로 유지한다. 같은 workspace 안의 task들이 탭으로 나열되는 형태가 현재 모델과 가장 잘 맞는다.

후속으로 여러 workspace를 동시에 열어야 하면 다음 구조로 확장한다.

```text
WorkspaceSwitcher
└── WorkspaceSession
    ├── tabs
    └── activeTabId
```

즉, 전역 store를 `workspaces -> tabs` 계층으로 확장할 수 있다.

### 8.3 Store 모델

1차는 전역 workspace 목록과 탭별 workspace 참조를 추가한다.

```ts
type WorkbenchState = {
  workspaces: Workspace[];
  checkoutsByWorkspaceId: Record<string, WorkspaceCheckout[]>;
  tabs: TabState[];
  activeTabId: string;

  loadWorkspaces: () => Promise<void>;
  registerWorkspaceFromPath: (path: string) => Promise<string>;
  setTabWorkspace: (tabId: string, workspaceId: string, checkoutId?: string) => void;
  setTabWorkdir: (tabId: string, workdir: string) => void;
};
```

기존 run 이벤트 dispatch는 유지한다.

```ts
dispatchRunEvent(runId, event)
```

workspace는 이벤트 라우팅 키가 아니다. 이벤트는 계속 runId로 라우팅하고, runId가 속한 탭이 workspace를 참조한다.

### 8.4 기존 `cwd`와의 호환

`useAgentRun`에서 request 생성 시:

```ts
const request: AgentRunRequest = {
  runId,
  goal: trimmedGoal,
  agentId: current.selectedAgentId,
  workspaceId: current.workspaceId ?? undefined,
  checkoutId: current.checkoutId ?? undefined,
  cwd: current.workdir.trim() || undefined,
  agentCommand: current.customCommand.trim() || undefined,
  stdioBufferLimitMb: clampBuffer(current.stdioBufferLimitMb),
  autoAllow: current.autoAllow,
};
```

레거시 탭은 `workspaceId == null`이고 `cwd`만 전달한다. 새 탭은 workspace 선택 후 `workdir`을 전달한다.

## 9. 데이터 저장

초기 저장은 로컬 JSON 파일로 한다. DB 도입은 검색/히스토리 요구가 커진 뒤 판단한다.

```json
{
  "schemaVersion": 1,
  "workspaces": [
    {
      "id": "ws_9d35f0a1f0d9c6a2",
      "name": "acp-agent-workbench",
      "origin": {
        "rawUrl": "git@github.com:org/acp-agent-workbench.git",
        "canonicalUrl": "github.com/org/acp-agent-workbench",
        "host": "github.com",
        "owner": "org",
        "repo": "acp-agent-workbench"
      },
      "defaultCheckoutId": "co_1",
      "createdAt": "2026-04-19T00:00:00Z",
      "updatedAt": "2026-04-19T00:00:00Z"
    }
  ],
  "checkouts": [
    {
      "id": "co_1",
      "workspaceId": "ws_9d35f0a1f0d9c6a2",
      "path": "/Users/yoophi/project/acp-agent-workbench",
      "kind": "clone",
      "branch": "main",
      "headSha": "abc123",
      "isDefault": true
    }
  ]
}
```

주의:

- 작업 이력 전체와 이벤트 스트림은 별도 저장 범위로 둔다.
- 1차 workspace metadata에는 민감한 토큰을 저장하지 않는다.
- origin URL은 private repo 이름을 포함할 수 있으므로 외부 전송하지 않는다.

## 10. 단계별 구현 계획

### Phase 1. 용어 정리와 호환 필드 추가

- `RunPanel`의 `Workspace` 라벨을 `Working directory`로 변경
- `TabState.cwd`를 내부적으로 유지하되 문서와 UI에서는 `workdir`로 표현
- `AgentRunRequest`에 optional `workspaceId`, `checkoutId` 추가
- 기존 `cwd` 단독 실행이 계속 동작하는지 테스트

완료 기준:

- 기존 단일/다중 탭 run이 회귀 없이 동작
- UI에서 workspace 용어가 repo 컨텍스트와 충돌하지 않음

### Phase 2. Workspace metadata 저장소 도입

- `domain/workspace.rs` 추가
- `WorkspaceStore` 포트 추가
- JSON 파일 기반 `LocalWorkspaceStore` adapter 구현
- `list_workspaces`, `register_workspace_from_path`, `list_workspace_checkouts` command 추가
- git origin resolver 구현

완료 기준:

- 로컬 repo path를 등록하면 origin 기준 workspace가 생성됨
- 같은 origin의 다른 path를 등록하면 같은 workspace 아래 checkout으로 추가됨
- 앱 재시작 후 workspace 목록이 복원됨

### Phase 3. 탭과 workspace 연결

- `TabState`에 `workspaceId`, `checkoutId`, `workdir` 추가
- workspace selector와 checkout selector 추가
- 새 탭 생성 시 현재 workspace/checkouts를 preset으로 복사
- run 시작 시 workspace/checkouts/workdir 정보를 request에 포함

완료 기준:

- 같은 workspace 아래 여러 탭을 만들 수 있음
- 탭별로 서로 다른 workdir을 선택해 병렬 실행 가능
- 탭을 전환해도 workspace/workdir 선택이 유지됨

### Phase 4. Workdir 검증 강제

- `WorkdirResolver` 포트 추가
- workspace 기반 run은 checkout 내부 경로만 허용
- checkout 밖 path, 존재하지 않는 path, origin 불일치를 명확한 에러로 surface
- ACP client의 기존 path sandbox와 resolver 정책을 맞춤

완료 기준:

- workspace run은 checkout 밖에서 실행되지 않음
- 에러 메시지가 UI에서 어떤 workspace/checkouts/path 문제인지 설명함
- 기존 `cwd` 단독 legacy run은 유지됨

### Phase 5. 병렬 작업 상태 표시

- 같은 workspace/workdir에서 실행 중인 task 목록 계산
- `TabBar` 또는 `WorkspaceBar`에 shared directory badge 표시
- 같은 directory에서 새 run 시작 시 경고 또는 confirm 제공
- workspace별 active run count, permission pending count 표시

완료 기준:

- 사용자가 같은 디렉토리 병렬 작업 상태를 명확히 볼 수 있음
- 서로 다른 디렉토리 병렬 작업은 불필요한 경고 없이 실행됨

### Phase 6. Worktree 지원

- `create_workspace_worktree` command 추가
- worktree checkout 등록
- task 생성 시 "same checkout" / "new worktree" 선택 제공
- worktree별 branch/head 상태 표시

완료 기준:

- 같은 origin 아래 여러 worktree를 만들고 task별로 배정 가능
- 병렬 agent 작업을 파일 충돌 없이 분리할 수 있음

## 11. 테스트 전략

### Rust

- origin canonicalization
  - SSH/HTTPS/.git suffix normalization
  - GitHub owner/repo parsing
- workspace store
  - upsert workspace
  - same origin duplicate 방지
  - checkout 추가/갱신
- workdir resolver
  - checkout 내부 path 허용
  - checkout 밖 path 거부
  - symlink/canonicalize escape 거부
- start run integration
  - workspace request가 resolved cwd로 launcher에 전달되는지
  - legacy cwd request가 기존처럼 동작하는지

### Frontend

- workspace selector
  - 목록 로딩
  - 등록 성공/실패
  - 탭별 선택 유지
- run request 생성
  - workspaceId/checkoutId/workdir 포함
  - legacy 탭 호환
- 병렬 상태 표시
  - 같은 workdir active run badge
  - 다른 workdir은 badge 미표시
- 이벤트 라우팅
  - 기존처럼 runId 기준으로 올바른 탭에 append

## 12. 위험과 대응

| 위험 | 영향 | 대응 |
| --- | --- | --- |
| origin 정규화 오류 | 같은 repo가 여러 workspace로 등록됨 | canonicalization 단위 테스트를 먼저 작성 |
| path 검증 미흡 | agent가 checkout 밖 파일에 접근 | `canonicalize` + prefix 검증을 백엔드에서 강제 |
| 같은 디렉토리 병렬 수정 충돌 | 사용자 작업 손실 가능 | 1차 경고, 후속 exclusive lease/worktree 권장 |
| metadata 저장 파일 손상 | workspace 목록 유실 | schemaVersion, atomic write, parse 실패 시 백업 파일 유지 |
| UI 상태 계층 과도화 | 탭/store 복잡도 증가 | 1차는 `tabs + workspace refs`로 제한, `workspaces -> tabs` 계층화는 후속 |
| 자동 clone/worktree가 OS별로 불안정 | 구현/권한 복잡도 증가 | 1차는 existing local repo 등록만 지원 |

## 13. 범위 밖

1차 구현에서 제외한다.

- GitHub API 연동
- PR/issue 목록 연동
- remote branch 자동 생성
- 작업 결과 자동 commit/push
- agent별 파일 lock 강제
- workspace/task 이벤트 히스토리 영구 저장
- local-only non-git workspace

## 14. 권장 구현 순서 요약

1. 기존 UI의 `Workspace` 용어를 `Working directory`로 정리한다.
2. `Workspace`, `Checkout`, origin canonicalization 도메인 타입을 추가한다.
3. 로컬 repo path 등록 command와 JSON metadata store를 구현한다.
4. `TabState`에 `workspaceId`, `checkoutId`, `workdir`를 추가한다.
5. run 시작 전에 workspace 기반 path 검증을 강제한다.
6. 같은 workspace/workdir 병렬 실행 상태를 UI에 표시한다.
7. 필요해진 시점에 worktree 생성/관리 기능을 추가한다.
