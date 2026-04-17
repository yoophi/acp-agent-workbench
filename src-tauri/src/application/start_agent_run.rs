use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::Result;

use crate::{
    domain::{
        events::RunEvent,
        run::{AgentRun, AgentRunRequest},
    },
    ports::{event_sink::RunEventSink, session_registry::SessionRegistry},
};

/// Future that drives a launched session to its natural completion
/// (initial prompt submission, process wait, stream cleanup, terminal
/// lifecycle event).
pub type DriverFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Future that tears a launched session down without running it to
/// completion (used when the orchestrator decides to stop the run
/// before the driver is awaited).
pub type AbortFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Adapter-facing controller over a freshly launched agent session.
///
/// Exactly one of `run_to_completion` or `abort` is awaited per run,
/// and both consume the commander by value.
pub trait RunCommander: Send + 'static {
    fn run_to_completion(self: Box<Self>) -> DriverFuture;
    fn abort(self: Box<Self>) -> AbortFuture;
}

/// Outcome of a successful launcher call. The session handle is shared
/// with the registry; the commander owns the process/tasks that back it.
pub struct LaunchedSession<Session>
where
    Session: Send + Sync + 'static,
{
    pub session: Arc<Session>,
    pub commander: Box<dyn RunCommander>,
}

/// Start a new agent run.
///
/// The caller supplies a launcher closure that is responsible for all
/// adapter-level setup (spawning the ACP subprocess, opening the session,
/// etc.). This use case owns the registry bookkeeping and the error/
/// cleanup flow around the launcher, so Tauri command handlers stay thin.
pub struct StartAgentRunUseCase<R>
where
    R: SessionRegistry,
{
    registry: R,
}

impl<R> StartAgentRunUseCase<R>
where
    R: SessionRegistry,
{
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub async fn execute<S, F, Fut>(
        self,
        sink: S,
        request: AgentRunRequest,
        launch: F,
    ) -> Result<AgentRun, String>
    where
        S: RunEventSink,
        F: FnOnce(AgentRunRequest, String, S) -> Fut + Send + 'static,
        Fut: Future<Output = Result<LaunchedSession<R::Session>>> + Send + 'static,
    {
        let run = build_run(&request);
        self.registry
            .reserve_run(run.id.clone())
            .await
            .map_err(|err| err.to_string())?;

        let registry = self.registry.clone();
        let run_id = run.id.clone();
        let sink_for_task = sink.clone();

        let handle = tokio::spawn(async move {
            let launched = match launch(request, run_id.clone(), sink_for_task.clone()).await {
                Ok(launched) => launched,
                Err(err) => {
                    sink_for_task.emit(
                        &run_id,
                        RunEvent::Error {
                            message: err.to_string(),
                        },
                    );
                    registry.finish_run(&run_id).await;
                    return;
                }
            };
            let LaunchedSession { session, commander } = launched;

            if let Err(err) = registry.attach_session(&run_id, session).await {
                sink_for_task.emit(
                    &run_id,
                    RunEvent::Diagnostic {
                        message: err.to_string(),
                    },
                );
                commander.abort().await;
                registry.finish_run(&run_id).await;
                return;
            }

            commander.run_to_completion().await;
            registry.finish_run(&run_id).await;
        });

        self.registry
            .attach_run_handle(&run.id, handle)
            .await
            .map_err(|err| err.to_string())?;

        Ok(run)
    }
}

fn build_run(request: &AgentRunRequest) -> AgentRun {
    match request.run_id.clone() {
        Some(id) if !id.trim().is_empty() => {
            AgentRun::with_id(id, request.goal.clone(), request.agent_id.clone())
        }
        _ => AgentRun::new(request.goal.clone(), request.agent_id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::{LifecycleStatus, RunEvent};
    use crate::ports::session_registry::SessionRegistry;
    use anyhow::anyhow;
    use std::{
        collections::HashMap,
        sync::{
            Arc, Mutex as StdMutex,
            atomic::{AtomicUsize, Ordering},
        },
    };
    use tokio::sync::{Mutex, Notify};
    use tokio::task::JoinHandle;

    struct FakeSession;

    #[derive(Clone, Default)]
    struct FakeRegistry {
        inner: Arc<Mutex<FakeRegistryState>>,
    }

    #[derive(Default)]
    struct FakeRegistryState {
        reserved: Vec<String>,
        finished: Vec<String>,
        sessions: HashMap<String, Arc<FakeSession>>,
        handles: HashMap<String, JoinHandle<()>>,
        fail_attach_session: bool,
    }

    impl FakeRegistry {
        async fn with_failing_attach() -> Self {
            let reg = Self::default();
            reg.inner.lock().await.fail_attach_session = true;
            reg
        }
    }

    impl SessionRegistry for FakeRegistry {
        type Session = FakeSession;

        async fn reserve_run(&self, run_id: String) -> Result<()> {
            self.inner.lock().await.reserved.push(run_id);
            Ok(())
        }

        async fn attach_run_handle(
            &self,
            run_id: &str,
            handle: JoinHandle<()>,
        ) -> Result<()> {
            self.inner
                .lock()
                .await
                .handles
                .insert(run_id.to_string(), handle);
            Ok(())
        }

        async fn attach_session(
            &self,
            run_id: &str,
            session: Arc<FakeSession>,
        ) -> Result<()> {
            let mut state = self.inner.lock().await;
            if state.fail_attach_session {
                return Err(anyhow!("simulated attach_session failure"));
            }
            state.sessions.insert(run_id.to_string(), session);
            Ok(())
        }

        async fn active_session(&self, run_id: &str) -> Option<Arc<FakeSession>> {
            self.inner.lock().await.sessions.get(run_id).cloned()
        }

        async fn finish_run(&self, run_id: &str) {
            self.inner.lock().await.finished.push(run_id.to_string());
        }

        async fn cancel_run(&self, _run_id: &str) -> bool {
            false
        }
    }

    #[derive(Clone, Default)]
    struct CollectingSink {
        events: Arc<StdMutex<Vec<(String, RunEvent)>>>,
    }

    impl RunEventSink for CollectingSink {
        fn emit(&self, run_id: &str, event: RunEvent) {
            self.events
                .lock()
                .unwrap()
                .push((run_id.to_string(), event));
        }
    }

    struct FakeCommander {
        aborted: Arc<AtomicUsize>,
        completed: Arc<AtomicUsize>,
        done: Arc<Notify>,
    }

    impl RunCommander for FakeCommander {
        fn run_to_completion(self: Box<Self>) -> DriverFuture {
            Box::pin(async move {
                self.completed.fetch_add(1, Ordering::SeqCst);
                self.done.notify_one();
            })
        }

        fn abort(self: Box<Self>) -> AbortFuture {
            Box::pin(async move {
                self.aborted.fetch_add(1, Ordering::SeqCst);
                self.done.notify_one();
            })
        }
    }

    fn make_request() -> AgentRunRequest {
        AgentRunRequest {
            goal: "hello".into(),
            agent_id: "agent".into(),
            cwd: None,
            agent_command: None,
            stdio_buffer_limit_mb: None,
            auto_allow: None,
            run_id: Some("run-1".into()),
        }
    }

    #[tokio::test]
    async fn driver_runs_when_launch_and_attach_succeed() {
        let registry = FakeRegistry::default();
        let sink = CollectingSink::default();
        let aborted = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(Notify::new());
        let aborted_clone = aborted.clone();
        let completed_clone = completed.clone();
        let done_clone = done.clone();

        let run = StartAgentRunUseCase::new(registry.clone())
            .execute(sink.clone(), make_request(), move |_, _, _| async move {
                Ok(LaunchedSession {
                    session: Arc::new(FakeSession),
                    commander: Box::new(FakeCommander {
                        aborted: aborted_clone,
                        completed: completed_clone,
                        done: done_clone,
                    }),
                })
            })
            .await
            .expect("start should succeed");

        done.notified().await;
        let handle = registry
            .inner
            .lock()
            .await
            .handles
            .remove(&run.id)
            .expect("handle stored");
        handle.await.expect("task should finish");

        assert_eq!(completed.load(Ordering::SeqCst), 1);
        assert_eq!(aborted.load(Ordering::SeqCst), 0);
        let state = registry.inner.lock().await;
        assert_eq!(state.reserved, vec!["run-1".to_string()]);
        assert_eq!(state.finished, vec!["run-1".to_string()]);
    }

    #[tokio::test]
    async fn aborter_runs_when_attach_session_fails() {
        let registry = FakeRegistry::with_failing_attach().await;
        let sink = CollectingSink::default();
        let aborted = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(Notify::new());
        let aborted_clone = aborted.clone();
        let completed_clone = completed.clone();
        let done_clone = done.clone();

        let run = StartAgentRunUseCase::new(registry.clone())
            .execute(sink.clone(), make_request(), move |_, _, _| async move {
                Ok(LaunchedSession {
                    session: Arc::new(FakeSession),
                    commander: Box::new(FakeCommander {
                        aborted: aborted_clone,
                        completed: completed_clone,
                        done: done_clone,
                    }),
                })
            })
            .await
            .expect("start call itself should succeed");

        done.notified().await;
        let handle = registry
            .inner
            .lock()
            .await
            .handles
            .remove(&run.id)
            .expect("handle stored");
        handle.await.expect("task should finish");

        assert_eq!(aborted.load(Ordering::SeqCst), 1);
        assert_eq!(completed.load(Ordering::SeqCst), 0);
        let events = sink.events.lock().unwrap();
        assert!(events.iter().any(|(_, event)| matches!(event, RunEvent::Diagnostic { .. })));
    }

    #[tokio::test]
    async fn error_during_launch_emits_run_error_and_finishes_run() {
        let registry = FakeRegistry::default();
        let sink = CollectingSink::default();
        let done = Arc::new(Notify::new());
        let done_clone = done.clone();
        let sink_clone = sink.clone();

        let run = StartAgentRunUseCase::new(registry.clone())
            .execute(
                sink.clone(),
                make_request(),
                move |_, run_id, _sink_for_launch| async move {
                    // emit is handled by the use case on our behalf; just fail.
                    let _ = (run_id, sink_clone);
                    done_clone.notify_one();
                    Err::<LaunchedSession<FakeSession>, _>(anyhow!("launch failed"))
                },
            )
            .await
            .expect("start call itself should succeed");

        done.notified().await;
        let handle = registry
            .inner
            .lock()
            .await
            .handles
            .remove(&run.id)
            .expect("handle stored");
        handle.await.expect("task should finish");

        let events = sink.events.lock().unwrap();
        assert!(events.iter().any(|(_, event)| matches!(event, RunEvent::Error { .. })));
        assert!(!events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::Lifecycle {
                status: LifecycleStatus::Completed,
                ..
            }
        )));
        let state = registry.inner.lock().await;
        assert_eq!(state.finished, vec!["run-1".to_string()]);
    }
}
