use crate::{
    application::errors::SendPromptError,
    domain::events::RunEvent,
    ports::{
        event_sink::RunEventSink, session_handle::SessionHandle, session_registry::SessionRegistry,
    },
};

/// Dispatch a follow-up prompt to an active run.
///
/// The use case looks up the active session via the registry port and
/// delegates prompt delivery to `SessionHandle::send_prompt`, so the
/// command handler stays independent of the ACP adapter.
pub struct SendPromptUseCase<R>
where
    R: SessionRegistry,
    R::Session: SessionHandle,
{
    registry: R,
}

impl<R> SendPromptUseCase<R>
where
    R: SessionRegistry,
    R::Session: SessionHandle,
{
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub async fn execute<S>(
        self,
        sink: S,
        run_id: String,
        prompt: String,
    ) -> Result<(), SendPromptError>
    where
        S: RunEventSink,
    {
        let trimmed = prompt.trim().to_string();
        if trimmed.is_empty() {
            return Err(SendPromptError::EmptyPrompt);
        }
        let session = self
            .registry
            .active_session(&run_id)
            .await
            .ok_or(SendPromptError::RunNotActive)?;
        let sink_for_task = sink.clone();
        tokio::spawn(async move {
            if let Err(err) = session.send_prompt(sink_for_task.clone(), trimmed).await {
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
    use crate::application::errors::SendPromptError;
    use crate::ports::session_registry::{ReserveRunError, SessionRegistry};
    use anyhow::{Result, anyhow};
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex as StdMutex},
    };
    use tokio::sync::{Mutex, Notify};
    use tokio::task::JoinHandle;

    #[allow(dead_code)]
    enum FakeBehavior {
        Ok,
        Err,
    }

    struct FakeSession {
        behavior: FakeBehavior,
        done: Arc<Notify>,
    }

    impl SessionHandle for FakeSession {
        async fn send_prompt<S>(&self, _sink: S, _text: String) -> Result<String>
        where
            S: RunEventSink,
        {
            self.done.notify_one();
            match self.behavior {
                FakeBehavior::Ok => Ok("end_turn".into()),
                FakeBehavior::Err => Err(anyhow!("dispatch exploded")),
            }
        }
    }

    #[derive(Clone, Default)]
    struct FakeRegistry {
        sessions: Arc<Mutex<HashMap<String, Arc<FakeSession>>>>,
    }

    impl FakeRegistry {
        async fn with_session(run_id: &str, behavior: FakeBehavior) -> (Self, Arc<Notify>) {
            let reg = Self::default();
            let done = Arc::new(Notify::new());
            reg.sessions.lock().await.insert(
                run_id.to_string(),
                Arc::new(FakeSession {
                    behavior,
                    done: done.clone(),
                }),
            );
            (reg, done)
        }
    }

    impl SessionRegistry for FakeRegistry {
        type Session = FakeSession;

        async fn reserve_run(&self, _: String, _: Option<String>) -> Result<(), ReserveRunError> {
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

        let result = SendPromptUseCase::new(registry)
            .execute(sink.clone(), "run-a".into(), "   ".into())
            .await;

        assert_eq!(result, Err(SendPromptError::EmptyPrompt));
        assert!(sink.events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_when_run_has_no_active_session() {
        let registry = FakeRegistry::default();
        let sink = CollectingSink::default();

        let result = SendPromptUseCase::new(registry)
            .execute(sink, "missing".into(), "hi".into())
            .await;

        assert_eq!(result, Err(SendPromptError::RunNotActive));
    }

    #[tokio::test]
    async fn dispatch_error_surfaces_as_run_event_error() {
        let (registry, done) = FakeRegistry::with_session("run-a", FakeBehavior::Err).await;
        let sink = CollectingSink::default();

        SendPromptUseCase::new(registry)
            .execute(sink.clone(), "run-a".into(), "hi".into())
            .await
            .expect("use case should accept the request");

        done.notified().await;
        tokio::task::yield_now().await;
        let events = sink.events.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|(_, event)| matches!(event, RunEvent::Error { .. }))
        );
    }
}
