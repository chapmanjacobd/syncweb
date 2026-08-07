use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

use iroh_docs::NamespaceId;
use n0_future::StreamExt;
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
};

use crate::{
    error::{Result, SyncwebError},
    filter::{FilterAction, FilterEngine, FilterEntry},
    folder::{FolderManager, PublicSubscription},
    fs::{FsWatcher, Importer},
    gossip::TopicChannel,
    indexing::{
        IndexingDatabase, IndexingService, REPORT_GOSSIP_TOPIC, ReportRecord,
        links::{MutablePointer, PrivateLink, REVOCATION_GOSSIP_TOPIC},
        resilience::{ReplicationBudget, ResilienceConfig},
        wot::{ATTESTATION_GOSSIP_TOPIC, Attestation},
    },
    net::{NetworkLogger, NetworkManager},
    node::{gossip_service::GossipService, identity::IdentityManager, iroh_node::IrohNode},
    schedule::ScheduleManager,
    storage::{config::SubscribeFilters, node_db::NodeDatabase, stats_db::StatsDatabase},
    sync::{SubscribeParams, SyncEngine, cancel_session, is_active},
};

use crate::node::iroh_node::discovery_scope;

use super::{
    DaemonHandle, DaemonState, DaemonStatus, FolderEntry, IpcServer, ManagedPool, PidLock, current_timestamp,
    daemon_socket_path,
    state::{BandwidthSnapshot, DaemonStatusReport, ScheduleStatus},
    supervisor::{IntentControls, IntentSupervisor, SupervisionOptions},
};

/// Configuration used to construct and run a daemon.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DaemonConfig {
    pub data_dir: PathBuf,
    pub network: Option<String>,
    pub sync_interval: Duration,
    pub observation_ttl: Duration,
    pub max_retries: u32,
    pub backoff_base: Duration,
    pub backoff_max: Duration,
    pub rayon_threads: usize,
    pub log_level: String,
    pub log_file: Option<PathBuf>,
    pub watch_debounce: Duration,
    pub relay_mode: crate::node::iroh_node::RelayMode,
    pub media_listen: Option<SocketAddr>,
    pub discovery: crate::node::iroh_node::DiscoveryConfig,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("."),
            network: None,
            sync_interval: Duration::from_mins(1),
            observation_ttl: Duration::from_hours(1),
            max_retries: 3,
            backoff_base: Duration::from_secs(1),
            backoff_max: Duration::from_mins(1),
            rayon_threads: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
            log_level: "info".to_owned(),
            log_file: None,
            watch_debounce: Duration::from_millis(500),
            relay_mode: crate::node::iroh_node::RelayMode::Default,
            media_listen: None,
            discovery: crate::node::iroh_node::DiscoveryConfig::default(),
        }
    }
}

impl DaemonConfig {
    #[must_use]
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            ..Self::default()
        }
    }
}

/// Owns the Iroh node, synchronization intents, IPC server, and daemon
/// lifecycle state.
pub struct Daemon {
    config: DaemonConfig,
    node_db: NodeDatabase,
    stats_db: StatsDatabase,
    pid_lock: PidLock,
    ipc_server: IpcServer,
    intent_supervisor: IntentSupervisor,
    folder_manager: FolderManager,
    sync_engine: SyncEngine,
    schedule_manager: tokio::sync::RwLock<Option<ScheduleManager>>,
    node: Arc<IrohNode>,
    handle: DaemonHandle,
    sync_receiver: tokio::sync::Mutex<mpsc::UnboundedReceiver<Option<String>>>,
    intent_tasks: Mutex<HashMap<NamespaceId, Option<JoinHandle<()>>>>,
    intent_controls: IntentControls,
    watchers: Mutex<HashMap<String, FsWatcher>>,
    pending_watch_events: Mutex<HashMap<String, PendingWatch>>,
    filter_engine: tokio::sync::RwLock<Option<FilterEngine>>,
    archive_pool: Arc<ManagedPool>,
    network_logger: NetworkLogger,
    network_manager: tokio::sync::RwLock<NetworkManager>,
    attestation_listener: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    report_listener: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    revocation_listener: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    maintenance_task: tokio::sync::Mutex<Option<JoinHandle<()>>>,
}

struct PendingWatch {
    paths: HashMap<PathBuf, bool>,
    ready_at: Instant,
}

type SignalTask<'a> = std::pin::Pin<std::boxed::Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>>;

impl Daemon {
    async fn open_identity_and_node(
        data_dir: &Path,
        node_db: &NodeDatabase,
        pid_lock: &PidLock,
        relay_mode: crate::node::iroh_node::RelayMode,
        discovery: crate::node::iroh_node::DiscoveryConfig,
        member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
    ) -> Result<(Arc<IrohNode>, FolderManager, SyncEngine)> {
        let identity = IdentityManager::new(data_dir.join("identity.key")).inspect_err(|_| {
            let _ = node_db.remove_lifecycle();
            let _ = pid_lock.release();
        })?;
        let node = Arc::new(
            IrohNode::new(identity, data_dir.join("data"), relay_mode, member_keys, discovery)
                .await
                .inspect_err(|_| {
                    let _ = node_db.remove_lifecycle();
                    let _ = pid_lock.release();
                })?,
        );
        let folder_manager = FolderManager::new(&node);
        let sync_engine = SyncEngine::new(
            folder_manager.clone(),
            node.blob_store().clone(),
            node.docs_engine().clone(),
            Some(node.topic_tracker().clone()),
        );
        Ok((node, folder_manager, sync_engine))
    }

    /// Open node and stats databases and acquire the process lock.
    ///
    /// # Errors
    ///
    /// Returns an error if a database cannot be opened or the lock is held by
    /// another process.
    fn init_databases(config: &DaemonConfig) -> Result<(NodeDatabase, StatsDatabase, PidLock)> {
        let node_db = NodeDatabase::open(config.data_dir.join("node.db"))?;
        let stats_db = StatsDatabase::open(config.data_dir.join("stats.db"))?;
        let pid_lock = PidLock::new(&config.data_dir);
        if !pid_lock.try_acquire()? {
            return Err(SyncwebError::operation(
                "daemon already running",
                config.data_dir.display(),
            ));
        }
        Ok((node_db, stats_db, pid_lock))
    }

    /// Load app config, schedule manager, filter engine, and write initial lifecycle state.
    ///
    /// # Errors
    ///
    /// Returns an error if loading config, filters, or saving the lifecycle fails.
    fn load_app_state(
        node_db: &NodeDatabase,
        pid_lock: &PidLock,
        data_dir: &Path,
    ) -> Result<(Option<ScheduleManager>, Option<FilterEngine>, DaemonState)> {
        let app_config = node_db.load_app_config().inspect_err(|_error| {
            let _ = pid_lock.release();
        })?;
        let use_schedule = app_config.schedule != crate::schedule::ScheduleConfig::default();
        let schedule_manager = match ScheduleManager::from_config(&app_config.schedule) {
            Ok(s) if use_schedule => Some(s),
            Ok(_) => None,
            Err(error) => {
                let _ = pid_lock.release();
                return Err(error);
            }
        };
        let filter_engine = node_db.load_filter_engine().inspect_err(|_error| {
            let _ = pid_lock.release();
        })?;
        let initial_state = DaemonState::new(
            std::process::id(),
            String::new(),
            current_timestamp(),
            data_dir,
            DaemonStatus::Starting,
        );
        node_db.save_lifecycle(&initial_state).inspect_err(|_error| {
            let _ = pid_lock.release();
        })?;
        Ok((schedule_manager, filter_engine, initial_state))
    }

    /// Create a daemon, acquire its process lock, and persist its running
    /// state.
    ///
    /// # Errors
    ///
    /// Returns an error if another daemon owns the data directory, the node
    /// cannot be opened, or configuration cannot be parsed.
    pub async fn new(config: DaemonConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.data_dir)?;
        let (node_db, stats_db, pid_lock) = Self::init_databases(&config)?;
        node_db.recover_transfer_jobs()?;
        let (schedule_manager, filter_engine, initial_state) =
            Self::load_app_state(&node_db, &pid_lock, &config.data_dir)?;

        let member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>> = {
            let networks = node_db.list_networks()?;
            let keys: HashSet<iroh::PublicKey> = networks.iter().flat_map(|n| n.members.iter().copied()).collect();
            Arc::new(RwLock::new(keys))
        };
        let mut node_discovery = config.discovery.clone();
        node_discovery.scope = discovery_scope(config.network.as_deref());
        let (node, folder_manager, mut sync_engine) = match Self::open_identity_and_node(
            &config.data_dir,
            &node_db,
            &pid_lock,
            config.relay_mode.clone(),
            node_discovery,
            member_keys.clone(),
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                let _ = node_db.remove_lifecycle();
                let _ = pid_lock.release();
                return Err(error);
            }
        };
        sync_engine = sync_engine.with_node_db(node_db.clone());

        let archive_pool = match ManagedPool::new("syncweb-archive", config.rayon_threads) {
            Ok(value) => Arc::new(value),
            Err(error) => {
                let _ = node.stop().await;
                let _ = node_db.remove_lifecycle();
                let _ = pid_lock.release();
                return Err(SyncwebError::operation("failed to create daemon thread pool", error));
            }
        };

        let initial_handle = DaemonHandle::new(initial_state);
        {
            let mut state = initial_handle.state.write().await;
            state.node_id = node.endpoint().id().to_string();
            state.status = DaemonStatus::Running;
            let running_state = state.clone();
            drop(state);
            if let Err(error) = node_db.save_lifecycle(&running_state) {
                let _ = node.stop().await;
                let _ = node_db.remove_lifecycle();
                let _ = pid_lock.release();
                return Err(error);
            }
        }
        let (sync_sender, sync_receiver) = mpsc::unbounded_channel();
        let handle = DaemonHandle::with_channels_and_reload(
            initial_handle.state.clone(),
            initial_handle.folder_registry.clone(),
            initial_handle.shutdown_sender.clone(),
            sync_sender,
            initial_handle.reload_requested.clone(),
        );
        let resilience = match IndexingService::new(config.data_dir.join("indexing.sqlite")) {
            Ok(indexing) => Some(indexing.resilience_service(ResilienceConfig::new(ReplicationBudget::default()))),
            Err(error) => {
                tracing::warn!(%error, "failed to open indexing database; resilience disabled");
                None
            }
        };
        let intent_supervisor = IntentSupervisor::new(config.max_retries, config.backoff_base, config.backoff_max);
        let network_logger = NetworkLogger::new(stats_db.clone());
        let local_node_id = node.endpoint().id();
        let nm_result =
            Self::create_network_manager(&node_db, &local_node_id, &network_logger, Arc::clone(&member_keys));
        let network_manager = match nm_result {
            Ok(nm) => nm,
            Err(error) => {
                tracing::warn!("failed to create network manager with logger: {error}");
                NetworkManager::new(node_db.clone(), local_node_id, member_keys)?
            }
        };
        let mut ipc_server = Self::build_ipc_server(&config, &handle, &node, &archive_pool, &folder_manager, &node_db);
        ipc_server = match resilience {
            Some(service) => ipc_server.with_resilience(service),
            None => ipc_server,
        };
        ipc_server = ipc_server.with_network_manager(Arc::new(tokio::sync::RwLock::new(network_manager.clone())));

        Ok(Self {
            config,
            node_db,
            stats_db,
            pid_lock,
            ipc_server,
            intent_supervisor,
            folder_manager,
            sync_engine,
            schedule_manager: tokio::sync::RwLock::new(schedule_manager),
            node,
            handle,
            sync_receiver: tokio::sync::Mutex::new(sync_receiver),
            intent_tasks: Mutex::new(HashMap::new()),
            intent_controls: Arc::new(Mutex::new(HashMap::new())),
            watchers: Mutex::new(HashMap::new()),
            pending_watch_events: Mutex::new(HashMap::new()),
            filter_engine: tokio::sync::RwLock::new(filter_engine),
            archive_pool,
            network_logger,
            network_manager: tokio::sync::RwLock::new(network_manager),
            attestation_listener: tokio::sync::Mutex::new(None),
            report_listener: tokio::sync::Mutex::new(None),
            revocation_listener: tokio::sync::Mutex::new(None),
            maintenance_task: tokio::sync::Mutex::new(None),
        })
    }

    fn create_network_manager(
        node_db: &NodeDatabase,
        local_node_id: &iroh::PublicKey,
        network_logger: &NetworkLogger,
        member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
    ) -> Result<NetworkManager> {
        NetworkManager::with_logger(node_db.clone(), *local_node_id, network_logger.clone(), member_keys)
    }

    fn build_ipc_server(
        config: &DaemonConfig,
        handle: &DaemonHandle,
        node: &Arc<IrohNode>,
        archive_pool: &Arc<ManagedPool>,
        folder_manager: &FolderManager,
        node_db: &NodeDatabase,
    ) -> IpcServer {
        IpcServer::with_archive_context(
            daemon_socket_path(&config.data_dir),
            handle.clone(),
            node.clone(),
            archive_pool.clone(),
        )
        .with_folder_manager(folder_manager.clone())
        .with_node_db(node_db.clone())
    }

    /// Run the daemon until IPC or operating-system shutdown is requested.
    ///
    /// # Errors
    ///
    /// Returns an error if an IPC listener, sync cycle, or cleanup operation
    /// fails.
    pub async fn run(&self) -> Result<()> {
        let run_result = self.run_inner().await;
        let cleanup_result = self.shutdown_resources().await;
        match run_result {
            Err(error) => {
                cleanup_result?;
                Err(error)
            }
            Ok(()) => cleanup_result,
        }
    }

    /// Return a snapshot of the daemon lifecycle state.
    pub async fn state(&self) -> DaemonState {
        self.handle.state.read().await.clone()
    }

    /// Return the daemon's fixed archive pool.
    #[must_use]
    pub fn archive_pool(&self) -> &ManagedPool {
        self.archive_pool.as_ref()
    }

    async fn run_inner(&self) -> Result<()> {
        tracing::debug!(
            rayon_threads = self.archive_pool.thread_count(),
            "daemon runtime initialized"
        );
        self.run_initial_setup().await?;
        let (mut server_task, media_task) = self.spawn_server_tasks();
        let (mut shutdown, mut signal_task, mut interval, mut watch_interval, shutdown_sender) = self.runtime_timers();
        if let Err(error) = self.run_cycle().await {
            tracing::error!(%error, "initial daemon cycle failed");
        }
        self.spawn_listeners().await;
        let result = self
            .run_event_loop(
                &mut server_task,
                &mut signal_task,
                &mut shutdown,
                &shutdown_sender,
                &mut interval,
                &mut watch_interval,
            )
            .await;
        send_shutdown(&shutdown_sender);
        self.wait_for_server_tasks(server_task, media_task).await?;
        result
    }

    async fn run_initial_setup(&self) -> Result<()> {
        self.load_folders().await?;
        self.reannounce_folders().await;
        self.load_subscriptions().await?;
        self.start_watching().await?;
        self.subscribe_network_gossip().await;
        self.spawn_membership_listeners().await;
        Ok(())
    }

    fn runtime_timers(
        &self,
    ) -> (
        broadcast::Receiver<()>,
        SignalTask<'_>,
        tokio::time::Interval,
        tokio::time::Interval,
        broadcast::Sender<()>,
    ) {
        let shutdown = self.handle.shutdown_sender.subscribe();
        let shutdown_sender = self.handle.shutdown_sender.clone();
        let signal_task: SignalTask<'_> = Box::pin(self.handle_signals(shutdown_sender.clone()));
        let interval_duration = self.config.sync_interval.max(Duration::from_millis(1));
        let interval = tokio::time::interval(interval_duration);
        let watch_interval = tokio::time::interval(Duration::from_millis(100));
        (shutdown, signal_task, interval, watch_interval, shutdown_sender)
    }

    async fn spawn_listeners(&self) {
        self.spawn_attestation_listener().await;
        self.spawn_report_listener().await;
        self.spawn_revocation_listener().await;
        self.spawn_maintenance_task().await;
    }

    fn spawn_server_tasks(&self) -> (JoinHandle<Result<()>>, Option<JoinHandle<Result<()>>>) {
        let server = self.ipc_server.clone();
        let server_task = tokio::spawn(async move { server.serve().await });

        let media_task = self.config.media_listen.map(|addr| {
            let media_srv = crate::media::MediaServer::new(addr, self.node.blob_store().clone());
            let shutdown = self.handle.shutdown_sender.clone();
            tokio::spawn(async move { media_srv.run(shutdown).await })
        });

        (server_task, media_task)
    }

    async fn wait_for_server_tasks(
        &self,
        server_task: JoinHandle<Result<()>>,
        media_task: Option<JoinHandle<Result<()>>>,
    ) -> Result<()> {
        if !server_task.is_finished() {
            match server_task.await {
                Ok(server_result) => server_result?,
                Err(error) => return Err(SyncwebError::operation("daemon IPC task failed", error)),
            }
        }
        if let Some(task) = media_task
            && !task.is_finished()
            && let Err(error) = task.await
        {
            tracing::warn!(%error, "media server task failed");
        }
        Ok(())
    }

    async fn run_event_loop(
        &self,
        server_task: &mut JoinHandle<Result<()>>,
        signal_task: &mut SignalTask<'_>,
        shutdown: &mut broadcast::Receiver<()>,
        shutdown_sender: &broadcast::Sender<()>,
        interval: &mut tokio::time::Interval,
        watch_interval: &mut tokio::time::Interval,
    ) -> Result<()> {
        loop {
            tokio::select! {
                signal_result = &mut *signal_task => {
                    signal_result?;
                    send_shutdown(shutdown_sender);
                    break Ok(());
                }
                shutdown_result = shutdown.recv() => {
                    if matches!(shutdown_result, Ok(()) | Err(broadcast::error::RecvError::Closed)) {
                        break Ok(());
                    }
                }
                server_result = &mut *server_task => {
                    match server_result {
                        Ok(result) => break result,
                        Err(error) => break Err(SyncwebError::operation("daemon IPC task failed", error)),
                    }
                }
                trigger = self.receive_sync_trigger() => {
                    match trigger {
                        Some(namespace) => self.run_trigger(namespace).await?,
                        None => break Ok(()),
                    }
                }
                _ = watch_interval.tick() => self.handle_watch_events().await?,
                _ = interval.tick() => self.run_cycle().await?,
            }
        }
    }

    async fn run_cycle(&self) -> Result<()> {
        self.reload_if_requested().await?;
        if let Err(error) = self.load_folders().await {
            tracing::warn!(%error, "folder load cycle failed — continuing");
        }
        self.start_watching().await?;
        let live_folders = self.enabled_subscribe_filters();
        let statuses = self.handle.folder_registry.read().await.statuses();
        for folder in statuses {
            let namespace = folder
                .namespace
                .parse::<NamespaceId>()
                .map_err(|error| SyncwebError::operation("invalid managed folder namespace", error))?;
            let folder_name = (!folder.path.as_os_str().is_empty()).then(|| folder.path.to_string_lossy().into_owned());
            let Some(filters) = live_folders.get(&namespace) else {
                if is_active(namespace) && !cancel_session(namespace) {
                    tracing::warn!(%namespace, "live syncing is disabled; intent did not accept cancellation");
                }
                continue;
            };
            let active = {
                let schedule_manager = self.schedule_manager.read().await;
                schedule_manager
                    .as_ref()
                    .is_none_or(|manager| manager.is_active(folder_name.as_deref()))
            };
            self.start_supervision(namespace, filters).await?;
            if active {
                self.set_intent_active(namespace, true)?;
            } else {
                self.set_intent_active(namespace, false)?;
            }
        }
        self.save_status_report().await?;
        Ok(())
    }

    async fn run_trigger(&self, namespace: Option<String>) -> Result<()> {
        match namespace {
            Some(value) => {
                let parsed_namespace = value
                    .parse::<NamespaceId>()
                    .map_err(|error| SyncwebError::operation("invalid sync namespace", error))?;
                let tracked = self
                    .handle
                    .folder_registry
                    .read()
                    .await
                    .statuses()
                    .iter()
                    .any(|status| status.namespace == value);
                if !tracked {
                    tracing::warn!(%parsed_namespace, "not a syncweb folder; nothing to synchronize");
                    return Ok(());
                }
                let Some(filters) = self.enabled_subscribe_filters().get(&parsed_namespace).cloned() else {
                    tracing::warn!(
                        %parsed_namespace,
                        "live syncing is disabled for this folder; enable it with `join --subscribe` \
                         or `config set <folder>.subscribe on`"
                    );
                    return Ok(());
                };
                self.start_supervision(parsed_namespace, &filters).await?;
                self.set_intent_active(parsed_namespace, true)?;
                self.save_status_report().await?;
                Ok(())
            }
            None => self.run_cycle().await,
        }
    }

    /// The `subscribe-changes` folders with live syncing enabled, mapped to their filters.
    fn enabled_subscribe_filters(&self) -> std::collections::BTreeMap<NamespaceId, SubscribeFilters> {
        let Ok(config) = self.node_db.load_app_config() else {
            return std::collections::BTreeMap::new();
        };
        config
            .subscribe
            .folders
            .into_iter()
            .filter(|(_, entry)| entry.enabled)
            .filter_map(|(namespace, entry)| namespace.parse::<NamespaceId>().ok().map(|id| (id, entry.filters)))
            .collect()
    }

    async fn start_supervision(&self, namespace: NamespaceId, filters: &SubscribeFilters) -> Result<()> {
        {
            let mut tasks = self
                .intent_tasks
                .lock()
                .map_err(|error| SyncwebError::operation("daemon intent task mutex is poisoned", error))?;
            tasks.retain(|_, task| task.as_ref().is_none_or(|handle| !handle.is_finished()));
            if tasks.contains_key(&namespace) || is_active(namespace) {
                return Ok(());
            }
            tasks.insert(namespace, None);
        }
        let network_id = {
            let guard = self.network_manager.read().await;
            guard.network_for_folder(&namespace)?
        };
        let session_id = network_id.as_ref().and_then(|net_id| {
            self.network_logger
                .record_sync_start(net_id, &namespace.to_string())
                .ok()
        });
        let sync = self.sync_engine.clone();
        let supervisor = self.intent_supervisor;
        let shutdown = self.handle.shutdown_sender.subscribe();
        let controls = self.intent_controls.clone();
        let filter = self.filter_engine.read().await.clone();
        let network_logger = self.network_logger.clone();
        let folder_name = self
            .handle
            .folder_registry
            .read()
            .await
            .statuses()
            .into_iter()
            .find(|status| status.namespace == namespace.to_string())
            .and_then(|status| {
                (!status.path.as_os_str().is_empty()).then(|| status.path.to_string_lossy().into_owned())
            });
        let bandwidth = self
            .schedule_manager
            .read()
            .await
            .as_ref()
            .map(|manager| manager.current_limits(folder_name.as_deref()));
        let mut params = match SubscribeParams::from_filters(filters) {
            Ok(params) => params,
            Err(error) => {
                tracing::warn!(%namespace, %error, "invalid subscription filters; supervising without them");
                SubscribeParams::default()
            }
        };
        if let Some(limits) = bandwidth {
            params = params.with_bandwidth_limits(limits);
        }
        let (ready_sender, ready_receiver) = oneshot::channel();
        let task = tokio::spawn(async move {
            let result = supervisor
                .supervise_with_controls_and_ready(
                    &sync,
                    namespace,
                    params,
                    shutdown,
                    SupervisionOptions::with_ready(controls, filter, ready_sender),
                )
                .await;
            if network_id.is_some()
                && let Some(sid) = session_id
            {
                match &result {
                    Ok(supervised) => {
                        let has_error = supervised.last_error.is_some();
                        let files = 0;
                        let bytes = 0;
                        let errors = u64::from(has_error);
                        let status = if has_error { "failed" } else { "completed" };
                        let _ = network_logger.record_sync_finish(sid, files, bytes, errors, status);
                    }
                    Err(_) => {
                        let _ = network_logger.record_sync_finish(sid, 0, 0, 1, "failed");
                    }
                }
            }
            match &result {
                Ok(supervised) => {
                    if let Some(error) = &supervised.last_error {
                        tracing::warn!(%namespace, retry_count = supervised.retry_count, %error, "supervised intent stopped");
                    }
                }
                Err(error) => tracing::error!(%namespace, %error, "supervised intent failed"),
            }
        });
        self.intent_tasks
            .lock()
            .map_err(|error| SyncwebError::operation("daemon intent task mutex is poisoned", error))?
            .insert(namespace, Some(task));
        ready_receiver
            .await
            .map_err(|error| SyncwebError::operation("daemon intent startup notification failed", error))?;
        Ok(())
    }

    fn set_intent_active(&self, namespace: NamespaceId, active: bool) -> Result<()> {
        let control = self
            .intent_controls
            .lock()
            .map_err(|error| SyncwebError::operation("daemon intent control mutex is poisoned", error))?
            .get(&namespace)
            .cloned();
        let Some(intent_control) = control else {
            return Ok(());
        };
        let result = if active {
            intent_control.resume()
        } else {
            intent_control.pause()
        };
        result.map_err(|error| SyncwebError::operation("failed to update scheduled intent", error))
    }

    /// Return whether the global schedule is currently active.
    pub async fn is_in_active_window(&self) -> bool {
        self.schedule_manager
            .read()
            .await
            .as_ref()
            .is_none_or(|manager| manager.is_active(None))
    }

    /// Return the configured download limit at the current wall-clock time.
    pub async fn current_bandwidth_limit(&self) -> Option<u64> {
        self.schedule_manager.read().await.as_ref().and_then(|manager| {
            let limits = manager.current_limits(None);
            limits.max_download.or(limits.max_upload)
        })
    }

    async fn start_watching(&self) -> Result<()> {
        let statuses = self.handle.folder_registry.read().await.statuses();
        let wanted: HashMap<_, _> = statuses
            .iter()
            .filter(|status| !status.path.as_os_str().is_empty())
            .map(|status| (status.namespace.clone(), status.path.clone()))
            .collect();
        let mut watchers = self
            .watchers
            .lock()
            .map_err(|error| SyncwebError::operation("daemon watcher mutex is poisoned", error))?;
        watchers.retain(|namespace, _| wanted.contains_key(namespace));
        for (namespace, path) in wanted {
            if watchers.contains_key(&namespace) {
                continue;
            }
            if !path.exists() {
                tracing::warn!(%namespace, path = %path.display(), "managed folder path does not exist; watcher deferred");
                continue;
            }
            watchers.insert(namespace, FsWatcher::new(&path)?);
        }
        drop(watchers);
        Ok(())
    }

    async fn handle_watch_events(&self) -> Result<()> {
        let mut observed = Vec::new();
        {
            let mut watchers = self
                .watchers
                .lock()
                .map_err(|error| SyncwebError::operation("daemon watcher mutex is poisoned", error))?;
            for (namespace, watcher) in watchers.iter_mut() {
                loop {
                    match watcher.try_recv() {
                        Ok(Some(event)) => observed.push((namespace.clone(), event)),
                        Ok(None) => break,
                        Err(error) => {
                            tracing::warn!(%namespace, %error, "filesystem watcher event channel failed");
                            break;
                        }
                    }
                }
            }
        }

        if !observed.is_empty() {
            let mut pending = self
                .pending_watch_events
                .lock()
                .map_err(|error| SyncwebError::operation("daemon watch queue mutex is poisoned", error))?;
            let ready_at = Instant::now()
                .checked_add(self.config.watch_debounce)
                .unwrap_or_else(Instant::now);
            for (namespace, event) in observed {
                let removed = matches!(event.event.kind, notify::EventKind::Remove(_));
                let entry = pending.entry(namespace).or_insert_with(|| PendingWatch {
                    paths: HashMap::new(),
                    ready_at,
                });
                entry.ready_at = ready_at;
                for path in event.paths {
                    entry.paths.insert(path, removed);
                }
            }
            drop(pending);
        }

        let ready = {
            let mut pending = self
                .pending_watch_events
                .lock()
                .map_err(|error| SyncwebError::operation("daemon watch queue mutex is poisoned", error))?;
            let now = Instant::now();
            pending
                .iter_mut()
                .filter_map(|(namespace, batch)| {
                    (batch.ready_at <= now).then(|| (namespace.clone(), std::mem::take(&mut batch.paths)))
                })
                .filter(|(_, paths)| !paths.is_empty())
                .collect::<Vec<_>>()
        };
        if ready.is_empty() {
            return Ok(());
        }

        let roots: HashMap<_, _> = self
            .handle
            .folder_registry
            .read()
            .await
            .statuses()
            .into_iter()
            .map(|status| (status.namespace, status.path))
            .collect();
        let live_folders = self.enabled_subscribe_filters();
        for (namespace, paths) in ready {
            let Some(root) = roots.get(&namespace) else {
                continue;
            };
            let namespace_id = namespace
                .parse::<NamespaceId>()
                .map_err(|error| SyncwebError::operation("invalid watched folder namespace", error))?;
            for (path, removed) in paths {
                self.process_watch_event(namespace_id, root, &path, removed, live_folders.get(&namespace_id))
                    .await?;
            }
        }
        self.save_status_report().await?;
        Ok(())
    }

    async fn process_watch_event(
        &self,
        namespace: NamespaceId,
        root: &Path,
        path: &Path,
        removed: bool,
        live_filters: Option<&SubscribeFilters>,
    ) -> Result<()> {
        let relative = path.strip_prefix(root).unwrap_or(path);
        if relative.as_os_str().is_empty() {
            return Ok(());
        }
        let size = std::fs::metadata(path).map_or(0, |metadata| metadata.len());
        let accepted = self.filter_engine.read().await.as_ref().is_none_or(|filter| {
            filter.evaluate_for_folder(&namespace.to_string(), &FilterEntry::new(relative.to_path_buf(), size))
                != FilterAction::Reject
        });
        if !accepted {
            return Ok(());
        }

        let folder = self.folder_manager.get(namespace).await?;
        let result = if removed || !path.exists() {
            folder
                .delete_entry(relative.as_os_str().as_encoded_bytes())
                .await
                .map(|()| 1_u64)
        } else if path.is_file() {
            let importer = Importer::new(
                self.node.blob_store().clone(),
                self.node.docs_engine().clone(),
                folder.doc().clone(),
                folder.author(),
            )
            .with_root(root);
            importer
                .import_path(path)
                .await
                .map(|entries| u64::try_from(entries.len()).unwrap_or(u64::MAX))
        } else {
            Ok(0)
        };
        match result {
            Ok(entries) if entries > 0 => {
                self.handle
                    .folder_registry
                    .write()
                    .await
                    .record_import(namespace, entries, current_timestamp());
                if let Some(filters) = live_filters {
                    self.start_supervision(namespace, filters).await?;
                }
            }
            Ok(_) => {}
            Err(error) => {
                self.handle
                    .folder_registry
                    .write()
                    .await
                    .record_error(namespace, error.to_string());
                tracing::warn!(%namespace, path = %path.display(), %error, "filesystem change will be retried");
                if !removed && is_recoverable_watch_error(&error) {
                    let mut pending = self.pending_watch_events.lock().map_err(|poisoned| {
                        SyncwebError::operation("daemon watch queue mutex is poisoned", poisoned)
                    })?;
                    let ready_at = Instant::now()
                        .checked_add(self.config.watch_debounce)
                        .unwrap_or_else(Instant::now);
                    let entry = pending.entry(namespace.to_string()).or_insert_with(|| PendingWatch {
                        paths: HashMap::new(),
                        ready_at,
                    });
                    entry.ready_at = ready_at;
                    entry.paths.insert(path.to_path_buf(), false);
                    drop(pending);
                }
            }
        }
        Ok(())
    }

    async fn save_status_report(&self) -> Result<()> {
        let state = self.handle.state.read().await.clone();
        let statuses = self.handle.folder_registry.read().await.statuses();
        let schedule = self.schedule_manager.read().await.clone();
        let schedule_report = schedule.as_ref().map(|manager| {
            let minute = crate::schedule::current_minute();
            let next = manager.next_active_window_start_at(None, minute);
            let next_window_start = next.map(|next_minute| {
                let offset = if next_minute >= minute {
                    next_minute.saturating_sub(minute)
                } else {
                    1_440_u16.saturating_sub(minute).saturating_add(next_minute)
                };
                current_timestamp().saturating_add(u64::from(offset).saturating_mul(60))
            });
            ScheduleStatus {
                in_active_window: manager.is_active(None),
                next_window_start,
            }
        });
        let current_stats = self.stats_db.current_stats().unwrap_or_default();
        let report = DaemonStatusReport {
            pid: state.pid,
            node_id: state.node_id,
            started_at: state.started_at,
            uptime_seconds: current_timestamp().saturating_sub(state.started_at),
            folders: statuses.into_iter().collect(),
            bandwidth: BandwidthSnapshot {
                upload_total: current_stats.total_upload,
                download_total: current_stats.total_download,
                upload_rate: 0,
                download_rate: 0,
            },
            schedule: schedule_report,
            rayon_threads: self.archive_pool.thread_count(),
        };
        self.node_db.save_status(&report)
    }

    async fn load_folders(&self) -> Result<()> {
        let folders = self.folder_manager.list().await?;
        let mut registry = self.handle.folder_registry.write().await;
        for folder in folders {
            let namespace_key = folder.namespace_id().to_string();
            if registry.is_removed(&namespace_key) {
                continue;
            }
            if !registry
                .statuses()
                .iter()
                .any(|status| status.namespace == namespace_key)
            {
                registry.add(FolderEntry::new(folder.namespace_id(), PathBuf::new()))?;
            }
        }
        drop(registry);
        Ok(())
    }

    async fn reannounce_folders(&self) {
        let Ok(folders) = self.folder_manager.list().await else {
            return;
        };
        for folder in &folders {
            self.folder_manager.announce_namespace(folder.namespace_id()).await;
        }
    }

    async fn load_subscriptions(&self) -> Result<()> {
        let subscriptions = self.node_db.load_subscriptions_with_sizes()?;
        if subscriptions.is_empty() {
            return Ok(());
        }
        let hashes: Vec<_> = subscriptions.iter().map(|(hash, _)| *hash).collect();
        self.folder_manager.seed_subscriptions(hashes).await;

        let mut registry = self.handle.folder_registry.write().await;
        for (hash, size) in subscriptions {
            let subscription = PublicSubscription::new(hash, None, size);
            registry.add_subscription(subscription);
        }
        drop(registry);
        Ok(())
    }

    async fn reload_if_requested(&self) -> Result<()> {
        if !self
            .handle
            .reload_requested
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            return Ok(());
        }
        let app_config = self.node_db.load_app_config()?;
        let parsed_schedule = ScheduleManager::from_config(&app_config.schedule)?;
        let schedule_manager =
            (app_config.schedule != crate::schedule::ScheduleConfig::default()).then_some(parsed_schedule);
        let filter = self.node_db.load_filter_engine()?;
        *self.schedule_manager.write().await = schedule_manager;
        *self.filter_engine.write().await = filter;
        let statuses = self.handle.folder_registry.read().await.statuses();
        for status in statuses {
            if let Ok(namespace) = status.namespace.parse::<NamespaceId>()
                && is_active(namespace)
                && !cancel_session(namespace)
            {
                tracing::warn!(%namespace, "reloaded intent did not accept cancellation");
            }
        }
        tracing::info!("daemon configuration reloaded");
        Ok(())
    }

    async fn receive_sync_trigger(&self) -> Option<Option<String>> {
        self.sync_receiver.lock().await.recv().await
    }

    async fn shutdown_resources(&self) -> Result<()> {
        self.handle.set_status(DaemonStatus::Stopping).await;
        let stopping_state = self.handle.state.read().await.clone();
        self.node_db.save_lifecycle(&stopping_state)?;
        self.save_status_report().await?;

        self.cancel_active_sessions().await;
        self.join_intent_tasks().await?;
        self.abort_listeners().await;
        self.finalize_node_stop().await
    }

    async fn cancel_active_sessions(&self) {
        let namespaces: Vec<_> = self
            .handle
            .folder_registry
            .read()
            .await
            .statuses()
            .into_iter()
            .filter_map(|status| status.namespace.parse::<NamespaceId>().ok())
            .collect();
        for namespace in namespaces {
            if is_active(namespace) && !cancel_session(namespace) {
                tracing::warn!(%namespace, "intent did not accept shutdown cancellation");
            }
        }
    }

    async fn join_intent_tasks(&self) -> Result<()> {
        let tasks = {
            let mut task_map = self
                .intent_tasks
                .lock()
                .map_err(|error| SyncwebError::operation("daemon intent task mutex is poisoned", error))?;
            std::mem::take(&mut *task_map)
        };
        for (namespace, task) in tasks {
            if let Some(handle) = task {
                match tokio::time::timeout(Duration::from_secs(10), handle).await {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        tracing::warn!(%namespace, %error, "daemon intent task failed");
                    }
                    Err(_) => {
                        tracing::warn!(%namespace, "daemon intent task did not shut down within 10s timeout");
                    }
                }
            }
        }
        Ok(())
    }

    async fn abort_listeners(&self) {
        let attestation_handle = self.attestation_listener.lock().await.take();
        if let Some(inner) = attestation_handle {
            inner.abort();
        }
        let report_handle = self.report_listener.lock().await.take();
        if let Some(inner) = report_handle {
            inner.abort();
        }
        let revocation_handle = self.revocation_listener.lock().await.take();
        if let Some(inner) = revocation_handle {
            inner.abort();
        }
        let maintenance_handle = self.maintenance_task.lock().await.take();
        if let Some(inner) = maintenance_handle {
            inner.abort();
        }
    }

    async fn finalize_node_stop(&self) -> Result<()> {
        let node_result = tokio::time::timeout(Duration::from_secs(30), self.node.stop()).await;
        self.handle.set_status(DaemonStatus::Stopped).await;
        let remove_result = self.node_db.remove_lifecycle();
        let remove_status_result = self.node_db.remove_status();
        let release_result = self.pid_lock.release();
        match node_result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%error, "node stop reported an error"),
            Err(_) => tracing::warn!("node stop timed out after 30s"),
        }
        remove_result?;
        remove_status_result?;
        release_result?;
        Ok(())
    }

    async fn spawn_attestation_listener(&self) {
        let gossip_service = self.node.gossip_service().clone();
        let data_dir = self.config.data_dir.clone();
        let shutdown = self.handle.shutdown_sender.subscribe();
        let handle = tokio::spawn(async move {
            if let Err(error) = listen_for_attestations(gossip_service, data_dir, shutdown).await {
                tracing::error!(%error, "attestation gossip listener failed");
            }
        });
        *self.attestation_listener.lock().await = Some(handle);
    }

    async fn spawn_report_listener(&self) {
        let gossip_service = self.node.gossip_service().clone();
        let data_dir = self.config.data_dir.clone();
        let shutdown = self.handle.shutdown_sender.subscribe();
        let handle = tokio::spawn(async move {
            if let Err(error) = listen_for_reports(gossip_service, data_dir, shutdown).await {
                tracing::error!(%error, "report gossip listener failed");
            }
        });
        *self.report_listener.lock().await = Some(handle);
    }

    async fn spawn_revocation_listener(&self) {
        let gossip_service = self.node.gossip_service().clone();
        let data_dir = self.config.data_dir.clone();
        let shutdown = self.handle.shutdown_sender.subscribe();
        let handle = tokio::spawn(async move {
            if let Err(error) = listen_for_revocations(gossip_service, data_dir, shutdown).await {
                tracing::error!(%error, "revocation gossip listener failed");
            }
        });
        *self.revocation_listener.lock().await = Some(handle);
    }

    /// Spawn a background task that periodically vacuums databases with
    /// excessive freelist pages.
    async fn spawn_maintenance_task(&self) {
        let node_db = self.node_db.clone();
        let stats_db = self.stats_db.clone();
        let mut shutdown = self.handle.shutdown_sender.subscribe();
        let handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_hours(24));
            loop {
                tokio::select! {
                    _ = shutdown.recv() => {
                        tracing::info!("maintenance task shutting down");
                        break;
                    }
                    _ = interval.tick() => {
                        let dbs: [&dyn crate::storage::Vacuumable; 2] = [&node_db, &stats_db];
                        for db in dbs {
                            match db.freelist_count() {
                                Ok(count) if count > 100 => {
                                    tracing::info!(freelist = %count, "running vacuum");
                                    if let Err(error) = db.vacuum() {
                                        tracing::warn!(%error, "maintenance vacuum failed");
                                    }
                                }
                                Ok(_) => {}
                                Err(error) => tracing::warn!(%error, "maintenance freelist query failed"),
                            }
                        }
                    }
                }
            }
        });
        *self.maintenance_task.lock().await = Some(handle);
    }

    async fn subscribe_network_gossip(&self) {
        let networks: Vec<_> = {
            let guard = self.network_manager.read().await;
            guard.list().into_iter().cloned().collect::<Vec<_>>()
        };
        for network in &networks {
            let network_id = network.id;
            let topic = network.topic;
            let members: Vec<_> = network
                .members
                .iter()
                .copied()
                .filter(|m| *m != self.node.endpoint().id())
                .collect();
            match self.node.gossip_service().subscribe(topic, members).await {
                Ok(_topic) => {
                    tracing::debug!(%network_id, "subscribed to network gossip topic");
                }
                Err(error) => {
                    tracing::warn!(%network_id, %error, "failed to subscribe to network gossip topic");
                }
            }
        }
    }

    async fn spawn_membership_listeners(&self) {
        let networks: Vec<_> = {
            let guard = self.network_manager.read().await;
            guard.list().into_iter().cloned().collect::<Vec<_>>()
        };
        let docs_engine = self.node.docs_engine().clone();
        let blob_store = self.node.blob_store().clone();
        let shutdown = self.handle.shutdown_sender.subscribe();
        let local_pk = self.node.endpoint().id();
        let node_db = self.node_db.clone();
        for network in networks {
            let Some(ref doc_ticket_str) = network.doc_ticket else {
                continue;
            };
            let Ok(doc_ticket) = doc_ticket_str.parse::<iroh_docs::DocTicket>() else {
                tracing::warn!(network_id = %network.id, "invalid doc_ticket in network");
                continue;
            };
            let de = docs_engine.clone();
            let bs = blob_store.clone();
            let mut member_shutdown = shutdown.resubscribe();
            let network_id = network.id;
            let local_key_str = local_pk.to_string();
            let db = node_db.clone();
            tokio::spawn(async move {
                let doc = match de.import_ticket(doc_ticket).await {
                    Ok(doc) => doc,
                    Err(error) => {
                        tracing::warn!(%network_id, %error, "failed to import membership doc from ticket");
                        return;
                    }
                };
                if let Err(error) = de.start_sync(&doc, Vec::new()).await {
                    tracing::warn!(%network_id, %error, "failed to start membership doc sync");
                }
                let mut stream = match de.watch(&doc).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        tracing::warn!(%network_id, %error, "failed to watch membership doc");
                        return;
                    }
                };
                loop {
                    tokio::select! {
                        _ = member_shutdown.recv() => {
                            tracing::debug!(%network_id, "membership listener shutting down");
                            break;
                        }
                        event = stream.next() => {
                            let Some(event_result) = event else {
                                break;
                            };
                            match event_result {
                                Ok(iroh_docs::engine::LiveEvent::InsertLocal { entry } | iroh_docs::engine::LiveEvent::InsertRemote { entry, .. }) => {
                                    if entry.key() == b"sys/network/members" {
                                        let hash = entry.content_hash();
                                        let Ok(content) = bs.get(hash).await else { continue; };
                                        match serde_json::from_slice::<crate::net::membership_doc::SignedMemberList>(&content) {
                                            Ok(member_list) => {
                                                if let Err(error) = member_list.verify() {
                                                    tracing::warn!(%network_id, %error, "invalid membership list signature");
                                                    continue;
                                                }
                                                let is_member = member_list.members.iter().any(|m| m.key == local_key_str);
                                                if is_member {
                                                    tracing::debug!(%network_id, members = member_list.members.len(), "membership updated");
                                                } else {
                                                    tracing::warn!(%network_id, "local node was removed from network");
                                                    let _ = db.delete_network(network_id);
                                                }
                                            }
                                            Err(error) => {
                                                tracing::warn!(%network_id, %error, "failed to parse membership list");
                                            }
                                        }
                                    }
                                }
                                Ok(_) => {}
                                Err(error) => {
                                    tracing::warn!(%network_id, %error, "membership doc event error");
                                }
                            }
                        }
                    }
                }
            });
        }
    }

    async fn handle_signals(&self, shutdown: broadcast::Sender<()>) -> Result<()> {
        #[cfg(unix)]
        {
            let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
            tokio::select! {
                result = tokio::signal::ctrl_c() => result?,
                _ = terminate.recv() => {}
            }
        }
        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c().await?;
        }
        send_shutdown(&shutdown);
        Ok(())
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        if let Err(error) = self.node_db.remove_lifecycle() {
            tracing::warn!(%error, "failed to remove daemon lifecycle");
        }
        if let Err(error) = self.node_db.remove_status() {
            tracing::warn!(%error, "failed to remove daemon status");
        }
    }
}

fn send_shutdown(sender: &broadcast::Sender<()>) {
    match sender.send(()) {
        Ok(_) | Err(broadcast::error::SendError(())) => {}
    }
}

fn is_recoverable_watch_error(error: &SyncwebError) -> bool {
    let message = error.to_string();
    message.contains("file changed during import") || message.contains("input path does not exist")
}

fn attestation_topic_id() -> iroh_gossip::TopicId {
    iroh_gossip::TopicId::from_bytes(*blake3::hash(ATTESTATION_GOSSIP_TOPIC).as_bytes())
}

fn persist_incoming_attestation(att: &Attestation, db: &IndexingDatabase, existing: &mut Vec<Attestation>) {
    if existing.contains(att) {
        return;
    }
    existing.push(att.clone());
    if let Err(error) = db.save_attestations(existing) {
        tracing::warn!(%error, "failed to persist incoming attestation");
    }
    tracing::debug!(
        content = %att.content,
        issuer = %att.issuer,
        kind = %att.kind,
        "attestation received via gossip"
    );
}

async fn listen_for_attestations(
    gossip_service: GossipService,
    data_dir: PathBuf,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<()> {
    let topic = gossip_service
        .subscribe_and_join(attestation_topic_id(), Vec::new())
        .await?;
    let (sender, receiver) = GossipService::split(topic);
    let topic_channel = TopicChannel::<Attestation>::new(
        Arc::new(gossip_service.inner().clone()),
        ATTESTATION_GOSSIP_TOPIC,
        sender,
    );
    let mut stream = topic_channel.receive_from(receiver);
    let db = IndexingDatabase::open(data_dir.join("indexing.sqlite"))?;
    let mut existing = db.load_attestations()?;

    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                tracing::info!("attestation listener shutting down");
                break Ok(());
            }
            msg = stream.next() => {
                let Some(att) = msg else {
                    break Ok(());
                };
                persist_incoming_attestation(&att, &db, &mut existing);
            }
        }
    }
}

fn report_topic_id() -> iroh_gossip::TopicId {
    iroh_gossip::TopicId::from_bytes(*blake3::hash(REPORT_GOSSIP_TOPIC).as_bytes())
}

fn revocation_topic_id() -> iroh_gossip::TopicId {
    iroh_gossip::TopicId::from_bytes(*blake3::hash(REVOCATION_GOSSIP_TOPIC).as_bytes())
}

async fn listen_for_revocations(
    gossip_service: GossipService,
    data_dir: PathBuf,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<()> {
    let topic = gossip_service
        .subscribe_and_join(revocation_topic_id(), Vec::new())
        .await?;
    let (sender, receiver) = GossipService::split(topic);
    let topic_channel = TopicChannel::<PrivateLink>::new(
        Arc::new(gossip_service.inner().clone()),
        REVOCATION_GOSSIP_TOPIC,
        sender,
    );
    let mut stream = topic_channel.receive_from(receiver);
    let db = IndexingDatabase::open(data_dir.join("indexing.sqlite"))?;
    let (pointers, mirrors, mut revoked) = db.load_links()?;

    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                tracing::info!("revocation listener shutting down");
                break Ok(());
            }
            msg = stream.next() => {
                handle_revocation_message(msg, &mut revoked, &pointers, &mirrors, &db);
            }
        }
    }
}

fn handle_revocation_message(
    msg: Option<PrivateLink>,
    revoked: &mut Vec<PrivateLink>,
    pointers: &[MutablePointer],
    mirrors: &[String],
    db: &IndexingDatabase,
) {
    let Some(revocation) = msg else {
        return;
    };
    if !revoked.contains(&revocation) {
        revoked.push(revocation.clone());
        tracing::debug!(
            manifest = %revocation.manifest,
            "revocation received via gossip"
        );
        if let Err(error) = db.save_links(pointers, mirrors, revoked) {
            tracing::warn!(%error, "failed to persist incoming revocation");
        }
    }
}

async fn listen_for_reports(
    gossip_service: GossipService,
    data_dir: PathBuf,
    mut shutdown: broadcast::Receiver<()>,
) -> Result<()> {
    let topic = gossip_service.subscribe_and_join(report_topic_id(), Vec::new()).await?;
    let (sender, receiver) = GossipService::split(topic);
    let topic_channel =
        TopicChannel::<ReportRecord>::new(Arc::new(gossip_service.inner().clone()), REPORT_GOSSIP_TOPIC, sender);
    let mut stream = topic_channel.receive_from(receiver);
    let db = IndexingDatabase::open(data_dir.join("indexing.sqlite"))?;
    let mut existing = db.load_content_reports()?;

    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                tracing::info!("report listener shutting down");
                break Ok(());
            }
            msg = stream.next() => {
                if !handle_report_message(msg, &mut existing, &db) {
                    break Ok(());
                }
            }
        }
    }
}

fn handle_report_message(msg: Option<ReportRecord>, existing: &mut Vec<ReportRecord>, db: &IndexingDatabase) -> bool {
    let Some(report) = msg else {
        return false;
    };
    tracing::debug!(
        content = %report.content,
        reason = %report.reason,
        "report received via gossip"
    );
    if !existing.contains(&report) {
        existing.push(report);
        if let Err(error) = db.save_content_reports(existing) {
            tracing::warn!(%error, "failed to persist incoming report");
        }
    }
    true
}
