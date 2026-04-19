use anyhow::{Result, anyhow, bail};
use std::{path::Path, process::Command};

use crate::{
    domain::git::{GitHubPullRequestCreateRequest, GitHubPullRequestSummary, WorkspaceGitStatus},
    ports::github_pull_request::GitHubPullRequestPort,
};

#[derive(Clone, Debug, Default)]
pub struct GhCliPullRequestClient;

impl GitHubPullRequestPort for GhCliPullRequestClient {
    fn create_pull_request(
        &self,
        workdir: &Path,
        status: &WorkspaceGitStatus,
        request: &GitHubPullRequestCreateRequest,
    ) -> Result<GitHubPullRequestSummary> {
        let base = request.base.trim();
        let title = request.title.trim();
        if base.is_empty() {
            bail!("base branch is required");
        }
        if title.is_empty() {
            bail!("pull request title is required");
        }
        let head = request
            .head
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or(status.branch.as_deref())
            .ok_or_else(|| anyhow!("head branch is required"))?;

        let mut args = vec![
            "pr",
            "create",
            "--base",
            base,
            "--head",
            head,
            "--title",
            title,
            "--body",
            request.body.as_str(),
        ];
        if request.draft {
            args.push("--draft");
        }

        let output = Command::new("gh")
            .args(&args)
            .current_dir(workdir)
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("gh pr create failed: {}", stderr.trim());
        }

        let url = String::from_utf8(output.stdout)?.trim().to_string();
        if url.is_empty() {
            bail!("gh pr create did not return a pull request URL");
        }

        Ok(GitHubPullRequestSummary {
            number: parse_pr_number(&url),
            url,
            title: title.to_string(),
            base_ref: base.to_string(),
            head_ref: head.to_string(),
        })
    }
}

fn parse_pr_number(url: &str) -> Option<u64> {
    url.trim_end_matches('/').rsplit('/').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::parse_pr_number;

    #[test]
    fn parses_pull_request_number_from_url() {
        assert_eq!(
            parse_pr_number("https://github.com/org/repo/pull/123"),
            Some(123)
        );
    }
}
