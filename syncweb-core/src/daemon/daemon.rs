use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use iroh_docs::NamespaceId;
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};

use crate::{
    error::{Result, SyncwebError},
    filter::{FilterAction, FilterEngine, FilterEntry},
    folder::FolderManager,
    fs::{FsWatcher, Importer},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
    schedule::ScheduleManager,
    stats::BandwidthStats,
    storage::Config as AppConfig,
    sync::{SubscribeParams, SyncEngine, cancel_session, is_active},
};

use super::{
    DaemonHandle, DaemonState, DaemonStatus, FolderEntry, IpcServer, ManagedPool, PidLock, StateFile,
    current_timestamp, daemon_socket_path,
    state::{BandwidthSnapshot, DaemonStatusReport, FolderStatusReport, ScheduleStatus},
    supervisor::{IntentControls, IntentSupervisor},
};

/// Configuration used to construct and run a daemon.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DaemonConfig {
    pub data_dir: PathBuf,
    pub sync_interval: Duration,
    pub observation_ttl: Duration,
    pub max_retries: u32,
    pub backoff_base: Duration,
    pub backoff_max: Duration,
    pub rayon_threads: usize,
    pub log_level: String,
    pub log_file: Option<PathBuf>,
    pub watch_debounce: Duration,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("."),
            sync_interval: Duration::from_mins(1),
            observation_ttl: Duration::from_hours(1),
            max_retries: 3,
            backoff_base: Duration::from_secs(1),
            backoff_max: Duration::from_mins(1),
            rayon_threads: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
            log_level: "info".to_owned(),
            log_file: None,
            watch_debounce: Duration::from_millis(500),
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
    state_file: StateFile,
    pid_lock: PidLock,
    ipc_server: IpcServer,
    intent_supervisor: IntentSupervisor,
    folder_manager: FolderManager,
    sync_engine: SyncEngine,
    schedule_manager: tokio::sync::RwLock<Option<ScheduleManager>>,
    node: Arc<IrohNode>,
    handle: DaemonHandle,
    sync_receiver: tokio::sync::Mutex<mpsc::UnboundedReceiver<Option<String>>>,
    intent_tasks: Mutex<HashMap<NamespaceId, JoinHandle<()>>>,
    intent_controls: IntentControls,
    watchers: Mutex<HashMap<String, FsWatcher>>,
    pending_watch_events: Mutex<HashMap<String, PendingWatch>>,
    filter_engine: tokio::sync::RwLock<Option<FilterEngine>>,
    archive_pool: Arc<ManagedPool>,
}

struct PendingWatch {
    paths: HashMap<PathBuf, bool>,
    ready_at: Instant,
}

impl Daemon {
    /// Create a daemon, acquire its process lock, and persist its running
    /// state.
    ///
    /// # Errors
    ///
    /// Returns an error if another daemon owns the data directory, the node
    /// cannot be opened, or configuration cannot be parsed.
    pub async fn new(config: DaemonConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.data_dir)?;
        let state_file = StateFile::new(&config.data_dir);
        let pid_lock = PidLock::new(&config.data_dir);
        if !pid_lock.try_acquire()? {
            return Err(SyncwebError::operation(
                "daemon already running",
                config.data_dir.display(),
            ));
        }

        let app_config = match AppConfig::load(config.data_dir.join("config.toml")) {
            Ok(value) => value,
            Err(error) => {
                pid_lock.release()?;
                return Err(error);
            }
        };
        let schedule_manager = match ScheduleManager::from_config(&app_config.schedule) {
            Ok(value) if app_config.schedule != crate::schedule::ScheduleConfig::default() => Some(value),
            Ok(_) => None,
            Err(error) => {
                pid_lock.release()?;
                return Err(error);
            }
        };
        let filter_engine = match load_filter(&config.data_dir) {
            Ok(value) => value,
            Err(error) => {
                pid_lock.release()?;
                return Err(error);
            }
        };
        let initial_state = DaemonState::new(
            std::process::id(),
            String::new(),
            current_timestamp(),
            &config.data_dir,
            DaemonStatus::Starting,
        );
        if let Err(error) = state_file.save(&initial_state) {
            pid_lock.release()?;
            return Err(error);
        }

        let identity = match IdentityManager::new(config.data_dir.join("identity.key")) {
            Ok(value) => value,
            Err(error) => {
                let _cleanup_state = state_file.remove();
                let _cleanup_lock = pid_lock.release();
                return Err(error);
            }
        };
        let node = match IrohNode::new(identity, config.data_dir.join("data"), RelayMode::Default).await {
            Ok(value) => Arc::new(value),
            Err(error) => {
                let _cleanup_state = state_file.remove();
                let _cleanup_lock = pid_lock.release();
                return Err(error);
            }
        };
        let folder_manager = FolderManager::new(&node);
        let sync_engine = SyncEngine::new(
            folder_manager.clone(),
            node.blob_store().clone(),
            node.docs_engine().clone(),
            node.gossip_service().clone(),
        );
        let archive_pool = match ManagedPool::new("syncweb-archive", config.rayon_threads) {
            Ok(value) => Arc::new(value),
            Err(error) => {
                let _cleanup_node = node.stop().await;
                let _cleanup_state = state_file.remove();
                let _cleanup_lock = pid_lock.release();
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
            if let Err(error) = state_file.save(&running_state) {
                let _cleanup_node = node.stop().await;
                let _cleanup_state = state_file.remove();
                let _cleanup_lock = pid_lock.release();
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
        let ipc_server = IpcServer::with_archive_context(
            daemon_socket_path(&config.data_dir),
            handle.clone(),
            node.clone(),
            archive_pool.clone(),
        );
        let intent_supervisor = IntentSupervisor::new(config.max_retries, config.backoff_base, config.backoff_max);

        Ok(Self {
            config,
            state_file,
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
        })
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
        self.load_folders().await?;
        self.start_watching().await?;
        let server = self.ipc_server.clone();
        let mut server_task = tokio::spawn(async move { server.serve().await });
        let mut shutdown = self.handle.shutdown_sender.subscribe();
        let shutdown_sender = self.handle.shutdown_sender.clone();
        let mut signal_task = Box::pin(self.handle_signals(shutdown_sender.clone()));
        let interval_duration = self.config.sync_interval.max(Duration::from_millis(1));
        let mut interval = tokio::time::interval(interval_duration);
        let mut watch_interval = tokio::time::interval(Duration::from_millis(100));
        if let Err(error) = self.run_cycle().await {
            tracing::error!(%error, "initial daemon cycle failed");
        }
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
        if !server_task.is_finished() {
            match server_task.await {
                Ok(server_result) => server_result?,
                Err(error) => return Err(SyncwebError::operation("daemon IPC task failed", error)),
            }
        }
        result
    }

    async fn run_event_loop(
        &self,
        server_task: &mut JoinHandle<Result<()>>,
        signal_task: &mut std::pin::Pin<std::boxed::Box<impl std::future::Future<Output = Result<()>> + Send>>,
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
        self.load_folders().await?;
        self.start_watching().await?;
        let statuses = self.handle.folder_registry.read().await.statuses();
        for folder in statuses {
            let namespace = folder
                .namespace
                .parse::<NamespaceId>()
                .map_err(|error| SyncwebError::operation("invalid managed folder namespace", error))?;
            let folder_name = (!folder.path.as_os_str().is_empty()).then(|| folder.path.to_string_lossy().into_owned());
            let active = {
                let schedule_manager = self.schedule_manager.read().await;
                schedule_manager
                    .as_ref()
                    .is_none_or(|manager| manager.is_active(folder_name.as_deref()))
            };
            self.start_supervision(namespace).await?;
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
                self.start_supervision(parsed_namespace).await?;
                self.set_intent_active(parsed_namespace, true)?;
                self.save_status_report().await?;
                Ok(())
            }
            None => self.run_cycle().await,
        }
    }

    async fn start_supervision(&self, namespace: NamespaceId) -> Result<()> {
        if is_active(namespace) {
            return Ok(());
        }
        {
            let mut tasks = self
                .intent_tasks
                .lock()
                .map_err(|error| SyncwebError::operation("daemon intent task mutex is poisoned", error))?;
            tasks.retain(|_, task| !task.is_finished());
            if tasks.contains_key(&namespace) {
                return Ok(());
            }
        }
        let sync = self.sync_engine.clone();
        let supervisor = self.intent_supervisor;
        let shutdown = self.handle.shutdown_sender.subscribe();
        let controls = self.intent_controls.clone();
        let filter = self.filter_engine.read().await.clone();
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
        let params = bandwidth.map_or_else(SubscribeParams::default, |limits| {
            SubscribeParams::default().with_bandwidth_limits(limits)
        });
        let task = tokio::spawn(async move {
            match supervisor
                .supervise_with_controls(&sync, namespace, params, shutdown, controls, filter)
                .await
            {
                Ok(result) => {
                    if let Some(error) = result.last_error {
                        tracing::warn!(%namespace, retry_count = result.retry_count, %error, "supervised intent stopped");
                    }
                }
                Err(error) => tracing::error!(%namespace, %error, "supervised intent failed"),
            }
        });
        self.intent_tasks
            .lock()
            .map_err(|error| SyncwebError::operation("daemon intent task mutex is poisoned", error))?
            .insert(namespace, task);
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
        for (namespace, paths) in ready {
            let Some(root) = roots.get(&namespace) else {
                continue;
            };
            let namespace_id = namespace
                .parse::<NamespaceId>()
                .map_err(|error| SyncwebError::operation("invalid watched folder namespace", error))?;
            for (path, removed) in paths {
                self.process_watch_event(namespace_id, root, &path, removed).await?;
            }
        }
        self.save_status_report().await?;
        Ok(())
    }

    async fn process_watch_event(&self, namespace: NamespaceId, root: &Path, path: &Path, removed: bool) -> Result<()> {
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
                self.start_supervision(namespace).await?;
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
            let minute = current_minute();
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
        let bandwidth_stats = BandwidthStats::load(self.config.data_dir.join("stats.json"))?;
        let report = DaemonStatusReport {
            pid: state.pid,
            node_id: state.node_id,
            started_at: state.started_at,
            uptime_seconds: current_timestamp().saturating_sub(state.started_at),
            folders: statuses
                .into_iter()
                .map(|folder| FolderStatusReport {
                    namespace: folder.namespace,
                    path: folder.path,
                    session_active: folder.session_active,
                    last_sync_at: folder.last_sync_at,
                    entries_synced: folder.entries_synced,
                    errors: folder.errors,
                })
                .collect(),
            bandwidth: BandwidthSnapshot {
                upload_total: bandwidth_stats.total_upload,
                download_total: bandwidth_stats.total_download,
                upload_rate: 0,
                download_rate: 0,
            },
            schedule: schedule_report,
            rayon_threads: self.archive_pool.thread_count(),
        };
        self.state_file.save_status(&report)
    }

    async fn load_folders(&self) -> Result<()> {
        let folders = self.folder_manager.list().await?;
        let mut registry = self.handle.folder_registry.write().await;
        for folder in folders {
            let namespace = folder.namespace_id();
            if !registry
                .statuses()
                .iter()
                .any(|status| status.namespace == namespace.to_string())
            {
                registry.add(FolderEntry::new(namespace, PathBuf::new()))?;
            }
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
        let app_config = AppConfig::load(self.config.data_dir.join("config.toml"))?;
        let parsed_schedule = ScheduleManager::from_config(&app_config.schedule)?;
        let schedule_manager =
            (app_config.schedule != crate::schedule::ScheduleConfig::default()).then_some(parsed_schedule);
        let filter = load_filter(&self.config.data_dir)?;
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
        self.state_file.save(&stopping_state)?;
        self.save_status_report().await?;

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

        let tasks = {
            let mut task_map = self
                .intent_tasks
                .lock()
                .map_err(|error| SyncwebError::operation("daemon intent task mutex is poisoned", error))?;
            std::mem::take(&mut *task_map)
        };
        for (_, task) in tasks {
            task.await
                .map_err(|error| SyncwebError::operation("daemon intent task failed", error))?;
        }

        let node_result = self.node.stop().await;
        self.handle.set_status(DaemonStatus::Stopped).await;
        let remove_result = self.state_file.remove();
        let remove_status_result = self.state_file.remove_status();
        let release_result = self.pid_lock.release();
        node_result?;
        remove_result?;
        remove_status_result?;
        release_result?;
        Ok(())
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
        if self.state_file.exists()
            && let Err(error) = self.state_file.remove()
        {
            tracing::warn!(path = %self.state_file.path().display(), %error, "failed to remove daemon state");
        }
        if let Err(error) = self.state_file.remove_status() {
            tracing::warn!(path = %self.state_file.status_path().display(), %error, "failed to remove daemon status");
        }
    }
}

fn send_shutdown(sender: &broadcast::Sender<()>) {
    match sender.send(()) {
        Ok(_) | Err(broadcast::error::SendError(())) => {}
    }
}

fn load_filter(data_dir: &Path) -> Result<Option<FilterEngine>> {
    let path = data_dir.join("filters.toml");
    if path.exists() {
        FilterEngine::load(path).map(Some)
    } else {
        Ok(None)
    }
}

fn current_minute() -> u16 {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    u16::try_from((seconds % 86_400).div_euclid(60)).unwrap_or(0)
}

fn is_recoverable_watch_error(error: &SyncwebError) -> bool {
    let message = error.to_string();
    message.contains("file changed during import") || message.contains("input path does not exist")
}
