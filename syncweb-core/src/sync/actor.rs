use std::{error::Error, fmt, future::Future, panic::AssertUnwindSafe};

use n0_future::FutureExt;
use tokio::sync::{mpsc, oneshot};

/// Error returned when an actor handler panics.
#[derive(Debug)]
#[non_exhaustive]
pub struct ActorPanic(String);

impl ActorPanic {
    #[must_use]
    pub const fn new(msg: String) -> Self {
        Self(msg)
    }
}

impl fmt::Display for ActorPanic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "actor handler panicked: {}", self.0)
    }
}

impl Error for ActorPanic {}

/// Marker used to spawn a dedicated asynchronous storage actor.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct Actor;

/// Handle for sending requests to an [`Actor`].
pub struct ActorHandle<M, R> {
    sender: mpsc::UnboundedSender<(M, oneshot::Sender<std::result::Result<R, ActorPanic>>)>,
}

impl<M, R> Clone for ActorHandle<M, R> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<M, R> std::fmt::Debug for ActorHandle<M, R> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("ActorHandle").finish_non_exhaustive()
    }
}

impl Actor {
    /// Spawn an actor loop. Handler panics are isolated to the request and do
    /// not unwind the task or the caller.
    #[must_use]
    pub fn spawn<M, R, F, Fut>(handler: F) -> ActorHandle<M, R>
    where
        M: Send + 'static,
        R: Send + 'static,
        F: Fn(M) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
    {
        let (sender, mut receiver) =
            mpsc::unbounded_channel::<(M, oneshot::Sender<std::result::Result<R, ActorPanic>>)>();
        tokio::spawn(async move {
            while let Some((message, response)) = receiver.recv().await {
                let result = AssertUnwindSafe(handler(message)).catch_unwind().await;
                match result {
                    Ok(value) => {
                        let _ = response.send(Ok(value));
                    }
                    Err(panic) => {
                        let msg = panic
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| panic.downcast_ref::<&str>().map(ToString::to_string))
                            .unwrap_or_else(|| "unknown panic".to_string());
                        let _ = response.send(Err(ActorPanic::new(msg)));
                    }
                }
            }
        });
        ActorHandle { sender }
    }
}

impl<M, R> ActorHandle<M, R>
where
    M: Send + 'static,
    R: Send + 'static,
{
    /// Send a message and await its response.
    /// # Errors
    ///
    /// Returns an error if the actor has stopped, the response channel was
    /// dropped, or the handler panicked.
    pub async fn request(&self, message: M) -> crate::Result<R> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((message, sender))
            .map_err(|error| crate::error::SyncwebError::operation("storage actor stopped", error))?;
        match receiver.await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(panic)) => Err(crate::error::SyncwebError::operation("actor handler panicked", panic)),
            Err(error) => Err(crate::error::SyncwebError::operation(
                "storage actor response dropped",
                error,
            )),
        }
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}
