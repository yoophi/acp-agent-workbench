use anyhow::{Result, anyhow, bail};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    adapters::acp::util::{expand_tilde, normalize_path},
    domain::git::{
        WorkspaceCommitResult, WorkspaceDiffSummary, WorkspaceGitFileStatus, WorkspaceGitStatus,
        WorkspacePushResult,
    },
    domain::workspace::GitOrigin,
    ports::git_repository::GitRepositoryPort,
};

#[derive(Clone, Debug)]
pub struct GitRepository {
    pub root: PathBuf,
    pub origin: GitOrigin,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
}

impl GitRepository {
    pub fn from_path(path: &str) -> Result<Self> {
        let path = normalize_path(&expand_tilde(path))?;
        let root = run_git(&path, ["rev-parse", "--show-toplevel"])?;
        let root = normalize_path(Path::new(root.trim()))?;
        let raw_origin = run_git(&root, ["remote", "get-url", "origin"])?;
        let origin = parse_github_origin(raw_origin.trim())?;
        let branch = run_git(&root, ["branch", "--show-current"])
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let head_sha = run_git(&root, ["rev-parse", "HEAD"])
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Ok(Self {
            root,
            origin,
            branch,
            head_sha,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct LocalGitRepository;

impl GitRepositoryPort for LocalGitRepository {
    fn status(&self, workdir: &Path) -> Result<WorkspaceGitStatus> {
        git_status(workdir)
    }

    fn diff_summary(&self, workdir: &Path) -> Result<WorkspaceDiffSummary> {
        let status = git_status(workdir)?;
        let diff_stat = run_git_args(Path::new(&status.root), &["diff", "--stat", "HEAD", "--"])?;
        Ok(WorkspaceDiffSummary { status, diff_stat })
    }

    fn commit(
        &self,
        workdir: &Path,
        message: &str,
        files: &[String],
    ) -> Result<WorkspaceCommitResult> {
        let status = git_status(workdir)?;
        let root = Path::new(&status.root);
        let message = message.trim();
        if message.is_empty() {
            bail!("commit message is required");
        }
        if files.is_empty() {
            bail!("at least one file must be selected for commit");
        }
        let clean_files = files
            .iter()
            .map(|file| file.trim())
            .filter(|file| !file.is_empty())
            .collect::<Vec<_>>();
        if clean_files.is_empty() {
            bail!("at least one file must be selected for commit");
        }
        let add_files = dedupe_selected_paths(&clean_files);

        let mut add_args = vec!["add", "-A", "--"];
        add_args.extend(add_files.iter().map(String::as_str));
        run_git_args(root, &add_args)?;
        run_git_args(root, &["commit", "-m", message])?;
        let commit_sha = run_git_args(root, &["rev-parse", "HEAD"])?
            .trim()
            .to_string();
        Ok(WorkspaceCommitResult {
            commit_sha,
            status: git_status(root)?,
        })
    }

    fn push(
        &self,
        workdir: &Path,
        remote: &str,
        branch: &str,
        set_upstream: bool,
    ) -> Result<WorkspacePushResult> {
        let status = git_status(workdir)?;
        let root = Path::new(&status.root);
        let remote = remote.trim();
        let branch = branch.trim();
        if remote.is_empty() {
            bail!("remote is required");
        }
        if branch.is_empty() {
            bail!("branch is required");
        }

        let mut args = vec!["push"];
        if set_upstream {
            args.push("-u");
        }
        args.extend([remote, branch]);
        run_git_args(root, &args)?;
        Ok(WorkspacePushResult {
            remote: remote.to_string(),
            branch: branch.to_string(),
        })
    }

    fn create_worktree(
        &self,
        source_workdir: &Path,
        branch_name: &str,
        worktree_path: &Path,
    ) -> Result<WorkspaceGitStatus> {
        let path_arg = worktree_path.to_string_lossy().to_string();
        run_git_args(
            source_workdir,
            &["worktree", "add", "-b", branch_name, &path_arg, "HEAD"],
        )?;
        git_status(worktree_path)
    }

    fn remove_worktree(&self, worktree_path: &Path, branch_name: Option<&str>) -> Result<()> {
        let common_dir = run_git_args(worktree_path, &["rev-parse", "--git-common-dir"])?;
        let common_dir = normalize_git_dir(worktree_path, common_dir.trim())?;
        let path_arg = worktree_path.to_string_lossy().to_string();
        run_git_dir_args(&common_dir, &["worktree", "remove", "--force", &path_arg])?;
        if let Some(branch_name) = branch_name.map(str::trim).filter(|value| !value.is_empty()) {
            run_git_dir_args(&common_dir, &["branch", "-D", branch_name])?;
        }
        Ok(())
    }
}

pub fn parse_github_origin(raw_url: &str) -> Result<GitOrigin> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        bail!("git origin is empty");
    }

    let without_suffix = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    let path = if let Some(rest) = without_suffix.strip_prefix("git@github.com:") {
        rest
    } else if let Some(rest) = without_suffix.strip_prefix("ssh://git@github.com/") {
        rest
    } else if let Some(rest) = without_suffix.strip_prefix("https://github.com/") {
        rest
    } else if let Some(rest) = without_suffix.strip_prefix("http://github.com/") {
        rest
    } else {
        bail!("only GitHub origin URLs are supported: {trimmed}");
    };

    let mut parts = path.split('/').filter(|part| !part.is_empty());
    let owner = parts
        .next()
        .ok_or_else(|| anyhow!("GitHub origin URL is missing owner: {trimmed}"))?;
    let repo = parts
        .next()
        .ok_or_else(|| anyhow!("GitHub origin URL is missing repo: {trimmed}"))?;
    if parts.next().is_some() {
        bail!("GitHub origin URL has unexpected path segments: {trimmed}");
    }

    Ok(GitOrigin {
        raw_url: trimmed.to_string(),
        canonical_url: format!("github.com/{owner}/{repo}"),
        host: "github.com".to_string(),
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

fn git_status(workdir: &Path) -> Result<WorkspaceGitStatus> {
    let root = run_git_args(workdir, &["rev-parse", "--show-toplevel"])?;
    let root = normalize_path(Path::new(root.trim()))?;
    let branch = run_git_args(&root, &["branch", "--show-current"])
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let head_sha = run_git_args(&root, &["rev-parse", "HEAD"])
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let porcelain = run_git_bytes(
        &root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--find-renames",
        ],
    )?;
    let files = parse_porcelain_status_z(&porcelain);

    Ok(WorkspaceGitStatus {
        root: root.to_string_lossy().to_string(),
        branch,
        head_sha,
        is_dirty: !files.is_empty(),
        files,
    })
}

fn parse_porcelain_status_z(output: &[u8]) -> Vec<WorkspaceGitFileStatus> {
    let mut files = Vec::new();
    let mut entries = output
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty());

    while let Some(entry) = entries.next() {
        if entry.len() < 4 {
            continue;
        }
        let status_code = String::from_utf8_lossy(&entry[..2]).into_owned();
        let path = decode_status_path(&entry[3..]);
        if path.is_empty() {
            continue;
        }
        let is_rename_or_copy = status_code.contains('R') || status_code.contains('C');
        let previous_path = if is_rename_or_copy {
            entries
                .next()
                .map(decode_status_path)
                .filter(|path| !path.is_empty())
        } else {
            None
        };
        files.push(WorkspaceGitFileStatus {
            status_label: status_label(&status_code).to_string(),
            status_code,
            path,
            previous_path,
        });
    }

    files
}

fn decode_status_path(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw).into_owned()
}

fn dedupe_selected_paths(selected_files: &[&str]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut paths = Vec::new();

    for selected in selected_files {
        let selected = selected.trim();
        if selected.is_empty() {
            continue;
        }
        push_unique_path(&mut paths, &mut seen, selected);
    }

    paths
}

fn push_unique_path(paths: &mut Vec<String>, seen: &mut HashSet<String>, path: &str) {
    if seen.insert(path.to_string()) {
        paths.push(path.to_string());
    }
}

fn status_label(status_code: &str) -> &'static str {
    if status_code == "??" {
        return "untracked";
    }
    if status_code.contains('A') {
        return "added";
    }
    if status_code.contains('D') {
        return "deleted";
    }
    if status_code.contains('R') {
        return "renamed";
    }
    if status_code.contains('C') {
        return "copied";
    }
    if status_code.contains('U') {
        return "conflicted";
    }
    if status_code.contains('M') {
        return "modified";
    }
    "changed"
}

fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) -> Result<String> {
    run_git_args(cwd, &args)
}

fn run_git_args(cwd: &Path, args: &[&str]) -> Result<String> {
    Ok(String::from_utf8(run_git_bytes(cwd, args)?)?)
}

fn run_git_bytes(cwd: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git command failed: {}", stderr.trim());
    }
    Ok(output.stdout)
}

fn run_git_dir_args(git_dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(git_dir)
        .args(args)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git command failed: {}", stderr.trim());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn normalize_git_dir(workdir: &Path, raw: &str) -> Result<PathBuf> {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        Ok(path)
    } else {
        normalize_path(&workdir.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalGitRepository, parse_github_origin, parse_porcelain_status_z};
    use crate::ports::git_repository::GitRepositoryPort;
    use std::{fs, path::Path, process::Command};

    #[test]
    fn parses_github_ssh_origin() {
        let origin = parse_github_origin("git@github.com:org/repo.git").unwrap();
        assert_eq!(origin.canonical_url, "github.com/org/repo");
    }

    #[test]
    fn parses_github_https_origin() {
        let origin = parse_github_origin("https://github.com/org/repo").unwrap();
        assert_eq!(origin.canonical_url, "github.com/org/repo");
    }

    #[test]
    fn rejects_non_github_origin() {
        let err = parse_github_origin("https://gitlab.com/org/repo.git").unwrap_err();
        assert!(err.to_string().contains("only GitHub"));
    }

    #[test]
    fn parses_nul_delimited_porcelain_status_entries() {
        let files = parse_porcelain_status_z(
            b" M src/main.rs\0A  README.md\0?? scratch \"file\".txt\0R  new name.rs\0old name.rs\0",
        );

        assert_eq!(files.len(), 4);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[0].status_code, " M");
        assert_eq!(files[0].status_label, "modified");
        assert_eq!(files[1].status_label, "added");
        assert_eq!(files[2].status_label, "untracked");
        assert_eq!(files[2].path, "scratch \"file\".txt");
        assert_eq!(files[3].status_label, "renamed");
        assert_eq!(files[3].path, "new name.rs");
        assert_eq!(files[3].previous_path.as_deref(), Some("old name.rs"));
    }

    #[test]
    fn commits_renamed_path_with_spaces_from_status() {
        let repo = create_temp_repo();
        fs::write(repo.join("old name.rs"), "fn old() {}\n").unwrap();
        run_git_test(&repo, &["add", "--", "old name.rs"]);
        run_git_test(&repo, &["commit", "-m", "initial"]);
        run_git_test(&repo, &["mv", "old name.rs", "new name.rs"]);

        let status = LocalGitRepository.status(&repo).unwrap();
        assert_eq!(status.files.len(), 1);
        assert_eq!(status.files[0].status_label, "renamed");
        assert_eq!(status.files[0].path, "new name.rs");
        assert_eq!(
            status.files[0].previous_path.as_deref(),
            Some("old name.rs")
        );

        LocalGitRepository
            .commit(&repo, "rename file", &[status.files[0].path.clone()])
            .unwrap();
        assert!(LocalGitRepository.status(&repo).unwrap().files.is_empty());
    }

    fn create_temp_repo() -> std::path::PathBuf {
        let repo =
            std::env::temp_dir().join(format!("acp-status-{}", uuid::Uuid::new_v4().simple()));
        fs::create_dir_all(&repo).unwrap();
        run_git_test(&repo, &["init"]);
        run_git_test(&repo, &["config", "user.email", "test@example.com"]);
        run_git_test(&repo, &["config", "user.name", "Test User"]);
        repo
    }

    fn run_git_test(cwd: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
