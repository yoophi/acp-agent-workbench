use std::{env, fs, path::PathBuf};

use crate::{domain::agent::AgentDescriptor, ports::agent_catalog::AgentCatalog};

const AGENT_CATALOG_PATH_ENV: &str = "ACP_AGENT_CATALOG_PATH";

#[derive(Clone, Default)]
pub struct ConfigurableAgentCatalog {
    file_path: Option<PathBuf>,
    fallback: StaticAgentCatalog,
}

impl ConfigurableAgentCatalog {
    pub fn from_env() -> Self {
        Self {
            file_path: env::var_os(AGENT_CATALOG_PATH_ENV).map(PathBuf::from),
            fallback: StaticAgentCatalog,
        }
    }

    #[cfg(test)]
    fn from_path(file_path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: Some(file_path.into()),
            fallback: StaticAgentCatalog,
        }
    }
}

impl AgentCatalog for ConfigurableAgentCatalog {
    fn list_agents(&self) -> Vec<AgentDescriptor> {
        self.file_path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|content| serde_json::from_str::<Vec<AgentDescriptor>>(&content).ok())
            .filter(|agents| !agents.is_empty())
            .unwrap_or_else(|| self.fallback.list_agents())
    }
}

#[derive(Clone, Default)]
pub struct StaticAgentCatalog;

impl AgentCatalog for StaticAgentCatalog {
    fn list_agents(&self) -> Vec<AgentDescriptor> {
        vec![
            AgentDescriptor {
                id: "claude-code".into(),
                label: "Claude Code".into(),
                command: "npx -y @agentclientprotocol/claude-agent-acp".into(),
            },
            AgentDescriptor {
                id: "codex".into(),
                label: "Codex".into(),
                command: "npx -y @zed-industries/codex-acp".into(),
            },
            AgentDescriptor {
                id: "opencode".into(),
                label: "OpenCode".into(),
                command: "npx -y opencode-ai acp".into(),
            },
            AgentDescriptor {
                id: "pi".into(),
                label: "Pi".into(),
                command: "npx -y pi-acp".into(),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigurableAgentCatalog, StaticAgentCatalog};
    use crate::ports::agent_catalog::AgentCatalog;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn reads_agents_from_json_catalog_file() {
        let path = temp_catalog_path();
        fs::write(
            &path,
            r#"[{"id":"local","label":"Local Agent","command":"local-agent acp"}]"#,
        )
        .expect("write catalog");

        let agents = ConfigurableAgentCatalog::from_path(&path).list_agents();

        fs::remove_file(&path).ok();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, "local");
        assert_eq!(agents[0].command, "local-agent acp");
    }

    #[test]
    fn falls_back_to_default_agents_when_catalog_file_is_invalid() {
        let path = temp_catalog_path();
        fs::write(&path, "not json").expect("write catalog");

        let agents = ConfigurableAgentCatalog::from_path(&path).list_agents();

        fs::remove_file(&path).ok();
        assert_eq!(agents.len(), StaticAgentCatalog.list_agents().len());
        assert!(agents.iter().any(|agent| agent.id == "codex"));
    }

    fn temp_catalog_path() -> PathBuf {
        std::env::temp_dir().join(format!("acp-agent-catalog-{}.json", Uuid::new_v4()))
    }
}
