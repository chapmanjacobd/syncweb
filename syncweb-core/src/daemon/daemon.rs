use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use iroh_docs::NamespaceId;
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};

use crate::{
    error::{Result, SyncwebError},
    folder::FolderManager,
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
    schedule::ScheduleManager,
    storage::Config as AppConfig,
    sync::{SubscribeParams, SyncEngine, cancel_session, is_active},
};

use super::{
    DaemonHandle, DaemonState, DaemonStatus, FolderEntry, IpcServer, ManagedPool, PidLock, StateFile,
    current_timestamp, daemon_socket_path, supervisor::IntentSupervisor,
};

/// Configuration used to construct and run a daemon.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DaemonConfig {
    pub data_dir: PathBuf,
    pub foreground: bool,
    pub sync_interval: Duration,
    pub observation_ttl: Duration,
    pub max_retries: u32,
    pub backoff_base: Duration,
    pub backoff_max: Duration,
    pub rayon_threads: usize,
    pub log_level: String,
    pub log_file: Option<PathBuf>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("."),
            foreground: true,
            sync_interval: Duration::from_mins(1),
            observation_ttl: Duration::from_hours(1),
            max_retries: 3,
            backoff_base: Duration::from_secs(1),
            backoff_max: Duration::from_mins(1),
            rayon_threads: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
            log_level: "info".to_owned(),
            log_file: None,
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
    archive_pool: Arc<ManagedPool>,
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
            Ok(value) => Some(value),
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
        let server = self.ipc_server.clone();
        let mut server_task = tokio::spawn(async move { server.serve().await });
        let mut shutdown = self.handle.shutdown_sender.subscribe();
        let shutdown_sender = self.handle.shutdown_sender.clone();
        let mut signal_task = Box::pin(self.handle_signals(shutdown_sender.clone()));
        let interval_duration = self.config.sync_interval.max(Duration::from_millis(1));
        let mut interval = tokio::time::interval(interval_duration);
        let result = loop {
            tokio::select! {
                signal_result = &mut signal_task => {
                    signal_result?;
                    send_shutdown(&shutdown_sender);
                    break Ok(());
                }
                shutdown_result = shutdown.recv() => {
                    match shutdown_result {
                        Ok(()) | Err(broadcast::error::RecvError::Closed) => break Ok(()),
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                    }
                }
                server_result = &mut server_task => {
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
                _ = interval.tick() => self.run_cycle().await?,
            }
        };

        send_shutdown(&shutdown_sender);
        if !server_task.is_finished() {
            match server_task.await {
                Ok(server_result) => server_result?,
                Err(error) => return Err(SyncwebError::operation("daemon IPC task failed", error)),
            }
        }
        result
    }

    async fn run_cycle(&self) -> Result<()> {
        self.reload_if_requested().await?;
        self.load_folders().await?;
        let statuses = self.handle.folder_registry.read().await.statuses();
        for folder in statuses {
            let namespace = folder
                .namespace
                .parse::<NamespaceId>()
                .map_err(|error| SyncwebError::operation("invalid managed folder namespace", error))?;
            let active = {
                let schedule_manager = self.schedule_manager.read().await;
                schedule_manager
                    .as_ref()
                    .is_none_or(|manager| manager.is_active(folder.path.to_str()))
            };
            if active {
                self.start_supervision(namespace)?;
            } else {
                if is_active(namespace) && !cancel_session(namespace) {
                    tracing::warn!(%namespace, "scheduled folder session could not be cancelled");
                }
            }
        }
        Ok(())
    }

    async fn run_trigger(&self, namespace: Option<String>) -> Result<()> {
        match namespace {
            Some(value) => {
                let parsed_namespace = value
                    .parse::<NamespaceId>()
                    .map_err(|error| SyncwebError::operation("invalid sync namespace", error))?;
                self.start_supervision(parsed_namespace)
            }
            None => self.run_cycle().await,
        }
    }

    fn start_supervision(&self, namespace: NamespaceId) -> Result<()> {
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
            let sync = self.sync_engine.clone();
            let supervisor = self.intent_supervisor;
            let shutdown = self.handle.shutdown_sender.subscribe();
            let task = tokio::spawn(async move {
                match supervisor
                    .supervise(&sync, namespace, SubscribeParams::default(), shutdown)
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
            tasks.insert(namespace, task);
        }
        Ok(())
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
        let schedule = ScheduleManager::from_config(&app_config.schedule)?;
        *self.schedule_manager.write().await = Some(schedule);
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
        let release_result = self.pid_lock.release();
        node_result?;
        remove_result?;
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
    }
}

fn send_shutdown(sender: &broadcast::Sender<()>) {
    match sender.send(()) {
        Ok(_) | Err(broadcast::error::SendError(())) => {}
    }
}
