use std::time::Duration;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use iroh_docs::NamespaceId;
use n0_future::StreamExt;
use tokio::{sync::broadcast, time::sleep};

use crate::{
    daemon::current_timestamp,
    error::{Result, SyncwebError},
    filter::FilterEngine,
    sync::{ActiveSession, IntentControl, IntentHandle, SubscribeParams, SyncEngine, SyncEvent},
};

/// A synchronization intent together with the information needed to restart it.
#[non_exhaustive]
pub struct SupervisedIntent {
    pub namespace: NamespaceId,
    pub handle: Option<IntentHandle>,
    pub retry_count: u32,
    pub last_error: Option<String>,
    pub last_started_at: Option<u64>,
    session: Option<ActiveSession>,
}

impl std::fmt::Debug for SupervisedIntent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SupervisedIntent")
            .field("namespace", &self.namespace)
            .field("handle", &self.handle)
            .field("retry_count", &self.retry_count)
            .field("last_error", &self.last_error)
            .field("last_started_at", &self.last_started_at)
            .finish_non_exhaustive()
    }
}

/// Starts synchronization intents and restarts failed intents with bounded
/// exponential backoff.
#[derive(Clone, Copy, Debug)]
pub struct IntentSupervisor {
    max_retries: u32,
    backoff_base: Duration,
    backoff_max: Duration,
}

/// Shared controls for intents currently owned by the daemon.
pub type IntentControls = Arc<Mutex<HashMap<NamespaceId, IntentControl>>>;

impl IntentSupervisor {
    #[must_use]
    pub const fn new(max_retries: u32, backoff_base: Duration, backoff_max: Duration) -> Self {
        Self {
            max_retries,
            backoff_base,
            backoff_max,
        }
    }

    /// Start one continuous intent and record any startup failure.
    pub async fn run_intent(
        &self,
        sync: &SyncEngine,
        namespace: NamespaceId,
        params: SubscribeParams,
    ) -> SupervisedIntent {
        self.run_intent_with_filter(sync, namespace, params, None).await
    }

    /// Start one filtered synchronization intent.
    pub async fn run_intent_with_filter(
        &self,
        sync: &SyncEngine,
        namespace: NamespaceId,
        params: SubscribeParams,
        filter_engine: Option<FilterEngine>,
    ) -> SupervisedIntent {
        let result = match filter_engine {
            Some(engine) => {
                sync.sync_with_filter(namespace, crate::sync::SessionMode::Continuous, params, engine)
                    .await
            }
            None => sync.subscribe(namespace, params).await,
        };
        match result {
            Ok(handle) => {
                let session = ActiveSession::register(namespace, handle.cancel_sender());
                SupervisedIntent {
                    namespace,
                    handle: Some(handle),
                    retry_count: 0,
                    last_error: None,
                    last_started_at: Some(current_timestamp()),
                    session: Some(session),
                }
            }
            Err(error) => SupervisedIntent {
                namespace,
                handle: None,
                retry_count: 0,
                last_error: Some(error.to_string()),
                last_started_at: None,
                session: None,
            },
        }
    }

    /// Supervise one intent until shutdown or until its retry budget is spent.
    ///
    /// A normal `Finished` or `Cancelled` event ends supervision. A `Failed`
    /// event, an unexpectedly closed stream, or a startup error consumes one
    /// retry and restarts the intent after backoff.
    ///
    /// # Errors
    ///
    /// Returns an error if supervision infrastructure fails while waiting for
    /// shutdown or retrying an intent.
    pub async fn supervise(
        &self,
        sync: &SyncEngine,
        namespace: NamespaceId,
        params: SubscribeParams,
        shutdown: broadcast::Receiver<()>,
    ) -> Result<SupervisedIntent> {
        self.supervise_inner(sync, namespace, params, shutdown, None, None)
            .await
    }

    /// Supervise an intent while exposing pause/resume controls to its owner.
    ///
    /// # Errors
    ///
    /// Returns an error if the shared control registry is poisoned or the
    /// intent supervision infrastructure cannot wait for shutdown.
    pub async fn supervise_with_controls(
        &self,
        sync: &SyncEngine,
        namespace: NamespaceId,
        params: SubscribeParams,
        shutdown: broadcast::Receiver<()>,
        controls: IntentControls,
        filter: Option<FilterEngine>,
    ) -> Result<SupervisedIntent> {
        self.supervise_inner(sync, namespace, params, shutdown, Some(controls), filter)
            .await
    }

    async fn supervise_inner(
        &self,
        sync: &SyncEngine,
        namespace: NamespaceId,
        params: SubscribeParams,
        mut shutdown: broadcast::Receiver<()>,
        controls: Option<IntentControls>,
        filter: Option<FilterEngine>,
    ) -> Result<SupervisedIntent> {
        let mut supervised = self
            .run_intent_with_filter(sync, namespace, params.clone(), filter.clone())
            .await;

        loop {
            let Some(mut handle) = supervised.handle.take() else {
                Self::remove_control(controls.as_ref(), namespace)?;
                if supervised.retry_count >= self.max_retries {
                    return Ok(supervised);
                }
                let retry_number = supervised.retry_count.saturating_add(1);
                if !self
                    .wait_for_retry(self.backoff_delay(retry_number), &mut shutdown)
                    .await
                {
                    return Ok(supervised);
                }
                supervised.retry_count = retry_number;
                supervised = self
                    .run_intent_with_filter(sync, namespace, params.clone(), filter.clone())
                    .await;
                supervised.retry_count = retry_number;
                continue;
            };

            Self::insert_control(controls.as_ref(), namespace, handle.control())?;
            let session = supervised.session.take();
            let event = loop {
                tokio::select! {
                    shutdown_result = shutdown.recv() => {
                        match shutdown_result {
                            Ok(()) | Err(broadcast::error::RecvError::Closed) => {
                                if let Err(error) = handle.cancel() {
                                    supervised.last_error = Some(format!("failed to cancel intent: {error}"));
                                }
                                Self::remove_control(controls.as_ref(), namespace)?;
                                drop(session);
                                supervised.handle = None;
                                return Ok(supervised);
                            }
                            Err(broadcast::error::RecvError::Lagged(_)) => {}
                        }
                    }
                    next_event = handle.next() => break next_event,
                }
            };

            Self::remove_control(controls.as_ref(), namespace)?;
            match event {
                Some(SyncEvent::Started) => {
                    supervised.retry_count = 0;
                    supervised.last_error = None;
                    supervised.last_started_at = Some(current_timestamp());
                    supervised.handle = Some(handle);
                    supervised.session = session;
                }
                Some(SyncEvent::Cancelled | SyncEvent::Finished) => {
                    supervised.handle = None;
                    drop(session);
                    return Ok(supervised);
                }
                Some(SyncEvent::Failed(error)) => {
                    supervised.last_error = Some(error);
                    supervised.handle = None;
                    drop(session);
                }
                Some(SyncEvent::Progress { .. } | SyncEvent::Stats(_) | SyncEvent::Paused | SyncEvent::Resumed) => {
                    supervised.handle = Some(handle);
                    supervised.session = session;
                }
                None => {
                    supervised.last_error = Some("synchronization intent ended unexpectedly".to_owned());
                    supervised.handle = None;
                    drop(session);
                }
            }
        }
    }

    fn insert_control(controls: Option<&IntentControls>, namespace: NamespaceId, control: IntentControl) -> Result<()> {
        let Some(control_registry) = controls else {
            return Ok(());
        };
        control_registry
            .lock()
            .map_err(|error| SyncwebError::operation("daemon intent control mutex is poisoned", error))?
            .insert(namespace, control);
        Ok(())
    }

    fn remove_control(controls: Option<&IntentControls>, namespace: NamespaceId) -> Result<()> {
        let Some(control_registry) = controls else {
            return Ok(());
        };
        control_registry
            .lock()
            .map_err(|error| SyncwebError::operation("daemon intent control mutex is poisoned", error))?
            .remove(&namespace);
        Ok(())
    }

    /// Return the delay for a one-based retry number.
    #[must_use]
    pub fn backoff_delay(&self, retry_number: u32) -> Duration {
        if retry_number == 0 {
            return Duration::ZERO;
        }
        let mut delay = self.backoff_base;
        for _ in 1..retry_number {
            delay = match delay.checked_mul(2) {
                Some(value) if value <= self.backoff_max => value,
                _ => self.backoff_max,
            };
            if delay == self.backoff_max {
                break;
            }
        }
        delay.min(self.backoff_max)
    }

    async fn wait_for_retry(&self, delay: Duration, shutdown: &mut broadcast::Receiver<()>) -> bool {
        tokio::select! {
            () = sleep(delay) => true,
            shutdown_result = shutdown.recv() => {
                !matches!(shutdown_result, Ok(()) | Err(broadcast::error::RecvError::Closed))
            }
        }
    }
}
