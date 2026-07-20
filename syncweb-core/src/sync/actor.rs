use std::{future::Future, panic::AssertUnwindSafe};

use n0_future::FutureExt;
use tokio::sync::{mpsc, oneshot};

/// Marker used to spawn a dedicated asynchronous storage actor.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct Actor;

/// Handle for sending requests to an [`Actor`].
pub struct ActorHandle<M, R> {
    sender: mpsc::UnboundedSender<(M, oneshot::Sender<R>)>,
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
        R: Default + Send + 'static,
        F: Fn(M) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = R> + Send + 'static,
    {
        let (sender, mut receiver) = mpsc::unbounded_channel::<(M, oneshot::Sender<R>)>();
        tokio::spawn(async move {
            while let Some((message, response)) = receiver.recv().await {
                let result = AssertUnwindSafe(handler(message)).catch_unwind().await;
                if let Ok(value) = result {
                    let _ = response.send(value);
                } else {
                    let _ = response.send(R::default());
                }
            }
        });
        ActorHandle { sender }
    }
}

impl<M, R> ActorHandle<M, R>
where
    M: Send + 'static,
    R: Default + Send + 'static,
{
    /// Send a message and await its response.
    /// # Errors
    ///
    /// Returns an error if the actor has stopped or the response channel was dropped.
    pub async fn request(&self, message: M) -> crate::Result<R> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send((message, sender))
            .map_err(|error| crate::error::SyncwebError::operation("storage actor stopped", error))?;
        receiver
            .await
            .map_err(|error| crate::error::SyncwebError::operation("storage actor response dropped", error))
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}
