# Work Context 용어 체계

## 1. 목표

ACP Agent Workbench는 agent가 작업할 수 있는 여러 종류의 위치를 지원해야 한다.
기존 `Workspace` 용어는 너무 넓다. 모든 working directory가 GitHub 기반 repository는 아니기 때문이다.

이 문서는 세 가지 context type의 제품 용어, 도메인 경계, capability, UI 흐름을 정의한다.

엔티티 정의, 의존관계, Mermaid diagram은 `docs/work-context-entities.md`를 참고한다.

## 2. 권장 용어

내부 상위 개념으로는 `WorkContext`를 사용한다.

사용자에게 보이는 용어:

| 용어 | 한국어 라벨 | 의미 |
| --- | --- | --- |
| GitHub Repository | GitHub 저장소 | GitHub origin이 연결된 git repository |
| Local Repository | 로컬 저장소 | GitHub origin을 요구하지 않는 로컬 git repository |
| Local Folder | 로컬 폴더 | git repository가 아닌 일반 로컬 디렉터리 |

공간이 좁은 UI에서는 짧은 라벨을 쓸 수 있다.

| 전체 라벨 | 짧은 라벨 |
| --- | --- |
| GitHub Repository | Repository |
| Local Repository | Local Repo |
| Local Folder | Folder |

피해야 할 용어:

| 용어 | 이유 |
| --- | --- |
| Workspace | 앱 session, IDE workspace, run directory 등으로 해석될 수 있어 너무 넓다. |
| Project | git repository와 plain folder를 구분하지 못해 너무 넓다. |
| Inline Project | 구현 관점의 표현이라 사용자에게 의미가 불명확하다. |
| Local Project | 로컬 git repo와 plain folder를 모두 뜻할 수 있어 애매하다. |

## 2.1 Canonical Entity Names

새 product copy, documentation, domain model에는 아래 이름을 사용한다.

| Canonical entity | 의미 | 현재 구현 이름 | Migration note |
| --- | --- | --- | --- |
| `WorkContext` | agent가 작업할 수 있는 위치의 상위 개념 | `Workspace` / tab state 언어가 섞여 있음 | 향후 공유 추상화 이름 |
| `GitHubRepository` | GitHub origin이 연결된 git repository | `Workspace` | 기존 `Workspace`는 이 narrower concept에 대응 |
| `LocalRepository` | GitHub origin을 요구하지 않는 로컬 git repository | 미구현 | 새 context kind로 추가 |
| `LocalFolder` | 일반 로컬 디렉터리 | legacy direct `cwd` mode | 직접 run directory 모드를 이름 있는 context로 승격 |
| `RunTarget` | run이 실행되는 구체 경로 | `WorkspaceCheckout` + `cwd` | git checkout과 plain folder를 모두 지원하도록 일반화 |
| `RunDirectory` | agent process에 전달되는 directory | `cwd`, `workdir`, "Working directory" | user-facing은 "Run Directory" 선호. `cwd`는 protocol boundary에만 유지 |
| `WorkTask` | context의 issue system에서 선택된 task | beads용 `LocalTaskSummary` | GitHub Issues, beads, backlog.md를 아우르도록 일반화 |

기존 코드는 API, database, command 이름을 함께 바꾸는 명시적 migration 전까지 `workspaceId`와 `Workspace*` 이름을 유지할 수 있다.
새 UI 라벨에서는 `Workspace`를 umbrella term으로 쓰지 않는다.

## 3. Context Types

### 3.1 GitHub Repository

`GitHub Repository`는 GitHub origin이 연결된 git 기반 project다.

속성:

- Git이 필요하다.
- GitHub origin이 필요하다.
- issue/task system은 GitHub Issues다.
- task isolation에 worktree를 사용할 수 있다.
- pull request를 사용할 수 있다.
- agent가 branch, commit, push, pull request 생성을 수행할 수 있다.

앱이 관리할 것:

- repository 등록
- GitHub origin metadata
- local checkout 및 worktree path
- issue 선택 또는 issue context
- agent run lifecycle
- worktree cleanup

앱이 관리하지 않을 것:

- PR content를 1급 로컬 상태로 관리하지 않는다.
- PR review draft를 앱의 핵심 엔티티로 두지 않는다.
- 수동 PR 생성/리뷰 form을 기본 workflow로 제공하지 않는다.

pull request 생성은 별도 앱 workflow가 아니라 agent task의 결과물이다.

### 3.2 Local Repository

`Local Repository`는 GitHub origin을 요구하지 않는 디스크상의 git repository다.

속성:

- Git이 필요하다.
- GitHub origin은 필요하지 않다.
- beads 같은 로컬 issue system을 사용한다.
- task isolation에 worktree를 사용할 수 있다.
- 앱은 pull request를 지원하지 않는다.
- 완료 후 사용자가 로컬에서 review하고 manual merge한다.

앱이 관리할 것:

- local repository 등록
- local checkout 및 worktree path
- local task source 연동
- agent run lifecycle
- worktree cleanup

이 context에서는 GitHub pull request 생성 control을 보여주지 않는다.

### 3.3 Local Folder

`Local Folder`는 git repository가 아닌 일반 디렉터리다.

속성:

- Git이 필요하지 않다.
- GitHub origin이 없다.
- worktree를 사용할 수 없다.
- pull request를 사용할 수 없다.
- `backlog.md` 같은 파일 기반 issue system을 사용할 수 있다.
- 완료 후 사용자가 파일 변경을 직접 review한다.

앱이 관리할 것:

- folder 등록
- file-based task source 연동
- agent run lifecycle

이 context에서는 worktree, commit, push, pull request 같은 git 전용 control을 보여주지 않는다.

## 4. Capability Matrix

```text
+----------------------+--------------------------+--------------------------+--------------------------+
| Capability           | GitHub Repository        | Local Repository         | Local Folder             |
+----------------------+--------------------------+--------------------------+--------------------------+
| Directory type       | Git repo                 | Git repo                 | Plain directory          |
| GitHub origin        | Required                 | Not required             | Not available            |
| Git required         | Yes                      | Yes                      | No                       |
| Issue system         | GitHub Issues            | beads / local tasks      | backlog.md / local docs  |
| Worktree             | Supported                | Supported                | Not supported            |
| Branch isolation     | Supported                | Supported                | Not supported            |
| Agent run            | Supported                | Supported                | Supported                |
| Commit               | Supported                | Supported                | Not supported by app     |
| Push                 | Supported                | Optional if remote exists| Not supported            |
| Pull Request         | Supported                | Not supported            | Not supported            |
| Merge flow           | GitHub PR merge          | Manual local merge       | Manual file review       |
+----------------------+--------------------------+--------------------------+--------------------------+
```

## 5. Entity Relationships

```text
                           +----------------+
                           |  WorkContext   |
                           +-------+--------+
                                   |
              +--------------------+--------------------+
              |                    |                    |
              v                    v                    v
       +-------------------+ +------------------+ +----------------+
       | GitHub Repository | | Local Repository | | Local Folder   |
       +---------+---------+ +---------+--------+ +--------+-------+
                 |                     |                   |
                 | GitHub origin       | Local git root    | Plain directory
                 v                     v                   v
       +-------------------+ +------------------+ +----------------+
       | GitHub Issue      | | beads task       | | backlog.md task|
       +---------+---------+ +---------+--------+ +--------+-------+
                 |                     |                   |
                 | create worktree     | create worktree   | use same dir
                 v                     v                   v
       +-------------------+ +------------------+ +----------------+
       | Worktree          | | Worktree         | | Run directory  |
       +---------+---------+ +---------+--------+ +--------+-------+
                 |                     |                   |
                 v                     v                   v
            +----------+          +----------+         +----------+
            | AgentRun |          | AgentRun |         | AgentRun |
            +----+-----+          +----+-----+         +----+-----+
                 |                     |                   |
                 v                     v                   v
       +-------------------+ +------------------+ +----------------+
       | Pull Request      | | Manual merge     | | Manual review  |
       +-------------------+ +------------------+ +----------------+
```

## 6. 제안 타입 형태

상위 context는 discriminated union으로 둔다.

```ts
type WorkContext =
  | GitHubRepositoryContext
  | LocalRepositoryContext
  | LocalFolderContext;

type GitHubRepositoryContext = {
  id: string;
  kind: "githubRepository";
  name: string;
  rootPath: string;
  origin: GitOrigin;
  issueSource: "githubIssues";
  isolation: "worktree";
  completion: "pullRequest";
};

type LocalRepositoryContext = {
  id: string;
  kind: "localRepository";
  name: string;
  rootPath: string;
  issueSource: "beads";
  isolation: "worktree";
  completion: "manualMerge";
};

type LocalFolderContext = {
  id: string;
  kind: "localFolder";
  name: string;
  rootPath: string;
  issueSource: "backlogMd";
  isolation: "sharedDirectory";
  completion: "manualReview";
};
```

실행 대상은 top-level context와 분리한다.

```ts
type RunTarget =
  | {
      kind: "gitCheckout";
      contextId: string;
      checkoutId: string;
      path: string;
      checkoutKind: "clone" | "worktree";
      branch?: string | null;
      headSha?: string | null;
    }
  | {
      kind: "localFolder";
      contextId: string;
      path: string;
    };
```

task source는 context kind에 따라 다르다.

```ts
type WorkTask =
  | {
      source: "githubIssue";
      contextKind: "githubRepository";
      number: number;
      title: string;
      url: string;
      state: "open" | "closed";
    }
  | {
      source: "beads";
      contextKind: "localRepository";
      id: string;
      title: string;
      status: "open" | "in_progress" | "closed";
      blocked: boolean;
    }
  | {
      source: "backlogMd";
      contextKind: "localFolder";
      id: string;
      title: string;
      status: string;
      filePath: string;
    };
```

Capability는 label에서 추론하지 말고 명시적으로 계산한다.

```ts
type WorkContextCapabilities = {
  canUseGit: boolean;
  canCreateWorktree: boolean;
  canCreatePullRequest: boolean;
  canCommit: boolean;
  canPush: boolean;
  requiresManualMerge: boolean;
  requiresManualReview: boolean;
};
```

## 7. UI 흐름

초기 등록 화면은 세 가지 선택지를 제공한다.

```text
+---------------------+  +---------------------+  +---------------------+
| GitHub Repository   |  | Local Repository    |  | Local Folder        |
| GitHub origin,      |  | Git repo on disk,   |  | Plain directory,    |
| Issues, worktrees,  |  | local tasks,        |  | backlog.md tasks,   |
| pull requests       |  | worktrees           |  | direct edits        |
+---------------------+  +---------------------+  +---------------------+
```

공통 흐름:

```text
Select context
  |
  v
Select task
  |
  +-- GitHub Issue, for GitHub Repository
  +-- beads task, for Local Repository
  +-- backlog.md task, for Local Folder
  |
  v
Prepare run target
  |
  +-- create/select worktree, if supported
  +-- use folder directly, for Local Folder
  |
  v
Run agent
  |
  v
Review result
  |
  +-- GitHub Repository: PR flow may be performed by the agent
  +-- Local Repository: manual merge
  +-- Local Folder: manual file review
```

## 8. 현재 구현에 대한 영향

현재 코드는 GitHub origin 중심의 `Workspace` 모델을 갖고 있다. taxonomy migration 과정에서 이 모델은 좁히거나 이름을 바꿔야 한다.

권장 방향:

| 현재 개념 | 제안 개념 |
| --- | --- |
| umbrella term으로 쓰이는 `Workspace` | `WorkContext` |
| GitHub origin 기반 `Workspace` | `GitHubRepository` / `GitHubRepositoryContext` |
| 등록된 repository 없이 직접 쓰는 `cwd` | `LocalFolder` |
| 향후 GitHub origin 없는 git repo | `LocalRepository` |
| `WorkspaceCheckout` | `RunTarget` 또는 context 아래 git checkout |
| beads용 `LocalTaskList` | Local Repository task source |
| PR publish/review panel | 앱이 관리하는 첫 화면에서 제거 |
| `PullRequestReviewDraft` | 제거 후보 |

모든 context kind에서 agent run/session/event 처리는 공통으로 유지한다.

UI action은 capability로 gate한다.

- `canCreateWorktree`가 true일 때만 worktree control을 보여준다.
- `githubRepository`에서만 GitHub issue control을 보여준다.
- `localRepository`에서만 beads control을 보여준다.
- `localFolder`에서만 backlog.md control을 보여준다.
- 앱이 관리하는 pull request form은 기본으로 보여주지 않는다. pull request 생성은 agent task flow에 속한다.

## 9. Open Questions

- `Local Repository`가 GitHub가 아닌 remote에 대해 push-only flow를 지원해야 하는가?
- `Local Folder`에서 나중에 git init을 수행해 `Local Repository`로 전환할 수 있어야 하는가?
- context 등록 후 task source를 변경 가능하게 둘 것인가?
- PR 생성은 앱 command가 아니라 agent prompt template으로만 노출할 것인가?
