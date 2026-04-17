use std::{future::Future, sync::Arc};

use anyhow::Result;

use crate::{
    domain::events::RunEvent,
    ports::{event_sink::RunEventSink, session_registry::SessionRegistry},
};

/// Dispatch a follow-up prompt to an active run.
///
/// The actual prompt transport is provided by the caller as a closure so
/// the use case stays independent of the concrete session adapter. Errors
/// from the dispatch are surfaced to the run event stream instead of being
/// returned to the command caller, which matches the behavior of the
/// previous inline implementation (the command returns as soon as the
/// prompt is accepted; failures arrive asynchronously on the event
/// stream).
pub struct SendPromptUseCase<R>
where
    R: SessionRegistry,
{
    registry: R,
}

impl<R> SendPromptUseCase<R>
where
    R: SessionRegistry,
{
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub async fn execute<S, F, Fut>(
        self,
        sink: S,
        run_id: String,
        prompt: String,
        dispatch: F,
    ) -> Result<(), String>
    where
        S: RunEventSink,
        F: FnOnce(Arc<R::Session>, S, String) -> Fut + Send + 'static,
        Fut: Future<Output = Result<String>> + Send + 'static,
    {
        let trimmed = prompt.trim().to_string();
        if trimmed.is_empty() {
            return Err("prompt is empty".into());
        }
        let session = self
            .registry
            .active_session(&run_id)
            .await
            .ok_or_else(|| "agent run is not active".to_string())?;
        let sink_for_task = sink.clone();
        tokio::spawn(async move {
            if let Err(err) = dispatch(session, sink_for_task.clone(), trimmed).await {
                sink_for_task.emit(
                    &run_id,
                    RunEvent::Error {
                        message: err.to_string(),
                    },
                );
            }
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::session_registry::SessionRegistry;
    use anyhow::{Result, anyhow};
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
        sessions: Arc<Mutex<HashMap<String, Arc<FakeSession>>>>,
    }

    impl FakeRegistry {
        async fn with_session(run_id: &str) -> Self {
            let reg = Self::default();
            reg.sessions
                .lock()
                .await
                .insert(run_id.to_string(), Arc::new(FakeSession));
            reg
        }
    }

    impl SessionRegistry for FakeRegistry {
        type Session = FakeSession;

        async fn reserve_run(&self, _: String) -> Result<()> {
            Ok(())
        }
        async fn attach_run_handle(&self, _: &str, handle: JoinHandle<()>) -> Result<()> {
            handle.abort();
            Ok(())
        }
        async fn attach_session(&self, _: &str, _: Arc<FakeSession>) -> Result<()> {
            Ok(())
        }
        async fn active_session(&self, run_id: &str) -> Option<Arc<FakeSession>> {
            self.sessions.lock().await.get(run_id).cloned()
        }
        async fn finish_run(&self, _: &str) {}
        async fn cancel_run(&self, _: &str) -> bool {
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

    #[tokio::test]
    async fn rejects_empty_prompt_without_touching_registry() {
        let registry = FakeRegistry::default();
        let sink = CollectingSink::default();
        let invoked = Arc::new(AtomicUsize::new(0));
        let invoked_clone = invoked.clone();

        let result = SendPromptUseCase::new(registry)
            .execute(sink.clone(), "run-a".into(), "   ".into(), move |_, _, _| {
                let invoked = invoked_clone.clone();
                async move {
                    invoked.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, anyhow::Error>(String::new())
                }
            })
            .await;

        assert_eq!(result, Err("prompt is empty".into()));
        assert_eq!(invoked.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn rejects_when_run_has_no_active_session() {
        let registry = FakeRegistry::default();
        let sink = CollectingSink::default();

        let result = SendPromptUseCase::new(registry)
            .execute(sink, "missing".into(), "hi".into(), move |_, _, _| async move {
                Ok::<_, anyhow::Error>(String::new())
            })
            .await;

        assert_eq!(result, Err("agent run is not active".into()));
    }

    #[tokio::test]
    async fn dispatch_error_surfaces_as_run_event_error() {
        let registry = FakeRegistry::with_session("run-a").await;
        let sink = CollectingSink::default();
        let done = Arc::new(Notify::new());
        let done_clone = done.clone();

        SendPromptUseCase::new(registry)
            .execute(sink.clone(), "run-a".into(), "hi".into(), move |_, _, _| {
                let done = done_clone.clone();
                async move {
                    let err: Result<String> = Err(anyhow!("dispatch exploded"));
                    done.notify_one();
                    err
                }
            })
            .await
            .expect("use case should accept the request");

        done.notified().await;
        // Allow the spawned task to finish emitting the error.
        tokio::task::yield_now().await;
        let events = sink.events.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|(_, event)| matches!(event, RunEvent::Error { .. }))
        );
    }
}
