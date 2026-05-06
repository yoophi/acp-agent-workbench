use crate::{domain::agent::AgentDescriptor, ports::agent_catalog::AgentCatalog};

pub struct ListAgentsUseCase<C>
where
    C: AgentCatalog,
{
    catalog: C,
}

impl<C> ListAgentsUseCase<C>
where
    C: AgentCatalog,
{
    pub fn new(catalog: C) -> Self {
        Self { catalog }
    }

    pub fn execute(&self) -> Vec<AgentDescriptor> {
        self.catalog.list_agents()
    }
}
