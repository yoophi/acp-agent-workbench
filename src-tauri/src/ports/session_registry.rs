use anyhow::Result;
use std::{future::Future, sync::Arc};
use tokio::task::JoinHandle;

/// Storage for in-flight agent runs and their sessions.
///
/// The registry enforces identity/lifecycle invariants (unique run ids,
/// optional concurrency limit, cancellation) and owns the handles that allow
/// aborting a run. It is intentionally generic over the concrete session
/// type so the application layer can depend on this port without pulling in
/// adapter details.
pub trait SessionRegistry: Clone + Send + Sync + 'static {
    type Session: Send + Sync + 'static;

    fn reserve_run(&self, run_id: String) -> impl Future<Output = Result<()>> + Send;

    fn attach_run_handle(
        &self,
        run_id: &str,
        handle: JoinHandle<()>,
    ) -> impl Future<Output = Result<()>> + Send;

    fn attach_session(
        &self,
        run_id: &str,
        session: Arc<Self::Session>,
    ) -> impl Future<Output = Result<()>> + Send;

    fn active_session(
        &self,
        run_id: &str,
    ) -> impl Future<Output = Option<Arc<Self::Session>>> + Send;

    fn finish_run(&self, run_id: &str) -> impl Future<Output = ()> + Send;

    fn cancel_run(&self, run_id: &str) -> impl Future<Output = bool> + Send;

    fn active_run_count(&self) -> impl Future<Output = usize> + Send;
}
