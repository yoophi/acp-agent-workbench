# Workspace Task Worktree Isolation

Workspace task work should run from an isolated git worktree instead of the original checkout.

Workspace-scoped agent runs call `provision_workspace_task_worktree` before starting the ACP session.
The command provisions a sibling worktree for the selected workspace checkout and returns the new checkout.

- The branch name is `worktree/<task-slug>-<short-id>`.
- The directory name is `<checkout-directory>-<task-slug>-<short-id>`.
- The original checkout remains untouched, including any dirty files already present there.
- The created worktree is saved as a workspace checkout with `kind: "worktree"`.
- The active tab switches to the worktree checkout and clears the custom `cwd`, so the ACP session starts from the isolated worktree root.

For example, task slug `Issue #63: Worktree Isolation` from checkout `/repo/acp-agent-workbench` creates a path like:

```text
branch: worktree/issue-63-worktree-isolation-a1b2c3d4
path:   /repo/acp-agent-workbench-issue-63-worktree-isolation-a1b2c3d4
```

After the task is merged or abandoned, remove the isolated checkout from git and delete the task branch:

```sh
git worktree remove /repo/acp-agent-workbench-issue-63-worktree-isolation-a1b2c3d4
git branch -d worktree/issue-63-worktree-isolation-a1b2c3d4
```

Use `git branch -D` only for an abandoned branch whose unmerged commits are intentionally being discarded.
