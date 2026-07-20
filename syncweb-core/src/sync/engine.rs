use std::{
    collections::HashMap,
    path::Path,
    time::{Duration, Instant},
};

use iroh_blobs::Hash;
use iroh_docs::{NamespaceId, engine::LiveEvent};
use n0_future::StreamExt;
use tokio::sync::mpsc;

use crate::{
    error::Result,
    filter::{FilterAction, FilterEngine, FilterEntry},
    folder::FolderManager,
    node::{blob_store::BlobStore, docs_engine::DocsEngine, gossip_service::GossipService},
};

use super::{IntentHandle, SessionMode, SubscribeParams, SyncCommand, SyncEvent};

/// Current aggregate statistics for a synchronization intent.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct TransferStats {
    pub bytes_transferred: u64,
    pub bytes_total: Option<u64>,
    pub bytes_per_second: u64,
    pub peer_count: usize,
    pub eta: Option<Duration>,
}

impl TransferStats {
    #[must_use]
    pub fn from_progress(
        bytes_transferred: u64,
        bytes_total: Option<u64>,
        elapsed: Duration,
        peer_count: usize,
    ) -> Self {
        let elapsed_millis = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        let bytes_per_second = bytes_transferred
            .saturating_mul(1_000)
            .checked_div(elapsed_millis)
            .unwrap_or(0);
        let eta = bytes_total.and_then(|total| {
            let remaining = total.saturating_sub(bytes_transferred);
            remaining.checked_div(bytes_per_second).map(Duration::from_secs)
        });
        Self {
            bytes_transferred,
            bytes_total,
            bytes_per_second,
            peer_count,
            eta,
        }
    }
}

/// Coordinates document reconciliation and blob availability for a folder.
#[derive(Clone)]
pub struct SyncEngine {
    folder_manager: FolderManager,
    blob_store: BlobStore,
    docs_engine: DocsEngine,
}

impl SyncEngine {
    #[must_use]
    pub fn new(
        folder_manager: FolderManager,
        blob_store: BlobStore,
        docs_engine: DocsEngine,
        _gossip: GossipService,
    ) -> Self {
        Self {
            folder_manager,
            blob_store,
            docs_engine,
        }
    }

    /// Start reconciling a managed folder.
    ///
    /// Iroh Docs performs network reconciliation itself. This intent provides
    /// lifecycle, control, and aggregate transfer events around that process.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder is not managed locally.
    pub async fn sync(&self, folder_id: NamespaceId, mode: SessionMode) -> Result<IntentHandle> {
        self.sync_with_params(folder_id, mode, SubscribeParams::default()).await
    }

    /// Start a synchronization intent with subscription filtering.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder is not managed locally or live sync
    /// cannot be started.
    pub async fn sync_with_params(
        &self,
        folder_id: NamespaceId,
        mode: SessionMode,
        params: SubscribeParams,
    ) -> Result<IntentHandle> {
        self.start(folder_id, mode, params, None).await
    }

    /// Subscribe to live changes in a folder.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder is not managed locally or live sync
    /// cannot be started.
    pub async fn subscribe(&self, folder_id: NamespaceId, params: SubscribeParams) -> Result<IntentHandle> {
        self.sync_with_params(folder_id, SessionMode::Continuous, params).await
    }

    /// Start a synchronization intent and filter emitted entries.
    ///
    /// The document reconciliation remains content-addressed and is handled
    /// by iroh-docs; the filter controls which entries are reported to the
    /// automatic synchronization consumer.
    ///
    /// # Errors
    ///
    /// Returns an error if the folder is not managed locally or live sync
    /// cannot be started.
    pub async fn sync_with_filter(
        &self,
        folder_id: NamespaceId,
        mode: SessionMode,
        params: SubscribeParams,
        filter: FilterEngine,
    ) -> Result<IntentHandle> {
        self.start(folder_id, mode, params, Some(filter)).await
    }

    async fn start(
        &self,
        folder_id: NamespaceId,
        mode: SessionMode,
        params: SubscribeParams,
        filter: Option<FilterEngine>,
    ) -> Result<IntentHandle> {
        let folder = self.folder_manager.get(folder_id).await?;
        let live_events = self.docs_engine.watch(folder.doc()).await?;
        self.docs_engine.start_sync(folder.doc(), Vec::new()).await?;
        let (events, commands, handle) = IntentHandle::channel();
        tokio::spawn(run_intent(
            folder,
            IntentConfig { mode, params, filter },
            self.blob_store.clone(),
            live_events,
            events,
            commands,
        ));
        Ok(handle)
    }
}

struct IntentConfig {
    mode: SessionMode,
    params: SubscribeParams,
    filter: Option<FilterEngine>,
}

async fn run_intent(
    folder: crate::folder::SyncwebFolder,
    config: IntentConfig,
    blob_store: BlobStore,
    mut live_events: impl n0_future::Stream<Item = Result<LiveEvent>> + Send + Unpin + 'static,
    events: mpsc::UnboundedSender<SyncEvent>,
    mut commands: mpsc::UnboundedReceiver<SyncCommand>,
) {
    let started = Instant::now();
    if !send_initial_events(&events, started) {
        return;
    }
    if !config.mode.is_continuous() {
        let _result = events.send(SyncEvent::Finished);
        return;
    }

    let mut state = IntentState::default();
    loop {
        tokio::select! {
            received_command = commands.recv() => {
                let Some(command) = received_command else {
                    let _result = folder.doc().leave().await;
                    return;
                };
                if !handle_command(&folder, command, &mut state.paused, &events).await {
                    return;
                }
            }
            next_event = live_events.next(), if !state.paused => {
                let Some(event_result) = next_event else {
                    let _result = events.send(SyncEvent::Finished);
                    return;
                };
                let live_event = match event_result {
                    Ok(event) => event,
                    Err(error) => {
                        let _result = events.send(SyncEvent::Failed(error.to_string()));
                        return;
                    }
                };
                if !include_event(&config.params, &live_event) {
                    continue;
                }
                if let LiveEvent::ContentReady { hash } = &live_event {
                    match blob_store.has(*hash).await {
                        Ok(true) => {}
                        Ok(false) => {
                            let _result = events.send(SyncEvent::Failed(format!(
                                "content-ready blob {hash} is missing locally"
                            )));
                            return;
                        }
                        Err(error) => {
                            let _result = events.send(SyncEvent::Failed(error.to_string()));
                            return;
                        }
                    }
                }
                if let Err(error) = state.apply(live_event, &config.params, config.filter.as_ref()) {
                    let _result = events.send(SyncEvent::Failed(error));
                    return;
                }
                if !send_progress(&events, &state, started.elapsed()) {
                    return;
                }
            }
        }
    }
}

fn send_initial_events(events: &mpsc::UnboundedSender<SyncEvent>, started: Instant) -> bool {
    events.send(SyncEvent::Started).is_ok()
        && events
            .send(SyncEvent::Progress {
                completed: 0,
                total: Some(0),
            })
            .is_ok()
        && events
            .send(SyncEvent::Stats(TransferStats::from_progress(
                0,
                Some(0),
                started.elapsed(),
                0,
            )))
            .is_ok()
}

fn send_progress(events: &mpsc::UnboundedSender<SyncEvent>, state: &IntentState, elapsed: Duration) -> bool {
    events
        .send(SyncEvent::Progress {
            completed: state.completed,
            total: None,
        })
        .is_ok()
        && events
            .send(SyncEvent::Stats(TransferStats::from_progress(
                state.transferred,
                None,
                elapsed,
                state.peer_count,
            )))
            .is_ok()
}

#[derive(Default)]
struct IntentState {
    paused: bool,
    completed: u64,
    transferred: u64,
    peer_count: usize,
    sizes: HashMap<Hash, u64>,
    area_count: u64,
    area_size: u64,
}

impl IntentState {
    fn apply(
        &mut self,
        event: LiveEvent,
        params: &SubscribeParams,
        filter: Option<&FilterEngine>,
    ) -> std::result::Result<(), String> {
        match event {
            LiveEvent::NeighborUp(_) => self.peer_count = self.peer_count.saturating_add(1),
            LiveEvent::NeighborDown(_) => self.peer_count = self.peer_count.saturating_sub(1),
            LiveEvent::InsertLocal { entry } | LiveEvent::InsertRemote { entry, .. } => {
                self.apply_insert(&entry, params, filter);
            }
            LiveEvent::ContentReady { hash } => {
                self.transferred = self.transferred.saturating_add(self.sizes.remove(&hash).unwrap_or(0));
            }
            LiveEvent::PendingContentReady => {}
            LiveEvent::SyncFinished(sync_event) => match sync_event.result {
                Ok(details)
                    if params.area_filter.is_none() && params.area_of_interest.is_none() && filter.is_none() =>
                {
                    self.completed = self
                        .completed
                        .saturating_add(u64::try_from(details.entries_received).unwrap_or(u64::MAX));
                }
                Ok(_details) => {}
                Err(error) => return Err(error),
            },
        }
        Ok(())
    }

    fn apply_insert(&mut self, entry: &iroh_docs::Entry, params: &SubscribeParams, filter: Option<&FilterEngine>) {
        let path = entry_path(entry);
        let hash = entry.content_hash();
        let accepted = params.accepts(&path, &hash)
            && filter.is_none_or(|engine| {
                engine.evaluate(&FilterEntry::new(path.clone(), entry.content_len())) != FilterAction::Reject
            });
        if !accepted {
            return;
        }
        let size = entry.content_len();
        if params
            .area_of_interest
            .as_ref()
            .is_some_and(|area| !area.permits(self.area_count, self.area_size, size))
        {
            return;
        }
        if params.area_of_interest.is_some() {
            self.area_count = self.area_count.saturating_add(1);
            self.area_size = self.area_size.saturating_add(size);
        }
        self.completed = self.completed.saturating_add(1);
        self.sizes.insert(hash, size);
    }
}

async fn handle_command(
    folder: &crate::folder::SyncwebFolder,
    command: SyncCommand,
    paused: &mut bool,
    events: &mpsc::UnboundedSender<SyncEvent>,
) -> bool {
    match command {
        SyncCommand::Pause if !*paused => {
            if let Err(error) = folder.doc().leave().await {
                let _result = events.send(SyncEvent::Failed(error.to_string()));
                return false;
            }
            *paused = true;
            events.send(SyncEvent::Paused).is_ok()
        }
        SyncCommand::Resume if *paused => {
            if let Err(error) = folder.doc().start_sync(Vec::new()).await {
                let _result = events.send(SyncEvent::Failed(error.to_string()));
                return false;
            }
            *paused = false;
            events.send(SyncEvent::Resumed).is_ok()
        }
        SyncCommand::Cancel => {
            if let Err(error) = folder.doc().leave().await {
                let _result = events.send(SyncEvent::Failed(error.to_string()));
                return false;
            }
            let _result = events.send(SyncEvent::Cancelled);
            false
        }
        SyncCommand::Pause | SyncCommand::Resume => true,
    }
}

fn include_event(params: &SubscribeParams, event: &LiveEvent) -> bool {
    if params.ingest_only && matches!(event, LiveEvent::InsertLocal { .. }) {
        return false;
    }
    if params.ignore_session.is_some() && matches!(event, LiveEvent::InsertLocal { .. }) {
        return false;
    }
    match event {
        LiveEvent::InsertLocal { entry } | LiveEvent::InsertRemote { entry, .. } => {
            let path = entry_path(entry);
            params.accepts(&path, &entry.content_hash())
        }
        LiveEvent::ContentReady { hash } => params
            .area_of_interest
            .as_ref()
            .is_none_or(|area| area.area.matches_hash(hash)),
        LiveEvent::PendingContentReady
        | LiveEvent::NeighborUp(_)
        | LiveEvent::NeighborDown(_)
        | LiveEvent::SyncFinished(_) => true,
    }
}

fn entry_path(entry: &iroh_docs::Entry) -> std::path::PathBuf {
    Path::new(String::from_utf8_lossy(entry.key()).as_ref()).to_path_buf()
}
