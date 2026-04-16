use anyhow::Result;

use crate::{
    adapters::acp::runner::AcpAgentRunner,
    domain::run::AgentRunRequest,
    ports::{
        agent_catalog::AgentCatalog, event_sink::RunEventSink, permission::PermissionDecisionPort,
        runner::AgentRunner,
    },
};

pub struct RunAgentUseCase<C, P>
where
    C: AgentCatalog,
    P: PermissionDecisionPort,
{
    runner: AcpAgentRunner<C, P>,
}

impl<C, P> RunAgentUseCase<C, P>
where
    C: AgentCatalog,
    P: PermissionDecisionPort,
{
    pub fn new(catalog: C, permissions: P) -> Self {
        Self {
            runner: AcpAgentRunner::new(catalog, permissions),
        }
    }

    pub async fn execute_with_run_id<S>(
        &self,
        request: AgentRunRequest,
        run_id: String,
        sink: S,
    ) -> Result<()>
    where
        S: RunEventSink,
    {
        self.runner.run(request, run_id, sink).await
    }
}
