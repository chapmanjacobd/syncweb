use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use n0_future::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast, mpsc};

use crate::{
    error::{Result, SyncwebError},
    filter::{FilterConfig, FilterEngine},
    folder::{
        CollectionHead, CollectionManifest, CollectionStore, DropExportOptions, DropExportResult, DropExporter,
        DropImportOptions, DropImportResult, DropImporter, FolderManager, PackageAnnouncement, PackageCatalog,
        SyncMode,
    },
    fs::Importer,
    node::{gossip_service::GossipService, iroh_node::IrohNode},
    snapshot::SnapshotStore,
    sync::{
        ActiveSession, FetchCandidate, FetchStrategy, HealthReport, SubscribeParams, SyncEngine, SyncEvent,
        cancel_session,
    },
    verify::IntegrityChecker,
};

use super::{
    ManagedPool,
    state::{DaemonStatus, daemon_socket_path},
};

const IPC_TIMEOUT: Duration = Duration::from_millis(500); // 0.5 s

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
    RemoveFolder {
        namespace: String,
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
    },
    Publish {
        namespace: String,
        blob: Option<String>,
    },
    Subscribe {
        namespace: String,
        params: SubscribeParams,
    },
    CreateFolder {
        path: PathBuf,
        mode: String,
    },
    HealthCheck {
        path: PathBuf,
    },
    VerifyIntegrity {
        path: PathBuf,
    },
    Unsubscribe {
        namespace: String,
    },
    LeaveFolder {
        namespace: String,
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
    ImportFilesComplete { entries: u64 },
    ImportComplete(Box<DropImportResult>),
    ExportComplete(Box<DropExportResult>),
    Error { message: String },
}

/// A managed folder summary returned by the daemon.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FolderStatus {
    pub namespace: String,
    pub path: PathBuf,
    pub session_active: bool,
    pub last_sync_at: Option<u64>,
    pub sync_count: u64,
    pub entries_synced: u64,
    pub errors: Vec<String>,
}

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
    pub fn status(&self) -> FolderStatus {
        FolderStatus {
            namespace: self.namespace.to_string(),
            path: self.path.clone(),
            session_active: self.session.is_some() || crate::sync::is_active(self.namespace),
            last_sync_at: self.last_sync_at,
            sync_count: self.sync_count,
            entries_synced: self.entries_synced,
            errors: self.errors.clone(),
        }
    }
}

/// Registry of folders currently managed by the daemon.
#[derive(Default)]
pub struct FolderRegistry {
    folders: HashMap<String, FolderEntry>,
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
                return Ok(());
            }
            return Err(SyncwebError::FolderAlreadyManaged);
        }
        self.folders.insert(key, entry);
        Ok(())
    }

    pub fn remove(&mut self, namespace: &iroh_docs::NamespaceId) -> Option<FolderEntry> {
        self.folders.remove(&namespace.to_string())
    }

    #[must_use]
    pub fn statuses(&self) -> Vec<FolderStatus> {
        let mut statuses: Vec<_> = self.folders.values().map(FolderEntry::status).collect();
        statuses.sort_by(|left, right| left.namespace.cmp(&right.namespace));
        statuses
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.folders.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.folders.is_empty()
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
        let listener = tokio::net::UnixListener::bind(&self.socket_path)?;
        set_owner_only_permissions(&self.socket_path)?;
        Ok(listener)
    }
}

/// A server for the daemon's local control channel.
#[derive(Clone)]
pub struct IpcServer {
    listener: IpcListener,
    daemon_handle: DaemonHandle,
    archive_context: Option<Arc<ArchiveContext>>,
}

#[derive(Clone)]
struct ArchiveContext {
    node: Arc<IrohNode>,
    pool: Arc<ManagedPool>,
}

impl IpcServer {
    #[must_use]
    pub const fn new(socket_path: PathBuf, daemon_handle: DaemonHandle) -> Self {
        Self {
            listener: IpcListener::new(socket_path),
            daemon_handle,
            archive_context: None,
        }
    }

    /// Create an IPC server with access to daemon-owned archive resources.
    #[must_use]
    pub fn with_archive_context(
        socket_path: PathBuf,
        daemon_handle: DaemonHandle,
        node: Arc<IrohNode>,
        pool: Arc<ManagedPool>,
    ) -> Self {
        Self {
            listener: IpcListener::new(socket_path),
            daemon_handle,
            archive_context: Some(Arc::new(ArchiveContext { node, pool })),
        }
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
                        let (stream, _) = accepted?;
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

    /// Handle one decoded request without requiring a socket.
    pub async fn handle_request(&self, request: IpcRequest) -> IpcResponse {
        match request.command {
            IpcCommand::Status => IpcResponse::Status(self.daemon_handle.state.read().await.status),
            IpcCommand::ListFolders => {
                let folders = self.daemon_handle.folder_registry.read().await.statuses();
                IpcResponse::FolderList(folders)
            }
            IpcCommand::AddFolder { namespace, path } => self.handle_add_folder(namespace, path).await,
            IpcCommand::RemoveFolder { namespace } => self.handle_remove_folder(namespace).await,
            IpcCommand::TriggerSync { namespace } => self.handle_trigger_sync(namespace),
            IpcCommand::SetLogLevel { level } => IpcResponse::Ok {
                message: format!("log level set to {level}"),
            },
            IpcCommand::ReloadConfig => self.handle_reload_config(),
            IpcCommand::Shutdown { force } => self.handle_shutdown(force),
            IpcCommand::ImportArchive { input, target, filter } => {
                match self.handle_import_archive(input, target, filter).await {
                    Ok(result) => IpcResponse::ImportComplete(Box::new(result)),
                    Err(error) => response_from_error(error),
                }
            }
            IpcCommand::ImportFiles { namespace, path } => self.handle_import_files_response(namespace, path).await,
            IpcCommand::ExportArchive {
                namespace,
                version,
                output,
            } => match self.handle_export_archive(namespace, version, output).await {
                Ok(result) => IpcResponse::ExportComplete(Box::new(result)),
                Err(error) => response_from_error(error),
            },
            IpcCommand::Download { namespace, strategy } => self.handle_download_response(namespace, strategy).await,
            IpcCommand::Join { ticket, path, mode } => self.handle_join(ticket, path, mode).await,
            IpcCommand::Publish { namespace, blob } => self.handle_publish(namespace, blob).await,
            IpcCommand::Subscribe { namespace, params } => self.handle_subscribe(namespace, params).await,
            IpcCommand::CreateFolder { path, mode } => self.handle_create_folder(path, mode).await,
            IpcCommand::HealthCheck { path } => self.handle_health_check(path).await,
            IpcCommand::VerifyIntegrity { path } => self.handle_verify_integrity(path).await,
            IpcCommand::Unsubscribe { namespace } => self.handle_unsubscribe(&namespace),
            IpcCommand::LeaveFolder { namespace } => self.handle_leave_folder(namespace).await,
            IpcCommand::Unpublish { namespace, blob } => self.handle_unpublish(namespace, blob).await,
            IpcCommand::SnapshotCreate {
                path,
                description,
                threads,
            } => self.handle_snapshot_create(path, description, threads).await,
            IpcCommand::SnapshotList { path } => self.handle_snapshot_list(path).await,
            IpcCommand::SnapshotDelete { id } => self.handle_snapshot_delete(id).await,
            IpcCommand::CollectionPublish {
                path,
                namespace,
                sequence,
                bootstrap,
            } => {
                self.handle_collection_publish(path, namespace, sequence, bootstrap)
                    .await
            }
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

    async fn handle_remove_folder(&self, namespace: String) -> IpcResponse {
        match iroh_docs::NamespaceId::from_str(&namespace) {
            Ok(namespace_id) => {
                let removed = self.daemon_handle.folder_registry.write().await.remove(&namespace_id);
                if removed.is_some() {
                    let _ = cancel_session(namespace_id);
                    IpcResponse::Ok {
                        message: "folder removed".to_owned(),
                    }
                } else {
                    IpcResponse::Error {
                        message: format!("folder not found: {namespace}"),
                    }
                }
            }
            Err(error) => IpcResponse::Error {
                message: format!("invalid folder namespace: {error}"),
            },
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

    async fn handle_import_files_response(&self, namespace: Option<String>, path: PathBuf) -> IpcResponse {
        match self.handle_import_files(namespace, path).await {
            Ok(entries) => IpcResponse::ImportFilesComplete { entries },
            Err(error) => response_from_error(error),
        }
    }

    async fn handle_download(&self, namespace: String, strategy: FetchStrategy) -> Result<u64> {
        let context = self.archive_context.clone().ok_or_else(|| {
            SyncwebError::operation("daemon download IPC is unavailable", "server has no node context")
        })?;
        let namespace_id = iroh_docs::NamespaceId::from_str(&namespace)
            .map_err(|error| SyncwebError::operation("invalid download namespace", error))?;
        let sync = SyncEngine::new(
            FolderManager::new(&context.node),
            context.node.blob_store().clone(),
            context.node.docs_engine().clone(),
            context.node.gossip_service().clone(),
        );
        let mut intent = sync.fetch(namespace_id, strategy).await?;
        let mut bytes_transferred = 0_u64;
        while let Some(event) = intent.next().await {
            match event {
                SyncEvent::Stats(stats) => {
                    bytes_transferred = bytes_transferred.max(stats.bytes_transferred);
                }
                SyncEvent::Failed(message) => {
                    return Err(SyncwebError::operation("daemon download failed", message));
                }
                SyncEvent::Finished => break,
                SyncEvent::Started
                | SyncEvent::Progress { .. }
                | SyncEvent::Paused
                | SyncEvent::Resumed
                | SyncEvent::Cancelled => {}
            }
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

    async fn handle_join(&self, ticket: String, path: PathBuf, mode: SyncMode) -> IpcResponse {
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
                IpcResponse::Ok {
                    message: format!("joined: {namespace}"),
                }
            }
            Err(error) => response_from_error(error),
        }
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

    async fn handle_subscribe(&self, namespace: String, params: SubscribeParams) -> IpcResponse {
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
        let manager = FolderManager::new(&context.node);
        let sync = SyncEngine::new(
            manager,
            context.node.blob_store().clone(),
            context.node.docs_engine().clone(),
            context.node.gossip_service().clone(),
        );
        match sync.subscribe(namespace_id, params).await {
            Ok(_intent) => IpcResponse::Ok {
                message: format!("subscribed: {namespace}"),
            },
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

    async fn handle_health_check(&self, path: PathBuf) -> IpcResponse {
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
        let entries = match context.node.docs_engine().list_latest(folder.doc()).await {
            Ok(e) => e,
            Err(error) => return response_from_error(error),
        };
        let mut candidates = Vec::new();
        for entry in entries {
            if entry.key().starts_with(b"sys/") {
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
            let local = match folder.has_local(entry.content_hash()).await {
                Ok(l) => l,
                Err(error) => return response_from_error(error),
            };
            candidates.push(FetchCandidate::new(
                path_str,
                entry.content_hash(),
                entry.content_len(),
                0,
                local,
            ));
        }
        let report = HealthReport::from_candidates(&candidates, 4);
        IpcResponse::Ok {
            message: format!(
                "total: {}, well-seeded: {}, under-seeded: {}, unseeded: {}",
                report.total, report.well_seeded, report.under_seeded, report.unseeded,
            ),
        }
    }

    async fn handle_verify_integrity(&self, path: PathBuf) -> IpcResponse {
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
        match checker.verify_folder(&folder).await {
            Ok(result) => {
                let valid = result.is_valid();
                IpcResponse::Ok {
                    message: format!(
                        "total: {}, verified: {}, corrupted: {}, missing: {}, valid: {valid}",
                        result.total,
                        result.verified,
                        result.corrupted.len(),
                        result.missing.len(),
                    ),
                }
            }
            Err(error) => response_from_error(error),
        }
    }

    #[allow(clippy::unused_self)]
    fn handle_unsubscribe(&self, namespace: &str) -> IpcResponse {
        match iroh_docs::NamespaceId::from_str(namespace) {
            Ok(namespace_id) => {
                if cancel_session(namespace_id) {
                    IpcResponse::Ok {
                        message: format!("unsubscribed: {namespace}"),
                    }
                } else {
                    IpcResponse::Error {
                        message: format!("no active session for {namespace}"),
                    }
                }
            }
            Err(error) => IpcResponse::Error {
                message: format!("invalid namespace: {error}"),
            },
        }
    }

    async fn handle_leave_folder(&self, namespace: String) -> IpcResponse {
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
        match manager.drop(namespace_id).await {
            Ok(()) => {
                let _ = self.daemon_handle.folder_registry.write().await.remove(&namespace_id);
                IpcResponse::Ok {
                    message: format!("left: {namespace}"),
                }
            }
            Err(error) => response_from_error(error),
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

    async fn handle_collection_publish(
        &self,
        path: PathBuf,
        namespace: String,
        sequence: u64,
        bootstrap: Vec<String>,
    ) -> IpcResponse {
        let context = match &self.archive_context {
            Some(ctx) => ctx.clone(),
            None => {
                return IpcResponse::Error {
                    message: "daemon collection-publish IPC is unavailable: server has no node context".to_owned(),
                };
            }
        };
        let manifest_path = path.join(".syncweb-collection.json");
        let manifest_bytes = match tokio::fs::read(&manifest_path).await {
            Ok(bytes) => bytes,
            Err(error) => {
                return IpcResponse::Error {
                    message: format!("failed to read collection manifest: {error}"),
                };
            }
        };
        let manifest = match CollectionManifest::from_bytes(manifest_bytes) {
            Ok(m) => m,
            Err(error) => return response_from_error(error),
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
        let catalog = PackageCatalog::new(context.node.gossip_service());
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
                .map_err(|error| SyncwebError::operation("daemon IPC connection failed", error))?;
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

            let stream = UnixStream::connect(&self.socket_path)?;
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
                | IpcResponse::ExportComplete(_) => Err(SyncwebError::operation(
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

fn set_owner_only_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o600);
        std::fs::set_permissions(path, permissions)?;
    }
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc, time::Duration};

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

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
    async fn client_reports_missing_server() {
        let path = socket_path();
        let error = IpcClient::from_socket_path(path)
            .send(IpcRequest::new(IpcCommand::Status))
            .await
            .expect_err("missing server should fail");
        assert!(error.to_string().contains("daemon IPC connection failed"));
    }

    #[test]
    fn new_commands_round_trip_as_json() {
        let req1 = IpcRequest::new(IpcCommand::Unsubscribe {
            namespace: "ns".to_owned(),
        });
        let enc1 = serde_json::to_vec(&req1).expect("serialize");
        let dec1: IpcRequest = serde_json::from_slice(&enc1).expect("deserialize");
        assert!(matches!(dec1.command, IpcCommand::Unsubscribe { .. }));

        let req2 = IpcRequest::new(IpcCommand::LeaveFolder {
            namespace: "ns".to_owned(),
        });
        let enc2 = serde_json::to_vec(&req2).expect("serialize");
        let dec2: IpcRequest = serde_json::from_slice(&enc2).expect("deserialize");
        assert!(matches!(dec2.command, IpcCommand::LeaveFolder { .. }));

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
        });
        let enc7 = serde_json::to_vec(&req7).expect("serialize");
        let dec7: IpcRequest = serde_json::from_slice(&enc7).expect("deserialize");
        assert!(matches!(dec7.command, IpcCommand::CollectionPublish { .. }));
    }

    #[tokio::test]
    async fn test_ipc_unsubscribe_no_active_session() {
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
            IpcResponse::Error { message } if message.contains("no active session")
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
            IpcResponse::Error { message } if message.contains("invalid namespace")
        ));
    }

    #[tokio::test]
    async fn test_ipc_leave_folder_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::LeaveFolder {
                namespace: "ns".to_owned(),
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
            }))
            .await;
        assert!(matches!(
            response,
            IpcResponse::Error { message } if message.contains("no node context")
        ));
    }

    #[tokio::test]
    async fn test_ipc_verify_integrity_no_context() {
        let handle = DaemonHandle::new(state());
        let server = IpcServer::new(socket_path(), handle);
        let response = server
            .handle_request(IpcRequest::new(IpcCommand::VerifyIntegrity {
                path: PathBuf::from("."),
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
            .handle_request(IpcRequest::new(IpcCommand::Subscribe {
                namespace: "ns".to_owned(),
                params: SubscribeParams::ingest_only(),
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

        let directory = std::env::temp_dir().join(format!("syncweb-ipc-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("test directory should be created");

        let identity = IdentityManager::new(directory.join("identity.key")).expect("test identity should open");
        let node = Arc::new(
            IrohNode::new(identity, directory.join("data"), RelayMode::Default)
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
        let server = IpcServer::with_archive_context(socket_path(), handle, node.clone(), pool);

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
                .handle_request(IpcRequest::new(IpcCommand::Subscribe {
                    namespace: ns.clone(),
                    params: SubscribeParams::ingest_only(),
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
            let params = SubscribeParams {
                ingest_only: true,
                ..SubscribeParams::default()
            };
            let response2 = fixture
                .server
                .handle_request(IpcRequest::new(IpcCommand::Subscribe {
                    namespace: ns.clone(),
                    params,
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
                .handle_request(IpcRequest::new(IpcCommand::LeaveFolder { namespace: ns.clone() }))
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
