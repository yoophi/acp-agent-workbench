# Workspace Task Worktree Isolation

> 상태: 용어 주석.
>
> 이 문서의 `Workspace`는 현재 taxonomy의 git-capable context인
> `GitHub Repository`와 `Local Repository`에 대응한다. `Local Folder`는 worktree를 사용할 수 없다.

Workspace task는 원본 checkout 대신 격리된 git worktree에서 실행되어야 한다.

Workspace-scoped agent run은 ACP session을 시작하기 전에 `provision_workspace_task_worktree`를 호출한다.
이 command는 선택된 workspace checkout의 sibling worktree를 만들고 새 checkout을 반환한다.

- branch 이름은 `worktree/<task-slug>-<short-id>`다.
- directory 이름은 `<checkout-directory>-<task-slug>-<short-id>`다.
- 원본 checkout은 기존 dirty file을 포함해 변경하지 않는다.
- 생성된 worktree는 `kind: "worktree"`인 workspace checkout으로 저장한다.
- active tab은 worktree checkout으로 전환되고 custom `cwd`를 비운다. 따라서 ACP session은 격리된 worktree root에서 시작한다.

예를 들어 checkout `/repo/acp-agent-workbench`에서 task slug가 `Issue #63: Worktree Isolation`이면 다음과 같은 path를 만든다.

```text
branch: worktree/issue-63-worktree-isolation-a1b2c3d4
path:   /repo/acp-agent-workbench-issue-63-worktree-isolation-a1b2c3d4
```

작업이 merge되었거나 abandon되면 git에서 격리 checkout을 제거하고 task branch를 삭제한다.

```sh
git worktree remove /repo/acp-agent-workbench-issue-63-worktree-isolation-a1b2c3d4
git branch -d worktree/issue-63-worktree-isolation-a1b2c3d4
```

merge되지 않은 commit을 의도적으로 버리는 abandoned branch에만 `git branch -D`를 사용한다.
