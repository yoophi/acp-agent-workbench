use anyhow::{Result, anyhow, bail};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    adapters::acp::util::{expand_tilde, normalize_path},
    domain::workspace::GitOrigin,
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

#[cfg(test)]
mod tests {
    use super::parse_github_origin;

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
}
