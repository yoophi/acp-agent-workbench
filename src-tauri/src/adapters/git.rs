use anyhow::{Result, anyhow, bail};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    adapters::acp::util::{expand_tilde, normalize_path},
    domain::{
        workspace::{GitOrigin, WorkspaceCheckout},
        workspace_git::{WorkspaceDiffSummary, WorkspaceGitFileStatus, WorkspaceGitStatus},
    },
    ports::workspace_git::WorkspaceGitInspector,
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

#[derive(Clone, Default)]
pub struct GitWorkspaceInspector;

impl WorkspaceGitInspector for GitWorkspaceInspector {
    async fn status(
        &self,
        workspace_id: String,
        checkout: WorkspaceCheckout,
    ) -> Result<WorkspaceGitStatus> {
        let repo = GitRepository::from_path(&checkout.path.to_string_lossy())?;
        let files = status_files(&repo.root)?;
        Ok(WorkspaceGitStatus {
            workspace_id,
            checkout_id: checkout.id,
            path: repo.root.to_string_lossy().to_string(),
            branch: repo.branch,
            head_sha: repo.head_sha,
            is_clean: files.is_empty(),
            files,
        })
    }

    async fn diff_summary(
        &self,
        workspace_id: String,
        checkout: WorkspaceCheckout,
    ) -> Result<WorkspaceDiffSummary> {
        let repo = GitRepository::from_path(&checkout.path.to_string_lossy())?;
        let files = status_files(&repo.root)?;
        Ok(WorkspaceDiffSummary {
            workspace_id,
            checkout_id: checkout.id,
            path: repo.root.to_string_lossy().to_string(),
            branch: repo.branch,
            head_sha: repo.head_sha,
            staged_stat: run_git_optional(&repo.root, ["diff", "--cached", "--stat"])?,
            unstaged_stat: run_git_optional(&repo.root, ["diff", "--stat"])?,
            untracked_files: files
                .into_iter()
                .filter(|file| file.index_status == "?" && file.worktree_status == "?")
                .map(|file| file.path)
                .collect(),
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

fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) -> Result<String> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git command failed: {}", stderr.trim());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn run_git_optional<const N: usize>(cwd: &Path, args: [&str; N]) -> Result<String> {
    Ok(run_git(cwd, args)
        .map(|value| value.trim().to_string())
        .unwrap_or_default())
}

fn status_files(root: &Path) -> Result<Vec<WorkspaceGitFileStatus>> {
    let raw = run_git(root, ["status", "--porcelain=v1"])?;
    Ok(raw
        .lines()
        .filter_map(parse_status_line)
        .collect::<Vec<_>>())
}

fn parse_status_line(line: &str) -> Option<WorkspaceGitFileStatus> {
    if line.len() < 4 {
        return None;
    }
    let mut chars = line.chars();
    let index_status = chars.next()?.to_string();
    let worktree_status = chars.next()?.to_string();
    let path = line.get(3..)?.trim();
    let path = path
        .rsplit_once(" -> ")
        .map(|(_, renamed)| renamed)
        .unwrap_or(path)
        .trim_matches('"')
        .to_string();
    Some(WorkspaceGitFileStatus {
        path,
        index_status,
        worktree_status,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_github_origin, parse_status_line};

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
    fn parses_porcelain_status_line() {
        let file = parse_status_line(" M src/main.rs").unwrap();
        assert_eq!(file.index_status, " ");
        assert_eq!(file.worktree_status, "M");
        assert_eq!(file.path, "src/main.rs");
    }

    #[test]
    fn parses_porcelain_rename_line() {
        let file = parse_status_line("R  old.rs -> new.rs").unwrap();
        assert_eq!(file.index_status, "R");
        assert_eq!(file.worktree_status, " ");
        assert_eq!(file.path, "new.rs");
    }
}
