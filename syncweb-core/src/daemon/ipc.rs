use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use n0_future::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast, mpsc};

use crate::{
    bandwidth_stats::{FileStatsCollector, FileStatsReport},
    daemon::state::FolderStatusReport,
    error::{Result, SyncwebError},
    filter::{FilterConfig, FilterEngine},
    folder::{
        CollectionHead, CollectionManifest, CollectionStore, DropExportOptions, DropExportResult, DropExporter,
        DropImportOptions, DropImportResult, DropImporter, FolderLike, FolderManager, PublicSubscription, ShareOptions,
        SyncMode, share_folder,
    },
    fs::Importer,
    indexing::IndexingService,
    node::iroh_node::IrohNode,
    snapshot::SnapshotStore,
    storage::config::SubscribeFilters,
    storage::node_db::NodeDatabase,
    sync::{
        ActiveSession, AreaFilter, FetchFilter, FetchStrategy, SubscribeParams, SyncEngine, SyncEvent, cancel_session,
    },
    verify::IntegrityChecker,
};

use super::{
    ManagedPool,
    state::{DaemonStatus, daemon_socket_path},
};

use std::time::Duration;

const IPC_TIMEOUT: Duration = Duration::from_secs(2);

const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(1);

/// A request sent over the local daemon control channel.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct IpcRequest {
    pub command: IpcCommand,
}

impl IpcRequest {
    #[must_use]
    pub const fn new(command: IpcCommand) -> Self {
        Self { command }
    }
}

/// Commands supported by the daemon control channel.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "command", rename_all = "snake_case")]
#[non_exhaustive]
pub enum IpcCommand {
    Status,
    ListFolders,
    AddFolder {
        namespace: String,
        path: PathBuf,
    },
    TriggerSync {
        namespace: Option<String>,
    },
    SetLogLevel {
        level: String,
    },
    ReloadConfig,
    Shutdown {
        force: bool,
    },
    Download {
        namespace: String,
        strategy: FetchStrategy,
    },
    MaterializeTransfers {
        namespace: Option<String>,
    },
    ImportFiles {
        namespace: Option<String>,
        path: PathBuf,
    },
    ImportArchive {
        input: PathBuf,
        target: PathBuf,
        filter: Option<FilterConfig>,
    },
    ExportArchive {
        namespace: String,
        version: Option<String>,
        output: PathBuf,
    },
    Join {
        ticket: String,
        path: PathBuf,
        mode: SyncMode,
        #[serde(default)]
        subscribe: bool,
        #[serde(default)]
        filters: SubscribeFilters,
        #[serde(default)]
        download: bool,
        #[serde(default)]
        indexing: bool,
        #[serde(default)]
        metadata_only: bool,
    },
    SetSubscribe {
        namespace: String,
        enabled: bool,
        #[serde(default)]
        filters: Option<SubscribeFilters>,
    },
    SubscribePublic {
        ticket: String,
    },
    CreateFolder {
        path: PathBuf,
        mode: String,
        #[serde(default)]
        indexing: bool,
    },
    StatsFiles {
        folder: PathBuf,
    },
    ListEntries {
        folder: String,
        #[serde(default = "default_entry_enrich")]
        enrich: bool,
    },
    VerifyIntegrity {
        path: PathBuf,
        #[serde(default)]
        hash: Vec<String>,
        #[serde(default)]
        path_filter: Option<String>,
        #[serde(default)]
        glob_filter: Option<String>,
        #[serde(default)]
        fix: bool,
        #[serde(default)]
        from: Vec<String>,
    },
    Unsubscribe {
        namespace: String,
    },
    LeaveFolder {
        namespace: String,
        #[serde(default)]
        delete_files: bool,
    },
    Share {
        namespace: String,
        #[serde(default)]
        blob: Option<String>,
        writable: bool,
        #[serde(default)]
        pin: bool,
        #[serde(default)]
        persist: bool,
    },
    ShareList,
    Unshare {
        namespace: String,
        #[serde(default)]
        blob: Option<String>,
        writable: bool,
    },
    SnapshotCreate {
        path: PathBuf,
        description: Option<String>,
        threads: usize,
    },
    SnapshotList {
        path: PathBuf,
    },
    SnapshotDelete {
        id: String,
    },
    CollectionPublish {
        path: PathBuf,
        namespace: String,
        sequence: u64,
        bootstrap: Vec<String>,
        #[serde(default)]
        manifest_bytes: Option<Vec<u8>>,
    },
    EnrichSort {
        path: PathBuf,
    },
    NetworkInvite {
        network_id: String,
        device: String,
    },
    NetworkKick {
        network_id: String,
        device: String,
    },
    NetworkLeave {
        network_id: String,
    },
    NetworkCreate {
        name: String,
        label: String,
        invite_only: bool,
        doc_ticket: Option<String>,
    },
    NetworkJoin {
        ticket: String,
    },
    PeerAvailability {
        folder: String,
    },
}

/// A response returned by the daemon control channel.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "response", content = "data", rename_all = "snake_case")]
#[non_exhaustive]
pub enum IpcResponse {
    Ok { message: String },
    Status(DaemonStatus),
    FolderList(Vec<FolderStatus>),
    DownloadComplete { bytes_transferred: u64 },
    TransferJobsProcessed { completed: u64, failed: u64 },
    ImportFilesComplete { entries: u64 },
    ImportComplete(Box<DropImportResult>),
    ExportComplete(Box<DropExportResult>),
    EnrichData(HashMap<String, usize>),
    FileStats(Box<FileStatsReport>),
    Entries(Vec<EntryRow>),
    PeerAvailability(Box<PeerAvailabilityReport>),
    Error { message: String },
}

/// One row of a metadata-only folder listing returned by the daemon.
///
/// `size`/`modified` are the doc metadata unless the blob is local, in which
/// case they are overlaid with the on-disk `stat` of the materialized file.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct EntryRow {
    pub path: String,
    pub hash: iroh_blobs::Hash,
    pub size: u64,
    pub local: bool,
    pub modified: Option<u64>,
}

/// Serde default for `ListEntries.enrich`: enrichment stays on for callers
/// that predate the `--no-enrich` flag.
const fn default_entry_enrich() -> bool {
    true
}

/// A read-only snapshot of which peers can serve a folder's blobs and which
/// peers joined the folder.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PeerAvailabilityReport {
    /// The folder's namespace, as resolved by the daemon.
    pub folder: String,
    /// Inbound peers (who joined) for the folder. Syncthing device IDs only.
    #[serde(default)]
    pub peers: Vec<PeerInfo>,
    /// Per-blob availability: each blob's path/hash plus the peers observed
    /// serving it.
    #[serde(default)]
    pub per_blob: Vec<BlobPeerAvailability>,
}

/// One inbound peer that joined a folder.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct PeerInfo {
    /// The peer's Syncthing device ID.
    pub device_id: String,
    /// Display name, empty-typed until inbound-device acceptance adds names.
    #[serde(default)]
    pub name: Option<String>,
    /// Connection state ("online"/"offline"/"member") when known.
    #[serde(default)]
    pub connection: Option<String>,
}

/// One blob's peer availability within a folder.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct BlobPeerAvailability {
    /// Folder-relative path of the blob's entry.
    pub path: String,
    /// Content hash of the blob.
    pub hash: String,
    /// Number of peers observed serving the blob (point-in-time snapshot).
    #[serde(default)]
    pub peer_count: usize,
    /// Device IDs of the peers serving this blob.
    #[serde(default)]
    pub peers: Vec<String>,
    /// Percentage of the folder's inbound peers that serve the blob.
    #[serde(default)]
    pub pct_seeded: f64,
}

/// A managed folder summary returned by the daemon.
pub use crate::daemon::state::FolderStatusReport as FolderStatus;

/// A folder managed by the daemon.
#[non_exhaustive]
pub struct FolderEntry {
    pub namespace: iroh_docs::NamespaceId,
    pub path: PathBuf,
    pub mode: String,
    pub session: Option<ActiveSession>,
    pub last_sync_at: Option<u64>,
    pub sync_count: u64,
    pub entries_synced: u64,
    pub errors: Vec<String>,
}

impl FolderEntry {
    #[must_use]
    pub const fn new(namespace: iroh_docs::NamespaceId, path: PathBuf) -> Self {
        Self {
            namespace,
            path,
            mode: String::new(),
            session: None,
            last_sync_at: None,
            sync_count: 0,
            entries_synced: 0,
            errors: Vec::new(),
        }
    }

    /// Attach the folder's sync mode (empty when unknown).
    #[must_use]
    pub fn with_mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = mode.into();
        self
    }

    #[must_use]
    pub fn status(&self) -> FolderStatusReport {
        FolderStatusReport {
            namespace: self.namespace.to_string(),
            path: self.path.clone(),
            kind: "folder".to_owned(),
            mode: self.mode.clone(),
            session_active: self.session.is_some() || crate::sync::is_active(self.namespace),
            last_sync_at: self.last_sync_at,
            sync_count: self.sync_count,
            entries_synced: self.entries_synced,
            errors: self.errors.clone(),
        }
    }
}

/// Registry of folders and subscriptions currently managed by the daemon.
#[derive(Default)]
pub struct FolderRegistry {
    folders: HashMap<String, FolderEntry>,
    subscriptions: HashMap<String, PublicSubscription>,
    removed: HashSet<String>,
}

impl FolderRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a folder to the registry.
    ///
    /// # Errors
    ///
    /// Returns an error when the namespace is already registered.
    pub fn add(&mut self, entry: FolderEntry) -> Result<()> {
        let key = entry.namespace.to_string();
        if self.folders.contains_key(&key) {
            return Err(SyncwebError::FolderAlreadyManaged);
        }
        self.removed.remove(&key);
        self.folders.insert(key, entry);
        Ok(())
    }

    /// Add a folder, or attach a path to a folder restored without one.
    ///
    /// # Errors
    ///
    /// Returns an error when the namespace is already managed with a path or
    /// when the requested update conflicts with an existing registration.
    pub fn add_or_update(&mut self, entry: FolderEntry) -> Result<()> {
        let key = entry.namespace.to_string();
        if let Some(existing) = self.folders.get_mut(&key) {
            if existing.path.as_os_str().is_empty() && !entry.path.as_os_str().is_empty() {
                existing.path = entry.path;
                self.removed.remove(&key);
                return Ok(());
            }
            return Err(SyncwebError::FolderAlreadyManaged);
        }
        self.removed.remove(&key);
        self.folders.insert(key, entry);
        Ok(())
    }

    pub fn remove(&mut self, namespace: &iroh_docs::NamespaceId) -> Option<FolderEntry> {
        let key = namespace.to_string();
        self.removed.insert(key.clone());
        self.folders.remove(&key)
    }

    #[must_use]
    pub fn is_removed(&self, namespace: &str) -> bool {
        self.removed.contains(namespace)
    }

    /// Add a subscription to the registry.
    pub fn add_subscription(&mut self, subscription: PublicSubscription) {
        let key = subscription.namespace_id();
        self.subscriptions.insert(key, subscription);
    }

    /// Remove a subscription by its namespace ID (e.g. `"blob:<hash>"`).
    pub fn remove_subscription(&mut self, namespace_id: &str) -> Option<PublicSubscription> {
        self.subscriptions.remove(namespace_id)
    }

    #[must_use]
    pub fn subscription_statuses(&self) -> Vec<FolderStatusReport> {
        self.subscriptions
            .values()
            .map(|sub| FolderStatusReport {
                namespace: sub.namespace_id(),
                path: PathBuf::new(),
                kind: "subscription".to_owned(),
                mode: String::new(),
                session_active: false,
                last_sync_at: None,
                sync_count: 0,
                entries_synced: sub.size(),
                errors: Vec::new(),
            })
            .collect()
    }

    #[must_use]
    pub fn statuses(&self) -> Vec<FolderStatusReport> {
        let mut statuses: Vec<_> = self.folders.values().map(FolderEntry::status).collect();
        statuses.extend(self.subscription_statuses());
        statuses.sort_by(|left, right| left.namespace.cmp(&right.namespace));
        statuses
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.folders.len().saturating_add(self.subscriptions.len())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty() && self.subscriptions.is_empty()
    }

    /// Return the mount path registered for a folder namespace, if any.
    #[must_use]
    pub fn path_for(&self, namespace: &str) -> Option<PathBuf> {
        self.folders.get(namespace).map(|entry| entry.path.clone())
    }

    pub fn record_import(&mut self, namespace: iroh_docs::NamespaceId, entries: u64, timestamp: u64) {
        if let Some(folder) = self.folders.get_mut(&namespace.to_string()) {
            folder.entries_synced = folder.entries_synced.saturating_add(entries);
            folder.sync_count = folder.sync_count.saturating_add(1);
            folder.last_sync_at = Some(timestamp);
            folder.errors.clear();
        }
    }

    pub fn record_error(&mut self, namespace: iroh_docs::NamespaceId, error: impl Into<String>) {
        if let Some(folder) = self.folders.get_mut(&namespace.to_string()) {
            folder.errors.push(error.into());
            if folder.errors.len() > 16 {
                let remove_count = folder.errors.len().saturating_sub(16);
                folder.errors.drain(..remove_count);
            }
        }
    }
}

/// Shared daemon state used by the IPC server.
#[derive(Clone)]
#[non_exhaustive]
pub struct DaemonHandle {
    pub state: Arc<RwLock<super::state::DaemonState>>,
    pub folder_registry: Arc<RwLock<FolderRegistry>>,
    pub shutdown_sender: broadcast::Sender<()>,
    pub sync_trigger: mpsc::UnboundedSender<Option<String>>,
    pub reload_requested: Arc<AtomicBool>,
}

impl DaemonHandle {
    /// Create a handle with fresh control channels.
    #[must_use]
    pub fn new(state: super::state::DaemonState) -> Self {
        let (shutdown_sender, _) = broadcast::channel(16);
        let (sync_trigger, _) = mpsc::unbounded_channel();
        Self::with_channels(
            Arc::new(RwLock::new(state)),
            Arc::new(RwLock::new(FolderRegistry::new())),
            shutdown_sender,
            sync_trigger,
        )
    }

    #[must_use]
    pub fn with_channels(
        state: Arc<RwLock<super::state::DaemonState>>,
        folder_registry: Arc<RwLock<FolderRegistry>>,
        shutdown_sender: broadcast::Sender<()>,
        sync_trigger: mpsc::UnboundedSender<Option<String>>,
    ) -> Self {
        Self::with_channels_and_reload(
            state,
            folder_registry,
            shutdown_sender,
            sync_trigger,
            Arc::new(AtomicBool::new(false)),
        )
    }

    #[must_use]
    pub const fn with_channels_and_reload(
        state: Arc<RwLock<super::state::DaemonState>>,
        folder_registry: Arc<RwLock<FolderRegistry>>,
        shutdown_sender: broadcast::Sender<()>,
        sync_trigger: mpsc::UnboundedSender<Option<String>>,
        reload_requested: Arc<AtomicBool>,
    ) -> Self {
        Self {
            state,
            folder_registry,
            shutdown_sender,
            sync_trigger,
            reload_requested,
        }
    }

    /// Update the lifecycle status returned by future status requests.
    pub async fn set_status(&self, status: DaemonStatus) {
        self.state.write().await.status = status;
    }
}

/// Socket path and binding helper for the daemon.
#[derive(Clone, Debug)]
pub struct IpcListener {
    socket_path: PathBuf,
}

impl IpcListener {
    #[must_use]
    pub const fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    #[must_use]
    pub fn for_data_dir(data_dir: &Path) -> Self {
        Self::new(daemon_socket_path(data_dir))
    }

    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Bind the daemon socket with owner-only permissions on Unix.
    ///
    /// # Errors
    ///
    /// Returns an error when the parent directory or socket cannot be created.
    #[cfg(unix)]
    pub fn bind(&self) -> Result<tokio::net::UnixListener> {
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if self.socket_path.exists() {
            match std::os::unix::net::UnixStream::connect(&self.socket_path) {
                Ok(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AddrInUse,
                        "daemon IPC socket is already in use",
                    )
                    .into());
                }
                Err(_) => std::fs::remove_file(&self.socket_path)?,
            }
        }
        let listener = tokio::net::UnixListener::bind(&self.socket_path).map_err(|error| {
            SyncwebError::operation(
                format!("failed to bind daemon socket at {}", self.socket_path.display()),
                error,
            )
        })?;
        set_owner_only_permissions(&self.socket_path)?;
        Ok(listener)
    }
}

/// A server for the daemon's local control channel.
#[derive(Clone)]
pub struct IpcServer {
    #[cfg_attr(not(unix), allow(dead_code))]
    listener: IpcListener,
    daemon_handle: DaemonHandle,
    archive_context: Option<Arc<ArchiveContext>>,
    folder_manager: Option<FolderManager>,
    node_db: Option<NodeDatabase>,
    network_manager: Option<std::sync::Arc<tokio::sync::RwLock<crate::net::NetworkManager>>>,
}

#[derive(Clone)]
struct ArchiveContext {
    node: Arc<IrohNode>,
    pool: Arc<ManagedPool>,
    indexing: Option<IndexingService>,
}

/// Boolean toggles for the join workflow, bundled to keep the handler signature
/// small and free of excessive boolean parameters.
#[derive(Clone, Copy, Debug, Default)]
struct JoinOptions {
    subscribe: bool,
    download: bool,
    indexing: bool,
    metadata_only: bool,
}

impl IpcServer {
    #[must_use]
    pub const fn new(socket_path: PathBuf, daemon_handle: DaemonHandle) -> Self {
        Self {
            listener: IpcListener::new(socket_path),
            daemon_handle,
            archive_context: None,
            folder_manager: None,
            node_db: None,
            network_manager: None,
        }
    }

    /// Create an IPC server with access to daemon-owned archive resources.
    #[must_use]
    pub fn with_archive_context(
        socket_path: PathBuf,
        daemon_handle: DaemonHandle,
        node: Arc<IrohNode>,
        pool: Arc<ManagedPool>,
        indexing: Option<IndexingService>,
    ) -> Self {
        Self {
            listener: IpcListener::new(socket_path),
            daemon_handle,
            archive_context: Some(Arc::new(ArchiveContext { node, pool, indexing })),
            folder_manager: None,
            node_db: None,
            network_manager: None,
        }
    }

    /// Set the folder manager for this IPC server.
    #[must_use]
    pub fn with_folder_manager(mut self, folder_manager: FolderManager) -> Self {
        self.folder_manager = Some(folder_manager);
        self
    }

    /// Set the node database for persistence.
    #[must_use]
    pub fn with_node_db(mut self, node_db: NodeDatabase) -> Self {
        self.node_db = Some(node_db);
        self
    }

    /// Attach a network manager for daemon-side network operations.
    #[must_use]
    pub fn with_network_manager(
        mut self,
        network_manager: std::sync::Arc<tokio::sync::RwLock<crate::net::NetworkManager>>,
    ) -> Self {
        self.network_manager = Some(network_manager);
        self
    }

    /// Accept and process requests until the daemon broadcasts shutdown.
    ///
    /// # Errors
    ///
    /// Returns an error when the socket cannot be bound, accepted, or written.
    pub async fn serve(&self) -> Result<()> {
        #[cfg(unix)]
        {
            let listener = self.listener.bind()?;
            let mut shutdown = self.daemon_handle.shutdown_sender.subscribe();
            let result = loop {
                tokio::select! {
                    shutdown_result = shutdown.recv() => {
                        match shutdown_result {
                            Ok(()) | Err(broadcast::error::RecvError::Closed) => break Ok(()),
                            Err(broadcast::error::RecvError::Lagged(_)) => {}
                        }
                    }
                    accepted = listener.accept() => {
                        let (stream, _) = accepted.map_err(|error| {
                            SyncwebError::operation(
                                format!("daemon socket accept failed at {}", self.listener.socket_path().display()),
                                error,
                            )
                        })?;
                        if let Err(error) = self.handle_connection(stream).await {
                            tracing::error!(%error, "daemon IPC connection failed");
                            return Err(error);
                        }
                    }
                }
            };
            if let Err(error) = std::fs::remove_file(self.listener.socket_path())
                && self.listener.socket_path().exists()
            {
                return Err(error.into());
            }
            result
        }
        #[cfg(not(unix))]
        {
            Err(SyncwebError::operation(
                "daemon IPC is unavailable",
                "Unix sockets are not supported on this platform",
            ))
        }
    }

    async fn handle_status(&self) -> IpcResponse {
        IpcResponse::Status(self.daemon_handle.state.read().await.status)
    }

    async fn handle_list_folders(&self) -> IpcResponse {
        let folders = self.daemon_handle.folder_registry.read().await.statuses();
        IpcResponse::FolderList(folders)
    }

    fn handle_set_log_level(level: &str) -> IpcResponse {
        IpcResponse::Ok {
            message: format!("log level set to {level}"),
        }
    }

    async fn handle_import_archive_response(
        &self,
        input: PathBuf,
        target: PathBuf,
        filter: Option<FilterConfig>,
    ) -> IpcResponse {
        match self.handle_import_archive(input, target, filter).await {
            Ok(result) => IpcResponse::ImportComplete(Box::new(result)),
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_export_archive_response(
        &self,
        namespace: String,
        version: Option<String>,
        output: PathBuf,
    ) -> IpcResponse {
        match self.handle_export_archive(namespace, version, output).await {
            Ok(result) => IpcResponse::ExportComplete(Box::new(result)),
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_network_group(&self, cmd: IpcCommand) -> IpcResponse {
        if let IpcCommand::NetworkInvite { network_id, device } = cmd {
            return self.handle_network_invite(network_id, device).await;
        }
        if let IpcCommand::NetworkKick { network_id, device } = cmd {
            return self.handle_network_kick(network_id, device).await;
        }
        if let IpcCommand::NetworkLeave { network_id } = cmd {
            return self.handle_network_leave(network_id).await;
        }
        if let IpcCommand::NetworkCreate {
            name,
            label,
            invite_only,
            doc_ticket,
        } = cmd
        {
            return self.handle_network_create(name, label, invite_only, doc_ticket).await;
        }
        if let IpcCommand::NetworkJoin { ticket } = cmd {
            return self.handle_network_join(ticket).await;
        }
        IpcResponse::Error {
            message: format!("unhandled network command: {cmd:?}"),
        }
    }

    async fn handle_simple_group(&self, cmd: IpcCommand) -> IpcResponse {
        if matches!(cmd, IpcCommand::Status) {
            return self.handle_status().await;
        }
        if matches!(cmd, IpcCommand::ListFolders) {
            return self.handle_list_folders().await;
        }
        if matches!(cmd, IpcCommand::ReloadConfig) {
            return self.handle_reload_config();
        }
        IpcResponse::Error {
            message: format!("unhandled simple command: {cmd:?}"),
        }
    }

    /// Handle one decoded request without requiring a socket.
    pub async fn handle_request(&self, request: IpcRequest) -> IpcResponse {
        use IpcCommand as C;
        match request.command {
            C::Status | C::ListFolders | C::ReloadConfig => self.handle_simple_group(request.command).await,
            C::AddFolder { namespace, path } => self.handle_add_folder(namespace, path).await,
            C::TriggerSync { namespace } => self.handle_trigger_sync(namespace),
            C::SetLogLevel { level } => Self::handle_set_log_level(&level),
            C::Shutdown { force } => self.handle_shutdown(force),
            C::ImportArchive { input, target, filter } => {
                self.handle_import_archive_response(input, target, filter).await
            }
            C::ImportFiles { namespace, path } => self.handle_import_files_response(namespace, path).await,
            C::ExportArchive {
                namespace,
                version,
                output,
            } => self.handle_export_archive_response(namespace, version, output).await,
            C::Download { namespace, strategy } => self.handle_download_response(namespace, strategy).await,
            C::MaterializeTransfers { namespace } => self.handle_materialize_transfers(namespace).await,
            C::Join {
                ticket,
                path,
                mode,
                subscribe,
                filters,
                download,
                indexing,
                metadata_only,
            } => {
                let options = JoinOptions {
                    subscribe,
                    download,
                    indexing,
                    metadata_only,
                };
                self.handle_join(ticket, path, mode, filters, options).await
            }
            C::SetSubscribe {
                namespace,
                enabled,
                filters,
            } => self.handle_set_subscribe(namespace, enabled, filters).await,
            C::SubscribePublic { ticket } => self.handle_subscribe_public(ticket).await,
            C::CreateFolder { path, mode, indexing } => self.handle_create_folder(path, mode, indexing).await,
            C::StatsFiles { folder } => self.handle_stats_files(folder).await,
            C::ListEntries { folder, enrich } => self.handle_list_entries(&folder, enrich).await,
            C::VerifyIntegrity {
                path,
                hash,
                path_filter,
                glob_filter,
                fix,
                from,
            } => {
                self.handle_verify_integrity(path, hash, path_filter, glob_filter, fix, from)
                    .await
            }
            C::Unsubscribe { namespace } => self.handle_unsubscribe_command(&namespace).await,
            C::LeaveFolder {
                namespace,
                delete_files,
            } => self.handle_leave_folder(namespace, delete_files).await,
            C::Share { .. } | C::ShareList | C::Unshare { .. } => self.handle_share_group(request.command).await,
            C::SnapshotCreate {
                path,
                description,
                threads,
            } => self.handle_snapshot_create(path, description, threads).await,
            C::SnapshotList { path } => self.handle_snapshot_list(path).await,
            C::SnapshotDelete { id } => self.handle_snapshot_delete(id).await,
            C::CollectionPublish {
                path,
                namespace,
                sequence,
                bootstrap,
                manifest_bytes,
            } => {
                self.handle_collection_publish(path, namespace, sequence, bootstrap, manifest_bytes)
                    .await
            }
            C::EnrichSort { path } => self.handle_enrich_sort(path).await,
            C::NetworkInvite { .. }
            | C::NetworkKick { .. }
            | C::NetworkLeave { .. }
            | C::NetworkCreate { .. }
            | C::NetworkJoin { .. } => self.handle_network_group(request.command).await,
            C::PeerAvailability { folder } => self.handle_peer_availability(folder).await,
        }
    }

    async fn handle_add_folder(&self, namespace: String, path: PathBuf) -> IpcResponse {
        match iroh_docs::NamespaceId::from_str(&namespace) {
            Ok(namespace_id) => {
                let mut registry = self.daemon_handle.folder_registry.write().await;
                match registry.add_or_update(FolderEntry::new(namespace_id, path)) {
                    Ok(()) => IpcResponse::Ok {
                        message: "folder added".to_owned(),
                    },
                    Err(error) => response_from_error(error),
                }
            }
            Err(error) => IpcResponse::Error {
                message: format!("invalid folder namespace: {error}"),
            },
        }
    }

    async fn handle_leave_folder(&self, namespace: String, delete_files: bool) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon leave-folder IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let namespace_id = match iroh_docs::NamespaceId::from_str(&namespace) {
            Ok(id) => id,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("invalid namespace: {error}"),
                };
            }
        };
        let _ = cancel_session(namespace_id);
        let manager = FolderManager::new(&context.node);
        match manager.drop_when_ready(namespace_id).await {
            Ok(()) => {
                if let Some(node_db) = self.node_db.clone()
                    && let Ok(mut config) = node_db.load_app_config()
                {
                    config.remove_subscribe(&namespace);
                    let _ = node_db.save_app_config(&config);
                }
                let removed = self.daemon_handle.folder_registry.write().await.remove(&namespace_id);
                if let Some(ref node_db) = self.node_db
                    && let Err(error) = node_db.remove_folder_mount(&namespace)
                {
                    tracing::warn!(%error, %namespace, "failed to remove folder mount");
                }
                if delete_files
                    && let Some(entry) = removed.as_ref()
                    && !entry.path.as_os_str().is_empty()
                    && let Err(error) = FolderManager::delete_folder_files(&entry.path).await
                {
                    return IpcResponse::Error {
                        message: format!("folder left, but failed to delete its files: {error}"),
                    };
                }
                IpcResponse::Ok {
                    message: format!("left: {namespace}"),
                }
            }
            Err(error) => response_from_error(error),
        }
    }

    fn handle_trigger_sync(&self, namespace: Option<String>) -> IpcResponse {
        match self.daemon_handle.sync_trigger.send(namespace) {
            Ok(()) => IpcResponse::Ok {
                message: "synchronization requested".to_owned(),
            },
            Err(error) => response_from_error(error),
        }
    }

    fn handle_reload_config(&self) -> IpcResponse {
        self.daemon_handle.reload_requested.store(true, Ordering::Release);
        if self.daemon_handle.sync_trigger.send(None).is_err() {
            tracing::debug!("daemon reload wake-up channel is not connected");
        }
        IpcResponse::Ok {
            message: "configuration reload requested".to_owned(),
        }
    }

    fn handle_shutdown(&self, force: bool) -> IpcResponse {
        if let Err(error) = self.daemon_handle.shutdown_sender.send(()) {
            return response_from_error(error);
        }
        IpcResponse::Ok {
            message: if force {
                "forced shutdown requested".to_owned()
            } else {
                "shutdown requested".to_owned()
            },
        }
    }

    async fn handle_download_response(&self, namespace: String, strategy: FetchStrategy) -> IpcResponse {
        match self.handle_download(namespace, strategy).await {
            Ok(bytes_transferred) => IpcResponse::DownloadComplete { bytes_transferred },
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_materialize_transfers(&self, namespace: Option<String>) -> IpcResponse {
        let Some(context) = self.archive_context.clone() else {
            return IpcResponse::Error {
                message: "transfer materialization requires daemon node context".to_owned(),
            };
        };
        let Some(node_db) = self.node_db.clone() else {
            return IpcResponse::Error {
                message: "transfer materialization requires node database".to_owned(),
            };
        };
        let summary = match super::transfers::process_transfer_jobs(
            context.node.as_ref(),
            &node_db,
            namespace.as_deref(),
        )
        .await
        {
            Ok(summary) => summary,
            Err(error) => return response_from_error(error),
        };
        IpcResponse::TransferJobsProcessed {
            completed: summary.completed,
            failed: summary.failed,
        }
    }

    async fn handle_import_files_response(&self, namespace: Option<String>, path: PathBuf) -> IpcResponse {
        match self.handle_import_files(namespace, path).await {
            Ok(entries) => IpcResponse::ImportFilesComplete { entries },
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_download(&self, namespace: String, strategy: FetchStrategy) -> Result<u64> {
        self.handle_download_with_timeout(namespace, strategy, DOWNLOAD_TIMEOUT)
            .await
    }

    async fn handle_download_with_timeout(
        &self,
        namespace: String,
        strategy: FetchStrategy,
        timeout_duration: Duration,
    ) -> Result<u64> {
        let context = self.archive_context.clone().ok_or_else(|| {
            SyncwebError::operation("daemon download IPC is unavailable", "server has no node context")
        })?;
        let namespace_id = iroh_docs::NamespaceId::from_str(&namespace)
            .map_err(|error| SyncwebError::operation("invalid download namespace", error))?;
        let manager = FolderManager::new(&context.node);
        let folder = manager.get(namespace_id).await?;
        // A metadata-only folder keeps content out of the store; a doc re-sync
        // will not re-download it, so an explicit download fetches each selected
        // blob directly from the folder's peers.
        if folder.is_metadata_only().await? {
            return crate::sync::fetch_selected_content(&context.node, &folder, &strategy).await;
        }
        let sync = SyncEngine::new(
            manager,
            context.node.blob_store().clone(),
            context.node.docs_engine().clone(),
            Some(context.node.topic_tracker().clone()),
        );
        let mut intent = sync.fetch(namespace_id, strategy).await?;
        let bytes_transferred = if timeout_duration == DOWNLOAD_TIMEOUT {
            self.run_download_loop(&mut intent).await?
        } else {
            self.run_download_loop_with_timeout(&mut intent, timeout_duration)
                .await?
        };
        Ok(bytes_transferred)
    }

    async fn run_download_loop(&self, intent: &mut crate::sync::IntentHandle) -> Result<u64> {
        self.run_download_loop_with_timeout(intent, DOWNLOAD_TIMEOUT).await
    }

    async fn run_download_loop_with_timeout(
        &self,
        intent: &mut crate::sync::IntentHandle,
        timeout_duration: Duration,
    ) -> Result<u64> {
        let mut bytes_transferred = 0_u64;

        let loop_body = async {
            while let Some(event) = intent.next().await {
                match event {
                    SyncEvent::Stats(stats) => {
                        bytes_transferred = bytes_transferred.max(stats.bytes_transferred);
                    }
                    SyncEvent::Failed(message) => {
                        return Err(SyncwebError::operation("daemon download failed", message));
                    }
                    SyncEvent::Finished => return Ok(()),
                    SyncEvent::Started
                    | SyncEvent::Progress { .. }
                    | SyncEvent::Paused
                    | SyncEvent::Resumed
                    | SyncEvent::Cancelled => {}
                }
            }
            Ok(())
        };

        #[cfg(unix)]
        {
            use tokio::time::timeout;
            match timeout(timeout_duration, loop_body).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => return Err(error),
                Err(_elapsed) => {
                    let _ = intent.cancel();
                }
            }
        }

        #[cfg(not(unix))]
        {
            let _ = timeout_duration;
            loop_body.await?;
        }

        Ok(bytes_transferred)
    }

    async fn handle_import_files(&self, namespace: Option<String>, path: PathBuf) -> Result<u64> {
        let context = self.archive_context.clone().ok_or_else(|| {
            SyncwebError::operation("daemon filesystem import is unavailable", "server has no node context")
        })?;
        let namespace_id = if let Some(value) = namespace {
            iroh_docs::NamespaceId::from_str(&value)
                .map_err(|error| SyncwebError::operation("invalid import namespace", error))?
        } else {
            let folders = self.daemon_handle.folder_registry.read().await.statuses();
            let [folder] = folders.as_slice() else {
                return Err(SyncwebError::operation(
                    "cannot infer import namespace",
                    "specify a folder when more than one folder is managed",
                ));
            };
            iroh_docs::NamespaceId::from_str(&folder.namespace)
                .map_err(|error| SyncwebError::operation("invalid managed folder namespace", error))?
        };
        let folder = FolderManager::new(&context.node).get(namespace_id).await?;
        let root = if path.is_dir() {
            path.clone()
        } else {
            path.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf)
        };
        let importer = Importer::new(
            context.node.blob_store().clone(),
            context.node.docs_engine().clone(),
            folder.doc().clone(),
            folder.author(),
        )
        .with_root(root);
        let entries = importer.import_path(path).await?;
        u64::try_from(entries.len()).map_err(|error| SyncwebError::operation("import entry count overflowed", error))
    }

    async fn handle_import_archive(
        &self,
        input: PathBuf,
        target: PathBuf,
        filter: Option<FilterConfig>,
    ) -> Result<DropImportResult> {
        let context = self.archive_context.clone().ok_or_else(|| {
            SyncwebError::operation("daemon archive IPC is unavailable", "server has no node context")
        })?;
        let filter_engine = filter.map(FilterEngine::new).transpose()?;
        let options = filter_engine.map_or_else(DropImportOptions::default, |value| {
            DropImportOptions::default().with_filter(value)
        });
        let importer = DropImporter::new(context.node.blob_store().clone());
        let mut result = importer
            .import_archive(&input, options, Some(context.pool.as_ref()))
            .await?;
        importer.materialize(&result, &target).await?;
        let folder = FolderManager::new(&context.node).create(SyncMode::SendReceive).await?;
        CollectionStore::for_node(&context.node, &folder)
            .publish(&result.collection_manifest, 1)
            .await?;
        result.namespace_id = Some(folder.namespace_id());
        Ok(result)
    }

    async fn handle_export_archive(
        &self,
        namespace: String,
        version: Option<String>,
        output: PathBuf,
    ) -> Result<DropExportResult> {
        let context = self.archive_context.clone().ok_or_else(|| {
            SyncwebError::operation("daemon archive IPC is unavailable", "server has no node context")
        })?;
        let namespace_id = iroh_docs::NamespaceId::from_str(&namespace)
            .map_err(|error| SyncwebError::operation("invalid export namespace", error))?;
        let folder = FolderManager::new(&context.node).get(namespace_id).await?;
        let head = Self::latest_collection_head(&context.node, folder.doc()).await?;
        let manifests = Self::collection_manifests(&context.node, folder.doc(), head).await?;
        let options = version.map_or_else(DropExportOptions::default, |value| {
            DropExportOptions::default().with_version(value)
        });
        DropExporter::new(context.node.blob_store().clone())
            .export_drop_with_options(&manifests, output, options, Some(context.pool.as_ref()))
            .await
    }

    async fn register_folder(
        &self,
        namespace_id: iroh_docs::NamespaceId,
        namespace: &str,
        path: &Path,
        mode: SyncMode,
    ) {
        if self
            .daemon_handle
            .folder_registry
            .write()
            .await
            .add(FolderEntry::new(namespace_id, path.to_path_buf()).with_mode(mode.to_string()))
            .is_err()
        {
            tracing::warn!(%namespace, "folder already in daemon registry");
        }
        if let Some(ref node_db) = self.node_db
            && let Err(error) = node_db.upsert_folder_mount(namespace, path)
        {
            tracing::warn!(%error, %namespace, "failed to record folder mount");
        }
    }

    async fn handle_join(
        &self,
        ticket: String,
        path: PathBuf,
        mode: SyncMode,
        filters: SubscribeFilters,
        options: JoinOptions,
    ) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon join IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        match tokio::fs::create_dir_all(&path).await {
            Ok(()) => {}
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("failed to create folder path: {error}"),
                };
            }
        }
        let manager = FolderManager::new(&context.node);
        match manager.join_with_options(ticket, mode, options.metadata_only).await {
            Ok(folder) => {
                let namespace = folder.namespace_id().to_string();
                self.register_folder(folder.namespace_id(), &namespace, &path, mode)
                    .await;
                if options.indexing
                    && let Some(indexing_service) = &context.indexing
                    && let Err(error) = indexing_service.enable_folder(&folder).await
                {
                    tracing::warn!(%error, %namespace, "failed to enable indexing for joined folder");
                }
                self.finalize_join(&context, &manager, &folder, &filters, &path, options)
                    .await
            }
            Err(error) => response_from_error(error),
        }
    }

    async fn finalize_join(
        &self,
        context: &ArchiveContext,
        manager: &FolderManager,
        folder: &crate::folder::SyncwebFolder,
        filters: &SubscribeFilters,
        path: &Path,
        options: JoinOptions,
    ) -> IpcResponse {
        let namespace = folder.namespace_id().to_string();
        if let Some(ref node_db) = self.node_db {
            let mut config = match node_db.load_app_config() {
                Ok(config) => config,
                Err(error) => return response_from_error(error),
            };
            config.set_subscribe(&namespace, options.subscribe, filters);
            if let Err(error) = node_db.save_app_config(&config) {
                return response_from_error(error);
            }
        }
        if options.subscribe {
            let sync = SyncEngine::new(
                manager.clone(),
                context.node.blob_store().clone(),
                context.node.docs_engine().clone(),
                Some(context.node.topic_tracker().clone()),
            );
            let params = match SubscribeParams::from_filters(filters) {
                Ok(params) => params,
                Err(error) => return response_from_error(error),
            };
            if let Err(error) = sync.subscribe(folder.namespace_id(), params).await {
                return response_from_error(error);
            }
        }
        let downloaded = if options.download {
            match self.materialize_folder(context, manager, folder, filters, path).await {
                Ok(count) => count,
                Err(error) => return response_from_error(error),
            }
        } else {
            0
        };
        IpcResponse::Ok {
            message: if options.download {
                format!("joined: {namespace}\ndownloaded: {downloaded} files")
            } else {
                format!("joined: {namespace}")
            },
        }
    }

    async fn materialize_folder(
        &self,
        context: &ArchiveContext,
        manager: &FolderManager,
        folder: &crate::folder::SyncwebFolder,
        filters: &SubscribeFilters,
        destination: &Path,
    ) -> Result<usize> {
        let sync = SyncEngine::new(
            manager.clone(),
            context.node.blob_store().clone(),
            context.node.docs_engine().clone(),
            Some(context.node.topic_tracker().clone()),
        );
        let strategy =
            FetchStrategy::Filter(FetchFilter::new().with_paths(filters.sync_prefix.clone().into_iter().collect()));
        let mut intent = sync.fetch(folder.namespace_id(), strategy).await?;
        self.run_download_loop_with_timeout(&mut intent, DOWNLOAD_TIMEOUT)
            .await?;
        let area = filters
            .sync_prefix
            .clone()
            .map(AreaFilter::Prefix)
            .or_else(|| filters.glob.clone().map(AreaFilter::Glob))
            .unwrap_or(AreaFilter::All);
        let entries = folder.list_entries().await?;
        let mut count = 0_usize;
        for entry in entries {
            let rel = Path::new(&entry.path);
            if !area.matches_path(rel) {
                continue;
            }
            let dest = destination.join(rel);
            if let Some(parent) = dest.parent()
                && let Err(error) = tokio::fs::create_dir_all(parent).await
            {
                return Err(SyncwebError::operation("failed to create download directory", error));
            }
            context.node.blob_store().export_to_path(entry.hash, &dest).await?;
            count = count.saturating_add(1);
        }
        Ok(count)
    }

    async fn handle_set_subscribe(
        &self,
        namespace: String,
        enabled: bool,
        requested_filters: Option<SubscribeFilters>,
    ) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon subscribe IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let namespace_id = match iroh_docs::NamespaceId::from_str(&namespace) {
            Ok(id) => id,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("invalid namespace: {error}"),
                };
            }
        };
        let filters = requested_filters.unwrap_or_default();
        if let Some(ref node_db) = self.node_db {
            let mut config = match node_db.load_app_config() {
                Ok(config) => config,
                Err(error) => return response_from_error(error),
            };
            if enabled {
                config.set_subscribe(&namespace, true, &filters);
            } else {
                config.remove_subscribe(&namespace);
            }
            if let Err(error) = node_db.save_app_config(&config) {
                return response_from_error(error);
            }
        }
        if enabled {
            let manager = FolderManager::new(&context.node);
            let sync = SyncEngine::new(
                manager,
                context.node.blob_store().clone(),
                context.node.docs_engine().clone(),
                Some(context.node.topic_tracker().clone()),
            );
            let params = match SubscribeParams::from_filters(&filters) {
                Ok(params) => params,
                Err(error) => return response_from_error(error),
            };
            match sync.subscribe(namespace_id, params).await {
                Ok(_intent) => IpcResponse::Ok {
                    message: format!("subscribed: {namespace}"),
                },
                Err(error) => response_from_error(error),
            }
        } else {
            let _ = cancel_session(namespace_id);
            IpcResponse::Ok {
                message: format!("unsubscribed: {namespace}"),
            }
        }
    }

    async fn handle_subscribe_public(&self, ticket: String) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon subscribe-public IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let blob_ticket = match ticket.parse::<iroh_blobs::ticket::BlobTicket>() {
            Ok(t) => t,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("invalid blob ticket: {error}"),
                };
            }
        };
        let manager = FolderManager::new(&context.node);
        match manager.subscribe_public(&blob_ticket).await {
            Ok(hash) => {
                let provider = Some(blob_ticket.addr().clone());
                let size = match context.node.blob_store().get(hash).await {
                    Ok(bytes) => u64::try_from(bytes.len()).unwrap_or_default(),
                    Err(error) => return response_from_error(error),
                };
                let subscription = PublicSubscription::new(hash, provider, size);
                let namespace = subscription.namespace_id();
                self.daemon_handle
                    .folder_registry
                    .write()
                    .await
                    .add_subscription(subscription);
                if let Some(ref node_db) = self.node_db
                    && let Err(error) = node_db.save_subscription(&hash, size)
                {
                    tracing::warn!(%hash, %error, "failed to persist subscription");
                }
                IpcResponse::Ok {
                    message: format!("subscribed: {namespace}\nhash: {hash}\nsize: {size}"),
                }
            }
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_create_folder(&self, path: PathBuf, mode: String, indexing: bool) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon create-folder IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        match std::fs::create_dir_all(&path) {
            Ok(()) => {}
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("failed to create directory: {error}"),
                };
            }
        }
        let sync_mode = match SyncMode::from_str(&mode) {
            Ok(m) => m,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("invalid sync mode: {error}"),
                };
            }
        };
        let manager = FolderManager::new(&context.node);
        match manager.create(sync_mode).await {
            Ok(folder) => {
                let namespace = folder.namespace_id().to_string();
                if indexing
                    && let Some(indexing_service) = &context.indexing
                    && let Err(error) = indexing_service.enable_folder(&folder).await
                {
                    tracing::warn!(%error, %namespace, "failed to enable indexing for new folder");
                }
                self.register_folder(folder.namespace_id(), &namespace, &path, sync_mode)
                    .await;
                match folder.ticket(true).await {
                    Ok(_ticket) => IpcResponse::Ok {
                        message: format!("namespace: {namespace}"),
                    },
                    Err(error) => response_from_error(error),
                }
            }
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_stats_files(&self, folder_path: PathBuf) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon stats-files IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let manager = FolderManager::new(&context.node);
        let folder = match resolve_folder_for_daemon(&manager, &folder_path).await {
            Ok(f) => f,
            Err(error) => return error,
        };
        let entries = match folder.content_entries().await {
            Ok(e) => e,
            Err(error) => return response_from_error(error),
        };
        let mut collector = FileStatsCollector::new();
        for entry in entries {
            collector.add_entry_bytes_with_time(entry.key(), entry.content_len(), Some(entry.timestamp()));
        }
        IpcResponse::FileStats(Box::new(collector.report()))
    }

    async fn handle_list_entries(&self, namespace: &str, enrich: bool) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon list-entries IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let namespace_id = match iroh_docs::NamespaceId::from_str(namespace) {
            Ok(id) => id,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("invalid namespace: {error}"),
                };
            }
        };
        let manager = FolderManager::new(&context.node);
        let folder = match manager.get(namespace_id).await {
            Ok(folder) => folder,
            Err(error) => return response_from_error(error),
        };
        let mount_root = self.daemon_handle.folder_registry.read().await.path_for(namespace);
        let entries = match folder.list_entries().await {
            Ok(entries) => entries,
            Err(error) => return response_from_error(error),
        };
        let mut rows = Vec::with_capacity(entries.len());
        for entry in entries {
            let local = match folder.has_local(entry.hash).await {
                Ok(local) => local,
                Err(error) => return response_from_error(error),
            };
            let (size, modified) = if enrich
                && local
                && let Some(root) = mount_root.as_deref()
                && let Ok(metadata) = std::fs::metadata(root.join(&entry.path))
            {
                (
                    metadata.len(),
                    metadata
                        .modified()
                        .ok()
                        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|duration| duration.as_secs()),
                )
            } else {
                (entry.size, None)
            };
            rows.push(EntryRow {
                path: entry.path,
                hash: entry.hash,
                size,
                local,
                modified,
            });
        }
        IpcResponse::Entries(rows)
    }

    async fn handle_enrich_sort(&self, path: PathBuf) -> IpcResponse {
        let Some(context) = self.archive_context.clone() else {
            return IpcResponse::EnrichData(HashMap::new());
        };
        let manager = FolderManager::new(&context.node);
        let Ok(folder) = resolve_folder_for_daemon(&manager, &path).await else {
            return IpcResponse::EnrichData(HashMap::new());
        };
        let Ok(entries) = context.node.docs_engine().list_latest(folder.doc()).await else {
            return IpcResponse::EnrichData(HashMap::new());
        };
        let mut path_map: Vec<(String, iroh_blobs::Hash)> = Vec::new();
        for entry in entries {
            if entry.key().starts_with(b"sys/") {
                continue;
            }
            let Ok(path_str) = String::from_utf8(entry.key().to_vec()) else {
                continue;
            };
            let hash = entry.content_hash();
            path_map.push((path_str, hash));
        }

        let peers_per_hash: HashMap<iroh_blobs::Hash, usize> = HashMap::new();

        let result: HashMap<String, usize> = path_map
            .into_iter()
            .map(|(path_str, hash)| {
                let count = peers_per_hash.get(&hash).copied().unwrap_or(0);
                (path_str, count)
            })
            .collect();
        IpcResponse::EnrichData(result)
    }

    /// Read-only aggregation of which peers joined a folder and which of its
    /// blobs each peer can serve. No new state: the peer set is drawn from the
    /// network manager's membership records and the per-blob counts from the
    /// (currently empty) `EnrichSort` peer map.
    async fn handle_peer_availability(&self, folder_selection: String) -> IpcResponse {
        let Some(context) = self.archive_context.clone() else {
            return IpcResponse::Error {
                message: "daemon peer-availability IPC is unavailable: server has no node context".to_owned(),
            };
        };
        let manager = FolderManager::new(&context.node);
        let folder = match resolve_folder_for_daemon(&manager, Path::new(&folder_selection)).await {
            Ok(folder) => folder,
            Err(error) => return error,
        };
        let namespace_id = folder.namespace_id();
        let namespace_str = namespace_id.to_string();

        // Inbound peers: members of the networks that contain this folder.
        let mut peers: Vec<PeerInfo> = Vec::new();
        if let Some(net_mgr_ref) = self.network_manager.as_ref() {
            let member_devices = {
                let guard = net_mgr_ref.read().await;
                let local_node = *guard.local_node();
                let mut member_devices: Vec<String> = Vec::new();
                if let Ok(network_ids) = guard.networks_for_folder(&namespace_id) {
                    for network_id in network_ids {
                        let Ok(net_id) = network_id.parse::<crate::net::NetworkId>() else {
                            continue;
                        };
                        let Some(network) = guard.get(&net_id) else {
                            continue;
                        };
                        for member in &network.members {
                            if *member == local_node {
                                continue;
                            }
                            let device_id = crate::node::identity::DeviceId::from_node_id(*member).to_syncthing();
                            if !member_devices.contains(&device_id) {
                                member_devices.push(device_id);
                            }
                        }
                    }
                }
                drop(guard);
                member_devices
            };
            for device_id in member_devices {
                peers.push(PeerInfo {
                    device_id,
                    name: None,
                    connection: Some("member".to_owned()),
                });
            }
        }
        peers.sort_by(|left, right| left.device_id.cmp(&right.device_id));

        // Live gossip neighbors mark peers that are currently connected.
        if let Ok(live_peers) = context.node.topic_tracker().find_peers(namespace_id).await {
            let live: std::collections::HashSet<String> = live_peers
                .into_iter()
                .map(|peer| crate::node::identity::DeviceId::from_node_id(peer).to_syncthing())
                .collect();
            for peer in &mut peers {
                if live.contains(&peer.device_id) {
                    peer.connection = Some("online".to_owned());
                }
            }
        }

        // Per-blob counts reuse the empty `EnrichSort` map path: a blob's peers
        // are not tracked per-blob yet, so `peer_count` is a point-in-time 0.
        let Ok(entries) = context.node.docs_engine().list_latest(folder.doc()).await else {
            return IpcResponse::PeerAvailability(Box::new(PeerAvailabilityReport {
                folder: namespace_str,
                peers,
                per_blob: Vec::new(),
            }));
        };
        let mut per_blob = Vec::with_capacity(entries.len());
        for entry in entries {
            if entry.key().starts_with(b"sys/") {
                continue;
            }
            let Ok(path) = String::from_utf8(entry.key().to_vec()) else {
                continue;
            };
            let hash = entry.content_hash();
            let peer_count = 0;
            let pct_seeded = if peers.is_empty() {
                0.0
            } else {
                f64::from(i32::try_from(peer_count).unwrap_or(0))
                    .mul_add(f64::from(i32::try_from(peers.len()).unwrap_or(1)).recip(), 0.0)
                    .mul_add(100.0, 0.0)
            };
            per_blob.push(BlobPeerAvailability {
                path,
                hash: hash.to_string(),
                peer_count,
                peers: Vec::new(),
                pct_seeded,
            });
        }
        IpcResponse::PeerAvailability(Box::new(PeerAvailabilityReport {
            folder: namespace_str,
            peers,
            per_blob,
        }))
    }

    async fn handle_verify_integrity(
        &self,
        path: PathBuf,
        hash: Vec<String>,
        path_filter: Option<String>,
        glob_filter: Option<String>,
        fix: bool,
        from: Vec<String>,
    ) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon verify IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let manager = FolderManager::new(&context.node);
        let folder = match resolve_folder_for_daemon(&manager, &path).await {
            Ok(f) => f,
            Err(error) => return error,
        };
        let checker = IntegrityChecker::new(context.node.blob_store().clone(), context.node.docs_engine().clone());

        let filter = build_ipc_verify_filter(&hash, path_filter.as_ref(), glob_filter.as_ref());

        match checker.verify_folder_filtered(&folder, filter.as_ref()).await {
            Ok(result) => {
                let mut message = format!(
                    "total: {}, verified: {}, corrupted: {}, missing: {}",
                    result.total,
                    result.verified,
                    result.corrupted.len(),
                    result.missing.len(),
                );
                if fix {
                    let namespace_id = folder.namespace_id();
                    let repair = self
                        .run_daemon_repair(&context, namespace_id, &result.corrupted, &from)
                        .await;
                    let _ = write!(
                        message,
                        ", repair: attempted {}, repaired {}",
                        repair.attempted, repair.repaired,
                    );
                }
                let valid = result.is_valid();
                let _ = write!(message, ", valid: {valid}");
                IpcResponse::Ok { message }
            }
            Err(error) => response_from_error(error),
        }
    }

    async fn run_daemon_repair(
        &self,
        context: &ArchiveContext,
        namespace_id: iroh_docs::NamespaceId,
        corrupted: &[crate::verify::CorruptionInfo],
        tickets: &[String],
    ) -> crate::verify::RepairResult {
        let mut result = crate::verify::RepairResult::default();
        if corrupted.is_empty() {
            return result;
        }

        let provider_tickets: Vec<(iroh_blobs::Hash, iroh_blobs::ticket::BlobTicket)> = tickets
            .iter()
            .filter_map(|t| {
                let ticket: iroh_blobs::ticket::BlobTicket = t.parse().ok()?;
                Some((ticket.hash(), ticket))
            })
            .collect();

        let namespace_peers = context
            .node
            .topic_tracker()
            .find_peers(namespace_id)
            .await
            .unwrap_or_default();

        for item in corrupted {
            result.attempted = result.attempted.saturating_add(1);
            let hash = item.expected_hash;

            // Try --from tickets first, then namespace peers.
            let candidate_tickets: Vec<iroh_blobs::ticket::BlobTicket> = provider_tickets
                .iter()
                .filter(|(ticket_hash, _)| *ticket_hash == hash)
                .map(|(_, ticket)| ticket.clone())
                .collect();
            let repaired = crate::node::blob_store::try_fetch_first_working(&candidate_tickets, |ticket| {
                context.node.blob_store().force_fetch(context.node.endpoint(), ticket)
            })
            .await
            .is_ok()
                || crate::node::blob_store::try_fetch_first_working(&namespace_peers, |peer| async {
                    context
                        .node
                        .blob_store()
                        .force_fetch_from_peer(context.node.endpoint(), peer, hash)
                        .await
                })
                .await
                .is_ok();
            if repaired {
                result.repaired = result.repaired.saturating_add(1);
            } else {
                result.failed.push(crate::verify::RepairOutcome {
                    path: item.path.clone(),
                    hash,
                    success: false,
                    error: Some("no alternative providers available".to_owned()),
                });
            }
        }
        result
    }

    async fn handle_unsubscribe_command(&self, namespace: &str) -> IpcResponse {
        if !namespace.starts_with("blob:") {
            return IpcResponse::Error {
                message: "unsubscribe applies only to public blob subscriptions; use `leave` for folders".to_owned(),
            };
        }
        self.daemon_handle
            .folder_registry
            .write()
            .await
            .remove_subscription(namespace);
        if let Some(ref manager) = self.folder_manager
            && let Some(hash_str) = namespace.strip_prefix("blob:")
            && let Ok(hash) = hash_str.parse::<iroh_blobs::Hash>()
        {
            manager.drop_subscription(&hash).await;
        }
        if let Some(ref node_db) = self.node_db
            && let Some(hash_str) = namespace.strip_prefix("blob:")
            && let Ok(hash) = hash_str.parse::<iroh_blobs::Hash>()
            && let Err(error) = node_db.remove_subscription(&hash)
        {
            tracing::warn!(%hash, %error, "failed to remove subscription from database");
        }
        IpcResponse::Ok {
            message: format!("unsubscribed: {namespace}"),
        }
    }

    async fn handle_share_group(&self, cmd: IpcCommand) -> IpcResponse {
        if let IpcCommand::Share {
            namespace,
            blob,
            writable,
            pin,
            persist,
        } = cmd
        {
            let options = ShareOptions { writable, pin, persist };
            return self.handle_share(namespace, blob, options).await;
        }
        if matches!(cmd, IpcCommand::ShareList) {
            return self.handle_share_list();
        }
        if let IpcCommand::Unshare {
            namespace,
            blob,
            writable,
        } = cmd
        {
            return self.handle_unshare(namespace, blob, writable).await;
        }
        IpcResponse::Error {
            message: format!("unhandled share command: {cmd:?}"),
        }
    }

    async fn handle_share(&self, namespace: String, blob: Option<String>, options: ShareOptions) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon share IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let namespace_id = match iroh_docs::NamespaceId::from_str(&namespace) {
            Ok(id) => id,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("invalid namespace: {error}"),
                };
            }
        };
        let manager = FolderManager::new(&context.node);
        let folder = match manager.get(namespace_id).await {
            Ok(f) => f,
            Err(error) => return response_from_error(error),
        };
        if let Some(hash_str) = blob {
            let hash = match hash_str.parse::<iroh_blobs::Hash>() {
                Ok(h) => h,
                Err(error) => {
                    return IpcResponse::Error {
                        message: format!("invalid blob hash: {error}"),
                    };
                }
            };
            return match folder.publish_blob(context.node.endpoint().addr(), hash).await {
                Ok(ticket) => IpcResponse::Ok {
                    message: ticket.to_string(),
                },
                Err(error) => response_from_error(error),
            };
        }
        let result = match share_folder(&folder, options, |ns, access, ticket| {
            if let Some(node_db) = &self.node_db {
                node_db.add_share(&ns.to_string(), access, &ticket.to_string())?;
            }
            Ok(())
        })
        .await
        {
            Ok(result) => result,
            Err(error) => return response_from_error(error),
        };
        IpcResponse::Ok { message: result.url }
    }

    fn handle_share_list(&self) -> IpcResponse {
        let Some(node_db) = &self.node_db else {
            return IpcResponse::Error {
                message: "daemon share-list IPC is unavailable: no node database".to_owned(),
            };
        };
        match node_db.list_shares() {
            Ok(shares) => {
                if shares.is_empty() {
                    return IpcResponse::Ok {
                        message: "no shares".to_owned(),
                    };
                }
                let mut lines = Vec::new();
                for (namespace, access, ticket) in shares {
                    lines.push(format!("namespace: {namespace}\naccess: {access}\nticket: {ticket}"));
                }
                IpcResponse::Ok {
                    message: lines.join("\n\n"),
                }
            }
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_unshare(&self, namespace: String, blob: Option<String>, writable: bool) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon unshare IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let namespace_id = match iroh_docs::NamespaceId::from_str(&namespace) {
            Ok(id) => id,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("invalid namespace: {error}"),
                };
            }
        };
        let manager = FolderManager::new(&context.node);
        if let Some(hash_str) = blob {
            let hash = match hash_str.parse::<iroh_blobs::Hash>() {
                Ok(h) => h,
                Err(error) => {
                    return IpcResponse::Error {
                        message: format!("invalid blob hash: {error}"),
                    };
                }
            };
            let folder = match manager.get(namespace_id).await {
                Ok(f) => f,
                Err(error) => return response_from_error(error),
            };
            return match folder.unpublish_blob(hash).await {
                Ok(()) => IpcResponse::Ok {
                    message: format!("unshared: {namespace_id} (blob {hash_str})"),
                },
                Err(error) => response_from_error(error),
            };
        }
        let access = if writable { "write" } else { "read" };
        if let Some(node_db) = &self.node_db
            && let Err(error) = node_db.remove_share(&namespace_id.to_string(), access)
        {
            return response_from_error(error);
        }
        if let Ok(folder) = manager.get(namespace_id).await {
            let _ = folder.unpin_all_content().await;
        }
        IpcResponse::Ok {
            message: format!("unshared: {namespace_id} ({access})"),
        }
    }

    async fn handle_snapshot_create(&self, path: PathBuf, description: Option<String>, threads: usize) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon snapshot IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let snapshots = SnapshotStore::with_docs(context.node.blob_store().clone(), context.node.docs_engine().clone());
        let result = if path.exists() {
            snapshots.create_from_path(&path, threads, description).await
        } else {
            let manager = FolderManager::new(&context.node);
            let folder = match resolve_folder_for_daemon(&manager, &path).await {
                Ok(f) => f,
                Err(error) => return error,
            };
            snapshots.create_for_folder(&folder, description).await
        };
        match result {
            Ok(snapshot) => IpcResponse::Ok {
                message: snapshot.id.to_string(),
            },
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_snapshot_list(&self, path: PathBuf) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon snapshot IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let snapshots = SnapshotStore::with_docs(context.node.blob_store().clone(), context.node.docs_engine().clone());
        let namespace = path.to_string_lossy().parse::<iroh_docs::NamespaceId>().ok();
        match snapshots.list().await {
            Ok(all) => {
                let count = all
                    .into_iter()
                    .filter(|s| namespace.is_none_or(|id| s.namespace_id == Some(id)))
                    .count();
                IpcResponse::Ok {
                    message: format!("snapshots: {count}"),
                }
            }
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_snapshot_delete(&self, id: String) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon snapshot IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let hash = match id.parse::<iroh_blobs::Hash>() {
            Ok(h) => h,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("invalid snapshot id: {error}"),
                };
            }
        };
        let snapshots = SnapshotStore::with_docs(context.node.blob_store().clone(), context.node.docs_engine().clone());
        match snapshots.delete(hash).await {
            Ok(()) => IpcResponse::Ok {
                message: format!("deleted: {id}"),
            },
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_network_invite(&self, network_id: String, device: String) -> IpcResponse {
        let Some(ref net_mgr) = self.network_manager else {
            return IpcResponse::Error {
                message: "network manager not available in IPC server".to_owned(),
            };
        };
        let net_id = match network_id.parse::<crate::net::NetworkId>() {
            Ok(id) => id,
            Err(e) => return response_from_error(e),
        };
        let peer = match device.parse::<iroh::PublicKey>() {
            Ok(pk) => pk,
            Err(e) => {
                return IpcResponse::Error {
                    message: format!("invalid device ID: {e}"),
                };
            }
        };
        let result = net_mgr.write().await.invite(net_id, peer);
        match result {
            Ok(ticket) => IpcResponse::Ok {
                message: ticket.to_string(),
            },
            Err(e) => response_from_error(e),
        }
    }

    async fn handle_network_kick(&self, network_id: String, device: String) -> IpcResponse {
        let Some(ref net_mgr) = self.network_manager else {
            return IpcResponse::Error {
                message: "network manager not available in IPC server".to_owned(),
            };
        };
        let net_id = match network_id.parse::<crate::net::NetworkId>() {
            Ok(id) => id,
            Err(e) => return response_from_error(e),
        };
        let peer = match device.parse::<iroh::PublicKey>() {
            Ok(pk) => pk,
            Err(e) => {
                return IpcResponse::Error {
                    message: format!("invalid device ID: {e}"),
                };
            }
        };
        let result = net_mgr.write().await.kick(net_id, &peer);
        match result {
            Ok(()) => IpcResponse::Ok {
                message: "member kicked".to_owned(),
            },
            Err(e) => response_from_error(e),
        }
    }

    async fn handle_network_leave(&self, network_id: String) -> IpcResponse {
        let Some(ref net_mgr) = self.network_manager else {
            return IpcResponse::Error {
                message: "network manager not available in IPC server".to_owned(),
            };
        };
        let net_id = match network_id.parse::<crate::net::NetworkId>() {
            Ok(id) => id,
            Err(e) => return response_from_error(e),
        };
        let result = net_mgr.write().await.leave(net_id);
        match result {
            Ok(()) => IpcResponse::Ok {
                message: "left network".to_owned(),
            },
            Err(e) => response_from_error(e),
        }
    }

    async fn handle_network_create(
        &self,
        name: String,
        label: String,
        invite_only: bool,
        doc_ticket: Option<String>,
    ) -> IpcResponse {
        let Some(ref net_mgr) = self.network_manager else {
            return IpcResponse::Error {
                message: "network manager not available in IPC server".to_owned(),
            };
        };
        let options = crate::net::NetworkOptions {
            label,
            invite_only,
            ..crate::net::NetworkOptions::default()
        };
        let result = net_mgr.write().await.create_with_doc_ticket(&name, options, doc_ticket);
        match result {
            Ok(id) => IpcResponse::Ok {
                message: format!("created network {id}"),
            },
            Err(e) => response_from_error(e),
        }
    }

    async fn handle_network_join(&self, ticket: String) -> IpcResponse {
        let Some(ref net_mgr) = self.network_manager else {
            return IpcResponse::Error {
                message: "network manager not available in IPC server".to_owned(),
            };
        };
        let parsed = match ticket.parse::<crate::net::NetworkTicket>() {
            Ok(t) => t,
            Err(e) => return response_from_error(e),
        };
        let result = net_mgr.write().await.join(parsed);
        match result {
            Ok(id) => IpcResponse::Ok {
                message: format!("joined network {id}"),
            },
            Err(e) => response_from_error(e),
        }
    }

    async fn handle_collection_publish(
        &self,
        path: PathBuf,
        namespace: String,
        sequence: u64,
        _bootstrap: Vec<String>,
        manifest_bytes: Option<Vec<u8>>,
    ) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon collection-publish IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let manifest = match manifest_bytes {
            Some(bytes) => match CollectionManifest::from_bytes(bytes) {
                Ok(m) => m,
                Err(error) => return response_from_error(error),
            },
            None => {
                return IpcResponse::Error {
                    message: "collection publish requires manifest_bytes; run `package init` first".to_owned(),
                };
            }
        };
        for entry in &manifest.entries {
            let file_path = path.join(&entry.logical_path);
            match context.node.blob_store().add_file(&file_path).await {
                Ok(hash) => {
                    if hash != entry.content_id {
                        return IpcResponse::Error {
                            message: format!(
                                "collection content changed while publishing: {}",
                                entry.logical_path.display()
                            ),
                        };
                    }
                }
                Err(error) => return response_from_error(error),
            }
        }
        let namespace_id = match namespace.parse::<iroh_docs::NamespaceId>() {
            Ok(id) => id,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("invalid namespace: {error}"),
                };
            }
        };
        let manager = FolderManager::new(&context.node);
        let folder = match manager.get(namespace_id).await {
            Ok(f) => f,
            Err(error) => return response_from_error(error),
        };
        let store = CollectionStore::for_node(&context.node, &folder);
        let head = match store.publish(&manifest, sequence).await {
            Ok(h) => h,
            Err(error) => return response_from_error(error),
        };
        let ticket = context.node.blob_store().ticket(context.node.endpoint(), head.manifest);
        IpcResponse::Ok {
            message: format!(
                "manifest: {}\nmanifest_ticket: {}\nsequence: {}",
                head.manifest, ticket, head.sequence,
            ),
        }
    }

    #[cfg(unix)]
    async fn handle_connection(&self, stream: tokio::net::UnixStream) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let (read_half, mut write_half) = stream.into_split();
        let mut line = Vec::new();
        BufReader::new(read_half).read_until(b'\n', &mut line).await?;
        let response = match serde_json::from_slice::<IpcRequest>(line.trim_ascii()) {
            Ok(request) => self.handle_request(request).await,
            Err(error) => IpcResponse::Error {
                message: format!("invalid daemon request: {error}"),
            },
        };
        let mut bytes = serde_json::to_vec(&response)
            .map_err(|error| SyncwebError::operation("failed to serialize IPC response", error))?;
        bytes.push(b'\n');
        write_half.write_all(&bytes).await?;
        Ok(())
    }

    async fn latest_collection_head(node: &IrohNode, doc: &iroh_docs::api::Doc) -> Result<CollectionHead> {
        let entries = node.docs_engine().list_latest(doc).await?;
        let head_entry = entries
            .iter()
            .find(|entry| entry.key().starts_with(b"collections/") && entry.key().ends_with(b"/head"))
            .ok_or_else(|| SyncwebError::InvalidConfig("folder has no published collection head".to_owned()))?;
        let bytes = node.blob_store().get(head_entry.content_hash()).await?;
        let head = serde_json::from_slice(&bytes)
            .map_err(|error| SyncwebError::operation("failed to deserialize collection head", error))?;
        Ok(head)
    }

    async fn collection_manifests(
        node: &IrohNode,
        doc: &iroh_docs::api::Doc,
        head: CollectionHead,
    ) -> Result<Vec<CollectionManifest>> {
        let prefix = format!("collections/{}/manifests/", head.collection_id);
        let entries = node.docs_engine().list_latest(doc).await?;
        let mut manifests = Vec::new();
        for entry in entries {
            if !entry.key().starts_with(prefix.as_bytes()) {
                continue;
            }
            let bytes = node.blob_store().get(entry.content_hash()).await?;
            let manifest = CollectionManifest::from_bytes(bytes)?;
            manifests.push(manifest);
        }
        if manifests.is_empty() {
            return Err(SyncwebError::InvalidConfig(
                "folder has no published collection manifests".to_owned(),
            ));
        }
        Ok(manifests)
    }
}

fn response_from_error(error: impl std::fmt::Display) -> IpcResponse {
    IpcResponse::Error {
        message: error.to_string(),
    }
}

fn build_ipc_verify_filter(
    hash: &[String],
    path_filter: Option<&String>,
    glob_filter: Option<&String>,
) -> Option<crate::verify::VerifyFilter> {
    let has_filter = !hash.is_empty() || path_filter.is_some() || glob_filter.is_some();
    if !has_filter {
        return None;
    }
    let mut filter = crate::verify::VerifyFilter::new();
    if !hash.is_empty() {
        let hashes: Vec<iroh_blobs::Hash> = hash.iter().filter_map(|h| h.parse::<iroh_blobs::Hash>().ok()).collect();
        if !hashes.is_empty() {
            filter = filter.with_hashes(hashes);
        }
    }
    if let Some(p) = path_filter {
        filter = filter.with_path(std::path::PathBuf::from(p));
    }
    if let Some(g) = glob_filter {
        filter.glob = Some(g.clone());
    }
    Some(filter)
}

async fn resolve_folder_for_daemon(
    manager: &FolderManager,
    selector: &Path,
) -> std::result::Result<crate::folder::SyncwebFolder, IpcResponse> {
    manager.resolve(selector).await.map_err(|error| IpcResponse::Error {
        message: error.to_string(),
    })
}

/// A client for sending requests to the daemon.
#[derive(Clone, Debug)]
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    #[must_use]
    pub fn new(data_dir: &Path) -> Self {
        Self::from_socket_path(daemon_socket_path(data_dir))
    }

    #[must_use]
    pub const fn from_socket_path(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Send one newline-delimited JSON request and await its response.
    ///
    /// # Errors
    ///
    /// Returns an error when the socket is unavailable, the operation times
    /// out, or either JSON message is malformed.
    pub async fn send(&self, request: IpcRequest) -> Result<IpcResponse> {
        #[cfg(unix)]
        {
            use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
            use tokio::time::timeout;
            let mut stream = timeout(IPC_TIMEOUT, tokio::net::UnixStream::connect(&self.socket_path))
                .await
                .map_err(|error| SyncwebError::operation("daemon IPC connection timed out", error))?
                .map_err(|error| {
                    SyncwebError::operation(
                        format!("failed to connect to daemon socket at {}", self.socket_path.display()),
                        error,
                    )
                })?;
            let mut message = serde_json::to_vec(&request)
                .map_err(|error| SyncwebError::operation("failed to serialize IPC request", error))?;
            message.push(b'\n');
            timeout(IPC_TIMEOUT, stream.write_all(&message))
                .await
                .map_err(|error| SyncwebError::operation("daemon IPC write timed out", error))?
                .map_err(|error| SyncwebError::operation("daemon IPC write failed", error))?;
            let mut response = Vec::new();
            let mut reader = BufReader::new(stream);
            timeout(IPC_TIMEOUT, reader.read_until(b'\n', &mut response))
                .await
                .map_err(|error| SyncwebError::operation("daemon IPC read timed out", error))?
                .map_err(|error| SyncwebError::operation("daemon IPC read failed", error))?;
            if response.is_empty() {
                return Err(SyncwebError::operation(
                    "daemon IPC returned no response",
                    "connection closed",
                ));
            }
            serde_json::from_slice(response.trim_ascii())
                .map_err(|error| SyncwebError::operation("failed to deserialize IPC response", error))
        }
        #[cfg(not(unix))]
        {
            let _ = request;
            Err(SyncwebError::operation(
                "daemon IPC is unavailable",
                "Unix sockets are not supported on this platform",
            ))
        }
    }

    /// Perform a bounded synchronous status probe for routing decisions.
    ///
    /// # Errors
    ///
    /// Returns an error when the daemon does not answer a status request.
    pub fn status_sync(&self) -> Result<DaemonStatus> {
        #[cfg(unix)]
        {
            use std::{
                io::{BufRead, Write},
                os::unix::net::UnixStream,
            };

            let stream = UnixStream::connect(&self.socket_path).map_err(|error| {
                SyncwebError::operation(
                    format!("failed to connect to daemon socket at {}", self.socket_path.display()),
                    error,
                )
            })?;
            stream.set_read_timeout(Some(IPC_TIMEOUT))?;
            stream.set_write_timeout(Some(IPC_TIMEOUT))?;
            let mut writer = stream.try_clone()?;
            let request = serde_json::to_vec(&IpcRequest::new(IpcCommand::Status))
                .map_err(|error| SyncwebError::operation("failed to serialize IPC request", error))?;
            writer.write_all(&request)?;
            writer.write_all(b"\n")?;
            let mut line = String::new();
            std::io::BufReader::new(stream).read_line(&mut line)?;
            match serde_json::from_str::<IpcResponse>(&line)
                .map_err(|error| SyncwebError::operation("failed to deserialize IPC response", error))?
            {
                IpcResponse::Status(status) => Ok(status),
                IpcResponse::Error { message } => Err(SyncwebError::operation("daemon status request failed", message)),
                IpcResponse::Ok { .. }
                | IpcResponse::FolderList(_)
                | IpcResponse::DownloadComplete { .. }
                | IpcResponse::ImportFilesComplete { .. }
                | IpcResponse::ImportComplete(_)
                | IpcResponse::ExportComplete(_)
                | IpcResponse::EnrichData(_)
                | IpcResponse::FileStats(_)
                | IpcResponse::Entries(_)
                | IpcResponse::PeerAvailability(_)
                | IpcResponse::TransferJobsProcessed { .. } => Err(SyncwebError::operation(
                    "daemon status request returned an unexpected response",
                    "unexpected response",
                )),
            }
        }
        #[cfg(not(unix))]
        {
            Err(SyncwebError::operation(
                "daemon IPC is unavailable",
                "Unix sockets are not supported on this platform",
            ))
        }
    }
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    Ok(std::fs::set_permissions(path, permissions)?)
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    #[cfg(unix)]
    use std::{os::unix::fs::PermissionsExt, time::Duration};

    use super::*;
    use crate::daemon::{DaemonState, DaemonStatus};

    fn socket_path() -> PathBuf {
        std::env::temp_dir().join(format!("syncweb-ipc-{}.sock", uuid::Uuid::new_v4()))
    }

    fn state() -> DaemonState {
        DaemonState::new(
            std::process::id(),
            "node",
            1,
            std::env::temp_dir(),
            DaemonStatus::Running,
        )
    }

    #[test]
    fn request_round_trips_as_json() {
        let request = IpcRequest::new(IpcCommand::Download {
            namespace: "namespace".to_owned(),
            strategy: FetchStrategy::default(),
        });
        let encoded = serde_json::to_vec(&request).expect("serialize request");
        let decoded: IpcRequest = serde_json::from_slice(&encoded).expect("deserialize request");
        assert!(matches!(decoded.command, IpcCommand::Download { .. }));
    }

    #[tokio::test]
    async fn handle_request_updates_registry_and_trigger() {
        let (sync_trigger, mut sync_receiver) = mpsc::unbounded_channel();
        let handle = DaemonHandle::with_channels(
            Arc::new(RwLock::new(state())),
            Arc::new(RwLock::new(FolderRegistry::new())),
            broadcast::channel(4).0,
            sync_trigger,
        );
        let server = IpcServer::new(socket_path(), handle);
        let namespace = iroh_docs::NamespaceSecret::from_bytes(&[7; 32]).id().to_string();

        assert!(matches!(
            server
                .handle_request(IpcRequest::new(IpcCommand::AddFolder {
                    namespace: namespace.clone(),
                    path: PathBuf::from("/tmp/folder"),
                }))
                .await,
            IpcResponse::Ok { .. }
        ));
        assert!(matches!(
            server
                .handle_request(IpcRequest::new(IpcCommand::ListFolders))
                .await,
            IpcResponse::FolderList(folders)
                if folders.len() == 1 && folders.first().is_some_and(|folder| folder.namespace == namespace)
        ));
        assert!(matches!(
            server
                .handle_request(IpcRequest::new(IpcCommand::TriggerSync {
                    namespace: Some(namespace.clone()),
                }))
                .await,
            IpcResponse::Ok { .. }
        ));
        assert_eq!(sync_receiver.recv().await, Some(Some(namespace)));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn client_round_trips_with_server() {
        let path = socket_path();
        let handle = DaemonHandle::new(state());
        let mut shutdown_receiver = handle.shutdown_sender.subscribe();
        let server = IpcServer::new(path.clone(), handle);
        let server_task = tokio::spawn(async move { server.serve().await });

        tokio::time::timeout(Duration::from_secs(1), async {
            while !path.exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("server socket should appear");

        let client = IpcClient::from_socket_path(path.clone());
        assert!(matches!(
            client
                .send(IpcRequest::new(IpcCommand::Status))
                .await
                .expect("status response"),
            IpcResponse::Status(DaemonStatus::Running)
        ));
        assert!(matches!(
            client
                .send(IpcRequest::new(IpcCommand::Shutdown { force: false }))
                .await
                .expect("shutdown response"),
            IpcResponse::Ok { .. }
        ));
        shutdown_receiver.recv().await.expect("shutdown broadcast");
        server_task
            .await
            .expect("server task should join")
            .expect("server should stop cleanly");
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn socket_is_owner_only() {
        let path = socket_path();
        let listener = IpcListener::new(path.clone()).bind().expect("bind socket");
        assert_eq!(
            std::fs::metadata(&path).expect("socket metadata").permissions().mode() & 0o777,
            0o600
        );
        drop(listener);
        std::fs::remove_file(path).expect("remove socket");
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn client_reports_missing_server() {
        let path = socket_path();
        let error = IpcClient::from_socket_path(path)
            .send(IpcRequest::new(IpcCommand::Status))
            .await
            .expect_err("missing server should fail");
        assert!(error.to_string().contains("failed to connect to daemon socket"));
    }

    #[test]
    fn new_commands_round_trip_as_json() {
        let req1 = IpcRequest::new(IpcCommand::Unsubscribe {
            namespace: "blob:baead9a5c1f7b3d2e4f60897c5a1b3d8e2f40796a8c0b5d3e7f10829c4a6b0d".to_owned(),
        });
        let enc1 = serde_json::to_vec(&req1).expect("serialize");
        let dec1: IpcRequest = serde_json::from_slice(&enc1).expect("deserialize");
        assert!(matches!(dec1.command, IpcCommand::Unsubscribe { .. }));

        let req2 = IpcRequest::new(IpcCommand::LeaveFolder {
            namespace: "ns".to_owned(),
            delete_files: false,
        });
        let enc2 = serde_json::to_vec(&req2).expect("serialize");
        let dec2: IpcRequest = serde_json::from_slice(&enc2).expect("deserialize");
        assert!(matches!(
            dec2.command,
            IpcCommand::LeaveFolder {
                delete_files: false,
                ..
            }
        ));

        let req3 = IpcRequest::new(IpcCommand::Share {
            namespace: "ns".to_owned(),
            blob: Some("baead9a5c1f7b3d2e4f60897c5a1b3d8e2f40796a8c0b5d3e7f10829c4a6b0d".to_owned()),
            writable: false,
            pin: false,
            persist: false,
        });
        let enc3 = serde_json::to_vec(&req3).expect("serialize");
        let dec3: IpcRequest = serde_json::from_slice(&enc3).expect("deserialize");
        assert!(matches!(dec3.command, IpcCommand::Share { .. }));

        let req4 = IpcRequest::new(IpcCommand::SnapshotCreate {
            path: PathBuf::from("."),
            description: Some("test".to_owned()),
            threads: 0,
        });
        let enc4 = serde_json::to_vec(&req4).expect("serialize");
        let dec4: IpcRequest = serde_json::from_slice(&enc4).expect("deserialize");
        assert!(matches!(dec4.command, IpcCommand::SnapshotCreate { .. }));

        let req5 = IpcRequest::new(IpcCommand::SnapshotList {
            path: PathBuf::from("."),
        });
        let enc5 = serde_json::to_vec(&req5).expect("serialize");
        let dec5: IpcRequest = serde_json::from_slice(&enc5).expect("deserialize");
        assert!(matches!(dec5.command, IpcCommand::SnapshotList { .. }));

        let req6 = IpcRequest::new(IpcCommand::SnapshotDelete {
            id: "baead9a5c1f7b3d2e4f60897c5a1b3d8e2f40796a8c0b5d3e7f10829c4a6b0d".to_owned(),
        });
        let enc6 = serde_json::to_vec(&req6).expect("serialize");
        let dec6: IpcRequest = serde_json::from_slice(&enc6).expect("deserialize");
        assert!(matches!(dec6.command, IpcCommand::SnapshotDelete { .. }));

        let req7 = IpcRequest::new(IpcCommand::CollectionPublish {
            path: PathBuf::from("."),
            namespace: "ns".to_owned(),
            sequence: 1,
            bootstrap: vec![],
            manifest_bytes: None,
        });
        let enc7 = serde_json::to_vec(&req7).expect("serialize");
        let dec7: IpcRequest = serde_json::from_slice(&enc7).expect("deserialize");
        assert!(matches!(dec7.command, IpcCommand::CollectionPublish { .. }));

        let req8 = IpcRequest::new(IpcCommand::SubscribePublic {
            ticket: "blob_ticket".to_owned(),
        });
        let enc8 = serde_json::to_vec(&req8).expect("serialize");
        let dec8: IpcRequest = serde_json::from_slice(&enc8).expect("deserialize");
        assert!(matches!(dec8.command, IpcCommand::SubscribePublic { .. }));
    }

    #[tokio::test]
    async fn test_ipc_unsubscribe_rejects_folder_namespaces() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let namespace = iroh_docs::NamespaceSecret::from_bytes(&[7; 32]).id().to_string();
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::Unsubscribe {
                namespace: namespace.clone(),
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("use `leave` for folders")
        ));
    }

    #[tokio::test]
    async fn test_ipc_unsubscribe_invalid_namespace() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::Unsubscribe {
                namespace: "not-a-namespace".to_owned(),
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("use `leave` for folders")
        ));
    }

    #[tokio::test]
    async fn test_ipc_leave_folder_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::LeaveFolder {
                namespace: "ns".to_owned(),
                delete_files: false,
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_share_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::Share {
                namespace: "ns".to_owned(),
                blob: Some("baead9a5c1f7b3d2e4f60897c5a1b3d8e2f40796a8c0b5d3e7f10829c4a6b0d".to_owned()),
                writable: false,
                pin: false,
                persist: false,
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_snapshot_create_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::SnapshotCreate {
                path: PathBuf::from("."),
                description: None,
                threads: 0,
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_snapshot_list_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::SnapshotList {
                path: PathBuf::from("."),
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_snapshot_delete_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::SnapshotDelete {
                id: "baead9a5c1f7b3d2e4f60897c5a1b3d8e2f40796a8c0b5d3e7f10829c4a6b0d".to_owned(),
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_collection_publish_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::CollectionPublish {
                path: PathBuf::from("."),
                namespace: "ns".to_owned(),
                sequence: 1,
                bootstrap: vec![],
                manifest_bytes: None,
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_create_folder_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: PathBuf::from("."),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_create_folder_no_context_with_invalid_mode() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: PathBuf::from("/tmp/test-create-folder"),
                mode: "invalid".to_owned(),
                indexing: false,
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_stats_files_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::StatsFiles {
                folder: PathBuf::from("."),
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_materialize_transfers_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::MaterializeTransfers { namespace: None }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_verify_integrity_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::VerifyIntegrity {
                path: PathBuf::from("."),
                hash: Vec::new(),
                path_filter: None,
                glob_filter: None,
                fix: false,
                from: Vec::new(),
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_join_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::Join {
                ticket: "ticket".to_owned(),
                path: PathBuf::from("/tmp"),
                mode: SyncMode::SendReceive,
                subscribe: false,
                filters: SubscribeFilters::default(),
                download: false,
                indexing: false,
                metadata_only: false,
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_unshare_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::Unshare {
                namespace: "ns".to_owned(),
                blob: None,
                writable: false,
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_subscribe_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::SetSubscribe {
                namespace: "ns".to_owned(),
                enabled: true,
                filters: Some(SubscribeFilters {
                    ingest_only: true,
                    ..Default::default()
                }),
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    struct IpcTestFixture {
        server: IpcServer,
        node: Arc<IrohNode>,
        directory: PathBuf,
    }

    async fn setup_ipc_test() -> IpcTestFixture {
        use crate::node::identity::IdentityManager;
        use crate::node::iroh_node::RelayMode;
        use std::collections::HashSet;
        let directory = std::env::temp_dir().join(format!("syncweb-ipc-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("test directory should be created");

        let identity = IdentityManager::new(directory.join("identity.key")).expect("test identity should open");
        let node = Arc::new(
            IrohNode::new(
                identity,
                directory.join("data"),
                RelayMode::Default,
                Arc::new(std::sync::RwLock::new(HashSet::new())),
                crate::node::iroh_node::DiscoveryConfig::disabled(),
            )
            .await
            .expect("test node should start"),
        );
        let pool = Arc::new(ManagedPool::new("syncweb-test", 1).expect("test pool should start"));

        let daemon_state = DaemonState::new(
            std::process::id(),
            node.endpoint().id().to_string(),
            1,
            &directory,
            DaemonStatus::Running,
        );
        let handle = DaemonHandle::new(daemon_state);
        let server = IpcServer::with_archive_context(socket_path(), handle, node.clone(), pool, None);

        IpcTestFixture {
            server,
            node,
            directory,
        }
    }

    async fn cleanup_ipc_test(fixture: IpcTestFixture) {
        let _ = fixture.node.stop().await;
        let _ = std::fs::remove_dir_all(&fixture.directory);
    }

    #[tokio::test]
    async fn test_ipc_create_folder_creates_and_returns_message() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("create-folder-test");
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await;
        assert!(matches!(response, IpcResponse::Ok { .. }));
        if let IpcResponse::Ok { message } = response {
            assert!(message.contains("namespace:"));
            assert!(!message.contains("ticket:"));
        }
        cleanup_ipc_test(fixture).await;
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_ipc_create_folder_invalid_mode() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("create-folder-invalid");
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir.clone(),
                mode: "invalid-mode".to_owned(),
                indexing: false,
            }))
            .await;
        assert!(matches!(response, IpcResponse::Error { .. }));
        if let IpcResponse::Error { message } = response {
            assert!(message.contains("invalid sync mode"));
        }
        cleanup_ipc_test(fixture).await;
    }

    #[tokio::test]
    async fn test_ipc_create_folder_duplicate_namespace() {
        let fixture = setup_ipc_test().await;
        let test_dir1 = fixture.directory.join("create-folder-dup-1");
        let test_dir2 = fixture.directory.join("create-folder-dup-2");

        let response1 = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir1.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await;
        assert!(matches!(response1, IpcResponse::Ok { .. }));
        let namespace = if let IpcResponse::Ok { message } = &response1 {
            message
                .lines()
                .find(|line| line.starts_with("namespace:"))
                .and_then(|line| line.strip_prefix("namespace:").map(str::trim).map(str::to_owned))
        } else {
            None
        };

        if let Some(ref ns) = namespace {
            let response2 = fixture
                .server
                .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                    path: test_dir2.clone(),
                    mode: "sendreceive".to_owned(),
                    indexing: false,
                }))
                .await;
            assert!(matches!(response2, IpcResponse::Ok { .. }));
            let ns2 = if let IpcResponse::Ok { message } = &response2 {
                message
                    .lines()
                    .find(|line| line.starts_with("namespace:"))
                    .and_then(|line| line.strip_prefix("namespace:").map(str::trim).map(str::to_owned))
            } else {
                None
            };
            assert_ne!(Some(ns), ns2.as_ref(), "each create should produce a unique namespace");
        }

        cleanup_ipc_test(fixture).await;
        let _ = std::fs::remove_dir_all(&test_dir1);
        let _ = std::fs::remove_dir_all(&test_dir2);
    }

    #[tokio::test]
    async fn test_ipc_stats_files_returns_report() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("stats-files-test");
        std::fs::create_dir_all(&test_dir).expect("test dir should be created");

        let response1 = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await;
        let namespace = if let IpcResponse::Ok { message } = &response1 {
            message
                .lines()
                .find(|line| line.starts_with("namespace:"))
                .and_then(|line| line.strip_prefix("namespace:").map(str::trim).map(str::to_owned))
        } else {
            None
        };

        if let Some(ns) = namespace {
            let response2 = fixture
                .server
                .handle_request(IpcRequest::new(IpcCommand::StatsFiles {
                    folder: PathBuf::from(&ns),
                }))
                .await;
            assert!(matches!(response2, IpcResponse::FileStats(_)));
        }

        cleanup_ipc_test(fixture).await;
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_ipc_stats_files_unknown_folder() {
        let fixture = setup_ipc_test().await;
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::StatsFiles {
                folder: PathBuf::from("/nonexistent/path/that/does/not/exist"),
            }))
            .await;
        assert!(matches!(response, IpcResponse::Error { .. }));
        cleanup_ipc_test(fixture).await;
    }

    #[tokio::test]
    async fn test_ipc_verify_integrity_returns_result() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("verify-test");
        std::fs::create_dir_all(&test_dir).expect("test dir should be created");

        let response1 = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await;
        let namespace = if let IpcResponse::Ok { message } = &response1 {
            message
                .lines()
                .find(|line| line.starts_with("namespace:"))
                .and_then(|line| line.strip_prefix("namespace:").map(str::trim).map(str::to_owned))
        } else {
            None
        };

        if let Some(ns) = namespace {
            let response2 = fixture
                .server
                .handle_request(IpcRequest::new(IpcCommand::VerifyIntegrity {
                    path: PathBuf::from(&ns),
                    hash: Vec::new(),
                    path_filter: None,
                    glob_filter: None,
                    fix: false,
                    from: Vec::new(),
                }))
                .await;
            assert!(matches!(response2, IpcResponse::Ok { .. }));
            if let IpcResponse::Ok { message } = response2 {
                assert!(message.contains("total:"));
                assert!(message.contains("verified:"));
                assert!(message.contains("corrupted:"));
                assert!(message.contains("missing:"));
                assert!(message.contains("valid:"));
            }
        }

        cleanup_ipc_test(fixture).await;
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_ipc_verify_integrity_unknown_folder() {
        let fixture = setup_ipc_test().await;
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::VerifyIntegrity {
                path: PathBuf::from("/nonexistent/path/that/does/not/exist"),
                hash: Vec::new(),
                path_filter: None,
                glob_filter: None,
                fix: false,
                from: Vec::new(),
            }))
            .await;
        assert!(matches!(response, IpcResponse::Error { .. }));
        cleanup_ipc_test(fixture).await;
    }

    #[tokio::test]
    async fn test_ipc_join_folder_invalid_ticket() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("join-invalid");
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::Join {
                ticket: "not-a-valid-ticket".to_owned(),
                path: test_dir.clone(),
                mode: SyncMode::SendReceive,
                subscribe: false,
                filters: SubscribeFilters::default(),
                download: false,
                indexing: false,
                metadata_only: false,
            }))
            .await;
        assert!(matches!(response, IpcResponse::Error { .. }));
        cleanup_ipc_test(fixture).await;
    }

    #[tokio::test]
    async fn test_ipc_share_folder_ticket() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("share-test");
        std::fs::create_dir_all(&test_dir).expect("test dir should be created");

        let response1 = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await;
        let namespace = if let IpcResponse::Ok { message } = &response1 {
            message
                .lines()
                .find(|line| line.starts_with("namespace:"))
                .and_then(|line| line.strip_prefix("namespace:").map(str::trim).map(str::to_owned))
        } else {
            None
        };

        if let Some(ns) = namespace {
            let response2 = fixture
                .server
                .handle_request(IpcRequest::new(IpcCommand::Share {
                    namespace: ns.clone(),
                    blob: None,
                    writable: false,
                    pin: false,
                    persist: false,
                }))
                .await;
            assert!(matches!(response2, IpcResponse::Ok { .. }));
            if let IpcResponse::Ok { message } = response2 {
                assert!(
                    crate::uri::is_folder_url(&message),
                    "share should emit a URL: {message}"
                );
            }
        }

        cleanup_ipc_test(fixture).await;
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_ipc_share_invalid_namespace() {
        let fixture = setup_ipc_test().await;
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::Share {
                namespace: "not-a-namespace".to_owned(),
                blob: None,
                writable: false,
                pin: false,
                persist: false,
            }))
            .await;
        assert!(matches!(response, IpcResponse::Error { .. }));
        if let IpcResponse::Error { message } = response {
            assert!(message.contains("invalid namespace"));
        }
        cleanup_ipc_test(fixture).await;
    }

    #[tokio::test]
    async fn test_ipc_subscribe_returns_ok() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("subscribe-test");
        std::fs::create_dir_all(&test_dir).expect("test dir should be created");

        let response1 = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await;
        let namespace = if let IpcResponse::Ok { message } = &response1 {
            message
                .lines()
                .find(|line| line.starts_with("namespace:"))
                .and_then(|line| line.strip_prefix("namespace:").map(str::trim).map(str::to_owned))
        } else {
            None
        };

        if let Some(ns) = namespace {
            let response = fixture
                .server
                .handle_request(IpcRequest::new(IpcCommand::SetSubscribe {
                    namespace: ns.clone(),
                    enabled: true,
                    filters: Some(SubscribeFilters {
                        ingest_only: true,
                        ..Default::default()
                    }),
                }))
                .await;
            assert!(matches!(response, IpcResponse::Ok { .. }));
            if let IpcResponse::Ok { message } = response {
                assert!(message.contains("subscribed:"));
            }
        }

        cleanup_ipc_test(fixture).await;
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_ipc_subscribe_with_params() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("subscribe-params-test");
        std::fs::create_dir_all(&test_dir).expect("test dir should be created");

        let response1 = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await;
        let namespace = if let IpcResponse::Ok { message } = &response1 {
            message
                .lines()
                .find(|line| line.starts_with("namespace:"))
                .and_then(|line| line.strip_prefix("namespace:").map(str::trim).map(str::to_owned))
        } else {
            None
        };

        if let Some(ns) = namespace {
            let filters = SubscribeFilters {
                ingest_only: true,
                ..Default::default()
            };
            let response2 = fixture
                .server
                .handle_request(IpcRequest::new(IpcCommand::SetSubscribe {
                    namespace: ns.clone(),
                    enabled: true,
                    filters: Some(filters),
                }))
                .await;
            assert!(matches!(response2, IpcResponse::Ok { .. }));
        }

        cleanup_ipc_test(fixture).await;
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_ipc_leave_folder_removes_from_registry() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("leave-test");
        std::fs::create_dir_all(&test_dir).expect("test dir should be created");

        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await;
        let namespace = if let IpcResponse::Ok { message } = &response {
            message
                .lines()
                .find(|line| line.starts_with("namespace:"))
                .and_then(|line| line.strip_prefix("namespace:").map(str::trim).map(str::to_owned))
        } else {
            None
        };

        if let Some(ref ns) = namespace {
            let statuses1 = fixture.server.daemon_handle.folder_registry.read().await.statuses();
            assert!(statuses1.iter().any(|s| s.namespace == *ns));

            let response2 = fixture
                .server
                .handle_request(IpcRequest::new(IpcCommand::LeaveFolder {
                    namespace: ns.clone(),
                    delete_files: false,
                }))
                .await;
            assert!(matches!(response2, IpcResponse::Ok { .. }));

            let statuses2 = fixture.server.daemon_handle.folder_registry.read().await.statuses();
            assert!(!statuses2.iter().any(|s| s.namespace == *ns));
        }

        cleanup_ipc_test(fixture).await;
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_ipc_leave_folder_nonexistent() {
        let fixture = setup_ipc_test().await;
        let fake_ns = iroh_docs::NamespaceSecret::from_bytes(&[99; 32]).id().to_string();
        let _response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::LeaveFolder {
                namespace: fake_ns.clone(),
                delete_files: false,
            }))
            .await;
        let registry = fixture.server.daemon_handle.folder_registry.read().await;
        let statuses = registry.statuses();
        assert!(!statuses.iter().any(|s| s.namespace == fake_ns));
        drop(registry);
        cleanup_ipc_test(fixture).await;
    }

    #[tokio::test]
    async fn test_ipc_unshare_invalid_hash() {
        let fixture = setup_ipc_test().await;
        let fake_ns = iroh_docs::NamespaceSecret::from_bytes(&[88; 32]).id().to_string();
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::Unshare {
                namespace: fake_ns,
                blob: Some("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_owned()),
                writable: false,
            }))
            .await;
        assert!(matches!(response, IpcResponse::Error { .. }));
        if let IpcResponse::Error { message } = response {
            assert!(message.contains("invalid blob hash"));
        }
        cleanup_ipc_test(fixture).await;
    }

    #[tokio::test]
    async fn test_ipc_snapshot_list_empty() {
        let fixture = setup_ipc_test().await;
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::SnapshotList {
                path: PathBuf::from("."),
            }))
            .await;
        assert!(matches!(response, IpcResponse::Ok { .. }));
        if let IpcResponse::Ok { message } = response {
            assert!(message.contains("snapshots:"));
        }
        cleanup_ipc_test(fixture).await;
    }

    #[tokio::test]
    async fn test_ipc_snapshot_delete_invalid_id() {
        let fixture = setup_ipc_test().await;
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::SnapshotDelete {
                id: "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_owned(),
            }))
            .await;
        assert!(matches!(response, IpcResponse::Error { .. }));
        if let IpcResponse::Error { message } = response {
            assert!(message.contains("invalid snapshot id"));
        }
        cleanup_ipc_test(fixture).await;
    }
}
