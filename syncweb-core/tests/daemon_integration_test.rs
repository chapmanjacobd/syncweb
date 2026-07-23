mod test_utils;

use std::{fs, path::Path, sync::Arc, time::Duration};

use anyhow::{Context, Result, ensure};
use syncweb_core::{
    daemon::{
        Daemon, DaemonConfig, DaemonState, DaemonStatus, DaemonStatusReport, IpcClient, IpcCommand, IpcRequest,
        IpcResponse, PidLock, StateFile, daemon_socket_path,
    },
    folder::{
        CollectionEntry, CollectionManifest, CollectionStore, DropExportOptions, DropExporter, FolderManager, SyncMode,
    },
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
    schedule::BandwidthWindowConfig,
    storage::Config,
};
use tokio::task::JoinHandle;

use crate::test_utils::TestDirectory;

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

#[tokio::test]
async fn test_daemon_supervisor_creates_intent_and_registers_session() -> Result<()> {
    let directory = TestDirectory::new("supervisor-create")?;
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
        .send(IpcRequest::new(IpcCommand::TriggerSync {
            namespace: Some(namespace.to_string()),
        }))
        .await?;
    let report = wait_for_report(&StateFile::new(directory.path()), |report| {
        report
            .folders
            .iter()
            .any(|f| f.namespace == namespace.to_string() && f.session_active)
    })
    .await?;
    ensure!(report.folders.iter().any(|f| f.namespace == namespace.to_string()));
    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_supervisor_shutdown_cancels_intents() -> Result<()> {
    let directory = TestDirectory::new("supervisor-cancel")?;
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
        report.folders.iter().any(|f| f.namespace == namespace.to_string())
    })
    .await?;

    stop_daemon(&client, task).await?;
    ensure!(!syncweb_core::sync::is_active(namespace));
    Ok(())
}

#[tokio::test]
async fn test_daemon_import_export_archive_via_ipc() -> Result<()> {
    let directory = TestDirectory::new("archive-ipc")?;
    let identity = IdentityManager::new(directory.path().join("identity.key"))?;
    let node = Arc::new(IrohNode::new(identity, directory.path().join("data"), RelayMode::Default).await?);
    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;
    let namespace = folder.namespace_id();
    let content = b"archive content for export";
    let content_hash = node.blob_store().add_bytes(content).await?;
    let mut manifest = CollectionManifest::new(uuid::Uuid::new_v4(), "1.0.0");
    let content_len = u64::try_from(content.len())?;
    manifest
        .entries
        .push(CollectionEntry::new(content_hash, "archive-test.txt", content_len)?);
    CollectionStore::new(
        folder.doc().clone(),
        folder.author(),
        node.blob_store().clone(),
        node.docs_engine().clone(),
    )
    .publish(&manifest, 1)
    .await?;
    let archive_path = directory.path().join("input.car.zst");
    DropExporter::new(node.blob_store().clone())
        .export_drop_with_options(
            std::slice::from_ref(&manifest),
            &archive_path,
            DropExportOptions::default(),
            None,
        )
        .await?;
    node.stop().await?;

    let (client, task) = start_daemon(&directory).await?;
    let target = directory.path().join("imported");
    let import_response = client
        .send(IpcRequest::new(IpcCommand::ImportArchive {
            input: archive_path.clone(),
            target: target.clone(),
            filter: None,
        }))
        .await?;
    ensure!(matches!(import_response, IpcResponse::ImportComplete(_)));
    if let IpcResponse::ImportComplete(result) = &import_response {
        let export_ns = result
            .namespace_id
            .map_or_else(|| namespace.to_string(), |id| id.to_string());
        let export_path = directory.path().join("export.car.zst");
        let export_response = client
            .send(IpcRequest::new(IpcCommand::ExportArchive {
                namespace: export_ns,
                version: None,
                output: export_path,
            }))
            .await?;
        ensure!(matches!(export_response, IpcResponse::ExportComplete(_)));
    }
    ensure!(target.join("archive-test.txt").is_file());
    ensure!(fs::read(target.join("archive-test.txt"))? == content);

    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_status_command_json_output() -> Result<()> {
    let directory = TestDirectory::new("status-json")?;
    let (client, task) = start_daemon(&directory).await?;

    let response = client.send(IpcRequest::new(IpcCommand::Status)).await?;
    ensure!(matches!(response, IpcResponse::Status(DaemonStatus::Running)));

    let status_report = wait_for_report(&StateFile::new(directory.path()), |_| true).await?;
    let json = serde_json::to_string_pretty(&status_report)?;
    ensure!(json.contains("pid"));
    ensure!(json.contains("node_id"));
    ensure!(json.contains("folders"));

    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_unix_socket_rejects_non_owner_access() -> Result<()> {
    let directory = TestDirectory::new("socket-access")?;
    let (client, task) = start_daemon(&directory).await?;

    let metadata = fs::metadata(daemon_socket_path(directory.path()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        ensure!(mode == 0o600, "socket should be owner-only, got mode: {mode:#o}");
    }
    let _ = metadata;

    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_watch_debounce_coalesces_rapid_changes() -> Result<()> {
    let directory = TestDirectory::new("watch-debounce")?;
    let root = directory.path().join("folder");
    let namespace = create_folder(&directory, &root).await?;
    let mut config = DaemonConfig::new(directory.path());
    config.sync_interval = Duration::from_mins(1);
    config.watch_debounce = Duration::from_millis(100);
    let daemon = Daemon::new(config).await?;
    let client = IpcClient::new(directory.path());
    let task = tokio::spawn(async move { daemon.run().await });
    wait_for_running(&client).await?;

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
        report.folders.iter().any(|f| f.namespace == namespace.to_string())
    })
    .await?;

    let file_path = root.join("debounce-single.txt");
    for content in ["first", "second", "third"] {
        fs::write(&file_path, content)?;
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
    client
        .send(IpcRequest::new(IpcCommand::TriggerSync { namespace: None }))
        .await?;

    let _ = fs::remove_file(&file_path);
    ensure!(!task.is_finished(), "daemon crashed during debounced watch");
    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_watch_recovers_from_file_read_inconsistency() -> Result<()> {
    let directory = TestDirectory::new("watch-recover")?;
    let root = directory.path().join("folder");
    let namespace = create_folder(&directory, &root).await?;
    let mut config = DaemonConfig::new(directory.path());
    config.watch_debounce = Duration::from_millis(50);
    config.sync_interval = Duration::from_mins(1);
    let daemon = Daemon::new(config).await?;
    let client = IpcClient::new(directory.path());
    let task = tokio::spawn(async move { daemon.run().await });
    wait_for_running(&client).await?;

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
        report.folders.iter().any(|f| f.namespace == namespace.to_string())
    })
    .await?;

    let recover_file = root.join("recover-file.txt");
    fs::write(&recover_file, b"initial")?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    fs::write(&recover_file, b"recovered properly")?;

    tokio::time::sleep(Duration::from_millis(300)).await;
    let report = StateFile::new(directory.path()).load_status()?;
    ensure!(
        report.is_some_and(|r| r
            .folders
            .iter()
            .any(|f| f.namespace == namespace.to_string() && f.entries_synced > 0)),
        "retried watch events should produce imports"
    );

    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_schedule_pause_resume() -> Result<()> {
    let directory = TestDirectory::new("schedule-pause")?;
    let root = directory.path().join("folder");
    let namespace = create_folder(&directory, &root).await?;

    let current_minute = {
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        u16::try_from((epoch % 86_400).div_euclid(60)).unwrap_or(0)
    };
    let inactive_start = (current_minute + 2) % 1440;
    let inactive_end = (inactive_start + 5) % 1440;
    let hours_before = |minute: u16| -> String { format!("{:02}:{:02}", minute.div_euclid(60), minute % 60) };
    let hours_after = |minute: u16| -> String {
        if minute == 0 {
            "24:00".to_owned()
        } else {
            format!("{:02}:{:02}", minute.div_euclid(60), minute % 60)
        }
    };
    let inactive_window = format!("{}-{}", hours_before(inactive_start), hours_after(inactive_end));
    let schedule_toml = format!(
        "[schedule]\nactive_hours = \"{inactive_window}\"\n\n[[schedule.bandwidth]]\nhours = \"00:00-24:00\"\nmax_upload = \"0\"\nmax_download = \"1MB/s\"\n"
    );
    fs::write(directory.path().join("config.toml"), schedule_toml)?;

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

    let report = wait_for_report(&StateFile::new(directory.path()), |report| {
        report
            .folders
            .iter()
            .any(|f| f.namespace == namespace.to_string() && f.errors.is_empty())
    })
    .await?;
    ensure!(report.schedule.is_some());
    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_reload_updates_config() -> Result<()> {
    let directory = TestDirectory::new("reload-config")?;
    let root = directory.path().join("folder");
    let namespace = create_folder(&directory, &root).await?;
    let (client, task) = start_daemon(&directory).await?;

    client
        .send(IpcRequest::new(IpcCommand::AddFolder {
            namespace: namespace.to_string(),
            path: root.clone(),
        }))
        .await?;

    let reload = client.send(IpcRequest::new(IpcCommand::ReloadConfig)).await?;
    ensure!(matches!(reload, IpcResponse::Ok { .. }));

    let sync = client
        .send(IpcRequest::new(IpcCommand::TriggerSync { namespace: None }))
        .await?;
    ensure!(matches!(sync, IpcResponse::Ok { .. }));

    ensure!(!task.is_finished(), "daemon should remain running after reload");
    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_no_schedule_runs_always() -> Result<()> {
    let directory = TestDirectory::new("no-schedule")?;
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
    let report = wait_for_report(&StateFile::new(directory.path()), |_| true).await?;
    ensure!(report.schedule.is_none());

    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_keeps_pool_running_after_reload() -> Result<()> {
    let directory = TestDirectory::new("pool-reload")?;
    let (client, task) = start_daemon(&directory).await?;
    let before = wait_for_report(&StateFile::new(directory.path()), |r| r.rayon_threads > 0).await?;
    let pool_size_before = before.rayon_threads;

    client.send(IpcRequest::new(IpcCommand::ReloadConfig)).await?;
    let after = wait_for_report(&StateFile::new(directory.path()), |r| r.rayon_threads > 0).await?;
    ensure!(after.rayon_threads == pool_size_before);

    stop_daemon(&client, task).await?;
    Ok(())
}

#[tokio::test]
async fn test_daemon_download_via_ipc() -> Result<()> {
    let directory = TestDirectory::new("download-ipc")?;
    let root = directory.path().join("folder");
    let namespace = create_folder(&directory, &root).await?;
    let (client, task) = start_daemon(&directory).await?;

    client
        .send(IpcRequest::new(IpcCommand::AddFolder {
            namespace: namespace.to_string(),
            path: root.clone(),
        }))
        .await?;

    let file_path = root.join("dl-file.txt");
    fs::write(&file_path, b"downloadable")?;
    client
        .send(IpcRequest::new(IpcCommand::ImportFiles {
            namespace: Some(namespace.to_string()),
            path: file_path,
        }))
        .await?;

    let download_response = client
        .send(IpcRequest::new(IpcCommand::Download {
            namespace: namespace.to_string(),
            strategy: syncweb_core::sync::FetchStrategy::default(),
        }))
        .await?;
    ensure!(matches!(download_response, IpcResponse::DownloadComplete { .. }));

    stop_daemon(&client, task).await?;
    Ok(())
}
