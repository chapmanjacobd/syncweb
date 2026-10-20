#![cfg(unix)]

use std::{fs, path::Path, sync::Arc, time::Duration};

use anyhow::{Context, Result, ensure};
use syncweb_core::{
    daemon::{
        Daemon, DaemonConfig, DaemonState, DaemonStatus, DaemonStatusReport, EntryRow, IpcClient, IpcCommand,
        IpcRequest, IpcResponse, PidLock, StateFile, daemon_socket_path,
    },
    folder::{
        CollectionEntry, CollectionManifest, CollectionStore, DropExportOptions, DropExporter, FolderManager, SyncMode,
    },
    node::{
        identity::IdentityManager,
        iroh_node::{DiscoveryConfig, IrohNode, RelayMode},
    },
    storage::{config::SubscribeFilters, node_db::NodeDatabase},
};
use tokio::task::JoinHandle;

use crate::test_utils::TestDirectory;

async fn create_folder(directory: &TestDirectory, root: &Path) -> Result<iroh_docs::NamespaceId> {
    fs::create_dir_all(root)?;
    let identity = IdentityManager::new(directory.path().join("identity.key"))?;
    let node = IrohNode::new(
        identity,
        directory.path().join("data"),
        RelayMode::Default,
        crate::test_utils::empty_member_keys(),
        DiscoveryConfig::disabled(),
    )
    .await?;
    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;
    let namespace = folder.namespace_id();
    node.stop().await?;
    Ok(namespace)
}

async fn start_daemon(directory: &TestDirectory) -> Result<(IpcClient, JoinHandle<syncweb_core::Result<()>>)> {
    let mut config = DaemonConfig::new(directory.path());
    config.sync_interval = Duration::from_mins(1);
    config.watch_debounce = Duration::from_millis(50);
    config.relay_mode = RelayMode::None;
    let daemon = Daemon::new(config).await?;
    let client = IpcClient::new(directory.path());
    let task = tokio::spawn(async move { daemon.run().await });
    wait_for_running(&client).await?;
    Ok((client, task))
}

async fn wait_for_running(client: &IpcClient) -> Result<()> {
    for _ in 0..300 {
        if matches!(
            client.send(IpcRequest::new(IpcCommand::Status)).await,
            Ok(IpcResponse::Status(DaemonStatus::Running))
        ) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
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
    for _ in 0..300 {
        if let Some(report) = state_file.load_status()?
            && predicate(&report)
        {
            return Ok(report);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    anyhow::bail!("daemon status report did not reach the expected state")
}

async fn wait_for_local_entry(
    client: &IpcClient,
    namespace: &str,
    path: &str,
    excluded_hashes: &[iroh_blobs::Hash],
) -> Result<EntryRow> {
    for _ in 0..120 {
        let response = client
            .send(IpcRequest::new(IpcCommand::ListEntries {
                folder: namespace.to_owned(),
                enrich: false,
            }))
            .await?;
        if let IpcResponse::Entries(entries) = response
            && let Some(entry) = entries.into_iter().find(|entry| {
                entry.path == path && entry.local && excluded_hashes.iter().all(|hash| entry.hash != *hash)
            })
        {
            return Ok(entry);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    anyhow::bail!("daemon did not observe local entry {path} in {namespace}")
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

    {
        let node_db = NodeDatabase::open(directory.path().join("node.db"))?;
        let mut config = node_db.load_app_config()?;
        config.schedule.active_hours = "00:00-24:00".to_owned();
        node_db.save_app_config(&config)?;
    }

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
        .send(IpcRequest::new(IpcCommand::SetSubscribe {
            namespace: namespace.to_string(),
            enabled: true,
            filters: None,
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
    let node = Arc::new(
        IrohNode::new(
            identity,
            directory.path().join("data"),
            RelayMode::Default,
            crate::test_utils::empty_member_keys(),
            DiscoveryConfig::disabled(),
        )
        .await?,
    );
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
            seed: true,
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
                filter: None,
                mode: None,
                block_hashes: Vec::new(),
                max_bytes: None,
                source_community: None,
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
    config.relay_mode = RelayMode::None;
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
    config.relay_mode = RelayMode::None;
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
    {
        let node_db = NodeDatabase::open(directory.path().join("node.db"))?;
        let mut config = node_db.load_app_config()?;
        config.schedule.active_hours = inactive_window.clone();
        node_db.save_app_config(&config)?;
    }

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

// === Two-daemon subscription filter tests ===
//
// These tests spin up two daemons connected through a single in-process relay
// server and verify that `join --subscribe` filter flags (--ignore-self,
// --sync-prefix, --glob, --max-count, --max-size) flow through IPC, persist in
// the node database, and activate a supervised live-sync intent.

struct TwoDaemons {
    client_a: IpcClient,
    task_a: JoinHandle<syncweb_core::Result<()>>,
    client_b: IpcClient,
    task_b: JoinHandle<syncweb_core::Result<()>>,
    _relay: Box<dyn std::any::Any + Send + Sync>,
}

async fn start_two_daemons(dir_a: &TestDirectory, dir_b: &TestDirectory) -> Result<TwoDaemons> {
    let (relay_map, relay_url, server) = iroh::test_utils::run_relay_server_with(false).await?;
    let memory_lookup = iroh::address_lookup::memory::MemoryLookup::new();
    let relay_mode = RelayMode::Custom {
        map: relay_map.clone(),
        insecure: true,
    };

    let mut config_a = DaemonConfig::new(dir_a.path());
    config_a.sync_interval = Duration::from_mins(1);
    config_a.watch_debounce = Duration::from_millis(50);
    config_a.relay_mode = relay_mode.clone();
    config_a.discovery = DiscoveryConfig::disabled();
    config_a.address_lookup = Some(memory_lookup.clone());
    let daemon_a = Daemon::new(config_a).await?;
    let endpoint_a = daemon_a.endpoint();
    let client_a = IpcClient::new(dir_a.path());
    let task_a = tokio::spawn(async move { daemon_a.run().await });
    wait_for_running(&client_a).await?;
    endpoint_a.online().await;

    let mut config_b = DaemonConfig::new(dir_b.path());
    config_b.sync_interval = Duration::from_mins(1);
    config_b.watch_debounce = Duration::from_millis(50);
    config_b.relay_mode = relay_mode.clone();
    config_b.discovery = DiscoveryConfig::disabled();
    config_b.address_lookup = Some(memory_lookup.clone());
    let daemon_b = Daemon::new(config_b).await?;
    let endpoint_b = daemon_b.endpoint();
    let client_b = IpcClient::new(dir_b.path());
    let task_b = tokio::spawn(async move { daemon_b.run().await });
    wait_for_running(&client_b).await?;
    endpoint_b.online().await;

    let node_id_a = daemon_node_id(dir_a).await?;
    let node_id_b = daemon_node_id(dir_b).await?;
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node_id_a).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(node_id_b).with_relay_url(relay_url.clone()));

    Ok(TwoDaemons {
        client_a,
        task_a,
        client_b,
        task_b,
        _relay: Box::new((relay_map, relay_url, server, memory_lookup)),
    })
}

async fn daemon_node_id(dir: &TestDirectory) -> Result<iroh::PublicKey> {
    let report = wait_for_report(&StateFile::new(dir.path()), |report| !report.node_id.is_empty()).await?;
    report.node_id.parse().context("parse daemon node id")
}

impl TwoDaemons {
    async fn stop(self) -> Result<()> {
        let Self {
            client_a,
            task_a,
            client_b,
            task_b,
            _relay,
        } = self;
        let response_a = client_a
            .send(IpcRequest::new(IpcCommand::Shutdown { force: false }))
            .await?;
        ensure!(matches!(response_a, IpcResponse::Ok { .. }));
        task_a.await.context("join daemon A task")??;
        let response_b = client_b
            .send(IpcRequest::new(IpcCommand::Shutdown { force: false }))
            .await?;
        ensure!(matches!(response_b, IpcResponse::Ok { .. }));
        task_b.await.context("join daemon B task")??;
        Ok(())
    }
}

fn parse_namespace(message: &str) -> Result<String> {
    message
        .lines()
        .find_map(|line| line.strip_prefix("namespace: "))
        .map(|value| value.trim().to_owned())
        .context("no namespace in response")
}

/// Share a folder on daemon A and return a writable `syncweb://folder/...` URL
/// for joining on B.
async fn share_ticket(client: &IpcClient, namespace: &str) -> Result<String> {
    let share = client
        .send(IpcRequest::new(IpcCommand::Share {
            namespace: namespace.to_owned(),
            blob: None,
            writable: true,
            pin: false,
            persist: false,
            addr_info: iroh_docs::api::protocol::AddrInfoOptions::RelayAndAddresses,
        }))
        .await?;
    let IpcResponse::Ok { message } = &share else {
        anyhow::bail!("unexpected share response: {share:?}");
    };
    ensure!(
        message.starts_with("syncweb://folder/"),
        "share should emit a URL: {message}"
    );
    Ok(message.clone())
}

/// Create a folder on daemon A and join it on daemon B with live syncing and
/// the given subscription filters, then assert the filters persist and the
/// supervised intent becomes active.
async fn assert_join_subscribe_filters(
    dir_a: &TestDirectory,
    dir_b: &TestDirectory,
    filters: SubscribeFilters,
) -> Result<()> {
    Box::pin(async move {
        let daemons = Box::pin(start_two_daemons(dir_a, dir_b)).await?;

        let folder_path = dir_a.path().join("shared");
        let create = daemons
            .client_a
            .send(IpcRequest::new(IpcCommand::CreateFolder {
                path: folder_path.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await?;
        let IpcResponse::Ok { message } = &create else {
            anyhow::bail!("unexpected create-folder response: {create:?}");
        };
        let namespace = parse_namespace(message)?;
        let ticket = share_ticket(&daemons.client_a, &namespace).await?;

        let join_path = dir_b.path().join("joined");
        let join = daemons
            .client_b
            .send(IpcRequest::new(IpcCommand::Join {
                ticket,
                path: join_path,
                mode: SyncMode::SendReceive,
                subscribe: true,
                filters: filters.clone(),
                download: false,
                indexing: false,
                metadata_only: false,
            }))
            .await?;
        ensure!(
            matches!(join, IpcResponse::Ok { .. }),
            "join with filters failed: {join:?}"
        );

        let node_db = NodeDatabase::open(dir_b.path().join("node.db"))?;
        let config = node_db.load_app_config()?;
        let entry = config
            .subscribe
            .folders
            .get(&namespace)
            .with_context(|| format!("subscription entry for {namespace} missing"))?;
        ensure!(entry.enabled, "subscription should be enabled");
        ensure!(
            entry.filters == filters,
            "filters not persisted: got {:?}, want {:?}",
            entry.filters,
            filters
        );

        let sync = daemons
            .client_b
            .send(IpcRequest::new(IpcCommand::TriggerSync { namespace: None }))
            .await?;
        ensure!(matches!(sync, IpcResponse::Ok { .. }), "trigger sync failed: {sync:?}");

        let report = wait_for_report(&StateFile::new(dir_b.path()), |report| {
            report
                .folders
                .iter()
                .any(|f| f.namespace == namespace && f.session_active)
        })
        .await?;
        ensure!(
            report
                .folders
                .iter()
                .any(|f| f.namespace == namespace && f.session_active),
            "subscription intent should be active"
        );

        daemons.stop().await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_two_daemons_sync_folder_via_relay() -> Result<()> {
    Box::pin(async move {
        let dir_a = TestDirectory::new("two-relay-a")?;
        let dir_b = TestDirectory::new("two-relay-b")?;
        let daemons = Box::pin(start_two_daemons(&dir_a, &dir_b)).await?;

        let folder_path = dir_a.path().join("shared");
        let create = daemons
            .client_a
            .send(IpcRequest::new(IpcCommand::CreateFolder {
                path: folder_path.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await?;
        let IpcResponse::Ok { message } = &create else {
            anyhow::bail!("unexpected create-folder response: {create:?}");
        };
        let namespace = parse_namespace(message)?;
        let ticket = share_ticket(&daemons.client_a, &namespace).await?;

        let file_path = folder_path.join("hello.txt");
        fs::write(&file_path, b"hello from A")?;
        let import = daemons
            .client_a
            .send(IpcRequest::new(IpcCommand::ImportFiles {
                namespace: Some(namespace.clone()),
                path: file_path,
            }))
            .await?;
        ensure!(
            matches!(import, IpcResponse::ImportFilesComplete { entries } if entries >= 1),
            "import failed: {import:?}"
        );

        let join_path = dir_b.path().join("joined");
        let join = daemons
            .client_b
            .send(IpcRequest::new(IpcCommand::Join {
                ticket,
                path: join_path,
                mode: SyncMode::SendReceive,
                subscribe: true,
                filters: SubscribeFilters::default(),
                download: false,
                indexing: false,
                metadata_only: false,
            }))
            .await?;
        ensure!(matches!(join, IpcResponse::Ok { .. }), "join failed: {join:?}");

        let sync = daemons
            .client_b
            .send(IpcRequest::new(IpcCommand::TriggerSync { namespace: None }))
            .await?;
        ensure!(matches!(sync, IpcResponse::Ok { .. }), "trigger sync failed: {sync:?}");

        let mut synced = false;
        for _ in 0..40 {
            let health = daemons
                .client_b
                .send(IpcRequest::new(IpcCommand::StatsFiles {
                    folder: std::path::PathBuf::from(&namespace),
                }))
                .await?;
            if let IpcResponse::FileStats(report) = &health
                && report.total_files >= 1
            {
                synced = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        ensure!(synced, "B should sync A's content via relay");

        daemons.stop().await?;
        Ok(())
    })
    .await
}

#[expect(clippy::too_many_lines, reason = "covers the complete daemon conflict lifecycle")]
#[tokio::test]
async fn test_daemon_materializes_conflict_without_reimport_loop() -> Result<()> {
    Box::pin(async move {
        let dir_a = TestDirectory::new("conflict-daemon-a")?;
        let dir_b = TestDirectory::new("conflict-daemon-b")?;
        let daemons = Box::pin(start_two_daemons(&dir_a, &dir_b)).await?;

        let folder_path = dir_a.path().join("shared");
        let create = daemons
            .client_a
            .send(IpcRequest::new(IpcCommand::CreateFolder {
                path: folder_path.clone(),
                mode: "sendreceive".to_owned(),
                indexing: false,
            }))
            .await?;
        let IpcResponse::Ok { message } = &create else {
            anyhow::bail!("unexpected create-folder response: {create:?}");
        };
        let namespace = parse_namespace(message)?;
        let ticket = share_ticket(&daemons.client_a, &namespace).await?;
        let joined_path = dir_b.path().join("joined");
        let join = daemons
            .client_b
            .send(IpcRequest::new(IpcCommand::Join {
                ticket,
                path: joined_path.clone(),
                mode: SyncMode::SendReceive,
                subscribe: true,
                filters: SubscribeFilters::default(),
                download: true,
                indexing: false,
                metadata_only: false,
            }))
            .await?;
        ensure!(matches!(join, IpcResponse::Ok { .. }), "join failed: {join:?}");
        let folder_response = daemons.client_b.send(IpcRequest::new(IpcCommand::ListFolders)).await?;
        let IpcResponse::FolderList(folders) = folder_response else {
            anyhow::bail!("unexpected folder list response: {folder_response:?}");
        };
        let joined_status = folders
            .iter()
            .find(|folder| folder.namespace == namespace)
            .context("joined folder missing from daemon registry")?;
        ensure!(
            joined_status.mode == "sendreceive",
            "joined folder is not writable: {joined_status:?}"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;

        let subscribe = daemons
            .client_a
            .send(IpcRequest::new(IpcCommand::SetSubscribe {
                namespace: namespace.clone(),
                enabled: true,
                filters: None,
            }))
            .await?;
        ensure!(
            matches!(subscribe, IpcResponse::Ok { .. }),
            "failed to start daemon A subscription: {subscribe:?}"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;

        let note_path_b = joined_path.join("note.bin");
        fs::write(&note_path_b, b"from B\0")?;
        let import = daemons
            .client_b
            .send(IpcRequest::new(IpcCommand::ImportFiles {
                namespace: Some(namespace.clone()),
                path: note_path_b.clone(),
            }))
            .await?;
        ensure!(
            matches!(import, IpcResponse::ImportFilesComplete { entries } if entries >= 1),
            "B import failed: {import:?}"
        );
        let b_entry = wait_for_local_entry(&daemons.client_b, &namespace, "note.bin", &[]).await?;

        let note_path_a = folder_path.join("note.bin");
        fs::write(&note_path_a, b"from A\0")?;
        let import_a = daemons
            .client_a
            .send(IpcRequest::new(IpcCommand::ImportFiles {
                namespace: Some(namespace.clone()),
                path: note_path_a.clone(),
            }))
            .await?;
        ensure!(
            matches!(import_a, IpcResponse::ImportFilesComplete { entries } if entries >= 1),
            "A import failed: {import_a:?}"
        );
        let a_entry = wait_for_local_entry(&daemons.client_a, &namespace, "note.bin", &[]).await?;
        ensure!(
            a_entry.hash != b_entry.hash,
            "A import did not record a distinct local edit"
        );
        let sync = daemons
            .client_b
            .send(IpcRequest::new(IpcCommand::TriggerSync {
                namespace: Some(namespace.clone()),
            }))
            .await?;
        ensure!(matches!(sync, IpcResponse::Ok { .. }), "conflict sync failed: {sync:?}");

        let selector = joined_path.to_string_lossy().into_owned();
        let mut conflict_before = None;
        for _ in 0..120 {
            let response = daemons
                .client_b
                .send(IpcRequest::new(IpcCommand::ListConflicts {
                    folder: selector.clone(),
                }))
                .await?;
            if let IpcResponse::Conflicts(conflicts) = response
                && conflicts.iter().any(|report| {
                    report.path == "note.bin"
                        && report.variants.len() == 2
                        && report.variants.iter().all(|variant| variant.local)
                })
            {
                conflict_before = Some(conflicts);
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        let before = conflict_before.context("daemon did not observe the remote conflict")?;
        let before_note = before
            .iter()
            .find(|report| report.path == "note.bin")
            .context("conflict report did not include note.bin")?;

        let has_conflict_copy = || {
            fs::read_dir(&joined_path).is_ok_and(|entries| {
                entries
                    .flatten()
                    .any(|entry| entry.file_name().to_string_lossy().contains(".conflict."))
            })
        };
        for _ in 0..120 {
            if has_conflict_copy() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        ensure!(
            has_conflict_copy(),
            "daemon did not materialize a conflict copy; entries={:?}",
            fs::read_dir(&joined_path)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(std::result::Result::ok)
                .map(|entry| entry.file_name())
                .collect::<Vec<_>>()
        );

        tokio::time::sleep(Duration::from_millis(1_500)).await;
        let after_response = daemons
            .client_b
            .send(IpcRequest::new(IpcCommand::ListConflicts {
                folder: selector.clone(),
            }))
            .await?;
        let IpcResponse::Conflicts(after) = after_response else {
            anyhow::bail!("unexpected conflict response");
        };
        let after_note = after
            .iter()
            .find(|report| report.path == "note.bin")
            .context("conflict report disappeared after materialization")?;
        ensure!(
            before_note == after_note,
            "materializing the conflict changed the document variants: before={before_note:?}, after={after_note:?}"
        );

        let resolved = daemons
            .client_b
            .send(IpcRequest::new(IpcCommand::ResolveConflicts {
                folder: selector,
                path: Some("note.bin".to_owned()),
            }))
            .await?;
        ensure!(
            matches!(resolved, IpcResponse::ConflictsResolved { removed } if removed >= 1),
            "conflict resolution failed: {resolved:?}"
        );
        ensure!(!has_conflict_copy(), "conflict artifact was not removed");

        daemons.stop().await?;
        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_join_subscribe_ignore_self_persists_and_activates() -> Result<()> {
    let dir_a = TestDirectory::new("filter-ignore-self-a")?;
    let dir_b = TestDirectory::new("filter-ignore-self-b")?;
    assert_join_subscribe_filters(
        &dir_a,
        &dir_b,
        SubscribeFilters::new(false, true, None, None, None, None),
    )
    .await
}

#[tokio::test]
async fn test_join_subscribe_sync_prefix_persists_and_activates() -> Result<()> {
    let dir_a = TestDirectory::new("filter-prefix-a")?;
    let dir_b = TestDirectory::new("filter-prefix-b")?;
    assert_join_subscribe_filters(
        &dir_a,
        &dir_b,
        SubscribeFilters::new(false, false, Some(std::path::PathBuf::from("docs")), None, None, None),
    )
    .await
}

#[tokio::test]
async fn test_join_subscribe_glob_persists_and_activates() -> Result<()> {
    let dir_a = TestDirectory::new("filter-glob-a")?;
    let dir_b = TestDirectory::new("filter-glob-b")?;
    assert_join_subscribe_filters(
        &dir_a,
        &dir_b,
        SubscribeFilters::new(false, false, None, Some("*.txt".to_owned()), None, None),
    )
    .await
}

#[tokio::test]
async fn test_join_subscribe_max_count_persists_and_activates() -> Result<()> {
    let dir_a = TestDirectory::new("filter-count-a")?;
    let dir_b = TestDirectory::new("filter-count-b")?;
    assert_join_subscribe_filters(
        &dir_a,
        &dir_b,
        SubscribeFilters::new(false, false, None, None, Some(3), None),
    )
    .await
}

#[tokio::test]
async fn test_join_subscribe_max_size_persists_and_activates() -> Result<()> {
    let dir_a = TestDirectory::new("filter-size-a")?;
    let dir_b = TestDirectory::new("filter-size-b")?;
    assert_join_subscribe_filters(
        &dir_a,
        &dir_b,
        SubscribeFilters::new(false, false, None, None, None, Some(1024)),
    )
    .await
}
