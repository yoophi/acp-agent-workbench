# 문서 탭 기반 다중 ACP 에이전트 실행 설계

## 1. 목표

현재 ACP Agent Workbench는 **단일 run 전용 작업 공간**이다 — 한 번에 한 에이전트만 실행할 수 있고 화면 전체가 그 세션 하나에 고정된다. 본 문서는 브라우저 탭과 비슷한 **"도큐먼트 탭"** UI를 도입해, 서로 다른 에이전트/목표를 여러 탭에 열어두고 각 탭이 독립된 ACP run을 동시에 실행·관리할 수 있도록 확장하는 방안을 정리한다.

사용 시나리오:
- 탭 1 — `claude-code` 가 백엔드 리팩토링 작업 중
- 탭 2 — `codex` 가 동일 레포의 프런트 스모크 테스트 작성 중
- 탭 3 — 유휴, 새 goal 작성 중 (아직 run 시작 안 함)

탭 전환만으로 각 run의 타임라인, follow-up 큐, 설정이 그대로 유지되어야 한다.

## 2. 현재 구조 요약

### 2.1 Rust 백엔드 — 이미 run_id 기반 구조지만 동시성 차단

| 파일 | 역할 | 현재 제약 |
| --- | --- | --- |
| `src-tauri/src/adapters/tauri/session_state.rs:23` | `reserve_run_if_idle` | 다른 run이 있으면 `"another agent run is already in progress"` 로 거절 → **다중 실행 막힘** |
| `src-tauri/src/adapters/tauri/session_state.rs:9` | `runs: HashMap<String, RunSlot>` | 자료구조 자체는 다중 run을 보관 가능 |
| `src-tauri/src/adapters/tauri/session_state.rs:64` (permission broker) | `clear_run(run_id)` / `respond(permission_id)` | 이미 run 단위로 분리됨 |
| `src-tauri/src/adapters/tauri/commands.rs:47` | `start_agent_run` | `reserve_run_if_idle`에 의존 |
| `src-tauri/src/adapters/tauri/event_sink.rs` | Tauri event 이름 `"agent-run-event"` 단일 채널, payload에 `runId` 포함 | 모든 run이 한 채널로 브로드캐스트 → 프런트 라우팅 필요 |

### 2.2 프런트엔드 — 단일 전역 스토어 전제

| 파일 | 역할 | 현재 제약 |
| --- | --- | --- |
| `src/features/agent-run/model.ts` | Zustand `useAgentRunStore` | 단일 run의 goal/cwd/items/queue/idle 타이머 등이 flat 하게 존재 |
| `src/features/agent-run/useAgentRun.ts` | 이벤트 리스너 + run/send/cancel 훅 | `envelope.runId`를 검사하지 않고 바로 `appendItem` 수행 → 다중 run일 경우 이벤트가 뒤섞임 |
| `src/pages/agent-workbench/index.tsx` | 좌측 컬럼(Goal/RunPanel/Composer/Queue) + 우측 EventStream | 고정 레이아웃, 탭 개념 없음 |
| `src/widgets/*` | 모두 props-driven | 그대로 재사용 가능 |

### 2.3 현재 흐름 (Mermaid)

```mermaid
flowchart LR
    subgraph Frontend
      Page[AgentWorkbenchPage] --> Store[useAgentRunStore (global)]
    end
    subgraph Backend
      Cmds[Tauri commands] --> State[AppState (single active run)]
      State --> Runner[AcpAgentRunner]
    end
    Page -->|invoke| Cmds
    Runner -->|emit agent-run-event| Page
```

## 3. 핵심 설계 이슈

| # | 이슈 | 해결 방향 |
| --- | --- | --- |
| A | `reserve_run_if_idle`이 동시 실행을 차단 | 단순 `reserve_run(run_id)`로 완화. 필요 시 `MAX_CONCURRENT_RUNS` 상한만 부여 |
| B | 단일 스토어에 모든 run 상태가 flat 하게 섞여 있음 | 탭 단위 `TabState` 서브구조로 분리, 탭 컬렉션을 소유하는 `WorkbenchStore`로 재설계 |
| C | `listenRunEvents`가 모든 이벤트를 무조건 현재 상태에 append | `envelope.runId → tabId` 매핑으로 dispatch |
| D | 탭 id vs run id 관계 | 탭 id = UUID (탭 생성 시점). run id = `start_agent_run` 시 발급. `tab.activeRunId: string | null` 로 매핑 |
| E | 탭 닫을 때 실행 중인 run 처리 | 기본: `cancelAgentRun(activeRunId)` 후 탭 제거. UX로 확인 다이얼로그 옵션 |
| F | 설정(agent/cwd/command/idle 등)은 탭별로 유지되어야 함 | `TabState` 내부 필드. 새 탭 생성 시 기본값 또는 "현재 탭 복제" 프리셋 |
| G | 에이전트 실행 중 이벤트가 포커스되지 않은 탭에 계속 쌓임 | 탭 배지(신규 이벤트 카운터 / 상태 dot) + 메모리 상한(선택) |
| H | Permission 이벤트가 여러 탭에서 동시에 대기할 수 있음 | 이미 `run_id` 스코프라 논리적으론 안전. UI에서 탭별 pending 표시 |
| I | 단일 이벤트 리스너가 포커스 전환 시 사라지지 않도록 보장 | 리스너는 앱 수명 동안 1회만 설치, 탭 컬렉션을 참조해 라우팅 |

## 4. 제안 아키텍처

### 4.1 Rust: AppState 다중 run 허용

`src-tauri/src/adapters/tauri/session_state.rs` 변경:

```rust
pub async fn reserve_run(&self, run_id: String) -> Result<()> {
    let mut runs = self.runs.lock().await;
    if runs.contains_key(&run_id) {
        return Err(anyhow!("duplicate run id: {run_id}"));
    }
    if let Some(limit) = max_concurrent_runs() {
        if runs.len() >= limit {
            return Err(anyhow!("concurrent run limit ({limit}) reached"));
        }
    }
    runs.insert(run_id, RunSlot::Reserved);
    Ok(())
}
```

- `reserve_run_if_idle`은 제거하거나 legacy wrapper 유지
- `max_concurrent_runs()`는 환경 변수(`ACP_WORKBENCH_MAX_RUNS`)에서 읽어 옵션으로 상한 설정
- `finish_run`, `cancel_run`, `attach_session`, `active_session`은 이미 run_id 키라 수정 불필요

`src-tauri/src/adapters/tauri/commands.rs`의 `start_agent_run`은 단일 호출 줄만 바꾸면 됨.

단위 테스트 보강:
- 두 개의 run_id를 reserve → 각각 attach_session → 서로 간섭 없는지
- 하나를 cancel해도 다른 run의 permission 대기가 유지되는지 (이미 유사 테스트 존재)

### 4.2 프런트: `TabState` + `WorkbenchStore`

`src/features/agent-run/model.ts` 재설계:

```ts
export type TabState = {
  id: string;                       // 탭 UUID
  title: string;                    // 예: "claude-code #1"
  // --- 실행 설정 (탭별) ---
  selectedAgentId: string;
  goal: string;
  cwd: string;
  customCommand: string;
  stdioBufferLimitMb: number;
  autoAllow: boolean;
  idleTimeoutSec: number;
  // --- 런타임 상태 ---
  activeRunId: string | null;
  sessionActive: boolean;
  awaitingResponse: boolean;
  idleRemainingSec: number | null;
  followUpDraft: string;
  followUpQueue: FollowUpQueueItem[];
  items: TimelineItem[];
  filter: EventGroup | "all";
  error: string | null;
  unreadCount: number;              // 비활성 탭 배지
};

type WorkbenchStore = {
  tabs: TabState[];
  activeTabId: string | null;
  addTab: (preset?: Partial<TabState>) => string;
  closeTab: (tabId: string) => Promise<void>;
  activateTab: (tabId: string) => void;
  renameTab: (tabId: string, title: string) => void;
  patchTab: (tabId: string, patch: Partial<TabState>) => void;
  // 이벤트 라우팅 전용
  dispatchRunEvent: (runId: string, event: RunEvent) => void;
};
```

기본 탭 하나가 항상 존재. 새 goal 없이 열린 탭은 `activeRunId === null`, `sessionActive === false` 상태로 대기.

### 4.3 프런트: `useAgentRun(tabId)` 훅

현재 `useAgentRun()`이 전역 store를 사용하는 것을 **활성 탭 기준 훅**으로 리팩토링:

```ts
export function useAgentRun(tabId: string) {
  const tab = useWorkbenchStore((s) => s.tabs.find((t) => t.id === tabId));
  const patch = (update: Partial<TabState>) =>
    useWorkbenchStore.getState().patchTab(tabId, update);

  const run = useCallback(async () => {
    if (!tab) return;
    patch({ sessionActive: true, awaitingResponse: true, items: [], followUpQueue: [] });
    const started = await startAgentRun({ ...toRequest(tab) });
    patch({ activeRunId: started.id });
  }, [tab]);
  // ... send, cancel, cancelFollowUp, idle effect 모두 탭별로
}
```

포커스된 탭에서만 훅이 호출되므로 idle interval/effect는 해당 탭 전환 시 자연스럽게 정리된다 (단, 비활성 탭에서도 백엔드로부터의 이벤트 수신은 별도 라우터가 담당).

### 4.4 이벤트 라우터

앱 부팅 시 1회 설치:

```ts
// src/app/eventRouter.ts (신설)
export function installRunEventRouter() {
  return listenRunEvents((envelope) => {
    const store = useWorkbenchStore.getState();
    store.dispatchRunEvent(envelope.runId, envelope.event);
  });
}
```

`dispatchRunEvent`는 탭 목록에서 `tab.activeRunId === runId`인 탭을 찾아 `items` append, 라이프사이클 전이, idle 리셋 등을 수행. 활성 탭이 아니면 `unreadCount++`.

### 4.5 탭 UI

```
WorkbenchPage
├── TabBar
│   ├── 각 탭: [agent icon] [title] [status dot] [unread badge] [×]
│   └── 끝: [+]
└── ActiveTabPanel (tabId 전달)
    ├── GoalEditor
    ├── RunPanel
    ├── FollowUpComposer
    ├── FollowUpQueue
    └── EventStream
```

기존 위젯은 props-only 상태라 건드릴 필요가 거의 없다. 탭 하나만 열린 시나리오를 기존 동작과 동치로 유지해 회귀를 최소화한다.

상태 dot 색상:
- 회색: idle, run 시작 전
- 초록: `sessionActive && !awaitingResponse`
- 깜빡이는 파랑: `awaitingResponse`
- 주황: idle countdown 진행 중
- 빨강: `error`

### 4.6 탭 닫기 처리

`closeTab(tabId)` 동작:
1. `tab.sessionActive === true`면 `cancelAgentRun(tab.activeRunId)`
2. 마지막 탭이면 빈 신규 탭을 한 개 만들어 유지 (빈 상태 방지)
3. 탭 제거 후 인접 탭으로 `activeTabId` 이동

선택: "실행 중 탭 닫기"에 확인 다이얼로그. 초기 구현은 즉시 cancel.

### 4.7 변경 후 흐름 (Mermaid)

```mermaid
flowchart LR
  subgraph Frontend
    Page[WorkbenchPage]
    Store[useWorkbenchStore<br/>tabs + activeTabId]
    Router[Event router (single)]
    Page --> Store
    Router --> Store
  end
  subgraph Backend
    Cmds[Tauri commands]
    State[AppState<br/>HashMap&lt;run_id, RunContext&gt;]
    Cmds --> State
  end
  Page -->|start_agent_run / send_prompt / cancel| Cmds
  State -->|agent-run-event (runId=A)| Router
  State -->|agent-run-event (runId=B)| Router
  Router -->|dispatchRunEvent| Store
```

### 4.8 동시성 경계

- Tauri event 스트림은 FIFO 보장 — runA/runB가 한 채널로 섞여 들어와도 각 envelope은 원자적
- 프런트 dispatch는 동기 처리라 race 없음
- 백엔드 permission broker는 `run_id` 분리 저장 → 두 탭이 동시에 approve 대기하면 각각 독립적으로 UI에 노출
- follow-up queue는 탭별 독립. drain도 탭별

## 5. 변경 파일 체크리스트

### Rust

- [ ] `src-tauri/src/adapters/tauri/session_state.rs` — `reserve_run_if_idle` 제거, `reserve_run` 추가, 선택적 concurrent 상한
- [ ] `src-tauri/src/adapters/tauri/commands.rs:49` — `reserve_run` 호출로 교체
- [ ] (선택) `src-tauri/src/main.rs` or config — `ACP_WORKBENCH_MAX_RUNS` 환경 변수 파싱
- [ ] 테스트: 2개 동시 run 시나리오 추가

### Frontend

- [ ] `src/features/agent-run/model.ts` — `TabState` + `WorkbenchStore`로 전면 재설계
- [ ] `src/features/agent-run/useAgentRun.ts` → `useAgentRun(tabId: string)` 시그니처로 변경
- [ ] `src/app/eventRouter.ts` 신설 (또는 기존 App bootstrap에 삽입)
- [ ] `src/widgets/workbench-tabs/TabBar.tsx` 신설
- [ ] `src/widgets/workbench-tabs/TabItem.tsx` 신설 (상태 dot, unread 배지, 닫기 버튼)
- [ ] `src/pages/agent-workbench/index.tsx` — TabBar + 활성 TabContent 레이아웃
- [ ] `src/app/styles.css` — 탭 스트립, 상태 dot, unread 배지
- [ ] (선택) 단축키: `Cmd/Ctrl+T`, `Cmd/Ctrl+W`, `Cmd/Ctrl+1..9`
- [ ] 기존 위젯(`RunPanel`, `GoalEditor`, `FollowUpComposer`, `FollowUpQueue`, `EventStream`) — 수정 최소화, props-only 유지

## 6. 단계별 작업 순서

1. **백엔드 다중 run 허용** — `reserve_run_if_idle` → `reserve_run`. 단위 테스트 + `cargo test` 통과
2. **스토어 리팩토링 (탭 1개 전제)** — `TabState` 도입하되 초기 탭 하나만 존재하게 고정. 기존 UI 동작 그대로 회귀 검증
3. **`useAgentRun(tabId)` 훅 + 이벤트 라우터 분리** — 단일 탭에서도 `dispatchRunEvent` 경로가 동작
4. **TabBar UI & 새 탭/닫기 액션** — 빈 신규 탭 생성, 전환
5. **탭 닫기 시 `cancelAgentRun` 연동** — 실행 중 탭을 닫아도 프로세스가 남지 않는지 QA
6. **동시 실행 검증** — 2개 이상 에이전트 동시 시작, 이벤트 라우팅·UI 상태 분리 확인
7. **비활성 탭 unread 배지 / 상태 dot** — UX 다듬기
8. **(선택) 설정 영속화** — 탭 레이아웃, 에이전트 설정을 localStorage 또는 파일에 저장

## 7. 위험 및 미결 사항

### 7.1 리스크 요약

| # | 항목 | 심각도 | 영향 | 완화 방안 |
| --- | --- | --- | --- | --- |
| R1 | 자원 사용량 증가 | High | 탭 수 비례로 subprocess / tokio task / stdio 버퍼 / 메모리 팽창 | `ACP_WORKBENCH_MAX_RUNS` 상한, 실행 중 탭 수 표기, 동시 실행 불가 시 친절한 에러 |
| R2 | 타임라인 무한 누적 | High | 장시간 세션에서 `items` 배열이 수만 건 → 렌더 지연, 메모리 스파이크 | ring buffer(`items.slice(-MAX)`), 그룹별 compaction, 가상 스크롤 |
| R3 | 비활성 탭 permission 누락 | High | 사용자가 대기 중인 approval을 못 봐서 run이 멈춤 | 탭 헤더에 `⚠️ permission pending` 배지, 시스템 알림 옵션 |
| R4 | 고아 이벤트 | Medium | 탭을 닫는 찰나에 도착한 envelope의 runId가 매칭 탭 없음 | 조용히 drop, dev 모드에서만 경고 로그. 라우터에 `knownRunIds` 캐시로 검증 |
| R5 | Vite HMR 리스너 중복 | Medium | 핫 리로드마다 listener 누적 → 이벤트가 N배로 처리됨 | `installRunEventRouter`가 이전 dispose 핸들을 해제하도록 강제. `import.meta.hot` 정리 훅 |
| R6 | 탭 닫기 vs 자연 종료 경합 | Low | `cancelAgentRun`과 에이전트 exit이 동시 발생 | `cancel_run`은 task abort만 하므로 idempotent. emit 타이밍 확인만 |
| R7 | 이벤트 라우팅 오배송 | Medium | 라우팅 버그로 탭 A의 permission/스트림이 탭 B에 섞임 | `envelope.runId === tab.activeRunId` 엄격 비교, 탭 전환 시 activeRunId 초기화 테스트 |
| R8 | 설정 영속화 크래시 복구 | Low | 탭 레이아웃 저장 중 앱 크래시 시 다음 기동에 상태 소실 / 손상 | write-through + atomic rename. 로드 실패 시 빈 탭 1개로 fallback |
| R9 | 멀티 윈도우 확장 시 재설계 | Low | Tauri 멀티 윈도우 요구가 오면 `WorkbenchStore`를 윈도우 단위로 분리해야 함 | 본 단계에서는 단일 창 멀티 탭으로 범위 고정. 추후 per-window store factory |
| R10 | Run ID 충돌 | Low | UUID 기반이라 실질 위험 없음 | `reserve_run`에서 중복 체크 유지 |
| R11 | verbose 에이전트 로그 | Low | Claude Code 등 verbose stderr가 타임라인 성능 저하 유발 | per-group 필터 기본값, 라인당 길이 제한 |

### 7.2 미결 사항

- 동시 실행 상한을 하드코딩할지 설정 가능하게 둘지 (환경변수 vs 설정 UI)
- 탭 닫기 시 확인 다이얼로그 표시 조건 (sessionActive 일 때만 / 항상 / 안 함)
- 탭 설정 영속화를 localStorage vs Tauri FS vs SQLite 중 어디에 둘지
- 시스템 알림(OS notification) 지원 범위 — permission pending 시에만 / run 완료 시에도

## 8. 후속 개선 아이디어 (본 문서 범위 밖)

- 탭 드래그로 순서 변경 / 윈도우 분리 (detach)
- "세션 복제" — 현재 탭의 agent/cwd/history를 유지한 채 새 탭으로 복제
- 세션 간 출력 비교 뷰 (Claude vs Codex 대결 모드)
- 탭당 히스토리 영구 저장 (SQLite or JSONL) + 재열기
- 전역 검색 (모든 탭의 타임라인 검색)
