use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    fs,
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
use uuid::Uuid;

use crate::{
    allocation::{AllocationCandidate, StorageRoot, materialization_path},
    bandwidth_stats::{FileStatsCollector, FileStatsReport},
    daemon::state::FolderStatusReport,
    error::{Result, SyncwebError},
    filter::{FilterConfig, FilterEngine},
    folder::{
        CollectionHead, CollectionManifest, CollectionStore, DropExportOptions, DropExportResult, DropExporter,
        DropImportOptions, DropImportResult, DropImporter, FolderLike, FolderManager, PackageAnnouncement,
        PackageCatalog, PublicSubscription, SyncMode,
    },
    fs::Importer,
    indexing::{IndexingService, ProviderReputationStore, ProviderTrustSignal, resilience::ResilienceService},
    node::{gossip_service::GossipService, iroh_node::IrohNode},
    snapshot::SnapshotStore,
    storage::config::SubscribeFilters,
    storage::node_db::{NodeDatabase, TransferJobRecord},
    sync::{
        ActiveSession, AreaFilter, FetchCandidate, FetchFilter, FetchStrategy, HealthReport, SubscribeParams,
        SyncEngine, SyncEvent, cancel_session,
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

const TRANSFER_TIMEOUT: Duration = Duration::from_mins(5);

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
    },
    Publish {
        namespace: String,
        blob: Option<String>,
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
    },
    HealthCheck {
        path: PathBuf,
        #[serde(default)]
        hash: Vec<String>,
        #[serde(default)]
        path_prefix: Option<String>,
        #[serde(default)]
        glob: Option<String>,
    },
    StatsFiles {
        folder: PathBuf,
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
    Unpublish {
        namespace: String,
        blob: String,
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
    BroadcastTrustSignal(ProviderTrustSignal),
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

    /// Search for packages on a gossip channel.
    PackageSearch {
        /// Optional search query (name substring).
        #[serde(default)]
        query: Option<String>,
        /// Optional channel name (defaults to the public catalog).
        #[serde(default)]
        channel: Option<String>,
        /// Timeout in seconds for gossip collection.
        #[serde(default = "default_search_timeout")]
        timeout_secs: u64,
    },
    /// Install a package from a manifest blob ticket.
    PackageInstall {
        /// Blob ticket for the package manifest.
        ticket: String,
        /// Target directory for installed files.
        target_dir: PathBuf,
    },
    /// Upgrade an installed package to its latest announced version.
    PackageUpgrade {
        /// Collection UUID of the package to upgrade.
        collection_id: String,
    },
    /// Remove an installed package.
    PackageRemove {
        /// Collection UUID of the package to remove.
        collection_id: String,
    },
    /// List locally installed packages.
    PackageList,
    /// Get detailed info about an installed package.
    PackageInfo {
        collection_id: String,
    },
}

/// A response returned by the daemon control channel.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "response", content = "data", rename_all = "snake_case")]
#[non_exhaustive]
pub enum IpcResponse {
    Ok {
        message: String,
    },
    Status(DaemonStatus),
    FolderList(Vec<FolderStatus>),
    DownloadComplete {
        bytes_transferred: u64,
    },
    TransferJobsProcessed {
        completed: u64,
        failed: u64,
    },
    ImportFilesComplete {
        entries: u64,
    },
    ImportComplete(Box<DropImportResult>),
    ExportComplete(Box<DropExportResult>),
    EnrichData(HashMap<String, usize>),
    FileStats(Box<FileStatsReport>),
    Error {
        message: String,
    },
    PackageSearchResult {
        packages: Vec<PackageAnnouncement>,
    },
    PackageInstalled {
        collection_id: String,
        name: String,
        version: String,
        installed_path: PathBuf,
        manifest_hash: String,
    },
    PackageRemoved {
        collection_id: String,
    },
    PackageListResult {
        packages: Vec<InstalledPackageInfo>,
    },
    PackageInfoResult {
        info: InstalledPackageInfo,
    },
}

/// Summary of a locally installed package.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct InstalledPackageInfo {
    pub collection_id: String,
    pub name: String,
    pub version: String,
    pub installed_path: PathBuf,
    pub manifest_hash: String,
    pub installed_at: String,
    pub file_count: usize,
    pub total_size: u64,
}

/// A managed folder summary returned by the daemon.
pub use crate::daemon::state::FolderStatusReport as FolderStatus;

/// A folder managed by the daemon.
#[non_exhaustive]
pub struct FolderEntry {
    pub namespace: iroh_docs::NamespaceId,
    pub path: PathBuf,
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
            session: None,
            last_sync_at: None,
            sync_count: 0,
            entries_synced: 0,
            errors: Vec::new(),
        }
    }

    #[must_use]
    pub fn status(&self) -> FolderStatusReport {
        FolderStatusReport {
            namespace: self.namespace.to_string(),
            path: self.path.clone(),
            kind: "folder".to_owned(),
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
    resilience: Option<ResilienceService>,
    network_manager: Option<std::sync::Arc<tokio::sync::RwLock<crate::net::NetworkManager>>>,
}

#[derive(Clone)]
struct ArchiveContext {
    node: Arc<IrohNode>,
    pool: Arc<ManagedPool>,
    indexing: Option<IndexingService>,
}

#[derive(Clone, Copy)]
enum TransferJobOutcome {
    Completed,
    Failed,
    Skipped,
}

enum TransferPreparation {
    Ready,
    Failed,
    Skipped,
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
            resilience: None,
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
            resilience: None,
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

    /// Attach a resilience service for live peer-count queries.
    #[must_use]
    pub fn with_resilience(mut self, resilience: ResilienceService) -> Self {
        self.resilience = Some(resilience);
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
        if let IpcCommand::BroadcastTrustSignal(signal) = cmd {
            return self.handle_broadcast_trust_signal_response(signal).await;
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

    async fn handle_package_group(&self, cmd: IpcCommand) -> IpcResponse {
        if let IpcCommand::PackageSearch {
            query,
            channel,
            timeout_secs,
        } = cmd
        {
            return self.handle_package_search(query, channel, timeout_secs).await;
        }
        if let IpcCommand::PackageInstall { ticket, target_dir } = cmd {
            return self.handle_package_install(ticket, target_dir).await;
        }
        if let IpcCommand::PackageUpgrade { collection_id } = cmd {
            return self.handle_package_upgrade(collection_id).await;
        }
        if let IpcCommand::PackageRemove { collection_id } = cmd {
            return self.handle_package_remove(collection_id);
        }
        if matches!(cmd, IpcCommand::PackageList) {
            return self.handle_package_list();
        }
        if let IpcCommand::PackageInfo { collection_id } = cmd {
            return self.handle_package_info(collection_id);
        }
        IpcResponse::Error {
            message: format!("unhandled package command: {cmd:?}"),
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
            } => self.handle_join(ticket, path, mode, subscribe, filters, download).await,
            C::Publish { namespace, blob } => self.handle_publish(namespace, blob).await,
            C::SetSubscribe {
                namespace,
                enabled,
                filters,
            } => self.handle_set_subscribe(namespace, enabled, filters).await,
            C::SubscribePublic { ticket } => self.handle_subscribe_public(ticket).await,
            C::CreateFolder { path, mode } => self.handle_create_folder(path, mode).await,
            C::HealthCheck {
                path,
                hash: filter_hashes,
                path_prefix,
                glob,
            } => self.handle_health_check(path, filter_hashes, path_prefix, glob).await,
            C::StatsFiles { folder } => self.handle_stats_files(folder).await,
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
            C::Unpublish { namespace, blob } => self.handle_unpublish(namespace, blob).await,
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
            C::PackageSearch { .. }
            | C::PackageInstall { .. }
            | C::PackageUpgrade { .. }
            | C::PackageRemove { .. }
            | C::PackageList
            | C::PackageInfo { .. } => self.handle_package_group(request.command).await,
            C::NetworkInvite { .. }
            | C::NetworkKick { .. }
            | C::NetworkLeave { .. }
            | C::NetworkCreate { .. }
            | C::NetworkJoin { .. }
            | C::BroadcastTrustSignal(..) => self.handle_network_group(request.command).await,
        }
    }

    async fn handle_broadcast_trust_signal_response(&self, signal: ProviderTrustSignal) -> IpcResponse {
        match self.handle_broadcast_trust_signal(signal).await {
            Ok(()) => IpcResponse::Ok {
                message: "trust signal broadcast".to_owned(),
            },
            Err(error) => response_from_error(error),
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
        let jobs = match node_db.list_transfer_jobs(namespace.as_deref(), Some("queued")) {
            Ok(jobs) => jobs,
            Err(error) => return response_from_error(error),
        };
        let mut completed = 0_u64;
        let mut failed = 0_u64;
        for job in jobs {
            match self.process_transfer_job(&context, &node_db, job).await {
                TransferJobOutcome::Completed => completed = completed.saturating_add(1),
                TransferJobOutcome::Failed => failed = failed.saturating_add(1),
                TransferJobOutcome::Skipped => {}
            }
        }
        IpcResponse::TransferJobsProcessed { completed, failed }
    }

    async fn process_transfer_job(
        &self,
        context: &ArchiveContext,
        node_db: &NodeDatabase,
        job: TransferJobRecord,
    ) -> TransferJobOutcome {
        let Some(destination) = job.destination.as_ref() else {
            mark_transfer_job_failed(
                node_db,
                &job.id,
                &"job has no allocated destination",
                "missing transfer destination",
            );
            return TransferJobOutcome::Failed;
        };
        let hash = iroh_blobs::Hash::from(job.hash);
        if let Err(error) = Self::validate_transfer_job(node_db, &job, destination, hash) {
            mark_transfer_job_failed(node_db, &job.id, &error, "transfer path validation error");
            return TransferJobOutcome::Failed;
        }
        match self.prepare_transfer_job(context, node_db, &job, hash).await {
            TransferPreparation::Ready => {}
            TransferPreparation::Failed => return TransferJobOutcome::Failed,
            TransferPreparation::Skipped => return TransferJobOutcome::Skipped,
        }
        match materialize_transfer(context, destination, hash).await {
            Ok(size) => complete_transfer_job(node_db, &job, size),
            Err(error) => {
                mark_transfer_job_failed(node_db, &job.id, &error, "transfer materialization error");
                TransferJobOutcome::Failed
            }
        }
    }

    async fn prepare_transfer_job(
        &self,
        context: &ArchiveContext,
        node_db: &NodeDatabase,
        job: &TransferJobRecord,
        hash: iroh_blobs::Hash,
    ) -> TransferPreparation {
        let fetch_claimed = match node_db.transition_transfer_job_state(&job.id, "queued", "fetching", None) {
            Ok(claimed) => claimed,
            Err(error) => {
                tracing::error!(%error, job_id = %job.id, "failed to mark transfer job fetching");
                return TransferPreparation::Failed;
            }
        };
        if !fetch_claimed {
            return TransferPreparation::Skipped;
        }

        let has_blob = match context.node.blob_store().has(hash).await {
            Ok(has_blob) => has_blob,
            Err(error) => {
                mark_transfer_job_failed(node_db, &job.id, &error, "blob lookup error");
                return TransferPreparation::Failed;
            }
        };
        if !has_blob {
            let path = match String::from_utf8(job.entry_key.clone()) {
                Ok(path) => path,
                Err(error) => {
                    mark_transfer_job_failed(
                        node_db,
                        &job.id,
                        &format!("job entry path is not UTF-8: {error}"),
                        "invalid transfer path",
                    );
                    return TransferPreparation::Failed;
                }
            };
            let filter = FetchFilter::new().with_paths(vec![PathBuf::from(path)]);
            if let Err(error) = self
                .handle_download_with_timeout(
                    job.namespace_id.clone(),
                    FetchStrategy::filter(filter),
                    TRANSFER_TIMEOUT,
                )
                .await
            {
                mark_transfer_job_failed(node_db, &job.id, &error, "transfer fetch error");
                return TransferPreparation::Failed;
            }
        }

        let materialization_claimed =
            match node_db.transition_transfer_job_state(&job.id, "fetching", "materializing", None) {
                Ok(claimed) => claimed,
                Err(error) => {
                    tracing::error!(%error, job_id = %job.id, "failed to mark transfer job materializing");
                    return TransferPreparation::Failed;
                }
            };
        if materialization_claimed {
            TransferPreparation::Ready
        } else {
            TransferPreparation::Skipped
        }
    }

    /// Validate that a queued job still points at its allocated root and path.
    fn validate_transfer_job(
        node_db: &NodeDatabase,
        job: &TransferJobRecord,
        destination: &Path,
        hash: iroh_blobs::Hash,
    ) -> Result<()> {
        let root_id = job
            .root_id
            .as_deref()
            .ok_or_else(|| SyncwebError::InvalidConfig("job has no storage root".to_owned()))?;
        let root = node_db
            .list_storage_roots()?
            .into_iter()
            .find(|root| root.id == root_id)
            .ok_or_else(|| SyncwebError::InvalidConfig(format!("storage root not found: {root_id}")))?;
        let job_namespace = iroh_docs::NamespaceId::from_str(&job.namespace_id)
            .map_err(|error| SyncwebError::operation("job has invalid namespace", error))?;
        let path = String::from_utf8(job.entry_key.clone())
            .map_err(|error| SyncwebError::operation("job entry path is not UTF-8", error))?;
        let root_path = root.path.clone();
        let candidate = AllocationCandidate::new(
            job_namespace,
            PathBuf::from(path),
            hash,
            job.size,
            usize::try_from(job.peer_count).unwrap_or(usize::MAX),
            false,
        );
        let expected = materialization_path(
            &StorageRoot::new(root.id, &root_path, root.min_free).with_enabled(root.enabled),
            &candidate,
        )?;
        if expected != destination {
            return Err(SyncwebError::InvalidConfig(format!(
                "job destination does not match its allocated root: {}",
                destination.display()
            )));
        }
        reject_symlink_components(
            expected.strip_prefix(&root_path).unwrap_or_else(|_| Path::new("")),
            &root_path,
        )?;
        Ok(())
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
        let sync = SyncEngine::new(
            FolderManager::new(&context.node),
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
        let store = CollectionStore::new(
            folder.doc().clone(),
            folder.author(),
            context.node.blob_store().clone(),
            context.node.docs_engine().clone(),
        );
        store.publish(&result.collection_manifest, 1).await?;
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

    async fn handle_join(
        &self,
        ticket: String,
        path: PathBuf,
        mode: SyncMode,
        subscribe: bool,
        filters: SubscribeFilters,
        download: bool,
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
        match manager.join(ticket, mode).await {
            Ok(folder) => {
                let namespace = folder.namespace_id().to_string();
                if let Some(ref node_db) = self.node_db {
                    let mut config = match node_db.load_app_config() {
                        Ok(config) => config,
                        Err(error) => return response_from_error(error),
                    };
                    config.set_subscribe(&namespace, subscribe, &filters);
                    if let Err(error) = node_db.save_app_config(&config) {
                        return response_from_error(error);
                    }
                }
                if subscribe {
                    let sync = SyncEngine::new(
                        manager.clone(),
                        context.node.blob_store().clone(),
                        context.node.docs_engine().clone(),
                        Some(context.node.topic_tracker().clone()),
                    );
                    let params = match SubscribeParams::from_filters(&filters) {
                        Ok(params) => params,
                        Err(error) => return response_from_error(error),
                    };
                    if let Err(error) = sync.subscribe(folder.namespace_id(), params).await {
                        return response_from_error(error);
                    }
                }
                let downloaded = if download {
                    match self
                        .materialize_folder(&context, &manager, &folder, &filters, &path)
                        .await
                    {
                        Ok(count) => count,
                        Err(error) => return response_from_error(error),
                    }
                } else {
                    0
                };
                IpcResponse::Ok {
                    message: if download {
                        format!("joined: {namespace}\ndownloaded: {downloaded} files")
                    } else {
                        format!("joined: {namespace}")
                    },
                }
            }
            Err(error) => response_from_error(error),
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

    async fn handle_publish(&self, namespace: String, blob: Option<String>) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon publish IPC is unavailable: server has no node context".to_owned(),
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
        match blob {
            Some(blob_hash) => {
                let hash = match blob_hash.parse::<iroh_blobs::Hash>() {
                    Ok(h) => h,
                    Err(error) => {
                        return IpcResponse::Error {
                            message: format!("invalid blob hash: {error}"),
                        };
                    }
                };
                match folder.publish_blob(context.node.endpoint().addr(), hash).await {
                    Ok(ticket) => IpcResponse::Ok {
                        message: format!("blob_ticket: {ticket}"),
                    },
                    Err(error) => response_from_error(error),
                }
            }
            None => match folder.ticket(context.node.endpoint().addr(), false).await {
                Ok(ticket) => IpcResponse::Ok {
                    message: format!("ticket: {ticket}"),
                },
                Err(error) => response_from_error(error),
            },
        }
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

    async fn handle_create_folder(&self, path: PathBuf, mode: String) -> IpcResponse {
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
                let namespace_id = folder.namespace_id();
                if self
                    .daemon_handle
                    .folder_registry
                    .write()
                    .await
                    .add(FolderEntry::new(namespace_id, path))
                    .is_err()
                {
                    tracing::warn!(%namespace, "folder already in daemon registry");
                }
                match folder.ticket(context.node.endpoint().addr(), true).await {
                    Ok(ticket) => IpcResponse::Ok {
                        message: format!("namespace: {namespace}\nticket: {ticket}"),
                    },
                    Err(error) => response_from_error(error),
                }
            }
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_health_check(
        &self,
        path: PathBuf,
        filter_hashes: Vec<String>,
        path_prefix: Option<String>,
        glob: Option<String>,
    ) -> IpcResponse {
        use std::collections::HashMap;

        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon health IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let manager = FolderManager::new(&context.node);
        let folder = match resolve_folder_for_daemon(&manager, &path).await {
            Ok(f) => f,
            Err(error) => return error,
        };
        let filter = build_ipc_verify_filter(&filter_hashes, path_prefix.as_ref(), glob.as_ref());
        let entries = match context.node.docs_engine().list_latest(folder.doc()).await {
            Ok(e) => e,
            Err(error) => return response_from_error(error),
        };
        let mut candidates = Vec::new();
        let mut hashes = Vec::new();
        for entry in entries {
            if entry.key().starts_with(b"sys/") {
                continue;
            }
            let entry_hash = entry.content_hash();
            if let Some(ref f) = filter
                && !f.matches(entry.key(), &entry_hash)
            {
                continue;
            }
            let path_str = match String::from_utf8(entry.key().to_vec()) {
                Ok(s) => s,
                Err(error) => {
                    return IpcResponse::Error {
                        message: format!("folder entry path is not UTF-8: {error}"),
                    };
                }
            };
            let hash = entry.content_hash();
            let local = match folder.has_local(hash).await {
                Ok(l) => l,
                Err(error) => return response_from_error(error),
            };
            candidates.push(FetchCandidate::new(path_str, hash, entry.content_len(), 0, local));
            hashes.push(hash);
        }

        let peers_per_hash: HashMap<iroh_blobs::Hash, usize> = match &self.resilience {
            Some(resilience) => match resilience.health_batch(&hashes) {
                Ok(health_map) => health_map.into_iter().map(|(h, health)| (h, health.verified)).collect(),
                Err(error) => {
                    return response_from_error(error);
                }
            },
            None => HashMap::new(),
        };

        let report = HealthReport::from_candidates_with_peers_per_hash(&candidates, &peers_per_hash, 4);
        IpcResponse::Ok {
            message: format!(
                "total: {}, well-seeded: {}, under-seeded: {}, unseeded: {}",
                report.total, report.well_seeded, report.under_seeded, report.unseeded,
            ),
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
        let entries = match context.node.docs_engine().list_latest(folder.doc()).await {
            Ok(e) => e,
            Err(error) => return response_from_error(error),
        };
        let mut collector = FileStatsCollector::new();
        for entry in entries {
            if entry.key().starts_with(b"sys/") {
                continue;
            }
            collector.add_entry_bytes_with_time(entry.key(), entry.content_len(), Some(entry.timestamp()));
        }
        IpcResponse::FileStats(Box::new(collector.report()))
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
        let mut hashes = Vec::new();
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
            hashes.push(hash);
        }

        let peers_per_hash: HashMap<iroh_blobs::Hash, usize> =
            self.resilience.as_ref().map_or_else(HashMap::new, |resilience| {
                resilience.health_batch(&hashes).map_or_else(
                    |_| HashMap::new(),
                    |health_map| health_map.into_iter().map(|(h, health)| (h, health.verified)).collect(),
                )
            });

        let result: HashMap<String, usize> = path_map
            .into_iter()
            .map(|(path_str, hash)| {
                let count = peers_per_hash.get(&hash).copied().unwrap_or(0);
                (path_str, count)
            })
            .collect();
        IpcResponse::EnrichData(result)
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
            let mut repaired = false;

            // Try --from tickets first
            for (ticket_hash, ticket) in &provider_tickets {
                if *ticket_hash != hash {
                    continue;
                }
                if context
                    .node
                    .blob_store()
                    .force_fetch(context.node.endpoint(), ticket)
                    .await
                    .is_ok()
                {
                    repaired = true;
                    break;
                }
            }
            if repaired {
                result.repaired = result.repaired.saturating_add(1);
                continue;
            }

            // Try namespace peers
            for peer in &namespace_peers {
                if context
                    .node
                    .blob_store()
                    .force_fetch_from_peer(context.node.endpoint(), peer, hash)
                    .await
                    .is_ok()
                {
                    repaired = true;
                    break;
                }
            }
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

    async fn handle_unpublish(&self, namespace: String, blob: String) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon unpublish IPC is unavailable: server has no node context".to_owned(),
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
        let hash = match blob.parse::<iroh_blobs::Hash>() {
            Ok(h) => h,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("invalid blob hash: {error}"),
                };
            }
        };
        let manager = FolderManager::new(&context.node);
        let folder = match manager.get(namespace_id).await {
            Ok(f) => f,
            Err(error) => return response_from_error(error),
        };
        match folder.unpublish_blob(hash).await {
            Ok(()) => IpcResponse::Ok {
                message: format!("unpublished: {blob}"),
            },
            Err(error) => response_from_error(error),
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
                message: format!(
                    "snapshot: {}\nroot_hash: {}\nfiles: {}\nsize: {}",
                    snapshot.id, snapshot.root_hash, snapshot.file_count, snapshot.total_size,
                ),
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
        bootstrap: Vec<String>,
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
        let store = CollectionStore::new(
            folder.doc().clone(),
            folder.author(),
            context.node.blob_store().clone(),
            context.node.docs_engine().clone(),
        );
        let head = match store.publish(&manifest, sequence).await {
            Ok(h) => h,
            Err(error) => return response_from_error(error),
        };
        let name = manifest
            .package
            .as_ref()
            .map_or_else(|| manifest.collection_id.to_string(), |profile| profile.name.clone());
        let ticket = context.node.blob_store().ticket(context.node.endpoint(), head.manifest);
        let announcement = match PackageAnnouncement::new(
            manifest.collection_id,
            name,
            manifest.version.clone(),
            head.sequence,
            head.manifest,
            ticket.to_string(),
            context.node.endpoint().id(),
        ) {
            Ok(a) => a,
            Err(error) => return response_from_error(error),
        };
        let bootstrap_nodes: Vec<_> = bootstrap
            .into_iter()
            .filter_map(|b| b.parse::<iroh::PublicKey>().ok())
            .collect();
        let catalog = PackageCatalog::new(context.node.gossip_service(), context.node.endpoint());
        let topic = if bootstrap_nodes.is_empty() {
            match catalog.subscribe(bootstrap_nodes).await {
                Ok(t) => t,
                Err(error) => return response_from_error(error),
            }
        } else {
            match catalog.subscribe_and_join(bootstrap_nodes).await {
                Ok(t) => t,
                Err(error) => return response_from_error(error),
            }
        };
        let (sender, _receiver) = GossipService::split(topic);
        if let Err(error) = catalog.announce(&sender, &announcement).await {
            return response_from_error(error);
        }
        IpcResponse::Ok {
            message: format!(
                "manifest: {}\nmanifest_ticket: {}\nsequence: {}",
                head.manifest, announcement.manifest_ticket, head.sequence,
            ),
        }
    }

    async fn handle_package_search(
        &self,
        query: Option<String>,
        channel: Option<String>,
        timeout_secs: u64,
    ) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon package-search IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };

        // If a channel is specified, check for a catalog-backed namespace first.
        if let Some(ref channel_name) = channel
            && let Some(ref indexing) = context.indexing
            && let Ok(Some(namespace_id)) = indexing.database().get_channel_namespace(channel_name)
        {
            let author = match context.node.docs_engine().author().await {
                Ok(a) => a,
                Err(error) => return response_from_error(error),
            };
            let catalog_service =
                indexing.catalog_service(context.node.docs_engine(), context.node.blob_store(), author);
            let query_str = query.as_deref().unwrap_or("");
            let limit = 100;
            return match catalog_service.search(query_str, limit) {
                Ok(records) => {
                    let packages: Vec<_> = records
                        .into_iter()
                        .filter(|r| r.catalog_namespace_id == namespace_id)
                        .map(|r| r.to_package_announcement())
                        .collect();
                    IpcResponse::PackageSearchResult { packages }
                }
                Err(error) => response_from_error(error),
            };
        }

        // Fall back to gossip-based search.
        let catalog = PackageCatalog::new(context.node.gossip_service(), context.node.endpoint());
        let timeout = Duration::from_secs(timeout_secs);
        let bootstrap: Vec<iroh::PublicKey> = Vec::new();

        let result = if let Some(channel_name) = &channel {
            let channel_obj = crate::editorial::Channel::new(channel_name.as_str(), None::<String>);
            let mut topic = match catalog.subscribe_channel_and_join(&channel_obj, bootstrap).await {
                Ok(t) => t,
                Err(error) => return response_from_error(error),
            };
            catalog.search_channel(&mut topic, query.as_deref(), timeout).await
        } else {
            let mut topic = match catalog.subscribe_and_join(bootstrap).await {
                Ok(t) => t,
                Err(error) => return response_from_error(error),
            };
            catalog.search(&mut topic, query.as_deref(), timeout).await
        };

        match result {
            Ok(packages) => IpcResponse::PackageSearchResult { packages },
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_package_install(&self, ticket: String, target_dir: PathBuf) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon package-install IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let manifest_ticket = match ticket.parse::<iroh_blobs::ticket::BlobTicket>() {
            Ok(t) => t,
            Err(error) => return response_from_error(error),
        };
        let pkg_root = target_dir.join(crate::constants::PACKAGES_DIR_NAME);
        let Some(node_db) = self.node_db.clone() else {
            return IpcResponse::Error {
                message: "daemon package-install IPC is unavailable: no node database".to_owned(),
            };
        };
        let pkg_manager = crate::folder::PackageManager::new(&pkg_root, node_db);
        match pkg_manager
            .install_from_ticket(&manifest_ticket, context.node.endpoint(), context.node.blob_store())
            .await
        {
            Ok(manifest) => {
                let installed_path = pkg_root
                    .join(manifest.collection_id.to_string())
                    .join(&manifest.version);
                let manifest_hash = manifest.blob_id().map_or_else(|_| String::new(), |h| h.to_string());
                IpcResponse::PackageInstalled {
                    collection_id: manifest.collection_id.to_string(),
                    name: manifest
                        .package
                        .as_ref()
                        .map_or_else(|| manifest.collection_id.to_string(), |p| p.name.clone()),
                    version: manifest.version,
                    installed_path,
                    manifest_hash,
                }
            }
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_package_upgrade(&self, collection_id: String) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon package-upgrade IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let catalog = PackageCatalog::new(context.node.gossip_service(), context.node.endpoint());
        let bootstrap: Vec<iroh::PublicKey> = Vec::new();
        let mut topic = match catalog.subscribe_and_join(bootstrap).await {
            Ok(t) => t,
            Err(error) => return response_from_error(error),
        };
        let announcements = match catalog
            .search(&mut topic, Some(&collection_id.clone()), Duration::from_secs(10))
            .await
        {
            Ok(a) => a,
            Err(error) => return response_from_error(error),
        };
        let latest = announcements.iter().max_by(|a, b| a.version.cmp(&b.version));
        let Some(announcement) = latest else {
            return IpcResponse::Error {
                message: format!("no announcement found for collection {collection_id}"),
            };
        };
        let manifest_ticket = match announcement.ticket() {
            Ok(t) => t,
            Err(error) => return response_from_error(error),
        };
        let Some(node_db) = self.node_db.clone() else {
            return IpcResponse::Error {
                message: "daemon package-upgrade IPC is unavailable: no node database".to_owned(),
            };
        };
        let pkg_root = std::env::temp_dir().join("syncweb-packages");
        let pkg_manager = crate::folder::PackageManager::new(&pkg_root, node_db);
        match pkg_manager
            .install_from_ticket(&manifest_ticket, context.node.endpoint(), context.node.blob_store())
            .await
        {
            Ok(manifest) => {
                let manifest_hash = manifest.blob_id().map_or_else(|_| String::new(), |h| h.to_string());
                let version = manifest.version;
                let installed_path = pkg_root.join(manifest.collection_id.to_string()).join(&version);
                IpcResponse::PackageInstalled {
                    collection_id: manifest.collection_id.to_string(),
                    name: manifest
                        .package
                        .as_ref()
                        .map_or_else(|| manifest.collection_id.to_string(), |p| p.name.clone()),
                    version,
                    installed_path,
                    manifest_hash,
                }
            }
            Err(error) => response_from_error(error),
        }
    }

    fn handle_package_remove(&self, collection_id: String) -> IpcResponse {
        let collection_uuid = match collection_id.parse::<Uuid>() {
            Ok(id) => id,
            Err(error) => return response_from_error(error),
        };
        let Some(node_db) = self.node_db.clone() else {
            return IpcResponse::Error {
                message: "daemon package-remove IPC is unavailable: no node database".to_owned(),
            };
        };
        let pkg_root = std::env::temp_dir().join("syncweb-packages");
        let pkg_manager = crate::folder::PackageManager::new(&pkg_root, node_db);
        let state = match pkg_manager.state() {
            Ok(s) => s,
            Err(error) => return response_from_error(error),
        };
        let installed = match state.installed.get(&collection_uuid) {
            Some(i) => i.clone(),
            None => {
                return IpcResponse::Error {
                    message: format!("package {collection_id} is not installed"),
                };
            }
        };
        // Remove all versions except the current one by switching first
        let versions: Vec<String> = installed.versions.keys().cloned().collect();
        for version in &versions {
            if *version != installed.current
                && let Err(error) = pkg_manager.remove(collection_uuid, version)
            {
                return response_from_error(error);
            }
        }
        // For the current version, we still remove the directory directly
        if let Some(path) = installed.versions.get(&installed.current)
            && let Err(error) = fs::remove_dir_all(path)
        {
            return response_from_error(error);
        }
        IpcResponse::PackageRemoved { collection_id }
    }

    fn handle_package_list(&self) -> IpcResponse {
        let Some(node_db) = self.node_db.clone() else {
            return IpcResponse::Error {
                message: "daemon package-list IPC is unavailable: no node database".to_owned(),
            };
        };
        let pkg_root = std::env::temp_dir().join("syncweb-packages");
        let pkg_manager = crate::folder::PackageManager::new(&pkg_root, node_db);
        let state = match pkg_manager.state() {
            Ok(s) => s,
            Err(error) => return response_from_error(error),
        };
        let mut packages = Vec::new();
        for (collection_id, installed) in &state.installed {
            let path = installed.versions.get(&installed.current);
            let (file_count, total_size) = path.map_or((0, 0), |p| count_dir_files(p));
            packages.push(InstalledPackageInfo {
                collection_id: collection_id.to_string(),
                name: collection_id.to_string(),
                version: installed.current.clone(),
                installed_path: path.cloned().unwrap_or_default(),
                manifest_hash: installed.manifest.to_string(),
                installed_at: String::new(),
                file_count,
                total_size,
            });
        }
        IpcResponse::PackageListResult { packages }
    }

    fn handle_package_info(&self, collection_id: String) -> IpcResponse {
        let collection_uuid = match collection_id.parse::<Uuid>() {
            Ok(id) => id,
            Err(error) => return response_from_error(error),
        };
        let Some(node_db) = self.node_db.clone() else {
            return IpcResponse::Error {
                message: "daemon package-info IPC is unavailable: no node database".to_owned(),
            };
        };
        let pkg_root = std::env::temp_dir().join("syncweb-packages");
        let pkg_manager = crate::folder::PackageManager::new(&pkg_root, node_db);
        let state = match pkg_manager.state() {
            Ok(s) => s,
            Err(error) => return response_from_error(error),
        };
        let installed = match state.installed.get(&collection_uuid) {
            Some(i) => i.clone(),
            None => {
                return IpcResponse::Error {
                    message: format!("package {collection_id} is not installed"),
                };
            }
        };
        let path = installed.versions.get(&installed.current);
        let (file_count, total_size) = path.map_or((0, 0), |p| count_dir_files(p));
        IpcResponse::PackageInfoResult {
            info: InstalledPackageInfo {
                collection_id: collection_id.clone(),
                name: collection_id,
                version: installed.current.clone(),
                installed_path: path.cloned().unwrap_or_default(),
                manifest_hash: installed.manifest.to_string(),
                installed_at: String::new(),
                file_count,
                total_size,
            },
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

    async fn handle_broadcast_trust_signal(&self, signal: ProviderTrustSignal) -> Result<()> {
        let context = self.archive_context.clone().ok_or_else(|| {
            SyncwebError::operation(
                "broadcast trust signal IPC is unavailable",
                "server has no node context",
            )
        })?;
        let gossip_store = ProviderReputationStore::default();
        let topic = gossip_store
            .subscribe_trust_stream(context.node.gossip_service(), Vec::new())
            .await?;
        let (sender, _receiver) = GossipService::split(topic);
        gossip_store
            .publish_signal(context.node.gossip_service(), &sender, &signal)
            .await
    }
}

fn mark_transfer_job_failed(node_db: &NodeDatabase, job_id: &str, message: &dyn std::fmt::Display, context: &str) {
    let rendered_message = message.to_string();
    if let Err(error) = node_db.update_transfer_job_state(job_id, "failed", Some(&rendered_message)) {
        tracing::error!(%error, %job_id, context, "failed to record transfer job failure");
    }
}

async fn materialize_transfer(context: &ArchiveContext, destination: &Path, hash: iroh_blobs::Hash) -> Result<u64> {
    if let Ok(metadata) = std::fs::symlink_metadata(destination) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(SyncwebError::InvalidConfig(format!(
                "materialization destination is not a regular file: {}",
                destination.display()
            )));
        }
        let existing = std::fs::read(destination)?;
        if blake3::hash(&existing).as_bytes() != hash.as_bytes() {
            return Err(SyncwebError::InvalidConfig(format!(
                "materialization destination has a different blob: {}",
                destination.display()
            )));
        }
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    context.node.blob_store().export_to_path(hash, destination).await?;
    let bytes = std::fs::read(destination)?;
    if blake3::hash(&bytes).as_bytes() != hash.as_bytes() {
        return Err(SyncwebError::operation(
            "materialized transfer hash does not match",
            destination.display(),
        ));
    }
    u64::try_from(bytes.len()).map_err(|error| SyncwebError::operation("materialized transfer size is invalid", error))
}

fn complete_transfer_job(node_db: &NodeDatabase, job: &TransferJobRecord, size: u64) -> TransferJobOutcome {
    if let Err(error) = node_db.update_transfer_job_progress(&job.id, size, job.peer_count, None, job.retries) {
        tracing::error!(%error, job_id = %job.id, "failed to record transfer completion progress");
        return TransferJobOutcome::Failed;
    }
    match node_db.transition_transfer_job_state(&job.id, "materializing", "completed", None) {
        Ok(true) => TransferJobOutcome::Completed,
        Ok(false) => TransferJobOutcome::Skipped,
        Err(error) => {
            tracing::error!(%error, job_id = %job.id, "failed to record transfer completion");
            TransferJobOutcome::Failed
        }
    }
}

const fn default_search_timeout() -> u64 {
    10
}

fn count_dir_files(dir: &Path) -> (usize, u64) {
    let mut count = 0_usize;
    let mut total = 0_u64;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata()
                && meta.is_file()
            {
                count = count.saturating_add(1);
                total = total.saturating_add(meta.len());
            }
        }
    }
    (count, total)
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
    if let Ok(namespace) = selector.to_string_lossy().parse::<iroh_docs::NamespaceId>() {
        return manager.get(namespace).await.map_err(|error| IpcResponse::Error {
            message: format!("folder not found: {error}"),
        });
    }
    let folders = manager.list().await.map_err(|error| IpcResponse::Error {
        message: format!("failed to list folders: {error}"),
    })?;
    match folders.as_slice() {
        [folder] => Ok(folder.clone()),
        [] => Err(IpcResponse::Error {
            message: "no synchronized folders are available".to_owned(),
        }),
        _ => Err(IpcResponse::Error {
            message: "folder path is not a namespace ID and more than one synchronized folder is available".to_owned(),
        }),
    }
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
                | IpcResponse::TransferJobsProcessed { .. }
                | IpcResponse::PackageSearchResult { .. }
                | IpcResponse::PackageInstalled { .. }
                | IpcResponse::PackageRemoved { .. }
                | IpcResponse::PackageListResult { .. }
                | IpcResponse::PackageInfoResult { .. } => Err(SyncwebError::operation(
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

fn reject_symlink_components(relative: &Path, root: &Path) -> Result<()> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = match std::fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() {
            return Err(SyncwebError::InvalidConfig(format!(
                "materialization path contains a symlink: {}",
                current.display()
            )));
        }
    }
    Ok(())
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

        let req3 = IpcRequest::new(IpcCommand::Unpublish {
            namespace: "ns".to_owned(),
            blob: "baead9a5c1f7b3d2e4f60897c5a1b3d8e2f40796a8c0b5d3e7f10829c4a6b0d".to_owned(),
        });
        let enc3 = serde_json::to_vec(&req3).expect("serialize");
        let dec3: IpcRequest = serde_json::from_slice(&enc3).expect("deserialize");
        assert!(matches!(dec3.command, IpcCommand::Unpublish { .. }));

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
    async fn test_ipc_unpublish_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::Unpublish {
                namespace: "ns".to_owned(),
                blob: "baead9a5c1f7b3d2e4f60897c5a1b3d8e2f40796a8c0b5d3e7f10829c4a6b0d".to_owned(),
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
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_health_check_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::HealthCheck {
                path: PathBuf::from("."),
                hash: Vec::new(),
                path_prefix: None,
                glob: None,
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
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_publish_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::Publish {
                namespace: "ns".to_owned(),
                blob: None,
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
            }))
            .await;
        assert!(matches!(response, IpcResponse::Ok { .. }));
        if let IpcResponse::Ok { message } = response {
            assert!(message.contains("namespace:"));
            assert!(message.contains("ticket:"));
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
    async fn test_ipc_health_check_returns_report() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("health-test");
        std::fs::create_dir_all(&test_dir).expect("test dir should be created");

        let response1 = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir.clone(),
                mode: "sendreceive".to_owned(),
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
                .handle_request(IpcRequest::new(IpcCommand::HealthCheck {
                    path: PathBuf::from(&ns),
                    hash: Vec::new(),
                    path_prefix: None,
                    glob: None,
                }))
                .await;
            assert!(matches!(response2, IpcResponse::Ok { .. }));
            if let IpcResponse::Ok { message } = response2 {
                assert!(message.contains("total:"));
                assert!(message.contains("well-seeded:"));
            }
        }

        cleanup_ipc_test(fixture).await;
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_ipc_health_check_unknown_folder() {
        let fixture = setup_ipc_test().await;
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::HealthCheck {
                path: PathBuf::from("/nonexistent/path/that/does/not/exist"),
                hash: Vec::new(),
                path_prefix: None,
                glob: None,
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
            }))
            .await;
        assert!(matches!(response, IpcResponse::Error { .. }));
        cleanup_ipc_test(fixture).await;
    }

    #[tokio::test]
    async fn test_ipc_publish_folder_ticket() {
        let fixture = setup_ipc_test().await;
        let test_dir = fixture.directory.join("publish-test");
        std::fs::create_dir_all(&test_dir).expect("test dir should be created");

        let response1 = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::CreateFolder {
                path: test_dir.clone(),
                mode: "sendreceive".to_owned(),
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
                .handle_request(IpcRequest::new(IpcCommand::Publish {
                    namespace: ns.clone(),
                    blob: None,
                }))
                .await;
            assert!(matches!(response2, IpcResponse::Ok { .. }));
            if let IpcResponse::Ok { message } = response2 {
                assert!(message.contains("ticket:"));
            }
        }

        cleanup_ipc_test(fixture).await;
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_ipc_publish_invalid_namespace() {
        let fixture = setup_ipc_test().await;
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::Publish {
                namespace: "not-a-namespace".to_owned(),
                blob: None,
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
    async fn test_ipc_unpublish_invalid_hash() {
        let fixture = setup_ipc_test().await;
        let fake_ns = iroh_docs::NamespaceSecret::from_bytes(&[88; 32]).id().to_string();
        let response = fixture
            .server
            .handle_request(IpcRequest::new(IpcCommand::Unpublish {
                namespace: fake_ns,
                blob: "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_owned(),
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
