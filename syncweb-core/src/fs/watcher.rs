use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, TryRecvError},
};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::{Result, SyncwebError};

/// A filesystem event delivered by [`FsWatcher`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FsEvent {
    pub event: Event,
    pub paths: Vec<PathBuf>,
}

/// notify-rs backed recursive filesystem watcher.
pub struct FsWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<Result<FsEvent>>,
}

impl std::fmt::Debug for FsWatcher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("FsWatcher").finish_non_exhaustive()
    }
}

impl FsWatcher {
    /// Start watching a path recursively.
    /// # Errors
    ///
    /// Returns an error if the filesystem cannot be accessed or watched.
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let watcher = notify::recommended_watcher(move |event_result: notify::Result<Event>| {
            let result = event_result
                .map(|event| FsEvent {
                    event: event.clone(),
                    paths: event.paths,
                })
                .map_err(|error| SyncwebError::operation("filesystem watcher event failed", error));
            let _ = sender.send(result);
        })
        .map_err(|error| SyncwebError::operation("failed to create filesystem watcher", error))?;
        let mut this = Self { watcher, receiver };
        this.watcher
            .watch(path.as_ref(), RecursiveMode::Recursive)
            .map_err(|error| SyncwebError::operation("failed to watch filesystem path", error))?;
        Ok(this)
    }

    /// Wait for the next filesystem event.
    /// # Errors
    ///
    /// Returns an error if the filesystem cannot be accessed or watched.
    pub fn recv(&self) -> Result<FsEvent> {
        self.receiver
            .recv()
            .map_err(|error| SyncwebError::operation("filesystem watcher stopped", error))?
    }

    /// Poll for an event without blocking.
    /// # Errors
    ///
    /// Returns an error if the filesystem cannot be accessed or watched.
    pub fn try_recv(&self) -> Result<Option<FsEvent>> {
        match self.receiver.try_recv() {
            Ok(event) => event.map(Some),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(SyncwebError::operation(
                "filesystem watcher stopped",
                "event channel disconnected",
            )),
        }
    }

    /// Stop watching the supplied path.
    /// # Errors
    ///
    /// Returns an error if the filesystem cannot be accessed or watched.
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.watcher
            .unwatch(path.as_ref())
            .map_err(|error| SyncwebError::operation("failed to stop filesystem watcher", error))
    }
}
