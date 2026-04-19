# Workspace Task Worktree Isolation

Workspace task work should run from an isolated git worktree instead of the original checkout.

The backend command `provision_workspace_task_worktree` provisions a sibling worktree for a workspace checkout:

- The branch name is `worktree/<task-slug>`.
- The directory name is `<checkout-directory>-<task-slug>`.
- The original checkout remains untouched, including any dirty files already present there.
- The created worktree is saved as a workspace checkout with `kind: "worktree"` so the UI can show the active path in session context.

For example, task slug `Issue #63: Worktree Isolation` from checkout `/repo/acp-agent-workbench` creates:

```text
branch: worktree/issue-63-worktree-isolation
path:   /repo/acp-agent-workbench-issue-63-worktree-isolation
```

After the task is merged or abandoned, remove the isolated checkout from git and delete the task branch:

```sh
git worktree remove /repo/acp-agent-workbench-issue-63-worktree-isolation
git branch -d worktree/issue-63-worktree-isolation
```

Use `git branch -D` only for an abandoned branch whose unmerged commits are intentionally being discarded.
