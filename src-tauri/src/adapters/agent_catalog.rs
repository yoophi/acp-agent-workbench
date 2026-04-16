use crate::{domain::agent::AgentDescriptor, ports::agent_catalog::AgentCatalog};

#[derive(Clone, Default)]
pub struct StaticAgentCatalog;

impl AgentCatalog for StaticAgentCatalog {
    fn list_agents(&self) -> Vec<AgentDescriptor> {
        // TODO: Replace this static catalog with a configurable provider once agent definitions
        // move out of the reference CLI defaults.
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
