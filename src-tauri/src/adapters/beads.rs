use anyhow::{Result, anyhow, bail};
use serde_json::Value;
use std::{
    io::ErrorKind,
    path::Path,
    process::{Command, Output},
};

use crate::{
    domain::local_task::{LocalTaskStatus, LocalTaskSummary},
    ports::local_task_source::LocalTaskSource,
};

#[derive(Clone)]
pub struct BeadsCliTaskSource;

impl LocalTaskSource for BeadsCliTaskSource {
    fn has_task_data(&self, workdir: &Path) -> bool {
        workdir.join(".beads").is_dir()
    }

    fn list_tasks(&self, workdir: &Path) -> Result<Vec<LocalTaskSummary>> {
        let output = run_bd(workdir, ["list", "--json"])?;
        parse_beads_list_output(output)
    }

    fn update_status(
        &self,
        workdir: &Path,
        task_id: &str,
        status: LocalTaskStatus,
    ) -> Result<LocalTaskSummary> {
        let output = run_bd(
            workdir,
            [
                "update",
                task_id,
                "--status",
                status.as_beads_status(),
                "--json",
            ],
        )?;
        parse_beads_update_output(output)
    }
}

fn run_bd<const N: usize>(workdir: &Path, args: [&str; N]) -> Result<Output> {
    Command::new("bd")
        .args(args)
        .current_dir(workdir)
        .output()
        .map_err(|err| {
            if err.kind() == ErrorKind::NotFound {
                anyhow!("beads CLI not found; install `bd` to manage local tasks")
            } else {
                anyhow!("failed to run beads CLI: {err}")
            }
        })
}

fn parse_beads_list_output(output: Output) -> Result<Vec<LocalTaskSummary>> {
    let value = parse_beads_json_output(output)?;
    parse_beads_tasks(&value)
}

fn parse_beads_update_output(output: Output) -> Result<LocalTaskSummary> {
    let value = parse_beads_json_output(output)?;
    if let Some(task) = parse_beads_task(&value) {
        return Ok(task);
    }
    for key in ["issue", "task"] {
        if let Some(task) = value.get(key).and_then(parse_beads_task) {
            return Ok(task);
        }
    }
    bail!("beads JSON output did not contain an updated task");
}

fn parse_beads_json_output(output: Output) -> Result<Value> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        if message.is_empty() {
            bail!("beads CLI exited with status {}", output.status);
        }
        bail!("beads CLI failed: {message}");
    }

    serde_json::from_slice(&output.stdout)
        .map_err(|err| anyhow!("failed to parse beads JSON output: {err}"))
}

fn parse_beads_tasks(value: &Value) -> Result<Vec<LocalTaskSummary>> {
    let items = if let Some(items) = value.as_array() {
        items
    } else if let Some(items) = value.get("issues").and_then(Value::as_array) {
        items
    } else if let Some(items) = value.get("tasks").and_then(Value::as_array) {
        items
    } else {
        bail!("beads JSON output did not contain a task array");
    };

    let mut tasks = Vec::with_capacity(items.len());
    for item in items {
        if let Some(task) = parse_beads_task(item) {
            tasks.push(task);
        }
    }
    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(tasks)
}

fn parse_beads_task(value: &Value) -> Option<LocalTaskSummary> {
    let id = first_string(value, &["id", "issue_id", "issueId"])?;
    let title = first_string(value, &["title", "summary", "name"]).unwrap_or_else(|| id.clone());
    let description = first_string(value, &["description", "body", "content"]);
    let status = first_string(value, &["status", "state"]);
    let priority = first_string_or_number(value, &["priority", "priority_label", "priorityLabel"]);
    let labels = first_string_array(value, &["labels", "tags"]);
    let dependencies =
        first_string_array(value, &["dependencies", "deps", "depends_on", "dependsOn"]);
    let blocked = first_bool(value, &["blocked", "is_blocked", "isBlocked"])
        .unwrap_or_else(|| !dependencies.is_empty() && status.as_deref() != Some("closed"));
    let acceptance_criteria = first_string(
        value,
        &["acceptance_criteria", "acceptanceCriteria", "criteria"],
    );

    Some(LocalTaskSummary {
        id,
        title,
        description,
        status,
        priority,
        labels,
        dependencies,
        blocked,
        acceptance_criteria,
    })
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn first_string_or_number(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        let value = value.get(*key)?;
        if let Some(value) = value
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(value.to_owned())
        } else if let Some(value) = value.as_i64() {
            Some(value.to_string())
        } else {
            value.as_u64().map(|value| value.to_string())
        }
    })
}

fn first_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_bool))
}

fn first_string_array(value: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(string_array))
        .unwrap_or_default()
}

fn string_array(value: &Value) -> Option<Vec<String>> {
    let values = value.as_array()?;
    Some(
        values
            .iter()
            .filter_map(|item| {
                if let Some(value) = item.as_str() {
                    Some(value.trim().to_owned())
                } else {
                    first_string(item, &["id", "issue_id", "issueId"])
                }
            })
            .filter(|value| !value.is_empty())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::{parse_beads_tasks, parse_beads_update_output};
    use serde_json::json;
    use std::{os::unix::process::ExitStatusExt, process::Output};

    #[test]
    fn parses_array_output_from_bd_list_json() {
        let tasks = parse_beads_tasks(&json!([
            {
                "id": "bd-2",
                "title": "Run ACP from task",
                "description": "Compose prompt",
                "status": "open",
                "priority": 1,
                "labels": ["backend", "agent"],
                "dependencies": ["bd-1"],
                "blocked": true,
                "acceptance_criteria": "Prompt includes task fields"
            },
            {
                "id": "bd-1",
                "title": "List tasks",
                "status": "closed"
            }
        ]))
        .unwrap();

        assert_eq!(tasks[0].id, "bd-1");
        assert_eq!(tasks[1].priority.as_deref(), Some("1"));
        assert_eq!(tasks[1].labels, vec!["backend", "agent"]);
        assert_eq!(tasks[1].dependencies, vec!["bd-1"]);
        assert!(tasks[1].blocked);
        assert_eq!(
            tasks[1].acceptance_criteria.as_deref(),
            Some("Prompt includes task fields")
        );
    }

    #[test]
    fn parses_wrapped_output_and_dependency_objects() {
        let tasks = parse_beads_tasks(&json!({
            "issues": [
                {
                    "issueId": "bd-3",
                    "summary": "Inspect details",
                    "state": "open",
                    "priorityLabel": "P2",
                    "tags": ["ui"],
                    "dependsOn": [{ "id": "bd-2" }]
                }
            ]
        }))
        .unwrap();

        assert_eq!(tasks[0].id, "bd-3");
        assert_eq!(tasks[0].title, "Inspect details");
        assert_eq!(tasks[0].priority.as_deref(), Some("P2"));
        assert_eq!(tasks[0].dependencies, vec!["bd-2"]);
        assert!(tasks[0].blocked);
    }

    #[test]
    fn parses_updated_issue_output() {
        let task = parse_beads_update_output(Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: br#"{"issue":{"id":"bd-4","title":"Update status","status":"in_progress"}}"#
                .to_vec(),
            stderr: vec![],
        })
        .unwrap();

        assert_eq!(task.id, "bd-4");
        assert_eq!(task.status.as_deref(), Some("in_progress"));
    }
}
