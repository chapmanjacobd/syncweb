use std::{
    pin::Pin,
    task::{Context, Poll},
};

use n0_future::{Sink, Stream};
use tokio::sync::mpsc;

use super::TransferStats;

/// Commands accepted by a running synchronization intent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SyncCommand {
    Pause,
    Resume,
    Cancel,
}

/// Progress and lifecycle events emitted by an intent.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SyncEvent {
    Started,
    Progress { completed: u64, total: Option<u64> },
    Stats(TransferStats),
    Paused,
    Resumed,
    Cancelled,
    Finished,
    Failed(String),
}

/// Bidirectional handle for a synchronization operation.
///
/// It implements both halves of the futures API: callers can consume
/// progress events as a stream and send control commands as a sink.
pub struct IntentHandle {
    events: mpsc::UnboundedReceiver<SyncEvent>,
    commands: mpsc::UnboundedSender<SyncCommand>,
}

impl std::fmt::Debug for IntentHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("IntentHandle").finish_non_exhaustive()
    }
}

impl IntentHandle {
    #[must_use]
    pub fn channel() -> (
        mpsc::UnboundedSender<SyncEvent>,
        mpsc::UnboundedReceiver<SyncCommand>,
        Self,
    ) {
        let (event_sender, events) = mpsc::unbounded_channel();
        let (commands, command_receiver) = mpsc::unbounded_channel();
        (event_sender, command_receiver, Self { events, commands })
    }

    /// Request a pause without going through the Sink trait.
    /// # Errors
    ///
    /// Returns an error if the receiving end of the channel has been dropped.
    pub fn pause(&self) -> Result<(), mpsc::error::SendError<SyncCommand>> {
        self.commands.send(SyncCommand::Pause)
    }

    /// Request resumption without going through the Sink trait.
    /// # Errors
    ///
    /// Returns an error if the receiving end of the channel has been dropped.
    pub fn resume(&self) -> Result<(), mpsc::error::SendError<SyncCommand>> {
        self.commands.send(SyncCommand::Resume)
    }

    /// Request cancellation without going through the Sink trait.
    /// # Errors
    ///
    /// Returns an error if the receiving end of the channel has been dropped.
    pub fn cancel(&self) -> Result<(), mpsc::error::SendError<SyncCommand>> {
        self.commands.send(SyncCommand::Cancel)
    }
}

impl Stream for IntentHandle {
    type Item = SyncEvent;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.events.poll_recv(context)
    }
}

impl Sink<SyncCommand> for IntentHandle {
    type Error = mpsc::error::SendError<SyncCommand>;

    fn poll_ready(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.commands.is_closed() {
            Poll::Ready(Err(mpsc::error::SendError(SyncCommand::Cancel)))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(self: Pin<&mut Self>, item: SyncCommand) -> Result<(), Self::Error> {
        self.commands.send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
