mod cli;

use async_recursion::async_recursion;
use comfy_table::Table;

use std::{str::FromStr, time::Duration};

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use cli::{
    args::Cli,
    commands::{
        BackupArgs, CollectionCommand, Command, ConfigCommand, DropCommand, HealthArgs, ImportArgs, NetworkCommand,
        PackageCommand, ScheduleCommand, SnapshotCommand, StatsArgs, VerifyArgs, WatchArgs,
    },
    output::{init_tracing, print_version, run_repl},
};
use n0_future::StreamExt;
use rayon::prelude::*;
use syncweb_core::{
    filter::{FilterAction, FilterConfig, FilterEngine, FilterEntry, FilterRule, MatchCriteria},
    folder::{
        CollectionEntry, CollectionManifest, CollectionStore, DropExportOptions, DropExporter, FolderManager,
        PackageAnnouncement, PackageCatalog, PackageManager, SyncMode,
    },
    fs::{FileEntry, FileType, FsWatcher, Importer, ParallelImporter, ParallelScanner},
    init::InitResult,
    net::{NetworkManager, NetworkOptions, TransportFallback},
    node::{
        identity::{DeviceId, IdentityManager},
        iroh_node::{IrohNode, RelayMode},
    },
    schedule::{BandwidthWindowConfig, ScheduleManager, parse_rate},
    search::{FindEngine, FindQuery},
    snapshot::SnapshotStore,
    sort::{SortCriterion, SortEntry, Sorter},
    stat::{StatFormat, StatOutput},
    stats::BandwidthStats,
    storage::Config as AppConfig,
    sync::{
        AreaFilter, AreaOfInterest, FetchCandidate, FetchFilter, FetchStrategy, HealthReport, SessionMode,
        SubscribeParams, SyncEngine, SyncEvent,
    },
    verify::IntegrityChecker,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;
    tracing::debug!(command = ?cli.command, "cli initialized");
    execute_cli(cli).await
}

async fn execute_cli(cli: Cli) -> Result<()> {
    if is_auxiliary_command(&cli.command) {
        return execute_auxiliary_command(cli).await;
    }

    let output_json = cli.json;
    match cli.command {
        Command::Version => {
            if output_json {
                println!("{}", serde_json::json!({"version": env!("CARGO_PKG_VERSION")}));
            } else {
                print_version();
            }
        }
        Command::Repl => run_repl()?,
        Command::Create(command) => handle_create(&cli.data_dir, command, output_json).await?,
        Command::Join(command) => handle_join(&cli.data_dir, command, output_json).await?,
        Command::Accept { namespace } => handle_accept(&cli.data_dir, namespace, output_json).await?,
        Command::Drop { namespace } => handle_drop(&cli.data_dir, namespace, output_json).await?,
        Command::Folders => handle_folders(&cli.data_dir, output_json).await?,
        Command::Devices => handle_devices(&cli.data_dir, output_json)?,
        Command::Ls(command) => handle_ls(command, output_json)?,
        Command::Find(command) => handle_find(command, output_json)?,
        Command::Sort(command) => handle_sort(&command, output_json)?,
        Command::Stat(command) => handle_stat(command, output_json)?,
        Command::Download(command) => handle_download(&cli.data_dir, command, output_json).await?,
        Command::Import(command) => handle_import(&cli.data_dir, command, output_json).await?,
        Command::Backup(command) => handle_backup(&cli.data_dir, command, output_json).await?,
        Command::Restore(command) => handle_restore(&cli.data_dir, command, output_json).await?,
        Command::Snapshots(command) => {
            handle_snapshots(&cli.data_dir, command.path, command.command, output_json).await?;
        }
        Command::Health(command) => handle_health(&cli.data_dir, command, output_json).await?,
        Command::Init(command) => {
            std::fs::create_dir_all(&command.path)?;
            let node = open_node(&cli.data_dir).await?;
            let manager = FolderManager::new(&node);
            let folder = manager.create(SyncMode::from_str(&command.mode)?).await?;
            let ticket = folder.ticket(node.endpoint().addr(), true).await?;
            let result = InitResult::new(&command.path, folder.namespace_id(), ticket);
            if output_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "path": result.path,
                        "namespace": result.namespace,
                        "ticket": result.ticket,
                        "share_url": result.share_url,
                    }))?
                );
            } else {
                println!("path: {}", result.path.display());
                println!("namespace: {}", result.namespace);
                println!("ticket: {}", result.ticket);
                println!("share_url: {}", result.share_url);
            }
            node.stop().await?;
        }
        Command::Automatic(command) => handle_automatic(&cli.data_dir, command, output_json).await?,
        Command::Subscribe(command) => handle_subscribe(&cli.data_dir, command, output_json).await?,
        Command::Publish(command) => handle_publish(&cli.data_dir, command, output_json).await?,
        Command::Unpublish(command) => handle_unpublish(&cli.data_dir, command, output_json).await?,
        Command::Collection { command } => handle_collection(&cli.data_dir, command, output_json).await?,
        Command::Package { command } => handle_package(&cli.data_dir, command, output_json).await?,
        Command::Network { command } => handle_network(&cli.data_dir, command, output_json).await?,
        Command::Start
        | Command::Shutdown
        | Command::Watch(_)
        | Command::Stats(_)
        | Command::Verify(_)
        | Command::Schedule { .. }
        | Command::Config { .. }
        | Command::Completions { .. }
        | Command::Manpages { .. } => anyhow::bail!("auxiliary command dispatch failed"),
    }
    Ok(())
}

const fn is_auxiliary_command(command: &Command) -> bool {
    matches!(
        command,
        Command::Start
            | Command::Shutdown
            | Command::Watch(_)
            | Command::Stats(_)
            | Command::Verify(_)
            | Command::Schedule { .. }
            | Command::Config { .. }
            | Command::Completions { .. }
            | Command::Manpages { .. }
    )
}

async fn execute_auxiliary_command(cli: Cli) -> Result<()> {
    let Cli {
        data_dir,
        command,
        json: output_json,
        ..
    } = cli;
    if matches!(&command, Command::Start) {
        return handle_start(&data_dir, output_json).await;
    }
    if matches!(&command, Command::Shutdown) {
        return handle_shutdown(&data_dir, output_json).await;
    }
    if let Command::Watch(watch) = command {
        return handle_watch(&data_dir, watch, output_json).await;
    }
    if let Command::Stats(stats) = command {
        return handle_stats(&data_dir, stats, output_json);
    }
    if let Command::Verify(verify) = command {
        return handle_verify(&data_dir, verify, output_json).await;
    }
    if let Command::Schedule { command: schedule } = command {
        return handle_schedule(&data_dir, schedule, output_json);
    }
    if let Command::Config { command: config } = command {
        return handle_config(&data_dir, config, output_json);
    }
    if let Command::Completions { shell } = command {
        clap_complete::generate(shell, &mut Cli::command(), "syncweb", &mut std::io::stdout());
        return Ok(());
    }
    if let Command::Manpages { dir } = command {
        return generate_manpages(&dir);
    }
    anyhow::bail!("unsupported auxiliary command")
}

fn handle_config(data_dir: &std::path::Path, command: Option<ConfigCommand>, output_json: bool) -> Result<()> {
    let config_path = data_dir.join("config.toml");
    let mut config = AppConfig::load(&config_path)?;
    match command {
        None | Some(ConfigCommand::Show { section: None }) => {
            if output_json {
                println!("{}", serde_json::to_string_pretty(&config)?);
            } else {
                print_config(&config)?;
            }
        }
        Some(ConfigCommand::Show { section: Some(section) }) => match section.as_str() {
            "bep" => print_config_section(&config.bep, output_json)?,
            "schedule" => print_config_section(&config.schedule, output_json)?,
            _ => anyhow::bail!("unsupported config section {section:?}; supported sections: bep, schedule"),
        },
        Some(ConfigCommand::Set { key, value }) => {
            config.set(&key, &value)?;
            config.save(&config_path)?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({"status": "updated", "key": key, "value": value})
                );
            } else {
                println!("{key} updated");
            }
        }
    }
    Ok(())
}

fn print_config_section<T: serde::Serialize>(section: &T, output_json: bool) -> Result<()> {
    if output_json {
        println!("{}", serde_json::to_string_pretty(section)?);
    } else {
        println!("{}", toml::to_string_pretty(section)?);
    }
    Ok(())
}

fn generate_manpages(dir: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    let command = Cli::command();
    let man = clap_mangen::Man::new(command.clone());
    let mut buffer = Vec::default();
    man.render(&mut buffer)?;
    std::fs::write(dir.join("syncweb.1"), buffer)?;
    println!("Generated: syncweb.1");
    for subcommand in command.get_subcommands() {
        let name = subcommand.get_name();
        if name == "help" || name == "completions" || name == "manpages" {
            continue;
        }
        let subcommand_man = clap_mangen::Man::new(subcommand.clone());
        let mut subcommand_buffer = Vec::default();
        subcommand_man.render(&mut subcommand_buffer)?;
        let filename = format!("syncweb-{name}.1");
        std::fs::write(dir.join(&filename), subcommand_buffer)?;
        println!("Generated: {filename}");
    }
    println!("manpages generated in {}", dir.display());
    Ok(())
}

#[async_recursion]
async fn handle_start(data_dir: &std::path::Path, output_json: bool) -> Result<()> {
    let node = open_node(data_dir).await?;
    if output_json {
        println!(
            "{}",
            serde_json::json!({"status": "started", "node_id": node.endpoint().id().to_string()})
        );
    } else {
        println!("started: {}", node.endpoint().id());
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_shutdown(data_dir: &std::path::Path, output_json: bool) -> Result<()> {
    let node = open_node(data_dir).await?;
    node.stop().await?;
    if output_json {
        println!("{}", serde_json::json!({"status": "stopped"}));
    } else {
        println!("stopped");
    }
    Ok(())
}

#[async_recursion]
async fn handle_import(data_dir: &std::path::Path, command: ImportArgs, output_json: bool) -> Result<()> {
    if !command.path.exists() {
        anyhow::bail!("import path does not exist: {}", command.path.display());
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = if let Some(namespace) = command.folder {
        manager.get(namespace.parse()?).await?
    } else {
        resolve_folder(&manager, &command.path).await?
    };
    let root = if command.path.is_dir() {
        command.path.clone()
    } else {
        command
            .path
            .parent()
            .map_or_else(|| std::path::PathBuf::from("."), std::path::Path::to_path_buf)
    };
    let importer = ParallelImporter::new(
        node.blob_store().clone(),
        node.docs_engine().clone(),
        folder.doc().clone(),
        folder.author(),
    )
    .with_root(root)
    .with_threads(command.threads);
    let entries = importer.import_path(&command.path).await?;
    if output_json {
        let values = entries
            .iter()
            .map(|entry| serde_json::json!({"path": entry.relative_path}))
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&values)?);
    } else {
        for entry in &entries {
            println!("imported\t{}", entry.relative_path.display());
        }
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_download(
    data_dir: &std::path::Path,
    command: crate::cli::commands::DownloadArgs,
    output_json: bool,
) -> Result<()> {
    if let Some(destination) = command.destination {
        if command.max_peers.is_some()
            || command.min_peers.is_some()
            || command.min_count.is_some()
            || command.max_count.is_some()
        {
            anyhow::bail!("fetch filters require a folder source without a destination");
        }
        copy_path(&command.source, &destination, command.threads)?;
        if output_json {
            println!("{}", serde_json::json!({"destination": destination}));
        } else {
            println!("{}", destination.display());
        }
        return Ok(());
    }

    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = resolve_folder(&manager, &command.source).await?;
    let mut filter = FetchFilter::new();
    if let Some(peers) = command.min_peers {
        filter = filter.with_min_peers(peers);
    }
    if let Some(peers) = command.max_peers {
        filter = filter.with_max_peers(peers);
    }
    if let Some(count) = command.min_count {
        filter = filter.with_min_count(count);
    }
    if let Some(count) = command.max_count {
        filter = filter.with_max_count(count);
    }
    let strategy = if command.max_peers.is_some()
        || command.min_peers.is_some()
        || command.min_count.is_some()
        || command.max_count.is_some()
    {
        FetchStrategy::Filter(filter)
    } else {
        FetchStrategy::All
    };
    let sync = SyncEngine::new(
        manager,
        node.blob_store().clone(),
        node.docs_engine().clone(),
        node.gossip_service().clone(),
    );
    let stats_path = data_dir.join("stats.json");
    let mut bandwidth_stats = BandwidthStats::load(&stats_path)?;
    let folder_key = folder.namespace_id().to_string();
    let mut accounted_bytes = 0_u64;
    let mut intent = sync.fetch(folder.namespace_id(), strategy).await?;
    while let Some(event) = intent.next().await {
        match event {
            SyncEvent::Failed(message) => {
                node.stop().await?;
                anyhow::bail!("download failed: {message}");
            }
            SyncEvent::Finished => break,
            SyncEvent::Stats(transfer_stats) => {
                let delta = transfer_stats.bytes_transferred.saturating_sub(accounted_bytes);
                if delta > 0 {
                    bandwidth_stats.record_download(delta, 0, Some(&folder_key), None);
                    accounted_bytes = transfer_stats.bytes_transferred;
                }
            }
            SyncEvent::Started
            | SyncEvent::Progress { .. }
            | SyncEvent::Paused
            | SyncEvent::Resumed
            | SyncEvent::Cancelled
            | _ => {}
        }
    }
    bandwidth_stats.save(&stats_path)?;
    if output_json {
        println!(
            "{}",
            serde_json::json!({"status": "downloaded", "namespace": folder.namespace_id().to_string()})
        );
    } else {
        println!("downloaded: {}", folder.namespace_id());
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_backup(data_dir: &std::path::Path, command: BackupArgs, output_json: bool) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let snapshots = SnapshotStore::with_docs(node.blob_store().clone(), node.docs_engine().clone());
    let snapshot = if command.path.exists() {
        snapshots
            .create_from_path(&command.path, command.threads, command.description)
            .await?
    } else {
        let folder = resolve_folder(&manager, &command.path).await?;
        snapshots.create_for_folder(&folder, command.description).await?
    };
    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "snapshot": snapshot.id.to_string(),
                "root_hash": snapshot.root_hash.to_string(),
                "files": snapshot.file_count,
                "size": snapshot.total_size,
            })
        );
    } else {
        println!("snapshot: {}", snapshot.id);
        println!("root_hash: {}", snapshot.root_hash);
        println!("files: {}", snapshot.file_count);
        println!("size: {}", snapshot.total_size);
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_restore(
    data_dir: &std::path::Path,
    command: crate::cli::commands::RestoreArgs,
    output_json: bool,
) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let snapshots = SnapshotStore::with_docs(node.blob_store().clone(), node.docs_engine().clone());
    let id = command.snapshot.parse::<iroh_blobs::Hash>()?;
    let snapshot = snapshots.load(id).await?;
    if let Ok(namespace) = command.path.to_string_lossy().parse::<iroh_docs::NamespaceId>() {
        let folder = manager.get(namespace).await?;
        snapshots.restore_for_folder(&folder, &snapshot).await?;
        if output_json {
            println!(
                "{}",
                serde_json::json!({"status": "restored", "namespace": folder.namespace_id().to_string()})
            );
        } else {
            println!("restored: {}", folder.namespace_id());
        }
    } else {
        let paths = snapshots.restore_to_path(&snapshot, &command.path).await?;
        if output_json {
            println!("{}", serde_json::json!({"status": "restored", "files": paths.len()}));
        } else {
            println!("restored: {} files", paths.len());
        }
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_snapshots(
    data_dir: &std::path::Path,
    path: std::path::PathBuf,
    command: Option<SnapshotCommand>,
    output_json: bool,
) -> Result<()> {
    let node = open_node(data_dir).await?;
    let snapshots = SnapshotStore::with_docs(node.blob_store().clone(), node.docs_engine().clone());
    match command {
        None => {
            let namespace = path.to_string_lossy().parse::<iroh_docs::NamespaceId>().ok();
            let mut matching = Vec::new();
            for snapshot in snapshots.list().await? {
                if namespace.is_none_or(|id| snapshot.namespace_id == Some(id)) {
                    matching.push(snapshot);
                }
            }
            if output_json {
                let values = matching
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "id": s.id.to_string(),
                            "created_at": s.created_at,
                            "total_size": s.total_size,
                            "file_count": s.file_count,
                            "description": s.description,
                        })
                    })
                    .collect::<Vec<_>>();
                println!("{}", serde_json::to_string_pretty(&values)?);
            } else {
                let mut table = Table::new();
                table.set_header(["ID", "Created", "Size", "Files", "Description"]);
                for snapshot in &matching {
                    table.add_row([
                        snapshot.id.to_string(),
                        snapshot.created_at.to_string(),
                        snapshot.total_size.to_string(),
                        snapshot.file_count.to_string(),
                        snapshot.description.clone().unwrap_or_default(),
                    ]);
                }
                println!("{table}");
            }
        }
        Some(SnapshotCommand::Diff { path: _, first, second }) => {
            let left = snapshots.load(first.parse()?).await?;
            let right = snapshots.load(second.parse()?).await?;
            let diff = left.diff(&right)?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "added": diff.added.iter().map(|e| e.path.display().to_string()).collect::<Vec<_>>(),
                        "removed": diff.removed.iter().map(|e| e.path.display().to_string()).collect::<Vec<_>>(),
                        "modified": diff.modified.iter().map(|(old, new)| serde_json::json!({
                            "path": old.path,
                            "old_hash": old.hash.to_string(),
                            "new_hash": new.hash.to_string(),
                        })).collect::<Vec<_>>(),
                    })
                );
            } else {
                for entry in diff.added {
                    println!("added\t{}", entry.path.display());
                }
                for entry in diff.removed {
                    println!("removed\t{}", entry.path.display());
                }
                for (old, new) in diff.modified {
                    println!("modified\t{}\t{}\t{}", old.path.display(), old.hash, new.hash);
                }
            }
        }
        Some(SnapshotCommand::Delete { path: _, snapshot }) => {
            let id = snapshot.parse()?;
            snapshots.delete(id).await?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({"status": "deleted", "snapshot": id.to_string()})
                );
            } else {
                println!("deleted: {id}");
            }
        }
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_health(data_dir: &std::path::Path, command: HealthArgs, output_json: bool) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = resolve_folder(&manager, &command.path).await?;
    let entries = node.docs_engine().list_latest(folder.doc()).await?;
    let mut candidates = Vec::new();
    for entry in entries {
        if entry.key().starts_with(b"sys/") {
            continue;
        }
        let path = String::from_utf8(entry.key().to_vec())
            .map_err(|error| anyhow::anyhow!("folder entry path is not UTF-8: {error}"))?;
        candidates.push(FetchCandidate::new(
            path,
            entry.content_hash(),
            entry.content_len(),
            0,
            folder.has_local(entry.content_hash()).await?,
        ));
    }
    let report = HealthReport::from_candidates(&candidates, 4);
    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "total": report.total,
                "well_seeded": report.well_seeded,
                "under_seeded": report.under_seeded,
                "unseeded": report.unseeded,
                "least_seeded": report.least_seeded.iter().take(10).map(|b| serde_json::json!({
                    "hash": b.hash.to_string(),
                    "peer_count": b.peer_count,
                    "size": b.size,
                    "path": b.path,
                })).collect::<Vec<_>>(),
            })
        );
    } else {
        println!("Total blobs: {}", report.total);
        println!("Well-seeded (>=4 peers): {}", report.well_seeded);
        println!("Under-seeded (1-3 peers): {}", report.under_seeded);
        println!("Unseeded (0 peers): {}", report.unseeded);
        if !report.least_seeded.is_empty() {
            println!();
            let mut table = Table::new();
            table.set_header(["Hash", "Peers", "Size", "Path"]);
            for blob in report.least_seeded.iter().take(10) {
                table.add_row([
                    blob.hash.to_string(),
                    blob.peer_count.to_string(),
                    blob.size.to_string(),
                    blob.path.display().to_string(),
                ]);
            }
            println!("{table}");
        }
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_verify(data_dir: &std::path::Path, command: VerifyArgs, output_json: bool) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = resolve_folder(&manager, &command.path).await?;
    let checker = IntegrityChecker::new(node.blob_store().clone(), node.docs_engine().clone());
    let result = checker.verify_folder(&folder).await?;
    if output_json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("total: {}", result.total);
        println!("verified: {}", result.verified);
        println!("corrupted: {}", result.corrupted.len());
        println!("missing: {}", result.missing.len());
        for item in &result.corrupted {
            println!(
                "corrupted\t{}\t{}\t{}",
                item.path.display(),
                item.expected_hash,
                item.actual_hash
            );
        }
        for path in &result.missing {
            println!("missing\t{}", path.display());
        }
    }
    node.stop().await?;
    if !result.is_valid() {
        anyhow::bail!("integrity verification failed");
    }
    Ok(())
}

fn handle_stats(data_dir: &std::path::Path, command: StatsArgs, output_json: bool) -> Result<()> {
    if let Some(period) = command.period {
        parse_period(&period)?;
    }
    let path = data_dir.join("stats.json");
    let mut stats = BandwidthStats::load(&path)?;
    if command.reset {
        stats.reset();
        stats.save(&path)?;
    }
    if output_json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("total_upload:  {}", stats.total_upload);
    println!("total_download: {}", stats.total_download);
    println!("period_start:   {}", stats.period_start);
    match command.folder {
        Some(folder) => {
            let key = folder.to_string_lossy();
            if let Some(folder_stats) = stats.per_folder.get(key.as_ref()) {
                let mut table = Table::new();
                table.set_header(["Folder", "Upload", "Download", "Files"]);
                table.add_row([
                    key.as_ref(),
                    &folder_stats.upload.to_string(),
                    &folder_stats.download.to_string(),
                    &folder_stats.files_transferred.to_string(),
                ]);
                println!("{table}");
            }
        }
        None => {
            if !stats.per_folder.is_empty() {
                let mut table = Table::new();
                table.set_header(["Folder", "Upload", "Download", "Files"]);
                for (folder, folder_stats) in &stats.per_folder {
                    table.add_row([
                        folder.as_str(),
                        &folder_stats.upload.to_string(),
                        &folder_stats.download.to_string(),
                        &folder_stats.files_transferred.to_string(),
                    ]);
                }
                println!("{table}");
            }
        }
    }
    match command.peer {
        Some(peer) => {
            if let Some(peer_stats) = stats.per_peer.get(&peer) {
                let mut table = Table::new();
                table.set_header(["Peer", "Upload", "Download", "Connections"]);
                table.add_row([
                    &peer,
                    &peer_stats.upload.to_string(),
                    &peer_stats.download.to_string(),
                    &peer_stats.connection_count.to_string(),
                ]);
                println!("{table}");
            }
        }
        None => {
            if !stats.per_peer.is_empty() {
                let mut table = Table::new();
                table.set_header(["Peer", "Upload", "Download", "Connections"]);
                for (peer, peer_stats) in &stats.per_peer {
                    table.add_row([
                        peer.as_str(),
                        &peer_stats.upload.to_string(),
                        &peer_stats.download.to_string(),
                        &peer_stats.connection_count.to_string(),
                    ]);
                }
                println!("{table}");
            }
        }
    }
    Ok(())
}

fn handle_schedule(data_dir: &std::path::Path, command: Option<ScheduleCommand>, output_json: bool) -> Result<()> {
    let path = data_dir.join("config.toml");
    let mut config = AppConfig::load(&path)?;
    match command {
        None => {
            ScheduleManager::from_config(&config.schedule)?;
            if output_json {
                println!("{}", serde_json::to_string_pretty(&config.schedule)?);
            } else {
                println!("{}", toml::to_string_pretty(&config.schedule)?);
            }
        }
        Some(ScheduleCommand::Set {
            active,
            bandwidth,
            period,
        }) => {
            if active.is_none() && bandwidth.is_none() {
                anyhow::bail!("schedule set requires --active or --bandwidth");
            }
            if let Some(a) = active {
                syncweb_core::schedule::TimeWindow::parse(&a)?;
                config.schedule.active_hours = a;
            }
            if let Some(bw) = bandwidth {
                let p = period.ok_or_else(|| anyhow::anyhow!("--bandwidth requires --period"))?;
                syncweb_core::schedule::TimeWindow::parse(&p)?;
                parse_rate(&bw)?;
                config
                    .schedule
                    .bandwidth
                    .push(BandwidthWindowConfig::new(p, bw.clone(), bw));
            }
            ScheduleManager::from_config(&config.schedule)?;
            config.save(&path)?;
            if output_json {
                println!("{}", serde_json::to_string_pretty(&config.schedule)?);
            } else {
                println!("schedule updated");
            }
        }
        Some(ScheduleCommand::Folder {
            name,
            active,
            max_upload,
            max_download,
        }) => {
            if active.is_none() && max_upload.is_none() && max_download.is_none() {
                anyhow::bail!("schedule folder requires an override");
            }
            let folder = config.schedule.folders.entry(name).or_default();
            if let Some(a) = active {
                syncweb_core::schedule::TimeWindow::parse(&a)?;
                folder.active_hours = Some(a);
            }
            if let Some(rate) = max_upload {
                parse_rate(&rate)?;
                folder.max_upload = Some(rate);
            }
            if let Some(rate) = max_download {
                parse_rate(&rate)?;
                folder.max_download = Some(rate);
            }
            ScheduleManager::from_config(&config.schedule)?;
            config.save(&path)?;
            if output_json {
                println!("{}", serde_json::to_string_pretty(&config.schedule)?);
            } else {
                println!("schedule folder updated");
            }
        }
    }
    Ok(())
}

#[async_recursion]
async fn handle_watch(data_dir: &std::path::Path, command: WatchArgs, output_json: bool) -> Result<()> {
    let root_is_namespace = command.path.to_string_lossy().parse::<iroh_docs::NamespaceId>().is_ok();
    let root = if root_is_namespace {
        std::path::PathBuf::from(".")
    } else {
        command.path.clone()
    };
    if !root.exists() {
        anyhow::bail!("watch path does not exist: {}", root.display());
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = resolve_folder(&manager, &command.path).await?;
    let importer = Importer::new(
        node.blob_store().clone(),
        node.docs_engine().clone(),
        folder.doc().clone(),
        folder.author(),
    )
    .with_root(&root)
    .with_ignore_patterns(command.exclude);
    let watcher = FsWatcher::new(&root)?;
    loop {
        let Some(event) = watcher.try_recv()? else {
            tokio::time::sleep(Duration::from_millis(command.debounce_ms.max(1))).await;
            continue;
        };
        let removed = matches!(&event.event.kind, notify::EventKind::Remove(_));
        for changed_path in &event.paths {
            let relative = changed_path.strip_prefix(&root).unwrap_or(changed_path);
            if removed {
                folder.delete_entry(relative.as_os_str().as_encoded_bytes()).await?;
            } else if changed_path.is_file() {
                importer.import_path(changed_path).await?;
            } else {
                continue;
            }
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "path": changed_path,
                        "action": if removed { "deleted" } else { "imported" },
                    })
                );
            } else {
                println!(
                    "{}\t{}",
                    if removed { "deleted" } else { "imported" },
                    changed_path.display()
                );
            }
        }
        if command.once {
            break;
        }
    }
    node.stop().await?;
    Ok(())
}

fn parse_period(val: &str) -> Result<Duration> {
    let trimmed = val.trim();
    let (number, suffix) = trimmed.split_at(trimmed.len().saturating_sub(1));
    let amount = number
        .parse::<u64>()
        .map_err(|error| anyhow::anyhow!("invalid period {trimmed:?}: {error}"))?;
    let seconds = match suffix {
        "s" => amount,
        "m" => amount.saturating_mul(60),
        "h" => amount.saturating_mul(60).saturating_mul(60),
        "d" => amount.saturating_mul(60).saturating_mul(60).saturating_mul(24),
        _ => anyhow::bail!("invalid period {trimmed:?}; use a suffix of s, m, h, or d"),
    };
    Ok(Duration::from_secs(seconds))
}

async fn resolve_folder(
    manager: &FolderManager,
    selector: &std::path::Path,
) -> Result<syncweb_core::folder::SyncwebFolder> {
    if let Ok(namespace) = selector.to_string_lossy().parse() {
        return Ok(manager.get(namespace).await?);
    }
    let folders = manager.list().await?;
    match folders.as_slice() {
        [folder] => Ok(folder.clone()),
        [] => anyhow::bail!("no synchronized folders are available"),
        _ => anyhow::bail!("folder path is not a namespace ID and more than one synchronized folder is available"),
    }
}

#[async_recursion]
async fn handle_create(
    data_dir: &std::path::Path,
    command: crate::cli::commands::FolderCreate,
    output_json: bool,
) -> Result<()> {
    std::fs::create_dir_all(&command.path)
        .with_context(|| format!("failed to create folder path {}", command.path.display()))?;
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::from_str(&command.mode)?).await?;
    if let Some(network_name) = command.network {
        add_folder_to_network(data_dir, &network_name, folder.namespace_id())?;
    }
    let ticket = folder.ticket(node.endpoint().addr(), true).await?;
    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "namespace": folder.namespace_id().to_string(),
                "ticket": ticket.to_string(),
            })
        );
    } else {
        println!("namespace: {}", folder.namespace_id());
        println!("ticket: {ticket}");
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_join(
    data_dir: &std::path::Path,
    command: crate::cli::commands::FolderJoin,
    output_json: bool,
) -> Result<()> {
    std::fs::create_dir_all(&command.path)
        .with_context(|| format!("failed to create folder path {}", command.path.display()))?;
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.join(command.ticket, SyncMode::from_str(&command.mode)?).await?;
    if let Some(network_name) = command.network {
        add_folder_to_network(data_dir, &network_name, folder.namespace_id())?;
    }
    if output_json {
        println!(
            "{}",
            serde_json::json!({"status": "joined", "namespace": folder.namespace_id().to_string()})
        );
    } else {
        println!("joined: {}", folder.namespace_id());
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_automatic(
    data_dir: &std::path::Path,
    command: crate::cli::commands::AutomaticArgs,
    output_json: bool,
) -> Result<()> {
    let filter_path = command.filters.unwrap_or_else(|| data_dir.join("filters.toml"));
    let engine = if filter_path.exists() {
        FilterEngine::load(&filter_path)?
    } else {
        FilterEngine::new(FilterConfig::default())?
    };
    if command.show_filters {
        if output_json {
            println!("{}", serde_json::to_string_pretty(&engine.config())?);
        } else {
            print!("{}", toml::to_string_pretty(&engine.config())?);
        }
        return Ok(());
    }
    if command.dry_run {
        let mut results = Vec::new();
        for path in command.paths {
            for entry in ParallelScanner::new(&path, Vec::<String>::new(), 0).scan()? {
                let filter_entry = FilterEntry::from_file(&entry);
                let action = engine.evaluate(&filter_entry);
                let label = match action {
                    FilterAction::Accept => "accept",
                    FilterAction::Reject => "reject",
                    _ => "unknown",
                };
                if output_json {
                    results.push(serde_json::json!({"action": label, "path": entry.path}));
                } else {
                    println!("{label}\t{}", entry.path.display());
                }
            }
        }
        if output_json {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        return Ok(());
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let sync = SyncEngine::new(
        manager.clone(),
        node.blob_store().clone(),
        node.docs_engine().clone(),
        node.gossip_service().clone(),
    );
    let network_manager = open_network_manager(data_dir)?;
    let mut network_topics = Vec::new();
    for network in network_manager.list() {
        network_topics.push(network_manager.subscribe(network.id, node.gossip_service()).await?);
    }
    let mut intents = Vec::new();
    for folder in manager.list().await? {
        intents.push(
            sync.sync_with_filter(
                folder.namespace_id(),
                SessionMode::Continuous,
                syncweb_core::sync::SubscribeParams::default(),
                engine.clone(),
            )
            .await?,
        );
    }
    if output_json {
        println!("{}", serde_json::json!({"status": "running", "folders": intents.len()}));
    } else {
        println!("automatic synchronization running: {} folders", intents.len());
    }
    tokio::signal::ctrl_c().await?;
    for intent in &intents {
        let _result = intent.cancel();
    }
    drop(network_topics);
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_subscribe(
    data_dir: &std::path::Path,
    command: crate::cli::commands::SubscribeArgs,
    output_json: bool,
) -> Result<()> {
    std::fs::create_dir_all(&command.path)?;
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    if let Ok(ticket) = command.ticket.parse::<iroh_blobs::ticket::BlobTicket>() {
        let folder = manager.subscribe_public(&ticket).await?;
        if output_json {
            println!(
                "{}",
                serde_json::json!({
                    "status": "subscribed",
                    "namespace": folder.namespace_id().to_string(),
                    "blob": ticket.hash().to_string(),
                })
            );
        } else {
            println!("subscribed: {}", folder.namespace_id());
            println!("blob: {}", ticket.hash());
        }
        node.stop().await?;
        return Ok(());
    }
    let folder = manager.join(command.ticket, SyncMode::ReceiveOnly).await?;
    let session_id = uuid::Uuid::new_v4();
    let mut params = if command.ingest_only {
        SubscribeParams::ingest_only()
    } else {
        SubscribeParams::default()
    };
    if command.ignore_self {
        params.ignore_session = Some(session_id);
    }
    let area = command
        .prefix
        .map(AreaFilter::Prefix)
        .or_else(|| command.glob.map(AreaFilter::Glob));
    if let Some(filter) = area.clone() {
        params = params.with_area(filter);
    }
    if command.max_size.is_some() || command.max_count.is_some() {
        let limit_area = area.unwrap_or(AreaFilter::All);
        let limits = AreaOfInterest::with_limits(
            limit_area,
            command.max_size.unwrap_or(0),
            command.max_count.unwrap_or(0),
        );
        if limits.is_limit_reached(0, 0) {
            anyhow::bail!("subscription limits are already exhausted");
        }
        params = params.with_limits(limits);
    }
    let sync = SyncEngine::new(
        manager,
        node.blob_store().clone(),
        node.docs_engine().clone(),
        node.gossip_service().clone(),
    );
    let intent = sync.subscribe(folder.namespace_id(), params.clone()).await?;
    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "status": "subscribed",
                "namespace": folder.namespace_id().to_string(),
                "ingest_only": params.ingest_only,
            })
        );
    } else {
        println!("subscribed: {}", folder.namespace_id());
        println!("ingest_only: {}", params.ingest_only);
    }
    tokio::signal::ctrl_c().await?;
    let _result = intent.cancel();
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_publish(
    data_dir: &std::path::Path,
    command: crate::cli::commands::PublishArgs,
    output_json: bool,
) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.get(command.namespace.parse()?).await?;
    if let Some(blob) = command.blob {
        let hash = blob.parse()?;
        let ticket = folder.publish_blob(node.endpoint().addr(), hash).await?;
        if output_json {
            println!("{}", serde_json::json!({"blob_ticket": ticket.to_string()}));
        } else {
            println!("blob_ticket: {ticket}");
        }
    } else {
        let ticket = folder.ticket(node.endpoint().addr(), false).await?;
        if output_json {
            println!("{}", serde_json::json!({"ticket": ticket.to_string()}));
        } else {
            println!("ticket: {ticket}");
        }
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_unpublish(
    data_dir: &std::path::Path,
    command: crate::cli::commands::UnpublishArgs,
    output_json: bool,
) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.get(command.namespace.parse()?).await?;
    folder.unpublish_blob(command.blob.parse()?).await?;
    if output_json {
        println!("{}", serde_json::json!({"status": "unpublished", "blob": command.blob}));
    } else {
        println!("unpublished: {}", command.blob);
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_collection(data_dir: &std::path::Path, command: CollectionCommand, output_json: bool) -> Result<()> {
    match command {
        CollectionCommand::Init {
            path,
            version,
            name: package_name,
        } => {
            std::fs::create_dir_all(&path)?;
            let mut manifest = CollectionManifest::new(uuid::Uuid::new_v4(), version);
            if let Some(name) = package_name {
                manifest.package = Some(syncweb_core::folder::PackageProfile::new(name));
            }
            save_manifest(&path, &manifest)?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({"collection": manifest.collection_id.to_string()})
                );
            } else {
                println!("collection: {}", manifest.collection_id);
            }
        }
        CollectionCommand::Add { path } => {
            let mut manifest = load_manifest(&path)?;
            manifest.entries = scan_collection_entries(&path)?;
            save_manifest(&path, &manifest)?;
            if output_json {
                println!("{}", serde_json::json!({"entries": manifest.entries.len()}));
            } else {
                println!("entries: {}", manifest.entries.len());
            }
        }
        CollectionCommand::Versions {
            path,
            version,
            changelog,
        } => {
            let mut manifest = load_manifest(&path)?;
            let parent = manifest.blob_id()?;
            manifest.parent = Some(parent);
            manifest.version = version;
            manifest.changelog = changelog;
            save_manifest(&path, &manifest)?;
            if output_json {
                println!("{}", serde_json::json!({"version": manifest.version}));
            } else {
                println!("version: {}", manifest.version);
            }
        }
        CollectionCommand::Publish {
            path,
            namespace,
            sequence,
            bootstrap,
        } => {
            let manifest = load_manifest(&path)?;
            let node = open_node(data_dir).await?;
            for entry in &manifest.entries {
                let hash = node.blob_store().add_file(path.join(&entry.logical_path)).await?;
                if hash != entry.content_id {
                    anyhow::bail!(
                        "collection content changed while publishing: {}",
                        entry.logical_path.display()
                    );
                }
            }
            let manager = FolderManager::new(&node);
            let folder = manager.get(namespace.parse()?).await?;
            let store = CollectionStore::new(
                folder.doc().clone(),
                folder.author(),
                node.blob_store().clone(),
                node.docs_engine().clone(),
            );
            let head = store.publish(&manifest, sequence).await?;
            let name = manifest
                .package
                .as_ref()
                .map_or_else(|| manifest.collection_id.to_string(), |profile| profile.name.clone());
            let announcement = PackageAnnouncement::new(
                manifest.collection_id,
                name,
                manifest.version.clone(),
                head.sequence,
                head.manifest,
                node.blob_store().ticket(node.endpoint(), head.manifest).to_string(),
                node.endpoint().id(),
            )?;
            let bootstrap_nodes = parse_bootstrap(bootstrap)?;
            let catalog = PackageCatalog::new(node.gossip_service());
            let topic = if bootstrap_nodes.is_empty() {
                catalog.subscribe(bootstrap_nodes).await?
            } else {
                catalog.subscribe_and_join(bootstrap_nodes).await?
            };
            let (sender, _receiver) = syncweb_core::node::gossip_service::GossipService::split(topic);
            catalog.announce(&sender, &announcement).await?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "manifest": head.manifest.to_string(),
                        "manifest_ticket": announcement.manifest_ticket,
                        "sequence": head.sequence,
                    })
                );
            } else {
                println!("manifest: {}", head.manifest);
                println!("manifest_ticket: {}", announcement.manifest_ticket);
                println!("sequence: {}", head.sequence);
            }
            node.stop().await?;
        }
    }
    Ok(())
}

#[async_recursion]
async fn handle_package(data_dir: &std::path::Path, command: PackageCommand, output_json: bool) -> Result<()> {
    let packages = PackageManager::new(data_dir.join("packages"));
    match command {
        PackageCommand::Search {
            query,
            bootstrap: bootstrap_values,
            timeout_ms,
        } => handle_package_search(data_dir, query, bootstrap_values, timeout_ms, &packages, output_json).await?,
        PackageCommand::Drop { command: drop_command } => {
            handle_package_drop(data_dir, drop_command, output_json).await?;
        }
        PackageCommand::Info {
            manifest: manifest_path,
            ticket: ticket_value,
        } => {
            let collection_manifest = load_package_manifest(data_dir, manifest_path, ticket_value).await?;
            if output_json {
                println!("{}", serde_json::to_string_pretty(&collection_manifest)?);
            } else {
                let mut table = Table::new();
                table.add_row(["Collection", &collection_manifest.collection_id.to_string()]);
                if let Some(package) = &collection_manifest.package {
                    table.add_row(["Name", &package.name]);
                }
                table.add_row(["Version", &collection_manifest.version]);
                if let Some(parent) = &collection_manifest.parent {
                    table.add_row(["Parent", &parent.to_string()]);
                }
                table.add_row(["Entries", &collection_manifest.entries.len().to_string()]);
                println!("{table}");
            }
        }
        PackageCommand::Install {
            manifest: manifest_path,
            source,
            ticket: ticket_value,
        }
        | PackageCommand::Upgrade {
            manifest: manifest_path,
            source,
            ticket: ticket_value,
        } => {
            let collection_manifest = install_package(data_dir, &packages, manifest_path, source, ticket_value).await?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "installed",
                        "collection": collection_manifest.collection_id.to_string(),
                        "version": collection_manifest.version,
                    })
                );
            } else {
                println!(
                    "installed: {} {}",
                    collection_manifest.collection_id, collection_manifest.version
                );
            }
        }
        PackageCommand::Remove {
            collection: collection_id,
            version,
        } => {
            let collection = collection_id.parse()?;
            packages.remove(collection, &version)?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({"status": "removed", "collection": collection_id, "version": version})
                );
            } else {
                println!("removed: {collection} {version}");
            }
        }
        PackageCommand::Verify {
            manifest: manifest_path,
        } => {
            let collection_manifest = load_manifest_file(&manifest_path)?;
            packages.verify(&collection_manifest)?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "verified",
                        "collection": collection_manifest.collection_id.to_string(),
                        "version": collection_manifest.version,
                    })
                );
            } else {
                println!(
                    "verified: {} {}",
                    collection_manifest.collection_id, collection_manifest.version
                );
            }
        }
        PackageCommand::List => handle_package_list(&packages, output_json)?,
        PackageCommand::Versions {
            collection: collection_id,
        } => {
            let collection = collection_id.parse()?;
            let state = packages.state()?;
            let installed = state
                .current(collection)
                .ok_or_else(|| anyhow::anyhow!("collection is not installed: {collection}"))?;
            if output_json {
                let versions = installed.versions.keys().cloned().collect::<Vec<_>>();
                println!("{}", serde_json::to_string_pretty(&versions)?);
            } else {
                for version in installed.versions.keys() {
                    println!("{version}");
                }
            }
        }
        PackageCommand::Switch {
            collection: collection_id,
            version,
        } => {
            let collection = collection_id.parse()?;
            packages.switch(collection, &version)?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({"status": "current", "collection": collection_id, "version": version})
                );
            } else {
                println!("current: {collection} {version}");
            }
        }
    }
    Ok(())
}

#[async_recursion]
async fn handle_package_drop(data_dir: &std::path::Path, command: DropCommand, output_json: bool) -> Result<()> {
    match command {
        DropCommand::Export { paths, version, filter } => {
            handle_package_drop_export(data_dir, paths, version, filter, output_json).await?;
        }
    }
    Ok(())
}

#[async_recursion]
async fn handle_package_drop_export(
    data_dir: &std::path::Path,
    paths: Vec<std::path::PathBuf>,
    version: Option<String>,
    filters: Vec<String>,
    output_json: bool,
) -> Result<()> {
    let (sources, destination) = split_drop_paths(paths)?;
    let filter = parse_drop_filters(&filters)?;
    let multiple = sources.len() > 1;
    if multiple {
        let output_dir = destination
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("multiple packages require an output directory"))?;
        std::fs::create_dir_all(output_dir)?;
    }
    let node = open_node(data_dir).await?;
    let exporter = DropExporter::new(node.blob_store().clone());
    let mut results = Vec::with_capacity(sources.len());
    for source in sources {
        let manifests_with_roots = load_drop_manifests(&source)?;
        for (manifest, root) in &manifests_with_roots {
            add_drop_content(&node, manifest, root).await?;
        }
        let manifests = manifests_with_roots
            .iter()
            .map(|(manifest, _)| manifest.clone())
            .collect::<Vec<_>>();
        let output = drop_output_path(&source, destination.as_deref(), multiple)?;
        let mut options = DropExportOptions::default();
        if let Some(requested_version) = &version {
            options = options.with_version(requested_version.clone());
        }
        if let Some(engine) = &filter {
            options = options.with_filter(engine.clone());
        }
        results.push(exporter.export_manifests(&manifests, output, options).await?);
    }
    if output_json {
        let values = results
            .iter()
            .map(|result| {
                serde_json::json!({
                    "output": result.output,
                    "collection": result.collection_id.to_string(),
                    "version": result.version,
                    "manifest": result.manifest.to_string(),
                    "entries": result.entry_count,
                    "blocks": result.block_count,
                    "archive_size": result.archive_size,
                })
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&values)?);
    } else {
        for result in &results {
            println!(
                "exported: {} ({} {}, {} entries)",
                result.output.display(),
                result.collection_id,
                result.version,
                result.entry_count
            );
        }
    }
    node.stop().await?;
    Ok(())
}

fn split_drop_paths(
    mut package_paths: Vec<std::path::PathBuf>,
) -> Result<(Vec<std::path::PathBuf>, Option<std::path::PathBuf>)> {
    if package_paths.is_empty() {
        anyhow::bail!("at least one package path is required");
    }
    if package_paths.len() == 1 {
        return Ok((package_paths, None));
    }
    let last = package_paths
        .pop()
        .ok_or_else(|| anyhow::anyhow!("at least one package path is required"))?;
    if package_paths.len() == 1 && (last.is_file() || !last.exists()) {
        return Ok((package_paths, Some(last)));
    }
    Ok((package_paths, Some(last)))
}

fn drop_output_path(
    source: &std::path::Path,
    output_destination: Option<&std::path::Path>,
    multiple: bool,
) -> Result<std::path::PathBuf> {
    if let Some(destination) = output_destination {
        if multiple || destination.is_dir() {
            let name = source
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("package path has no file name: {}", source.display()))?;
            return Ok(destination.join(format!("{}.car.zst", name.to_string_lossy())));
        }
        return Ok(destination.to_path_buf());
    }
    let parent = source.parent().unwrap_or_else(|| std::path::Path::new("."));
    let name = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("package path has no file name: {}", source.display()))?;
    Ok(parent.join(format!("{}.car.zst", name.to_string_lossy())))
}

fn parse_drop_filters(expressions: &[String]) -> Result<Option<FilterEngine>> {
    if expressions.is_empty() {
        return Ok(None);
    }
    let mut rules = Vec::with_capacity(expressions.len());
    for expression in expressions {
        let (field_and_value, action) = if let Some(value) = expression.split_once("!=") {
            (value, FilterAction::Reject)
        } else if let Some(value) = expression.split_once("==") {
            (value, FilterAction::Accept)
        } else {
            anyhow::bail!("invalid drop filter {expression:?}; expected FIELD!=VALUE or FIELD==VALUE");
        };
        let field = field_and_value.0.trim();
        let value = field_and_value.1.trim();
        if value.is_empty() {
            anyhow::bail!("drop filter value cannot be empty: {expression:?}");
        }
        let criteria = match field {
            "ext" | "extension" => {
                let mut criteria = MatchCriteria::default();
                criteria.extensions = Some(vec![value.trim_start_matches('.').to_owned()]);
                criteria
            }
            "name" => {
                let mut criteria = MatchCriteria::default();
                criteria.name = Some(value.to_owned());
                criteria
            }
            "path" => {
                let mut criteria = MatchCriteria::default();
                criteria.path = Some(value.to_owned());
                criteria
            }
            _ => anyhow::bail!("unsupported drop filter field {field:?}; use ext, name, or path"),
        };
        rules.push(FilterRule::new(action, criteria));
    }
    let mut config = FilterConfig::default();
    config.rules = rules;
    Ok(Some(FilterEngine::new(config)?))
}

fn load_drop_manifests(source: &std::path::Path) -> Result<Vec<(CollectionManifest, std::path::PathBuf)>> {
    if !source.exists() {
        anyhow::bail!("package path does not exist: {}", source.display());
    }
    let (root, manifest) = if source.is_file() {
        let root = source
            .parent()
            .map_or_else(|| std::path::PathBuf::from("."), std::path::Path::to_path_buf);
        (root, load_manifest_file(source)?)
    } else {
        (source.to_path_buf(), load_manifest(source)?)
    };
    let mut manifests = vec![(manifest, root)];
    if source.is_dir() {
        for child_result in std::fs::read_dir(source)? {
            let child_entry = child_result?;
            if child_entry.file_type()?.is_dir() {
                let child_root = child_entry.path();
                let child_manifest = manifest_path(&child_root);
                if child_manifest.is_file() {
                    manifests.push((load_manifest_file(&child_manifest)?, child_root));
                }
            }
        }
    }
    Ok(manifests)
}

async fn add_drop_content(node: &IrohNode, manifest: &CollectionManifest, root: &std::path::Path) -> Result<()> {
    for entry in &manifest.entries {
        let path = root.join(&entry.logical_path);
        if !path.is_file() {
            anyhow::bail!("package entry is missing: {}", path.display());
        }
        let hash = node.blob_store().add_file(&path).await?;
        if hash != entry.content_id {
            anyhow::bail!(
                "package content changed while exporting: {}",
                entry.logical_path.display()
            );
        }
    }
    Ok(())
}

fn handle_package_list(packages: &PackageManager, output_json: bool) -> Result<()> {
    let state = packages.state()?;
    if output_json {
        let entries = state
            .installed
            .iter()
            .map(|(collection, installed)| {
                serde_json::json!({
                    "collection": collection.to_string(),
                    "current": installed.current,
                })
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        let mut table = Table::new();
        table.set_header(["Collection", "Current"]);
        for (collection, installed) in &state.installed {
            table.add_row([collection.to_string(), installed.current.clone()]);
        }
        println!("{table}");
    }
    Ok(())
}

#[async_recursion]
async fn handle_package_search(
    data_dir: &std::path::Path,
    query: Option<String>,
    bootstrap_values: Vec<String>,
    timeout_ms: u64,
    packages: &PackageManager,
    output_json: bool,
) -> Result<()> {
    let mut all_results = Vec::new();
    for (collection, installed) in packages.state()?.installed {
        let line = format!("{collection}\t{}", installed.current);
        if query.as_ref().is_none_or(|value| line.contains(value)) {
            all_results.push(serde_json::json!({
                "name": "",
                "version": installed.current,
                "collection": collection.to_string(),
                "manifest": "",
            }));
        }
    }
    let bootstrap = parse_bootstrap(bootstrap_values)?;
    let node = open_node(data_dir).await?;
    let catalog = PackageCatalog::new(node.gossip_service());
    let mut topic = if bootstrap.is_empty() {
        catalog.subscribe(bootstrap).await?
    } else {
        catalog.subscribe_and_join(bootstrap).await?
    };
    for announcement in catalog
        .search(
            &mut topic,
            query.as_deref(),
            std::time::Duration::from_millis(timeout_ms),
        )
        .await?
    {
        all_results.push(serde_json::json!({
            "name": announcement.name,
            "version": announcement.version,
            "collection": announcement.collection_id.to_string(),
            "manifest": announcement.manifest,
        }));
    }
    if output_json {
        println!("{}", serde_json::to_string_pretty(&all_results)?);
        node.stop().await?;
        return Ok(());
    }

    if !all_results.is_empty() {
        let mut table = Table::new();
        table.set_header(["Name", "Version", "Collection", "Manifest"]);
        for r in &all_results {
            table.add_row([
                r["name"].as_str().unwrap_or_default(),
                r["version"].as_str().unwrap_or_default(),
                r["collection"].as_str().unwrap_or_default(),
                r["manifest"].as_str().unwrap_or_default(),
            ]);
        }
        println!("{table}");
    }
    node.stop().await?;
    Ok(())
}

async fn load_package_manifest(
    data_dir: &std::path::Path,
    manifest_path: Option<std::path::PathBuf>,
    ticket_value: Option<String>,
) -> Result<CollectionManifest> {
    if let Some(ticket_text) = ticket_value {
        let node = open_node(data_dir).await?;
        let blob_ticket = ticket_text.parse::<iroh_blobs::ticket::BlobTicket>()?;
        if !node.blob_store().has(blob_ticket.hash()).await? {
            node.blob_store().fetch(node.endpoint(), &blob_ticket).await?;
        }
        let manifest = CollectionManifest::from_bytes(node.blob_store().get(blob_ticket.hash()).await?)?;
        if manifest.blob_id()? != blob_ticket.hash() {
            node.stop().await?;
            anyhow::bail!("manifest ticket hash does not match manifest content");
        }
        node.stop().await?;
        Ok(manifest)
    } else {
        load_manifest_file(&manifest_path.ok_or_else(|| anyhow::anyhow!("manifest path is required"))?)
    }
}

async fn install_package(
    data_dir: &std::path::Path,
    packages: &PackageManager,
    manifest_path: Option<std::path::PathBuf>,
    source: Option<std::path::PathBuf>,
    ticket_value: Option<String>,
) -> Result<CollectionManifest> {
    if let Some(ticket_text) = ticket_value {
        let node = open_node(data_dir).await?;
        let blob_ticket = ticket_text.parse::<iroh_blobs::ticket::BlobTicket>()?;
        let manifest = packages
            .install_from_ticket(&blob_ticket, node.endpoint(), node.blob_store())
            .await?;
        node.stop().await?;
        Ok(manifest)
    } else {
        let manifest = load_manifest_file(&manifest_path.ok_or_else(|| anyhow::anyhow!("manifest path is required"))?)?;
        packages.install(
            &manifest,
            source.ok_or_else(|| anyhow::anyhow!("package source is required"))?,
        )?;
        Ok(manifest)
    }
}

fn parse_bootstrap(values: Vec<String>) -> Result<Vec<iroh::PublicKey>> {
    values
        .into_iter()
        .map(|value| value.parse().map_err(anyhow::Error::from))
        .collect()
}

fn manifest_path(path: &std::path::Path) -> std::path::PathBuf {
    path.join(".syncweb-collection.json")
}

fn load_manifest(path: &std::path::Path) -> Result<CollectionManifest> {
    load_manifest_file(&manifest_path(path))
}

fn load_manifest_file(path: &std::path::Path) -> Result<CollectionManifest> {
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read collection manifest {}", path.display()))?;
    Ok(CollectionManifest::from_bytes(bytes)?)
}

fn save_manifest(path: &std::path::Path, manifest: &CollectionManifest) -> Result<()> {
    std::fs::write(manifest_path(path), manifest.to_bytes()?)?;
    Ok(())
}

fn scan_collection_entries(path: &std::path::Path) -> Result<Vec<CollectionEntry>> {
    ParallelScanner::new(path, vec![".syncweb-collection.json".to_owned()], 0)
        .scan()?
        .into_iter()
        .filter(|entry| entry.file_type == FileType::File)
        .map(|entry| {
            CollectionEntry::new(
                iroh_blobs::Hash::from_bytes(*entry.hash.as_bytes()),
                entry.relative_path,
                entry.size,
            )
            .map_err(anyhow::Error::from)
        })
        .collect()
}

#[async_recursion]
async fn handle_network(data_dir: &std::path::Path, command: NetworkCommand, output_json: bool) -> Result<()> {
    let mut manager = open_network_manager(data_dir)?;
    match command {
        NetworkCommand::Create {
            name,
            label,
            invite_only,
        } => {
            let mut options = NetworkOptions::default();
            options.label = label;
            options.invite_only = invite_only;
            let id = manager.create(&name, options)?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({"status": "created", "name": name, "id": id.to_string()})
                );
            } else {
                println!("created: {name}\t{id}");
            }
        }
        NetworkCommand::List { name } => handle_network_list(&manager, name, output_json)?,
        NetworkCommand::Join { ticket } => {
            let parsed = ticket.parse()?;
            let id = manager.join(parsed)?;
            if output_json {
                println!("{}", serde_json::json!({"status": "joined", "id": id.to_string()}));
            } else {
                println!("joined: {id}");
            }
        }
        NetworkCommand::Leave { name } => {
            let id = network_id_by_name(&manager, &name)?;
            manager.leave(id)?;
            if output_json {
                println!("{}", serde_json::json!({"status": "left", "name": name}));
            } else {
                println!("left: {name}");
            }
        }
        NetworkCommand::Invite { name, device } => {
            let id = network_id_by_name(&manager, &name)?;
            let ticket = if let Some(node_id) = device {
                manager.invite(id, node_id.parse()?)?
            } else {
                manager.invite_any(id)?
            };
            if output_json {
                println!("{}", serde_json::json!({"ticket": ticket.to_string()}));
            } else {
                println!("{ticket}");
            }
        }
        NetworkCommand::Kick { name, device } => {
            let id = network_id_by_name(&manager, &name)?;
            manager.kick(id, &device.parse()?)?;
            if output_json {
                println!("{}", serde_json::json!({"status": "kicked", "device": device}));
            } else {
                println!("kicked: {device}");
            }
        }
        NetworkCommand::TestRelay { relay_url } => {
            let identity = IdentityManager::new(data_dir.join("identity.key"))?;
            let app_config = AppConfig::load(data_dir.join("config.toml"))?;
            let mut config = app_config.relay_config();
            config.relay_urls = vec![relay_url.clone()];
            config.auto_fallback = true;
            TransportFallback::new(config)
                .connect_relay(DeviceId::from_node_id(identity.node_id()))
                .await?;
            if output_json {
                println!("{}", serde_json::json!({"status": "reachable", "relay_url": relay_url}));
            } else {
                println!("relay reachable: {relay_url}");
            }
        }
    }
    Ok(())
}

fn handle_network_list(manager: &NetworkManager, name: Option<String>, output_json: bool) -> Result<()> {
    if let Some(network_name) = name {
        let network = manager
            .get_by_name(&network_name)
            .with_context(|| format!("network not found: {network_name}"))?;
        if output_json {
            println!(
                "{}",
                serde_json::json!({
                    "name": network.name,
                    "id": network.id.to_string(),
                    "members": network.members.len(),
                    "folders": network.folders.len(),
                })
            );
        } else {
            let mut table = Table::new();
            table.set_header(["Name", "ID", "Members", "Folders"]);
            table.add_row([
                &network.name,
                &network.id.to_string(),
                &network.members.len().to_string(),
                &network.folders.len().to_string(),
            ]);
            println!("{table}");
        }
    } else {
        let networks = manager.list();
        if output_json {
            let values = networks
                .iter()
                .map(|n| {
                    serde_json::json!({
                        "name": n.name,
                        "id": n.id.to_string(),
                        "members": n.members.len(),
                        "folders": n.folders.len(),
                    })
                })
                .collect::<Vec<_>>();
            println!("{}", serde_json::to_string_pretty(&values)?);
        } else {
            let mut table = Table::new();
            table.set_header(["Name", "ID", "Members", "Folders"]);
            for network in networks {
                table.add_row([
                    &network.name,
                    &network.id.to_string(),
                    &network.members.len().to_string(),
                    &network.folders.len().to_string(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

fn open_network_manager(data_dir: &std::path::Path) -> Result<NetworkManager> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    Ok(NetworkManager::new(data_dir.join("networks.json"), identity.node_id())?)
}

fn network_id_by_name(manager: &NetworkManager, name: &str) -> Result<syncweb_core::net::NetworkId> {
    manager
        .get_by_name(name)
        .map(|network| network.id)
        .with_context(|| format!("network not found: {name}"))
}

fn add_folder_to_network(
    data_dir: &std::path::Path,
    network_name: &str,
    namespace: iroh_docs::NamespaceId,
) -> Result<()> {
    let mut networks = open_network_manager(data_dir)?;
    let id = network_id_by_name(&networks, network_name)?;
    networks.add_folder(id, namespace)?;
    Ok(())
}

#[async_recursion]
async fn handle_accept(data_dir: &std::path::Path, namespace: String, output_json: bool) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.accept(namespace.parse()?).await?;
    if output_json {
        println!(
            "{}",
            serde_json::json!({"status": "accepted", "namespace": folder.namespace_id().to_string()})
        );
    } else {
        println!("accepted: {}", folder.namespace_id());
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_drop(data_dir: &std::path::Path, namespace: String, output_json: bool) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    manager.drop(namespace.parse()?).await?;
    if output_json {
        println!("{}", serde_json::json!({"status": "dropped", "namespace": namespace}));
    } else {
        println!("dropped: {namespace}");
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_folders(data_dir: &std::path::Path, output_json: bool) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folders = manager.list().await?;
    if output_json {
        let values = folders
            .iter()
            .map(|folder| {
                serde_json::json!({
                    "namespace": folder.namespace_id().to_string(),
                    "mode": folder.mode().to_string(),
                })
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&values)?);
    } else {
        let mut table = Table::new();
        table.set_header(["Namespace", "Mode"]);
        for folder in &folders {
            table.add_row([folder.namespace_id().to_string(), folder.mode().to_string()]);
        }
        println!("{table}");
    }
    node.stop().await?;
    Ok(())
}

fn handle_devices(data_dir: &std::path::Path, output_json: bool) -> Result<()> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let device_id = DeviceId::from_node_id(identity.node_id());
    if output_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "iroh": identity.node_id().to_string(),
                "syncthing": device_id.to_syncthing(),
            }))?
        );
    } else {
        println!("iroh: {}", identity.node_id());
        println!("syncthing: {}", device_id.to_syncthing());
    }
    Ok(())
}

fn handle_ls(command: crate::cli::commands::LocalPathArgs, output_json: bool) -> Result<()> {
    let entries = ParallelScanner::new(&command.path, Vec::<String>::new(), command.threads).scan()?;
    if let Some(criteria) = command.sort {
        let mut sortable = entries.into_iter().map(sort_entry).collect::<Vec<_>>();
        Sorter::new(parse_sort_criterion(&criteria)?).sort(&mut sortable);
        if output_json {
            let paths = sortable
                .iter()
                .map(|entry| entry.path.display().to_string())
                .collect::<Vec<_>>();
            println!("{}", serde_json::to_string_pretty(&paths)?);
        } else {
            for entry in sortable {
                println!("{}", entry.path.display());
            }
        }
    } else {
        if output_json {
            let paths = entries
                .iter()
                .map(|entry| entry.relative_path.display().to_string())
                .collect::<Vec<_>>();
            println!("{}", serde_json::to_string_pretty(&paths)?);
        } else {
            for entry in entries {
                println!("{}", entry.relative_path.display());
            }
        }
    }
    Ok(())
}

fn handle_find(command: crate::cli::commands::FindArgs, output_json: bool) -> Result<()> {
    let mut query = match command.kind.as_str() {
        "exact" => FindQuery::exact(&command.pattern),
        "regex" => FindQuery::regex(&command.pattern),
        _ => FindQuery::glob(&command.pattern),
    };
    query.max_depth = command.max_depth;
    query.min_size = command.min_size;
    query.max_size = command.max_size;
    query.extension = command.extension;
    query.file_type = command.file_type.map(|kind| match kind.as_str() {
        "d" => FileType::Directory,
        "l" => FileType::Symlink,
        _ => FileType::File,
    });
    let entries = FindEngine::new(&command.path)
        .with_threads(command.threads)
        .find(&query)?;
    if output_json {
        let paths = entries
            .iter()
            .map(|entry| entry.relative_path.display().to_string())
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&paths)?);
    } else {
        for entry in entries {
            println!("{}", entry.relative_path.display());
        }
    }
    Ok(())
}

fn handle_sort(command: &crate::cli::commands::SortArgs, output_json: bool) -> Result<()> {
    let entries = ParallelScanner::new(&command.path, Vec::<String>::new(), command.threads).scan()?;
    let mut sortable = entries.into_iter().map(sort_entry).collect::<Vec<_>>();
    Sorter::new(parse_sort_criterion(&command.by)?).sort(&mut sortable);
    if output_json {
        let paths = sortable
            .iter()
            .map(|entry| entry.path.display().to_string())
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&paths)?);
    } else {
        for entry in sortable {
            println!("{}", entry.path.display());
        }
    }
    Ok(())
}

fn print_config(config: &AppConfig) -> Result<()> {
    print!("{}", toml::to_string_pretty(config)?);
    Ok(())
}

async fn open_node(data_dir: &std::path::Path) -> Result<IrohNode> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    Ok(IrohNode::new(identity, data_dir.join("data"), RelayMode::Default).await?)
}

fn parse_sort_criterion(value: &str) -> Result<SortCriterion> {
    Ok(match value {
        "niche" => SortCriterion::Niche,
        "frecency" => SortCriterion::Frecency,
        "peers" => SortCriterion::Peers,
        "random" => SortCriterion::Random,
        "folder" => SortCriterion::FolderAggregate,
        other => anyhow::bail!("unknown sort criterion: {other}"),
    })
}

fn sort_entry(entry: FileEntry) -> SortEntry {
    SortEntry::new(entry.relative_path).with_last_accessed(entry.modified)
}

fn copy_path(source: &std::path::Path, destination: &std::path::Path, threads: usize) -> Result<()> {
    if source.is_dir() {
        let source_root = std::fs::canonicalize(source)?;
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let destination_root =
            if destination.exists() {
                std::fs::canonicalize(destination)?
            } else {
                let parent = destination.parent().unwrap_or_else(|| std::path::Path::new("."));
                std::fs::canonicalize(parent)?.join(destination.file_name().ok_or_else(|| {
                    anyhow::anyhow!("destination has no final path component: {}", destination.display())
                })?)
            };
        if destination_root.starts_with(&source_root) {
            anyhow::bail!("cannot download a directory into itself: {}", destination.display());
        }
        let mut files = Vec::new();
        collect_copy_files(source, destination, &mut files)?;
        let copy_files = || {
            files.par_iter().try_for_each(|(src, dest)| {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(src, dest)?;
                Ok::<_, anyhow::Error>(())
            })
        };
        match threads.cmp(&1) {
            std::cmp::Ordering::Equal => {
                files.iter().try_for_each(|(src, dest)| {
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(src, dest)?;
                    Ok::<_, anyhow::Error>(())
                })?;
            }
            std::cmp::Ordering::Greater => {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .context("failed to create download thread pool")?
                    .install(copy_files)?;
            }
            std::cmp::Ordering::Less => {
                copy_files()?;
            }
        }
    } else {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, destination)?;
    }
    Ok(())
}

fn collect_copy_files(
    source: &std::path::Path,
    destination: &std::path::Path,
    files: &mut Vec<(std::path::PathBuf, std::path::PathBuf)>,
) -> Result<()> {
    std::fs::create_dir_all(destination)?;
    for child_res in std::fs::read_dir(source)? {
        let entry = child_res?;
        let child_destination = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_copy_files(&entry.path(), &child_destination, files)?;
        } else if file_type.is_file() {
            files.push((entry.path(), child_destination));
        } else {
            // do nothing
        }
    }
    Ok(())
}

fn handle_stat(command: crate::cli::commands::StatArgs, output_json: bool) -> Result<()> {
    let metadata = std::fs::symlink_metadata(&command.path)?;
    let file_type = if metadata.file_type().is_symlink() {
        FileType::Symlink
    } else if metadata.is_dir() {
        FileType::Directory
    } else {
        FileType::File
    };
    let hash = if file_type == FileType::File {
        let target = std::fs::canonicalize(&command.path)?;
        ParallelScanner::new(
            command.path.parent().unwrap_or_else(|| std::path::Path::new(".")),
            Vec::<String>::new(),
            command.threads,
        )
        .scan()?
        .into_iter()
        .find(|entry| std::fs::canonicalize(&entry.path).is_ok_and(|path| path == target))
        .map_or_else(|| blake3::hash(&[]), |entry| entry.hash)
    } else {
        blake3::hash(&[])
    };
    let entry = FileEntry::builder()
        .path(command.path.clone())
        .relative_path(
            command
                .path
                .file_name()
                .map_or_else(|| command.path.clone(), std::path::PathBuf::from),
        )
        .size(metadata.len())
        .modified(metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH))
        .hash(hash)
        .file_type(file_type)
        .build()
        .map_err(|e| anyhow::anyhow!(e))?;
    let output = StatOutput::from_entry(&entry);
    let format = if command.terse {
        StatFormat::Terse
    } else if let Some(template) = command.format {
        StatFormat::Custom(template)
    } else {
        StatFormat::Human
    };
    if output_json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", output.display(format));
    }
    Ok(())
}
