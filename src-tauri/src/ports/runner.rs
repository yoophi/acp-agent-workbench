use anyhow::Result;

use crate::{domain::run::AgentRunRequest, ports::event_sink::RunEventSink};

pub trait AgentRunner {
    async fn run<S>(&self, request: AgentRunRequest, run_id: String, sink: S) -> Result<()>
    where
        S: RunEventSink;
}
