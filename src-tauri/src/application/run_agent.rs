use anyhow::Result;

use crate::{
    domain::run::AgentRunRequest,
    ports::{event_sink::RunEventSink, runner::AgentRunner},
};

pub struct RunAgentUseCase<R>
where
    R: AgentRunner,
{
    runner: R,
}

impl<R> RunAgentUseCase<R>
where
    R: AgentRunner,
{
    pub fn new(runner: R) -> Self {
        Self { runner }
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
