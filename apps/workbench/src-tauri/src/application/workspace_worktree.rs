use anyhow::{Result, anyhow, bail};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::{
    domain::workspace::{CheckoutKind, WorkspaceCheckout},
    ports::{git_repository::GitRepositoryPort, workspace_store::WorkspaceStore},
};

#[derive(Clone)]
pub struct WorkspaceTaskWorktreeUseCase<S, G>
where
    S: WorkspaceStore,
    G: GitRepositoryPort,
{
    store: S,
    git: G,
}

impl<S, G> WorkspaceTaskWorktreeUseCase<S, G>
where
    S: WorkspaceStore,
    G: GitRepositoryPort,
{
    pub fn new(store: S, git: G) -> Self {
        Self { store, git }
    }

    pub async fn provision(
        &self,
        workspace_id: &str,
        checkout_id: Option<&str>,
        task_slug: Option<&str>,
    ) -> Result<WorkspaceCheckout> {
        let workspace_id = workspace_id.trim();
        if workspace_id.is_empty() {
            bail!("workspace id is required");
        }

        let workspace = self
            .store
            .get_workspace(workspace_id)
            .await?
            .ok_or_else(|| anyhow!("workspace not found: {workspace_id}"))?;

        let checkout_id = checkout_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or(workspace.default_checkout_id.as_deref())
            .ok_or_else(|| anyhow!("workspace has no checkout: {workspace_id}"))?;

        let checkout = self
            .store
            .get_checkout(checkout_id)
            .await?
            .ok_or_else(|| anyhow!("checkout not found: {checkout_id}"))?;

        if checkout.workspace_id != workspace.id {
            bail!("checkout {checkout_id} does not belong to workspace {workspace_id}");
        }

        let slug = task_worktree_slug(task_slug);
        let branch = format!("worktree/{slug}");
        let path = derive_worktree_path(&checkout.path, &slug)?;
        let status = self.git.create_worktree(&checkout.path, &branch, &path)?;
        let worktree = WorkspaceCheckout::new_worktree(
            workspace.id,
            &workspace.origin.canonical_url,
            PathBuf::from(status.root),
            status.branch,
            status.head_sha,
        );

        match self.store.save_checkout(worktree.clone()).await {
            Ok(saved) => Ok(saved),
            Err(err) => {
                let cleanup_result = self.git.remove_worktree(&worktree.path, Some(&branch));
                if let Err(cleanup_err) = cleanup_result {
                    bail!(
                        "failed to save provisioned checkout: {err}; rollback also failed: {cleanup_err}"
                    );
                }
                Err(err)
            }
        }
    }

    pub async fn cleanup(&self, checkout_id: &str) -> Result<bool> {
        let checkout_id = checkout_id.trim();
        if checkout_id.is_empty() {
            bail!("checkout id is required");
        }
        let Some(checkout) = self.store.get_checkout(checkout_id).await? else {
            return Ok(false);
        };
        if !matches!(checkout.kind, CheckoutKind::Worktree) || checkout.is_default {
            bail!("checkout is not a cleanup-safe task worktree: {checkout_id}");
        }
        let branch = checkout.branch.as_deref();
        self.git.remove_worktree(&checkout.path, branch)?;
        self.store.remove_checkout(&checkout.id).await?;
        Ok(true)
    }
}

fn derive_worktree_path(checkout_path: &Path, slug: &str) -> Result<PathBuf> {
    let parent = checkout_path
        .parent()
        .ok_or_else(|| anyhow!("checkout path has no parent: {}", checkout_path.display()))?;
    let checkout_name = checkout_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            anyhow!(
                "checkout path has no directory name: {}",
                checkout_path.display()
            )
        })?;
    Ok(parent.join(format!("{checkout_name}-{slug}")))
}

fn task_worktree_slug(value: Option<&str>) -> String {
    let mut slug = sanitize_worktree_slug(value.unwrap_or_default());
    let id = Uuid::new_v4().simple().to_string();
    if slug.is_empty() {
        format!("task-{}", &id[..8])
    } else {
        const MAX_SLUG_PREFIX_LEN: usize = 48;
        slug.truncate(MAX_SLUG_PREFIX_LEN);
        slug = slug.trim_matches('-').to_string();
        format!("{slug}-{}", &id[..8])
    }
}

fn sanitize_worktree_slug(value: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceTaskWorktreeUseCase, derive_worktree_path, sanitize_worktree_slug};
    use crate::{
        domain::{
            git::{WorkspaceGitFileStatus, WorkspaceGitStatus},
            workspace::{
                CheckoutId, CheckoutKind, GitOrigin, Workspace, WorkspaceCheckout, WorkspaceId,
            },
        },
        ports::{git_repository::GitRepositoryPort, workspace_store::WorkspaceStore},
    };
    use anyhow::{Result, anyhow};
    use std::{
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    };

    #[test]
    fn sanitizes_task_identifier_for_branch_and_path() {
        assert_eq!(
            sanitize_worktree_slug("Issue #63: Worktree Isolation"),
            "issue-63-worktree-isolation"
        );
        assert_eq!(sanitize_worktree_slug("  ///  "), "");
    }

    #[test]
    fn derives_sibling_worktree_path_from_checkout_name() {
        let path =
            derive_worktree_path(Path::new("/repo/acp-agent-workbench"), "issue-63").unwrap();
        assert_eq!(path, Path::new("/repo/acp-agent-workbench-issue-63"));
    }

    #[derive(Clone)]
    struct WorktreeTestStore {
        fail_save: bool,
        removed_checkouts: Arc<Mutex<Vec<String>>>,
    }

    impl WorktreeTestStore {
        fn new(fail_save: bool) -> Self {
            Self {
                fail_save,
                removed_checkouts: Arc::new(Mutex::new(vec![])),
            }
        }
    }

    impl WorkspaceStore for WorktreeTestStore {
        async fn list_workspaces(&self) -> Result<Vec<Workspace>> {
            Ok(vec![])
        }

        async fn get_workspace(&self, id: &str) -> Result<Option<Workspace>> {
            Ok((id == "workspace-1").then(|| Workspace {
                id: "workspace-1".into(),
                name: "repo".into(),
                origin: GitOrigin {
                    raw_url: "git@github.com:owner/repo.git".into(),
                    canonical_url: "github.com/owner/repo".into(),
                    host: "github.com".into(),
                    owner: "owner".into(),
                    repo: "repo".into(),
                },
                default_checkout_id: Some("checkout-1".into()),
                created_at: "1".into(),
                updated_at: "1".into(),
            }))
        }

        async fn list_checkouts(&self, _workspace_id: &str) -> Result<Vec<WorkspaceCheckout>> {
            Ok(vec![])
        }

        async fn get_checkout(&self, id: &str) -> Result<Option<WorkspaceCheckout>> {
            Ok(match id {
                "checkout-1" => Some(WorkspaceCheckout {
                    id: "checkout-1".into(),
                    workspace_id: "workspace-1".into(),
                    path: "/repo/main".into(),
                    kind: CheckoutKind::Clone,
                    branch: Some("main".into()),
                    head_sha: Some("abc".into()),
                    is_default: true,
                }),
                "worktree-checkout" => Some(WorkspaceCheckout {
                    id: "worktree-checkout".into(),
                    workspace_id: "workspace-1".into(),
                    path: "/repo/main-task".into(),
                    kind: CheckoutKind::Worktree,
                    branch: Some("worktree/task".into()),
                    head_sha: Some("abc".into()),
                    is_default: false,
                }),
                _ => None,
            })
        }

        async fn remove_workspace(&self, _workspace_id: &WorkspaceId) -> Result<()> {
            Ok(())
        }

        async fn remove_checkout(&self, checkout_id: &CheckoutId) -> Result<()> {
            self.removed_checkouts
                .lock()
                .unwrap()
                .push(checkout_id.clone());
            Ok(())
        }

        async fn save_checkout(&self, checkout: WorkspaceCheckout) -> Result<WorkspaceCheckout> {
            if self.fail_save {
                Err(anyhow!("database unavailable"))
            } else {
                Ok(checkout)
            }
        }

        async fn refresh_checkout(
            &self,
            _checkout_id: &CheckoutId,
        ) -> Result<Option<WorkspaceCheckout>> {
            Ok(None)
        }
    }

    #[derive(Clone)]
    struct WorktreeTestGit {
        removed: Arc<Mutex<Vec<(PathBuf, Option<String>)>>>,
    }

    impl WorktreeTestGit {
        fn new() -> Self {
            Self {
                removed: Arc::new(Mutex::new(vec![])),
            }
        }
    }

    impl GitRepositoryPort for WorktreeTestGit {
        fn status(&self, _workdir: &Path) -> Result<WorkspaceGitStatus> {
            unreachable!()
        }

        fn diff_summary(
            &self,
            _workdir: &Path,
        ) -> Result<crate::domain::git::WorkspaceDiffSummary> {
            unreachable!()
        }

        fn commit(
            &self,
            _workdir: &Path,
            _message: &str,
            _files: &[String],
        ) -> Result<crate::domain::git::WorkspaceCommitResult> {
            unreachable!()
        }

        fn push(
            &self,
            _workdir: &Path,
            _remote: &str,
            _branch: &str,
            _set_upstream: bool,
        ) -> Result<crate::domain::git::WorkspacePushResult> {
            unreachable!()
        }

        fn create_worktree(
            &self,
            _source_workdir: &Path,
            branch_name: &str,
            worktree_path: &Path,
        ) -> Result<WorkspaceGitStatus> {
            Ok(WorkspaceGitStatus {
                root: worktree_path.to_string_lossy().to_string(),
                branch: Some(branch_name.to_string()),
                head_sha: Some("abc".into()),
                is_dirty: false,
                files: Vec::<WorkspaceGitFileStatus>::new(),
            })
        }

        fn remove_worktree(&self, worktree_path: &Path, branch_name: Option<&str>) -> Result<()> {
            self.removed.lock().unwrap().push((
                worktree_path.to_path_buf(),
                branch_name.map(ToString::to_string),
            ));
            Ok(())
        }
    }

    #[tokio::test]
    async fn rolls_back_created_worktree_when_checkout_save_fails() {
        let store = WorktreeTestStore::new(true);
        let git = WorktreeTestGit::new();
        let result = WorkspaceTaskWorktreeUseCase::new(store, git.clone())
            .provision("workspace-1", Some("checkout-1"), Some("Task"))
            .await;

        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("database unavailable")
        );
        let removed = git.removed.lock().unwrap();
        assert_eq!(removed.len(), 1);
        assert!(
            removed[0]
                .1
                .as_deref()
                .is_some_and(|branch| branch.starts_with("worktree/task-"))
        );
        assert!(removed[0].0.to_string_lossy().contains("/repo/main-task-"));
    }

    #[tokio::test]
    async fn cleanup_removes_task_worktree_and_checkout_record() {
        let store = WorktreeTestStore::new(false);
        let removed_checkouts = store.removed_checkouts.clone();
        let git = WorktreeTestGit::new();

        let cleaned = WorkspaceTaskWorktreeUseCase::new(store, git.clone())
            .cleanup("worktree-checkout")
            .await
            .unwrap();

        assert!(cleaned);
        assert_eq!(
            removed_checkouts.lock().unwrap().as_slice(),
            ["worktree-checkout".to_string()]
        );
        assert_eq!(
            git.removed.lock().unwrap()[0].0,
            Path::new("/repo/main-task")
        );
        assert_eq!(
            git.removed.lock().unwrap()[0].1.as_deref(),
            Some("worktree/task")
        );
    }
}
