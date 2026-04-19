use anyhow::{Result, anyhow, bail};
use serde_json::Value;
use std::{path::Path, process::Command};

use crate::{
    domain::git::{
        GitHubPullRequestContext, GitHubPullRequestContextRequest, GitHubPullRequestCreateRequest,
        GitHubPullRequestSummary, WorkspaceGitStatus,
    },
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

    fn load_pull_request_context(
        &self,
        workdir: &Path,
        request: &GitHubPullRequestContextRequest,
    ) -> Result<GitHubPullRequestContext> {
        if request.number == 0 {
            bail!("pull request number is required");
        }
        let number = request.number.to_string();
        let view = run_gh(
            workdir,
            &[
                "pr",
                "view",
                &number,
                "--json",
                "number,title,body,author,baseRefName,headRefName,headRefOid,url",
            ],
        )?;
        let value: Value = serde_json::from_str(&view)?;
        let changed_files = run_gh(workdir, &["pr", "diff", &number, "--name-only"])?
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect();
        let diff = run_gh(workdir, &["pr", "diff", &number, "--patch"])?;

        Ok(GitHubPullRequestContext {
            number: value
                .get("number")
                .and_then(Value::as_u64)
                .unwrap_or(request.number),
            url: string_field(&value, "url")?,
            title: string_field(&value, "title")?,
            body: optional_string_field(&value, "body"),
            author: value
                .get("author")
                .and_then(|author| author.get("login"))
                .and_then(Value::as_str)
                .map(ToString::to_string),
            base_ref: string_field(&value, "baseRefName")?,
            head_ref: string_field(&value, "headRefName")?,
            head_sha: string_field(&value, "headRefOid")?,
            changed_files,
            diff,
        })
    }
}

fn parse_pr_number(url: &str) -> Option<u64> {
    url.trim_end_matches('/').rsplit('/').next()?.parse().ok()
}

fn run_gh(workdir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("gh")
        .args(args)
        .current_dir(workdir)
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn string_field(value: &Value, field: &str) -> Result<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| anyhow!("gh response missing field: {field}"))
}

fn optional_string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .filter(|value| !value.is_empty())
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
