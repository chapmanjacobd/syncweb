use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, ensure};
use syncweb_core::{
    daemon::{
        Daemon, DaemonConfig, DaemonState, DaemonStatus, DaemonStatusReport, IpcClient, IpcCommand, IpcRequest,
        IpcResponse, PidLock, StateFile, daemon_socket_path,
    },
    folder::{FolderManager, SyncMode},
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
    schedule::BandwidthWindowConfig,
    storage::Config,
};
use tokio::task::JoinHandle;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(prefix: &str) -> Result<Self> {
        let path = std::env::temp_dir().join(format!("syncweb-{prefix}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("failed to remove test directory {}: {error}", self.0.display());
        }
    }
}

async fn create_folder(directory: &TestDirectory, root: &Path) -> Result<iroh_docs::NamespaceId> {
    fs::create_dir_all(root)?;
    let identity = IdentityManager::new(directory.path().join("identity.key"))?;
    let node = IrohNode::new(identity, directory.path().join("data"), RelayMode::Default).await?;
    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;
    let namespace = folder.namespace_id();
    node.stop().await?;
    Ok(namespace)
}

async fn start_daemon(directory: &TestDirectory) -> Result<(IpcClient, JoinHandle<syncweb_core::Result<()>>)> {
    let mut config = DaemonConfig::new(directory.path());
    config.sync_interval = Duration::from_mins(1);
    config.watch_debounce = Duration::from_millis(50);
    let daemon = Daemon::new(config).await?;
    let client = IpcClient::new(directory.path());
    let task = tokio::spawn(async move { daemon.run().await });
    wait_for_running(&client).await?;
    Ok((client, task))
}

async fn wait_for_running(client: &IpcClient) -> Result<()> {
    for _ in 0..200 {
        if matches!(
            client.send(IpcRequest::new(IpcCommand::Status)).await,
            Ok(IpcResponse::Status(DaemonStatus::Running))
        ) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    anyhow::bail!("daemon did not become available at {}", client.socket_path().display())
}

async fn stop_daemon(client: &IpcClient, task: JoinHandle<syncweb_core::Result<()>>) -> Result<()> {
    let response = client
        .send(IpcRequest::new(IpcCommand::Shutdown { force: false }))
        .await?;
    ensure!(matches!(response, IpcResponse::Ok { .. }));
    task.await.context("join daemon task")??;
    Ok(())
}

async fn wait_for_report<F>(state_file: &StateFile, predicate: F) -> Result<DaemonStatusReport>
where
    F: Fn(&DaemonStatusReport) -> bool,
{
    for _ in 0..200 {
        if let Some(report) = state_file.load_status()?
            && predicate(&report)
        {
            return Ok(report);
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    anyhow::bail!("daemon status report did not reach the expected state")
}

#[tokio::test]
async fn test_full_daemon_start_stop_lifecycle() -> Result<()> {
    let directory = TestDirectory::new("lifecycle")?;
    let (client, task) = start_daemon(&directory).await?;
    let state_file = StateFile::new(directory.path());
    let report = wait_for_report(&state_file, |report| report.rayon_threads > 0).await?;
    ensure!(report.pid == std::process::id());
    ensure!(!report.node_id.is_empty());

    stop_daemon(&client, task).await?;
    ensure!(state_file.load()?.is_none());
    ensure!(state_file.load_status()?.is_none());
    ensure!(!daemon_socket_path(directory.path()).exists());
    Ok(())
}

#[tokio::test]
async fn test_daemon_syncs_folder_end_to_end() -> Result<()> {
    let directory = TestDirectory::new("sync")?;
    let root = directory.path().join("folder");
    let namespace = create_folder(&directory, &root).await?;
    let (client, task) = start_daemon(&directory).await?;

    let add_response = client
        .send(IpcRequest::new(IpcCommand::AddFolder {
            namespace: namespace.to_string(),
            path: root.clone(),
        }))
        .await
        .context("add folder request")?;
    ensure!(matches!(add_response, IpcResponse::Ok { .. }));
    let sync_response = client
        .send(IpcRequest::new(IpcCommand::TriggerSync { namespace: None }))
        .await
        .context("trigger sync request")?;
    ensure!(matches!(sync_response, IpcResponse::Ok { .. }));
    ensure!(!task.is_finished(), "daemon stopped after trigger sync");
    let folders = client
        .send(IpcRequest::new(IpcCommand::ListFolders))
        .await
        .context("list folders request")?;
    ensure!(
        matches!(
            folders,
            IpcResponse::FolderList(ref values)
                if values.iter().any(|value| value.namespace == namespace.to_string() && value.path == root)
        ),
        "folder path was not attached through IPC: {folders:?}"
    );

    stop_daemon(&client, task).await.context("stop daemon")?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_watches_and_syncs_new_file() -> Result<()> {
    let directory = TestDirectory::new("watch")?;
    let root = directory.path().join("folder");
    let namespace = create_folder(&directory, &root).await?;
    let (client, task) = start_daemon(&directory).await?;

    client
        .send(IpcRequest::new(IpcCommand::AddFolder {
            namespace: namespace.to_string(),
            path: root.clone(),
        }))
        .await?;
    client
        .send(IpcRequest::new(IpcCommand::TriggerSync { namespace: None }))
        .await?;
    wait_for_report(&StateFile::new(directory.path()), |report| {
        report.folders.iter().any(|folder| folder.path == root)
    })
    .await?;

    fs::write(root.join("new-file.txt"), b"daemon integration")?;
    let report = wait_for_report(&StateFile::new(directory.path()), |report| {
        report
            .folders
            .iter()
            .any(|folder| folder.namespace == namespace.to_string() && folder.entries_synced > 0)
    })
    .await?;
    ensure!(
        report
            .folders
            .iter()
            .any(|folder| folder.namespace == namespace.to_string() && folder.errors.is_empty())
    );

    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_concurrent_ipc_commands() -> Result<()> {
    let directory = TestDirectory::new("ipc")?;
    let (client, task) = start_daemon(&directory).await?;
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let request_client = client.clone();
        tasks.push(tokio::spawn(async move {
            request_client.send(IpcRequest::new(IpcCommand::Status)).await
        }));
    }
    for request_task in tasks {
        ensure!(matches!(
            request_task.await.context("join IPC request")??,
            IpcResponse::Status(DaemonStatus::Running)
        ));
    }
    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_reload_and_sync_over_ipc() -> Result<()> {
    let directory = TestDirectory::new("reload")?;
    let (client, task) = start_daemon(&directory).await?;

    let mut config = Config::default();
    config.schedule.bandwidth = vec![BandwidthWindowConfig::new("00:00-24:00", "0", "1MB/s")];
    config.save(directory.path().join("config.toml"))?;

    let reload_response = client.send(IpcRequest::new(IpcCommand::ReloadConfig)).await?;
    ensure!(matches!(reload_response, IpcResponse::Ok { .. }));
    let sync_response = client
        .send(IpcRequest::new(IpcCommand::TriggerSync { namespace: None }))
        .await?;
    ensure!(matches!(sync_response, IpcResponse::Ok { .. }));
    let report = wait_for_report(&StateFile::new(directory.path()), |report| report.schedule.is_some()).await?;
    ensure!(report.bandwidth.download_rate == 0);

    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_two_daemons_cannot_start_simultaneously() -> Result<()> {
    let directory = TestDirectory::new("lock")?;
    let (client, task) = start_daemon(&directory).await?;
    let second = Daemon::new(DaemonConfig::new(directory.path())).await;
    let Err(error) = second else {
        anyhow::bail!("second daemon should not acquire the lock");
    };
    ensure!(error.to_string().contains("daemon already running"));
    stop_daemon(&client, task).await?;
    Ok(())
}

#[test]
fn test_stale_pid_with_reused_pid_is_recovered() -> Result<()> {
    let directory = TestDirectory::new("stale-pid")?;
    let state_file = StateFile::new(directory.path());
    state_file.save(&DaemonState::new(
        std::process::id(),
        "stale",
        1,
        directory.path(),
        DaemonStatus::Running,
    ))?;
    let lock = PidLock::new(directory.path());
    ensure!(lock.try_acquire()?);
    ensure!(syncweb_core::daemon::daemon_client(directory.path())?.is_none());
    lock.release()?;
    Ok(())
}
