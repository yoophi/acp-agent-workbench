# SQLite WAL 저장소 도입 계획

## 1. 목표

현재 workspace metadata는 `workspaces.json` 파일에 저장된다. 이 방식은 초기 MVP에는 단순하지만, 다음 요구가 들어오면 한계가 빨리 온다.

- workspace/checkouts 증가
- task/run/event history 영구 저장
- workspace별 최근 작업 조회
- run event stream 검색/필터링
- 앱 재시작 후 task/run context 복원
- JSON 파일 전체 rewrite로 인한 손상 위험 축소

따라서 사용자 데이터를 **SQLite + WAL(Write-Ahead Logging)** 방식으로 저장하는 구조로 전환한다.

핵심 목표:

- app data directory 아래 단일 SQLite DB 사용
- WAL mode 활성화
- workspace/checkouts를 우선 SQLite로 이전
- 이후 task/run/event history를 같은 저장소에 확장
- 기존 `WorkspaceStore` 포트를 유지해 application layer 영향 최소화

## 2. 저장 위치

DB 파일은 Tauri app data directory 아래에 둔다.

```text
{app_data_dir}/workbench.sqlite
{app_data_dir}/workbench.sqlite-wal
{app_data_dir}/workbench.sqlite-shm
```

Rust command에서는 Tauri path API로 `app_data_dir`를 얻는다.

```rust
let app_data_dir = app.path().app_data_dir()?;
let db_path = app_data_dir.join("workbench.sqlite");
```

주의:

- DB는 repo checkout 안이 아니라 앱 데이터 디렉토리에 둔다.
- iCloud/Dropbox/네트워크 드라이브 동기화 경로는 피한다.
- WAL mode에서는 `.sqlite`, `.sqlite-wal`, `.sqlite-shm` 파일이 함께 생긴다.

## 3. SQLite 연결 설정

DB open 직후 다음 PRAGMA를 적용한다.

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
PRAGMA busy_timeout = 5000;
```

의미:

| PRAGMA | 목적 |
| --- | --- |
| `journal_mode = WAL` | reader와 writer 간 경합을 줄이고 append-heavy event 저장에 유리하게 함 |
| `synchronous = NORMAL` | 데스크톱 앱에서 성능과 안정성의 균형점 |
| `foreign_keys = ON` | workspace/checkouts/tasks/runs 참조 무결성 강제 |
| `busy_timeout = 5000` | writer lock 경합 시 즉시 실패하지 않고 최대 5초 대기 |

## 4. Rust crate 선택

권장 선택지는 두 가지다.

### 4.1 `sqlx`

장점:

- async-first라 현재 Tauri command/tokio 구조와 잘 맞음
- connection pool 제공
- migration 기능 제공
- SQLite feature 지원

예상 dependency:

```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate"] }
```

단점:

- 초기 설정이 `rusqlite`보다 조금 무겁다.
- compile-time checked query를 쓰려면 별도 준비가 필요하다. 이 프로젝트에서는 우선 runtime query로 충분하다.

### 4.2 `rusqlite`

장점:

- API가 단순하고 SQLite에 직접적
- `bundled` feature로 SQLite 배포 이슈를 줄일 수 있음

예상 dependency:

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
```

단점:

- sync API라 async command 안에서는 `spawn_blocking` 또는 별도 writer thread 설계가 필요하다.
- pool/migration을 직접 구성해야 한다.

### 4.3 권장

이 프로젝트는 이미 tokio async command 중심이므로 **`sqlx`를 기본 선택**으로 한다.

다만 작업 범위를 줄이고 싶으면 `rusqlite` 기반 adapter를 먼저 만들 수 있다. 이 경우 DB 접근이 길어지는 작업은 `spawn_blocking`으로 분리한다.

## 5. 저장 대상 구분

### 5.1 1차 영구 저장 대상

workspace/checkouts만 SQLite로 옮긴다.

| 데이터 | 저장 여부 | 이유 |
| --- | --- | --- |
| Workspace | 저장 | origin 기반 상위 컨텍스트 |
| WorkspaceCheckout | 저장 | 로컬 checkout/worktree path, branch/head |
| Git origin metadata | 저장 | workspace 식별 및 표시 |

### 5.2 2차 영구 저장 대상

workspace 저장이 안정화된 뒤 추가한다.

| 데이터 | 저장 여부 | 이유 |
| --- | --- | --- |
| Task | 저장 | 앱 재시작 후 탭/작업 context 복원 |
| Run | 저장 | 과거 실행 이력 조회 |
| RunEvent | 저장 | event stream history, 검색, 감사 로그 |
| Permission decision | 선택 | 권한 응답 이력/디버깅 |

### 5.3 계속 메모리에 둘 대상

| 데이터 | 저장 여부 | 이유 |
| --- | --- | --- |
| live ACP session handle | 저장 안 함 | 프로세스 핸들은 재시작 후 복원 불가 |
| tokio task handle | 저장 안 함 | 런타임 내부 핸들 |
| pending permission waiter | 저장 안 함 | live session 상태와 결합 |
| active follow-up queue | 2차 이후 검토 | task 복원 정책 결정 필요 |

## 6. Schema 초안

### 6.1 Migration table

```sql
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);
```

### 6.2 Workspace tables

```sql
CREATE TABLE IF NOT EXISTS workspaces (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  raw_origin_url TEXT NOT NULL,
  canonical_origin_url TEXT NOT NULL UNIQUE,
  host TEXT NOT NULL,
  owner TEXT NOT NULL,
  repo TEXT NOT NULL,
  default_checkout_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workspace_checkouts (
  id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  kind TEXT NOT NULL,
  branch TEXT,
  head_sha TEXT,
  is_default INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(workspace_id, path)
);

CREATE INDEX IF NOT EXISTS idx_workspace_checkouts_workspace_id
ON workspace_checkouts(workspace_id);
```

### 6.3 Task/run/event tables

2차 이후 추가한다.

```sql
CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  workspace_id TEXT REFERENCES workspaces(id) ON DELETE SET NULL,
  checkout_id TEXT REFERENCES workspace_checkouts(id) ON DELETE SET NULL,
  title TEXT NOT NULL,
  goal TEXT NOT NULL,
  workdir TEXT NOT NULL,
  selected_agent_id TEXT NOT NULL,
  custom_command TEXT,
  auto_allow INTEGER NOT NULL,
  idle_timeout_sec INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS runs (
  id TEXT PRIMARY KEY,
  task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
  workspace_id TEXT REFERENCES workspaces(id) ON DELETE SET NULL,
  checkout_id TEXT REFERENCES workspace_checkouts(id) ON DELETE SET NULL,
  agent_id TEXT NOT NULL,
  goal TEXT NOT NULL,
  workdir TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT
);

CREATE TABLE IF NOT EXISTS run_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
  event_type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_run_events_run_id_id
ON run_events(run_id, id);
```

## 7. Rust 구조

### 7.1 Adapter 추가

추가 파일:

```text
src-tauri/src/adapters/sqlite.rs
src-tauri/src/adapters/workspace_store_sqlite.rs
```

역할:

| 파일 | 책임 |
| --- | --- |
| `sqlite.rs` | DB open, PRAGMA, migration 실행, pool 생성 |
| `workspace_store_sqlite.rs` | `WorkspaceStore` 포트의 SQLite 구현 |

### 7.2 AppState 변경

현재는 workspace command마다 `LocalWorkspaceStore`를 생성한다.

```rust
fn workspace_store(app: &AppHandle) -> Result<LocalWorkspaceStore, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
    Ok(LocalWorkspaceStore::new(app_data_dir))
}
```

SQLite로 전환하면 DB pool을 앱 시작 시 1회 만들고 상태에 보관한다.

권장 구조:

```rust
pub struct StorageState {
    pub workspace_store: SqliteWorkspaceStore,
}
```

또는 기존 `AppState`에 통합한다.

```rust
pub struct AppState {
    runs: Arc<Mutex<HashMap<String, RunSlot>>>,
    permissions: PermissionBroker,
    max_concurrent_runs: Option<usize>,
    workspace_store: SqliteWorkspaceStore,
}
```

권장: **`StorageState` 분리**.

이유:

- runtime session state와 persistent storage state의 생명주기가 다르다.
- 테스트에서 저장소만 교체하기 쉽다.
- 이후 task/run/event 저장소가 늘어나도 `AppState`가 비대해지지 않는다.

### 7.3 Tauri setup

```rust
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let storage = StorageState::open(app_data_dir)?;
            app.manage(storage);
            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(...)
        .run(...)
}
```

`sqlx`를 사용하면 setup 내부에서 async 초기화가 필요할 수 있다. 선택지는 두 가지다.

1. `tauri::async_runtime::block_on`으로 setup에서 DB open/migration 실행
2. lazy initialization을 사용하고 첫 command에서 초기화

권장: setup에서 명시적으로 초기화한다. 앱이 뜬 뒤 저장소 에러가 늦게 터지는 것보다 부팅 실패가 낫다.

## 8. JSON에서 SQLite로 migration

현재 `workspaces.json` 사용자가 이미 있을 수 있으므로 1회 import를 제공한다.

Migration 정책:

1. SQLite DB를 연다.
2. schema migration을 적용한다.
3. `workspaces` 테이블이 비어 있는지 확인한다.
4. `{app_data_dir}/workspaces.json`이 존재하면 읽는다.
5. workspace/checkouts를 SQLite에 upsert한다.
6. 성공하면 원본 파일을 보존 이름으로 변경한다.

```text
workspaces.json
-> workspaces.json.migrated
```

실패 시:

- SQLite transaction rollback
- 원본 JSON 유지
- command error 또는 setup error로 surface

## 9. 동시성 정책

SQLite WAL은 여러 reader와 하나의 writer에 적합하다. 하지만 writer는 여전히 하나다.

정책:

- connection pool 사용
- write command는 transaction 사용
- event append는 batch insert 가능성을 열어둔다
- busy timeout은 5초
- long-running transaction 금지
- event stream 저장은 follow-up PR에서 debounce/batch 전략 검토

workspace/checkouts 저장은 빈도가 낮아 별도 batching이 필요 없다.

run event 저장을 추가할 때는 다음 중 하나를 선택한다.

| 방식 | 장점 | 단점 |
| --- | --- | --- |
| event마다 insert | 단순함 | write 빈도 높음 |
| in-memory buffer + batch insert | 효율적 | crash 직전 이벤트 유실 가능 |
| bounded channel + writer task | UI command와 분리 | shutdown flush 필요 |

권장: 처음에는 event마다 insert, 문제가 보이면 writer task로 분리한다.

## 10. 의존성 트리

```text
Legend:
  [ ] 작업
  --> 필수 의존성
  -.-> 후속/선택 의존성

                         +--------------------------------+
                         | [A0] 저장 범위 확정             |
                         | workspace/checkouts 먼저        |
                         | task/run/event는 후속           |
                         +----------------+---------------+
                                          |
                                          v
                         +--------------------------------+
                         | [A1] crate 선택                 |
                         | sqlx sqlite runtime-tokio       |
                         | 또는 rusqlite bundled           |
                         +----------------+---------------+
                                          |
                                          v
             +----------------------------+-----------------------------+
             |                                                          |
             v                                                          v
+-----------------------------+                         +-------------------------------+
| [B0] SQLite infra 추가       |                         | [B1] schema migration 파일     |
| adapters/sqlite.rs           |                         | 001_workspace.sql              |
| open pool + PRAGMA WAL       |                         | migrations runner              |
+--------------+--------------+                         +---------------+---------------+
               |                                                        |
               +---------------------------+----------------------------+
                                           |
                                           v
                         +--------------------------------+
                         | [B2] StorageState 도입          |
                         | app setup에서 DB 초기화         |
                         | app.manage(StorageState)        |
                         +----------------+---------------+
                                          |
                                          v
                         +--------------------------------+
                         | [C0] SqliteWorkspaceStore 구현  |
                         | WorkspaceStore trait 구현       |
                         | list/get/register/remove/refresh|
                         +----------------+---------------+
                                          |
                                          v
                         +--------------------------------+
                         | [C1] Tauri commands 교체        |
                         | LocalWorkspaceStore 생성 제거   |
                         | State<StorageState> 사용        |
                         +----------------+---------------+
                                          |
                                          v
                         +--------------------------------+
                         | [D0] JSON -> SQLite migration   |
                         | workspaces.json 1회 import      |
                         | 성공 후 .migrated 보존          |
                         +----------------+---------------+
                                          |
                                          v
                         +--------------------------------+
                         | [D1] 테스트 보강                |
                         | temp DB, migration, upsert,     |
                         | path refresh, JSON import       |
                         +----------------+---------------+
                                          |
                                          v
                         +--------------------------------+
                         | [E0] JSON store 제거/비활성화   |
                         | 필요 시 fallback만 유지         |
                         +----------------+---------------+
                                          |
                                          v
                         +--------------------------------+
                         | [F0] task 저장소 추가           |
                         | task table + TaskStore          |
                         +----------------+---------------+
                                          |
                                          v
                         +--------------------------------+
                         | [F1] run 저장소 추가            |
                         | runs table + RunStore           |
                         +----------------+---------------+
                                          |
                                          v
                         +--------------------------------+
                         | [F2] run event 저장소 추가      |
                         | run_events append/query         |
                         +--------------------------------+
```

## 11. 단계별 작업 순서

### Phase 1. SQLite 기반 workspace store

1. `sqlx` dependency 추가
2. `adapters/sqlite.rs` 추가
3. app data dir 아래 `workbench.sqlite` open
4. WAL PRAGMA 적용
5. workspace/checkouts schema migration 추가
6. `StorageState` 추가
7. `SqliteWorkspaceStore`가 `WorkspaceStore` 포트 구현
8. workspace Tauri command가 `State<StorageState>`를 사용하도록 교체

완료 기준:

- 기존 workspace 등록/목록/checkout 조회 UI가 SQLite 기반으로 동작
- `workspaces.json` 없이도 재시작 후 데이터 복원
- `cargo test` 통과

### Phase 2. JSON migration

1. 기존 `workspaces.json` parser 재사용
2. DB가 비어 있으면 JSON import
3. transaction으로 workspace/checkouts upsert
4. 성공 시 `workspaces.json.migrated`로 rename
5. 실패 시 JSON 유지

완료 기준:

- 기존 JSON 사용자가 데이터 손실 없이 SQLite로 이전
- migration 테스트 통과

### Phase 3. task/run/event persistence

1. task schema 추가
2. run schema 추가
3. run event schema 추가
4. task 저장/복원 command 추가
5. run lifecycle event 저장
6. event stream 조회 API 추가

완료 기준:

- 앱 재시작 후 최근 task 목록 복원
- 완료된 run의 event history 조회 가능

## 12. 테스트 전략

### Unit tests

- origin canonical URL이 `UNIQUE`로 중복 workspace를 막는지
- 같은 workspace의 checkout path upsert
- workspace 삭제 시 checkout cascade delete
- WAL PRAGMA 적용 여부
- migration idempotency

### Integration-style tests

- temp dir에 SQLite DB 생성
- `register_workspace_from_path` equivalent 흐름 실행
- 앱 재시작을 흉내 내기 위해 store 재생성 후 데이터 조회
- JSON 파일이 있을 때 1회 import 후 `.migrated`로 변경되는지 확인

### Regression tests

- `workspace_id == None`인 legacy run은 기존 `cwd` 방식 유지
- workspace 기반 run은 checkout 밖 path 거부
- workspace command error가 String으로 UI에 surface

## 13. 운영 고려사항

### Backup

SQLite WAL mode에서는 DB 백업 시 `.sqlite`, `.sqlite-wal`, `.sqlite-shm` 파일 관계를 고려해야 한다. 일반적인 앱 내부 백업은 SQLite backup API를 쓰는 편이 안전하다.

### Vacuum

event history가 커지면 주기적 cleanup 정책이 필요하다.

후속 옵션:

```sql
PRAGMA wal_checkpoint(TRUNCATE);
VACUUM;
```

단, 사용자 작업 중 자동 실행하지 않는다. 설정 화면의 maintenance action으로 두는 편이 낫다.

### Retention

run event를 영구 저장하면 DB가 커진다.

후속 정책:

- 최근 N일만 보관
- workspace별 최대 event 수
- 사용자가 특정 run을 pin하면 보존
- archive/export 기능

## 14. 결론

SQLite WAL 저장소는 workspace metadata를 넘어 task/run/event history까지 확장할 계획에 적합하다.

권장 순서는 다음이다.

1. `WorkspaceStore` 포트는 유지한다.
2. JSON adapter를 바로 키우지 않는다.
3. `SqliteWorkspaceStore`를 추가해 workspace/checkouts부터 이전한다.
4. 기존 `workspaces.json`은 1회 import 후 보존 이름으로 남긴다.
5. task/run/event 저장은 별도 PR로 확장한다.
