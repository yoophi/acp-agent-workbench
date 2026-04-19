use anyhow::{Result, anyhow, bail};
use std::{
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

        let mut add_args = vec!["add", "--"];
        add_args.extend(clean_files.iter().copied());
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
    let porcelain = run_git_args(
        &root,
        &["status", "--porcelain=v1", "--untracked-files=all"],
    )?;
    let files = parse_porcelain_status(&porcelain);

    Ok(WorkspaceGitStatus {
        root: root.to_string_lossy().to_string(),
        branch,
        head_sha,
        is_dirty: !files.is_empty(),
        files,
    })
}

fn parse_porcelain_status(output: &str) -> Vec<WorkspaceGitFileStatus> {
    output
        .lines()
        .filter_map(|line| {
            if line.len() < 4 {
                return None;
            }
            let status_code: String = line.chars().take(2).collect();
            let path = line.get(3..)?.trim().to_string();
            if path.is_empty() {
                return None;
            }
            Some(WorkspaceGitFileStatus {
                status_label: status_label(&status_code).to_string(),
                status_code,
                path,
            })
        })
        .collect()
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
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git command failed: {}", stderr.trim());
    }
    Ok(String::from_utf8(output.stdout)?)
}

#[cfg(test)]
mod tests {
    use super::{parse_github_origin, parse_porcelain_status};

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
    fn parses_porcelain_status_lines() {
        let files = parse_porcelain_status(
            " M src/main.rs\nA  README.md\n?? scratch.txt\nR  old.rs -> new.rs\n",
        );

        assert_eq!(files.len(), 4);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[0].status_code, " M");
        assert_eq!(files[0].status_label, "modified");
        assert_eq!(files[1].status_label, "added");
        assert_eq!(files[2].status_label, "untracked");
        assert_eq!(files[3].status_label, "renamed");
    }
}
