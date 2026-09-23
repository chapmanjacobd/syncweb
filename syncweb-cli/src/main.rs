mod cli;

use async_recursion::async_recursion;
use comfy_table::Table;

use std::{
    path::{Path, PathBuf},
    process::{Child, Command as ProcessCommand, Stdio},
    str::FromStr,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use cli::{
    args::{Cli, CliContext, category_of, effective_data_dir},
    commands::{
        Command, ConfigCommand, ImportArgs, ListingFlags, NetworkCommand, NetworkListArgs, PackageCommand,
        PublishCommand, ScheduleCommand, SearchArgs, SearchKind, ShareArgs, ShutdownArgs, SnapshotCommand,
        SnapshotCreateArgs, SnapshotRestoreArgs, StartArgs, StatsCommand, StatsFilesArgs, StatsNetworkArgs,
        TransferAllocateArgs, TransferCommand, TransferEnqueueArgs, TransferInfoArgs, TransferJobArgs,
        TransferMaterializeArgs, TransferRootArgs, UnshareArgs, VerifyArgs, WatchArgs,
    },
    output::{confirm_destructive, init_tracing, print_version},
};
use indicatif::{ProgressBar, ProgressStyle};
use iroh_blobs::Hash as BlobHash;
use n0_future::StreamExt;
use rayon::prelude::*;
use syncweb_core::{
    allocation::{AllocationCandidate, AllocationDecision, RootCapacity, StorageRoot, allocate},
    cancel_session,
    daemon::{Daemon, DaemonConfig, EntryRow, IpcClient, IpcCommand, IpcRequest, IpcResponse, PidLock, StateFile},
    filter::{FilterAction, FilterConfig, FilterEngine, FilterEntry, FilterRule, MatchCriteria},
    folder::{
        CollectionEntry, CollectionManifest, CollectionStore, DropExportOptions, DropExporter, DropImportOptions,
        DropImporter, FolderLike, FolderManager, PackageManager, SyncMode, SyncwebFolder,
    },
    fs::{FileEntry, FileType, FsWatcher, Importer, ParallelImporter, ParallelScanner},
    init::open_node,
    net::{NetworkLogger, NetworkManager, NetworkOptions, TransportFallback},
    node::{
        identity::{DeviceId, IdentityManager},
        iroh_node::IrohNode,
    },
    schedule::{BandwidthWindowConfig, ScheduleManager, parse_rate},
    search::{FindEngine, FindQuery, filter_entries},
    snapshot::SnapshotStore,
    sort::{SortConfig, SortEntry, Sorter},
    stat::{StatFormat, StatOutput},
    storage::{
        Config as AppConfig,
        config::SubscribeFilters,
        node_db::{NewTransferJob, NodeDatabase, StorageRootRecord},
        stats_db::StatsDatabase,
    },
    sync::{AreaFilter, FetchFilter, FetchStrategy, SyncEngine, SyncEvent},
    verify::IntegrityChecker,
};

const ERR_DAEMON_NOT_RUNNING: &str = "daemon is not running; start with `syncweb start`";
const ERR_NO_FOLDERS: &str = "no synchronized folders are available";
const ERR_UNEXPECTED_RESPONSE: &str = "daemon returned an unexpected response";
const DEFAULT_MEDIA_LISTEN: &str = "127.0.0.1:9193";

async fn run_media_server(listen: std::net::SocketAddr, data_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let node = syncweb_core::init::open_node(data_dir).await?;
    let server = syncweb_core::media::MediaServer::new(listen, node.blob_store().clone());
    let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

    println!("media server listening on {listen}");

    tokio::select! {
        result = server.run(shutdown_tx) => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("shutting down media server");
        }
    }

    node.stop().await?;
    Ok(())
}

fn open_node_db(data_dir: &Path) -> Result<NodeDatabase> {
    Ok(NodeDatabase::open(data_dir.join("node.db"))?)
}

fn open_stats_db(data_dir: &Path) -> Result<StatsDatabase> {
    Ok(StatsDatabase::open(data_dir.join("stats.db"))?)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes;
    let mut unit = 0_usize;
    while size >= 1024 && unit < UNITS.len().saturating_sub(1) {
        size = size.checked_div(1024).unwrap_or(0);
        unit = unit.wrapping_add(1);
    }
    let unit_label = UNITS.get(unit).copied().unwrap_or("B");
    format!("{size} {unit_label}")
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if let Some(code) = top_level_help_exit_code(&args) {
        cli::args::print_grouped_help();
        std::process::exit(code);
    }
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;
    tracing::debug!(command = ?cli.command, category = category_of(&cli.command), "cli initialized");
    Box::pin(execute_cli(cli)).await
}

/// Detect a top-level help request (no subcommand selected) in the raw argument
/// list, returning the exit code to use if the grouped help should be shown.
fn top_level_help_exit_code(args: &[std::ffi::OsString]) -> Option<i32> {
    let mut iter = args.iter().skip(1);
    let mut saw_help = false;
    while let Some(arg) = iter.next() {
        let s = arg.to_str().unwrap_or_default();
        if s == "-h" || s == "--help" {
            saw_help = true;
            continue;
        }
        if s == "--data-dir" || s == "--network" {
            iter.next();
            continue;
        }
        if s.starts_with("--data-dir=") || s.starts_with("--network=") {
            continue;
        }
        if s == "--verbose" || s == "--json" || s == "--no-daemon" || s == "--embedded" {
            continue;
        }
        return None;
    }
    if saw_help { Some(0) } else { Some(2) }
}

fn handle_help(command: Option<&str>) -> Result<()> {
    match command {
        None => {
            cli::args::print_grouped_help();
            Ok(())
        }
        Some(name) => {
            let mut cmd = Cli::command();
            cmd.build();
            let sc = cmd
                .find_subcommand_mut(name)
                .ok_or_else(|| anyhow::anyhow!("unrecognized subcommand '{name}'"))?;
            sc.write_help(&mut std::io::stdout()).context("write subcommand help")?;
            Ok(())
        }
    }
}

async fn execute_cli(cli: Cli) -> Result<()> {
    if let Command::Help { command } = &cli.command {
        return handle_help(command.as_deref());
    }
    if is_auxiliary_command(&cli.command) {
        return execute_auxiliary_command(cli).await;
    }

    let effective = effective_data_dir(&cli.data_dir, cli.network.as_deref());
    let ctx = CliContext {
        data_dir: &effective,
        output_json: cli.json,
        no_daemon: cli.no_daemon,
        network: cli.network.as_deref(),
    };
    match cli.command {
        Command::Version => {
            if ctx.output_json {
                println!("{}", serde_json::json!({"version": env!("CARGO_PKG_VERSION")}));
            } else {
                print_version();
            }
        }
        Command::Create(command) => handle_create(&ctx, command).await?,
        Command::Join(command) => handle_join(&ctx, command).await?,
        Command::Leave(command) => handle_leave(&ctx, command).await?,
        Command::Folders => handle_folders(&ctx).await?,
        Command::Status => handle_status(&ctx).await?,
        Command::Devices => handle_devices(&ctx)?,
        Command::Networks(args) => handle_networks(&ctx, &args)?,
        Command::Ls(command) => handle_ls(&ctx, command).await?,
        Command::Find(command) => handle_find(&ctx, command).await?,
        Command::Search(args) => handle_search(&ctx, args).await?,
        Command::Sort(command) => handle_sort(&ctx, &command).await?,
        Command::Stat(command) => handle_stat(&ctx, command)?,
        Command::Download(command) => handle_download(&ctx, command).await?,
        Command::Import(command) => handle_import(&ctx, command).await?,
        Command::Snapshot { command } => {
            handle_snapshot(&ctx, command).await?;
        }
        Command::Transfer { command } => handle_transfer(&ctx, command).await?,
        Command::Verify(command) => handle_verify(&ctx, command).await?,
        Command::Publish { command } => handle_publish(&ctx, command).await?,
        Command::Share(args) => handle_share(&ctx, args).await?,
        Command::Unshare(args) => handle_unshare(&ctx, args).await?,
        Command::Package { command } => handle_package(&ctx, command).await?,
        Command::Network { command } => handle_network(&ctx, command).await?,
        Command::Stats { command } => handle_stats(&ctx, command).await?,
        Command::Indexing { command } => cli::indexing::handle_indexing(&ctx, command).await?,
        Command::Link { command } => cli::indexing::handle_link(&ctx, command).await?,
        Command::Provider { command } => cli::indexing::handle_provider(&ctx, command)?,
        Command::Start(_)
        | Command::Shutdown(_)
        | Command::Reload
        | Command::DaemonSync(_)
        | Command::Watch(_)
        | Command::Config { .. }
        | Command::Completions { .. }
        | Command::Manpages { .. }
        | Command::Db { .. }
        | Command::Help { .. } => anyhow::bail!("auxiliary command dispatch failed"),
    }
    Ok(())
}

const fn is_auxiliary_command(command: &Command) -> bool {
    matches!(
        command,
        Command::Start(_)
            | Command::Shutdown(_)
            | Command::Reload
            | Command::DaemonSync(_)
            | Command::Watch(_)
            | Command::Config { .. }
            | Command::Completions { .. }
            | Command::Manpages { .. }
            | Command::Db { .. }
    )
}

async fn execute_auxiliary_command(cli: Cli) -> Result<()> {
    let Cli {
        data_dir,
        command,
        json: output_json,
        no_daemon,
        network,
        ..
    } = cli;
    let effective = effective_data_dir(&data_dir, network.as_deref());
    let ctx = CliContext {
        data_dir: &effective,
        output_json,
        no_daemon,
        network: network.as_deref(),
    };
    if let Command::Start(args) = command {
        let base = args.data_dir.as_ref().unwrap_or(&data_dir);
        let effective_start = effective_data_dir(base, network.as_deref());
        let daemon_ctx = CliContext {
            data_dir: &effective_start,
            output_json,
            no_daemon,
            network: network.as_deref(),
        };
        if args.bg && !args.media_only {
            let child = spawn_daemon_process(base, &args, network.as_deref())?;
            if output_json {
                println!("{}", serde_json::json!({"status": "started", "pid": child.id()}));
            } else {
                println!("daemon starting: {}", child.id());
            }
            return Ok(());
        }
        return handle_start(&daemon_ctx, args).await;
    }
    if let Command::Shutdown(args) = command {
        return handle_shutdown(&ctx, args).await;
    }
    if matches!(&command, Command::Reload) {
        return handle_reload(&ctx).await;
    }
    if let Command::DaemonSync(args) = command {
        return handle_daemon_sync(&ctx, args.namespace).await;
    }
    if let Command::Watch(watch) = command {
        return handle_watch(&ctx, watch).await;
    }
    if let Command::Config { command: config } = command {
        return handle_config(&ctx, config).await;
    }
    if let Command::Db { command: db_cmd } = command {
        return handle_db(&ctx, db_cmd);
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

async fn handle_config(ctx: &CliContext<'_>, command: Option<ConfigCommand>) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let node_db = open_node_db(data_dir)?;
    let mut config = node_db.load_app_config()?;
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
            "discovery" => print_config_section(&config.discovery, output_json)?,
            "subscribe" => print_config_section(&config.subscribe, output_json)?,
            _ => anyhow::bail!(
                "unsupported config section {section:?}; supported sections: bep, schedule, discovery, subscribe"
            ),
        },
        Some(ConfigCommand::Set { key, value }) => {
            let nudge = if let Some(selector) = key.strip_suffix(".subscribe") {
                let namespace = if let Ok(ns) = selector.parse::<iroh_docs::NamespaceId>() {
                    ns.to_string()
                } else if let Some(client) = syncweb_core::daemon::daemon_client(data_dir)? {
                    resolve_namespace_via_daemon(&client, selector).await?
                } else {
                    anyhow::bail!(
                        "'{selector}' is not a namespace ID; start the daemon to resolve folder paths, \
                         or pass the folder's namespace ID"
                    );
                };
                let enabled = parse_config_bool(&key, &value)?;
                if enabled {
                    config.set_subscribe(
                        &namespace,
                        true,
                        &config
                            .subscribe
                            .folders
                            .get(&namespace)
                            .map(|entry| entry.filters.clone())
                            .unwrap_or_default(),
                    );
                } else {
                    config.remove_subscribe(&namespace);
                }
                true
            } else {
                config.set(&key, &value)?;
                false
            };
            node_db.save_app_config(&config)?;
            // Keep the TOML config file in sync for user inspection/editing.
            let config_path = data_dir.join("config.toml");
            if let Err(error) = config.save(&config_path) {
                tracing::warn!(%error, "failed to write TOML config file");
            }
            if nudge && let Some(client) = syncweb_core::daemon::daemon_client(data_dir)? {
                let _ = client
                    .send(IpcRequest::new(IpcCommand::TriggerSync { namespace: None }))
                    .await;
            }
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({"status": "updated", "key": key, "value": value})
                );
            } else {
                println!("{key} updated");
            }
        }
        Some(ConfigCommand::Schedule { command: schedule }) => return handle_schedule(ctx, schedule),
    }
    Ok(())
}

fn parse_config_bool(key: &str, value: &str) -> Result<bool> {
    match value {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        _ => anyhow::bail!("{key} must be 'on' or 'off'"),
    }
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
async fn handle_start(ctx: &CliContext<'_>, args: StartArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;

    if args.media_only {
        let listen = match args.media_listen {
            Some(addr) => addr,
            None => DEFAULT_MEDIA_LISTEN.parse()?,
        };
        return run_media_server(listen, data_dir).await;
    }

    let app_config = open_node_db(data_dir)?.load_app_config()?;
    let mut daemon_config = DaemonConfig::new(data_dir);
    daemon_config.network = ctx.network.map(String::from);
    daemon_config.sync_interval = args.sync_interval.map_or(Duration::from_mins(1), Duration::from_secs);
    daemon_config.rayon_threads = args.max_threads.unwrap_or(app_config.parallel.threads);
    daemon_config.log_file = args.log_file;
    daemon_config.relay_mode = if args.no_relay {
        syncweb_core::node::iroh_node::RelayMode::None
    } else {
        syncweb_core::node::iroh_node::RelayMode::Default
    };
    daemon_config.media_listen = args.media_listen;
    daemon_config.discovery.mdns = !args.no_mdns;
    daemon_config.discovery.beacon = !args.no_beacon;
    if let Some(port) = args.beacon_port {
        daemon_config.discovery.beacon_base_port = port;
    }
    if let Some(interface) = args.discovery_interface.clone() {
        daemon_config.discovery.interface = Some(interface);
    }
    let daemon = Daemon::new(daemon_config).await?;
    let state = daemon.state().await;
    if output_json {
        println!("{}", serde_json::to_string_pretty(&state)?);
    } else {
        println!("daemon started: {}", state.node_id);
    }
    daemon.run().await?;
    if output_json {
        println!("{}", serde_json::json!({"status": "stopped"}));
    } else {
        println!("daemon stopped");
    }
    Ok(())
}

#[async_recursion]
async fn handle_shutdown(ctx: &CliContext<'_>, args: ShutdownArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    if !confirm_destructive("stop the daemon", output_json)? {
        println!("aborted");
        return Ok(());
    }
    send_daemon_command(
        data_dir,
        output_json,
        IpcRequest::new(IpcCommand::Shutdown { force: args.force }),
    )
    .await
}

/// Send a command to the running daemon and print its response.
///
/// Shared plumbing for the daemon control commands (`shutdown`, `reload`,
/// `daemon-sync`) that only differ in the [`IpcRequest`] they send.
async fn send_daemon_command(data_dir: &std::path::Path, output_json: bool, request: IpcRequest) -> Result<()> {
    let client =
        syncweb_core::daemon::daemon_client(data_dir)?.ok_or_else(|| anyhow::anyhow!("{ERR_DAEMON_NOT_RUNNING}"))?;
    let response = client.send(request).await?;
    print_daemon_message(response, output_json)
}

fn spawn_daemon_process(data_dir: &std::path::Path, args: &StartArgs, network: Option<&str>) -> Result<Child> {
    let executable = std::env::current_exe().context("resolve syncweb executable")?;
    let mut command = ProcessCommand::new(executable);
    command.arg("--data-dir").arg(data_dir);
    if let Some(net) = network {
        command.arg("--network").arg(net);
    }
    command
        .arg("start")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(log_file) = &args.log_file {
        command.arg("--log-file").arg(log_file);
    }
    if let Some(max_threads) = args.max_threads {
        command.arg("--max-threads").arg(max_threads.to_string());
    }
    if let Some(sync_interval) = args.sync_interval {
        command.arg("--sync-interval").arg(sync_interval.to_string());
    }
    if args.no_relay {
        command.arg("--no-relay");
    }
    if let Some(addr) = args.media_listen {
        command.arg("--media-listen").arg(addr.to_string());
    }
    command.spawn().context("spawn syncweb daemon")
}

async fn daemon_client_or_start(
    data_dir: &std::path::Path,
    no_daemon: bool,
    network: Option<&str>,
) -> Result<Option<syncweb_core::daemon::IpcClient>> {
    if no_daemon {
        return Ok(None);
    }
    if let Some(client) = syncweb_core::daemon::daemon_client(data_dir)? {
        return Ok(Some(client));
    }

    let base = data_dir
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(data_dir);
    let lock = PidLock::new(data_dir);
    let mut daemon_child = if lock.try_acquire()? {
        lock.release()?;
        Some(spawn_daemon_process(
            base,
            &StartArgs {
                bg: true,
                media_only: false,
                data_dir: None,
                log_file: None,
                max_threads: None,
                sync_interval: None,
                no_relay: false,
                no_mdns: false,
                no_beacon: false,
                beacon_port: None,
                discovery_interface: None,
                media_listen: None,
            },
            network,
        )?)
    } else {
        None
    };
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    pb.set_message("waiting for daemon to start...");
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(10))
        .ok_or_else(|| anyhow::anyhow!("failed to calculate daemon startup deadline"))?;
    loop {
        if let Some(client) = syncweb_core::daemon::daemon_client(data_dir)? {
            pb.finish_and_clear();
            return Ok(Some(client));
        }
        if let Some(child) = daemon_child.as_mut()
            && let Some(status) = child.try_wait()?
        {
            pb.finish_and_clear();
            anyhow::bail!("daemon exited before becoming ready (status: {status})");
        }
        if Instant::now() >= deadline {
            pb.finish_and_clear();
            anyhow::bail!("timed out waiting for daemon to become ready");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        pb.tick();
    }
}

#[async_recursion]
async fn handle_status(ctx: &CliContext<'_>) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let state_file = StateFile::new(data_dir);
    let Some(report) = state_file.load_status()? else {
        if output_json {
            println!("{}", serde_json::json!({"status": "stopped"}));
        } else {
            println!("daemon not running");
        }
        return Ok(());
    };
    if output_json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("daemon: running");
        println!("pid: {}", report.pid);
        println!("node: {}", report.node_id);
        println!("uptime: {} seconds", report.uptime_seconds);
        println!("rayon threads: {}", report.rayon_threads);
        println!(
            "bandwidth: {} uploaded, {} downloaded",
            report.bandwidth.upload_total, report.bandwidth.download_total
        );
        if let Some(schedule) = report.schedule {
            println!(
                "schedule: {}",
                if schedule.in_active_window {
                    "active"
                } else {
                    "inactive"
                }
            );
        }
        let mut table = Table::new();
        table.set_header(["Namespace", "Path", "Session", "Last sync", "Entries", "Errors"]);
        for folder in report.folders {
            table.add_row([
                folder.namespace,
                folder.path.display().to_string(),
                if folder.session_active {
                    "active".to_owned()
                } else {
                    "paused".to_owned()
                },
                folder
                    .last_sync_at
                    .map_or_else(|| "-".to_owned(), |value| value.to_string()),
                folder.entries_synced.to_string(),
                folder.errors.len().to_string(),
            ]);
        }
        println!("{table}");
    }
    Ok(())
}

#[async_recursion]
async fn handle_reload(ctx: &CliContext<'_>) -> Result<()> {
    send_daemon_command(ctx.data_dir, ctx.output_json, IpcRequest::new(IpcCommand::ReloadConfig)).await
}

async fn handle_daemon_sync(ctx: &CliContext<'_>, namespace: Option<String>) -> Result<()> {
    send_daemon_command(
        ctx.data_dir,
        ctx.output_json,
        IpcRequest::new(IpcCommand::TriggerSync { namespace }),
    )
    .await
}

#[async_recursion]
async fn resolve_namespace_via_daemon(client: &syncweb_core::daemon::IpcClient, selector: &str) -> Result<String> {
    if let Ok(ns) = selector.parse::<iroh_docs::NamespaceId>() {
        return Ok(ns.to_string());
    }
    let response = client.send(IpcRequest::new(IpcCommand::ListFolders)).await?;
    let IpcResponse::FolderList(folders) = response else {
        anyhow::bail!("unexpected response from daemon while resolving folder");
    };
    let path = std::path::Path::new(selector);
    let matched = if path.exists() {
        folders.iter().find(|f| f.path == path)
    } else {
        folders.iter().find(|f| f.namespace.starts_with(selector))
    };
    match matched {
        Some(f) => Ok(f.namespace.clone()),
        None => match folders.as_slice() {
            [folder] => Ok(folder.namespace.clone()),
            [] => anyhow::bail!("{ERR_NO_FOLDERS}"),
            _ => anyhow::bail!("'{selector}' is not a namespace ID and more than one folder is available"),
        },
    }
}

fn print_daemon_message(response: IpcResponse, output_json: bool) -> Result<()> {
    match response {
        IpcResponse::Ok { message } => {
            if output_json {
                println!("{}", serde_json::json!({"status": "ok", "message": message}));
            } else {
                println!("{message}");
            }
            Ok(())
        }
        IpcResponse::Error { message } => anyhow::bail!("{message}"),
        IpcResponse::Status(_)
        | IpcResponse::FolderList(_)
        | IpcResponse::DownloadComplete { .. }
        | IpcResponse::ImportFilesComplete { .. }
        | IpcResponse::ImportComplete(_)
        | IpcResponse::ExportComplete(_)
        | _ => anyhow::bail!("{ERR_UNEXPECTED_RESPONSE}"),
    }
}

#[async_recursion]
async fn handle_import(ctx: &CliContext<'_>, command: ImportArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    if !command.path.exists() {
        anyhow::bail!("import path does not exist: {}", command.path.display());
    }
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let response = client
            .send(IpcRequest::new(IpcCommand::ImportFiles {
                namespace: command.folder.clone(),
                path: command.path.clone(),
            }))
            .await?;
        match response {
            IpcResponse::ImportFilesComplete { entries } => {
                if output_json {
                    println!("{}", serde_json::json!({"entries": entries}));
                } else {
                    println!("import requested: {entries}");
                }
                return Ok(());
            }
            IpcResponse::Ok { .. }
            | IpcResponse::Status(_)
            | IpcResponse::FolderList(_)
            | IpcResponse::DownloadComplete { .. }
            | IpcResponse::ImportComplete(_)
            | IpcResponse::ExportComplete(_)
            | IpcResponse::Error { .. }
            | _ => return print_daemon_message(response, output_json),
        }
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = if let Some(namespace) = command.folder {
        manager.get(namespace.parse()?).await?
    } else {
        manager.resolve(&command.path).await.map_err(|_error| {
            anyhow::anyhow!(
                "import path '{}' is not a managed folder; use `syncweb create` to make a new folder",
                command.path.display()
            )
        })?
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

async fn download_via_daemon_or_node(
    data_dir: &std::path::Path,
    output_json: bool,
    no_daemon: bool,
    network: Option<&str>,
    command: &crate::cli::commands::DownloadArgs,
) -> Result<()> {
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
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, network).await? {
        let response = client
            .send(IpcRequest::new(IpcCommand::Download {
                namespace: command.source.to_string_lossy().into_owned(),
                strategy,
            }))
            .await?;
        match response {
            IpcResponse::DownloadComplete { bytes_transferred } => {
                if output_json {
                    println!("{}", serde_json::json!({"bytes_transferred": bytes_transferred}));
                } else {
                    println!("downloaded: {bytes_transferred} bytes");
                }
                return Ok(());
            }
            IpcResponse::Ok { .. }
            | IpcResponse::Status(_)
            | IpcResponse::FolderList(_)
            | IpcResponse::ImportFilesComplete { .. }
            | IpcResponse::ImportComplete(_)
            | IpcResponse::ExportComplete(_)
            | IpcResponse::Error { .. }
            | _ => return print_daemon_message(response, output_json),
        }
    }
    download_with_node(data_dir, output_json, command, strategy).await
}

async fn download_with_node(
    data_dir: &std::path::Path,
    output_json: bool,
    command: &crate::cli::commands::DownloadArgs,
    strategy: FetchStrategy,
) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.resolve(&command.source).await?;
    let sync = SyncEngine::from_node(&node, manager);
    let stats_db = open_stats_db(data_dir)?;
    let folder_key = folder.namespace_id().to_string();
    let mut accounted_bytes = 0_u64;
    let mut intent = sync.fetch(folder.namespace_id(), strategy).await?;
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner} {msg} {bar} {bytes}/{total} ({eta})")?
            .progress_chars("=> "),
    );
    pb.set_message("downloading...");
    while let Some(event) = intent.next().await {
        match event {
            SyncEvent::Failed(message) => {
                pb.finish_and_clear();
                node.stop().await?;
                anyhow::bail!("download failed: {message}");
            }
            SyncEvent::Finished => {
                pb.finish_and_clear();
                break;
            }
            SyncEvent::Stats(transfer_stats) => {
                let delta = transfer_stats.bytes_transferred.saturating_sub(accounted_bytes);
                if delta > 0 {
                    let _ = stats_db.record_download(delta, 0, Some(&folder_key), None, None);
                    accounted_bytes = transfer_stats.bytes_transferred;
                }
                pb.set_length(transfer_stats.bytes_total.unwrap_or(0));
                pb.set_position(transfer_stats.bytes_transferred);
            }
            SyncEvent::Started
            | SyncEvent::Progress { .. }
            | SyncEvent::Paused
            | SyncEvent::Resumed
            | SyncEvent::Cancelled
            | _ => {}
        }
    }
    pb.finish_and_clear();
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

async fn handle_download(ctx: &CliContext<'_>, command: crate::cli::commands::DownloadArgs) -> Result<()> {
    if command.source.as_os_str() == "-" {
        return download_from_stdin(ctx, command).await;
    }
    download_one(ctx, command).await
}

async fn download_from_stdin(ctx: &CliContext<'_>, command: crate::cli::commands::DownloadArgs) -> Result<()> {
    use std::io::BufRead;
    let stdin = std::io::stdin();
    let lines: Vec<String> = stdin.lock().lines().collect::<Result<_, _>>()?;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut one = command.clone();
        one.source = std::path::PathBuf::from(trimmed);
        download_one(ctx, one).await?;
    }
    Ok(())
}

async fn download_one(ctx: &CliContext<'_>, command: crate::cli::commands::DownloadArgs) -> Result<()> {
    if let Ok(ticket) = command
        .source
        .to_string_lossy()
        .parse::<iroh_blobs::ticket::BlobTicket>()
    {
        return cli::indexing::download_blob(
            ctx,
            &[ticket],
            command.providers.min_providers,
            command.providers.no_sharing,
            command.destination.as_deref(),
        )
        .await;
    }

    if let Some(hash) = command.filter.hash.first() {
        if command.providers.from.is_empty() {
            anyhow::bail!("--from (or --provider) is required with --hash");
        }
        if command.max_peers.is_some()
            || command.min_peers.is_some()
            || command.min_count.is_some()
            || command.max_count.is_some()
        {
            anyhow::bail!("fetch filters are not supported with --hash; use a blob ticket as the source instead");
        }
        let content_hash: iroh_blobs::Hash = hash.parse().map_err(|e| anyhow::anyhow!("invalid content hash: {e}"))?;
        let tickets: Vec<iroh_blobs::ticket::BlobTicket> = command
            .providers
            .from
            .iter()
            .map(|t| {
                let ticket: iroh_blobs::ticket::BlobTicket =
                    t.parse().map_err(|e| anyhow::anyhow!("invalid blob ticket: {e}"))?;
                if ticket.hash() != content_hash {
                    anyhow::bail!("blob ticket hash does not match --hash");
                }
                Ok(ticket)
            })
            .collect::<Result<_>>()?;
        return cli::indexing::download_blob(
            ctx,
            &tickets,
            command.providers.min_providers,
            command.providers.no_sharing,
            command.destination.as_deref(),
        )
        .await;
    }

    if let Some(destination) = command.destination {
        if command.max_peers.is_some()
            || command.min_peers.is_some()
            || command.min_count.is_some()
            || command.max_count.is_some()
        {
            anyhow::bail!("fetch filters require a folder source without a destination");
        }
        copy_path(&command.source, &destination, command.threads)?;
        if ctx.output_json {
            println!("{}", serde_json::json!({"destination": destination}));
        } else {
            println!("{}", destination.display());
        }
        return Ok(());
    }

    download_via_daemon_or_node(ctx.data_dir, ctx.output_json, ctx.no_daemon, ctx.network, &command).await
}

#[async_recursion]
async fn handle_snapshot_create(ctx: &CliContext<'_>, command: SnapshotCreateArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let snapshots = SnapshotStore::from_node(&node);
    let snapshot = if command.path.exists() {
        snapshots
            .create_from_path(&command.path, command.threads, command.description)
            .await?
    } else {
        let folder = manager.resolve(&command.path).await?;
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
        println!("{}", snapshot.id);
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_snapshot_restore(ctx: &CliContext<'_>, command: SnapshotRestoreArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let snapshots = SnapshotStore::from_node(&node);
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
async fn handle_snapshot(ctx: &CliContext<'_>, command: SnapshotCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    match command {
        SnapshotCommand::Create(args) => {
            if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
                let response = client
                    .send(IpcRequest::new(IpcCommand::SnapshotCreate {
                        path: args.path.clone(),
                        description: args.description.clone(),
                        threads: args.threads,
                    }))
                    .await?;
                return print_daemon_message(response, output_json);
            }
            handle_snapshot_create(ctx, args).await
        }
        SnapshotCommand::Restore(args) => handle_snapshot_restore(ctx, args).await,
        SnapshotCommand::List { path } => {
            if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
                let response = client
                    .send(IpcRequest::new(IpcCommand::SnapshotList { path: path.clone() }))
                    .await?;
                return print_daemon_message(response, output_json);
            }
            let node = open_node(data_dir).await?;
            let snapshots = SnapshotStore::from_node(&node);
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
            node.stop().await?;
            Ok(())
        }
        SnapshotCommand::Diff { path: _, first, second } => {
            let node = open_node(data_dir).await?;
            let snapshots = SnapshotStore::from_node(&node);
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
            node.stop().await?;
            Ok(())
        }
        SnapshotCommand::Delete { path: _, snapshot } => {
            if !confirm_destructive("delete this snapshot", output_json)? {
                println!("aborted");
                return Ok(());
            }
            if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
                let response = client
                    .send(IpcRequest::new(IpcCommand::SnapshotDelete { id: snapshot.clone() }))
                    .await?;
                return print_daemon_message(response, output_json);
            }
            let node = open_node(data_dir).await?;
            let snapshots = SnapshotStore::from_node(&node);
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
            node.stop().await?;
            Ok(())
        }
    }
}

fn transfer_root_capacity(db: &NodeDatabase, root: &StorageRootRecord) -> Result<RootCapacity> {
    let total = fs4::total_space(&root.path)
        .with_context(|| format!("failed to inspect storage root {}", root.path.display()))?;
    let free = fs4::available_space(&root.path)
        .with_context(|| format!("failed to inspect storage root {}", root.path.display()))?;
    let reserved = db.reserved_transfer_bytes(&root.id)?;
    Ok(RootCapacity::new(total, free, reserved))
}

fn existing_destination_size(path: &Path, expected_hash: &[u8; 32]) -> Result<Option<u64>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        anyhow::bail!("materialization destination is not a regular file: {}", path.display());
    }
    let bytes = std::fs::read(path)?;
    if blake3::hash(&bytes).as_bytes() != expected_hash {
        anyhow::bail!("materialization destination has a different blob: {}", path.display());
    }
    Ok(Some(
        u64::try_from(bytes.len()).context("materialized file is too large")?,
    ))
}

fn transfer_group(job: &syncweb_core::storage::node_db::TransferJobRecord, group_by: Option<&str>) -> String {
    match group_by {
        Some("namespace") => job.namespace_id.clone(),
        Some("root") => job.root_id.clone().unwrap_or_else(|| "-".to_owned()),
        Some("state") => job.state.clone(),
        _ => String::new(),
    }
}

async fn handle_transfer(ctx: &CliContext<'_>, command: TransferCommand) -> Result<()> {
    match command {
        TransferCommand::Info(args) => handle_transfer_info(ctx, &args),
        TransferCommand::Remaining => handle_transfer_remaining(ctx),
        TransferCommand::Root(args) => handle_transfer_root(ctx, args),
        TransferCommand::Enqueue(args) => handle_transfer_enqueue(ctx, args).await,
        TransferCommand::Allocate(args) => handle_transfer_allocate(ctx, &args),
        TransferCommand::Materialize(args) => handle_transfer_materialize(ctx, args).await,
        TransferCommand::Pause(args) => handle_transfer_state(ctx, &args, "paused", "paused"),
        TransferCommand::Resume(args) => handle_transfer_state(ctx, &args, "queued", "resumed"),
        TransferCommand::Cancel(args) => handle_transfer_state(ctx, &args, "cancelled", "cancelled"),
        TransferCommand::Retry(args) => handle_transfer_retry(ctx, &args),
    }
}

fn handle_transfer_info(ctx: &CliContext<'_>, args: &TransferInfoArgs) -> Result<()> {
    let db = open_node_db(ctx.data_dir)?;
    let mut jobs = db.list_transfer_jobs(args.namespace.as_deref(), args.state.as_deref())?;
    jobs.sort_by(|left, right| match args.sort.as_str() {
        "updated" => left.updated_at.cmp(&right.updated_at).then(left.id.cmp(&right.id)),
        "size" => right.size.cmp(&left.size).then(left.id.cmp(&right.id)),
        "peers" => right.peer_count.cmp(&left.peer_count).then(left.id.cmp(&right.id)),
        "path" => left.entry_key.cmp(&right.entry_key).then(left.id.cmp(&right.id)),
        _ => left.created_at.cmp(&right.created_at).then(left.id.cmp(&right.id)),
    });
    if let Some(limit) = args.limit {
        jobs.truncate(limit);
    }
    if ctx.output_json {
        let output = jobs
            .iter()
            .map(|job| {
                serde_json::json!({
                    "id": job.id,
                    "namespace": job.namespace_id,
                    "path": String::from_utf8_lossy(&job.entry_key),
                    "hash": hex::encode(job.hash),
                    "size": job.size,
                    "root": job.root_id,
                    "destination": job.destination.as_ref().map(|path| path.display().to_string()),
                    "state": job.state,
                    "bytes_transferred": job.bytes_transferred,
                    "peer_count": job.peer_count,
                    "eta_seconds": job.eta_seconds,
                    "retries": job.retries,
                    "error": job.error_message,
                    "group": transfer_group(job, args.group_by.as_deref()),
                    "created_at": job.created_at,
                    "updated_at": job.updated_at,
                })
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let mut table = Table::new();
        if args.group_by.is_some() {
            table.set_header([
                "Group",
                "ID",
                "Namespace",
                "Path",
                "State",
                "Size",
                "Progress",
                "Peers",
                "Root",
            ]);
        } else {
            table.set_header(["ID", "Namespace", "Path", "State", "Size", "Progress", "Peers", "Root"]);
        }
        for job in jobs {
            let group = transfer_group(&job, args.group_by.as_deref());
            let row = [
                job.id,
                job.namespace_id,
                String::from_utf8_lossy(&job.entry_key).into_owned(),
                job.state,
                format_bytes(job.size),
                format!("{}/{}", format_bytes(job.bytes_transferred), format_bytes(job.size)),
                job.peer_count.to_string(),
                job.root_id.unwrap_or_else(|| "-".to_owned()),
            ];
            if args.group_by.is_some() {
                table.add_row(std::iter::once(group).chain(row).collect::<Vec<_>>());
            } else {
                table.add_row(row);
            }
        }
        println!("{table}");
    }
    Ok(())
}

fn transfer_remaining_row(root: &serde_json::Value) -> [String; 7] {
    [
        root.get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
            .to_owned(),
        root.get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("-")
            .to_owned(),
        format_bytes(root.get("free").and_then(serde_json::Value::as_u64).unwrap_or_default()),
        format_bytes(
            root.get("reserved")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default(),
        ),
        format_bytes(
            root.get("available")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default(),
        ),
        format_bytes(
            root.get("allocatable")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default(),
        ),
        root.get("enabled")
            .map_or_else(|| "null".to_owned(), ToString::to_string),
    ]
}

fn handle_transfer_remaining(ctx: &CliContext<'_>) -> Result<()> {
    let db = open_node_db(ctx.data_dir)?;
    let roots = db.list_storage_roots()?;
    let mut output = Vec::new();
    for root in roots {
        let capacity = transfer_root_capacity(&db, &root)?;
        output.push(serde_json::json!({
            "id": root.id,
            "path": root.path,
            "enabled": root.enabled,
            "total": capacity.total,
            "free": capacity.free,
            "reserved": capacity.reserved,
            "available": capacity.available,
            "allocatable": capacity.allocatable(root.min_free),
            "min_free": root.min_free,
        }));
    }
    if ctx.output_json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let mut table = Table::new();
        table.set_header([
            "Root",
            "Path",
            "Free",
            "Reserved",
            "Available",
            "Allocatable",
            "Enabled",
        ]);
        for root in &output {
            table.add_row(transfer_remaining_row(root));
        }
        println!("{table}");
    }
    Ok(())
}

fn handle_transfer_root(ctx: &CliContext<'_>, args: TransferRootArgs) -> Result<()> {
    let input_root = StorageRootRecord::new(args.id, args.path, args.min_free, !args.disabled);
    std::fs::create_dir_all(&input_root.path)
        .with_context(|| format!("failed to create storage root {}", input_root.path.display()))?;
    let canonical_path = std::fs::canonicalize(&input_root.path)
        .with_context(|| format!("failed to resolve storage root {}", input_root.path.display()))?;
    let canonical_root = StorageRootRecord::new(input_root.id, canonical_path, input_root.min_free, input_root.enabled);
    let db = open_node_db(ctx.data_dir)?;
    db.upsert_storage_root(&canonical_root)?;
    if ctx.output_json {
        println!("{}", serde_json::json!({"status": "saved", "id": canonical_root.id}));
    } else {
        println!("saved storage root {}", canonical_root.id);
    }
    Ok(())
}

async fn handle_transfer_enqueue(ctx: &CliContext<'_>, args: TransferEnqueueArgs) -> Result<()> {
    let namespace = iroh_docs::NamespaceId::from_str(&args.namespace)
        .with_context(|| format!("invalid namespace: {}", args.namespace))?;
    let (hash, size) = if let Some(source) = &args.source {
        let bytes =
            std::fs::read(source).with_context(|| format!("failed to read source file {}", source.display()))?;
        let size = u64::try_from(bytes.len()).context("source file is too large")?;
        (iroh_blobs::Hash::from_bytes(*blake3::hash(&bytes).as_bytes()), size)
    } else {
        let hash = args
            .hash
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("provide --hash or --source"))?
            .parse()
            .with_context(|| format!("invalid blob hash: {:?}", args.hash))?;
        (hash, args.size)
    };
    let path = args.path;
    let entry_key = path.to_string_lossy().into_owned();
    let namespace_id = namespace.to_string();
    let db = open_node_db(ctx.data_dir)?;
    let job_id = db.enqueue_transfer_job(&NewTransferJob::new(
        &namespace_id,
        entry_key.as_bytes(),
        hash.as_bytes(),
        size,
        None,
        None,
    ))?;
    if args.now {
        return handle_transfer_enqueue_now(ctx, &db, &namespace_id).await;
    }
    if ctx.output_json {
        println!("{}", serde_json::json!({"status": "queued", "id": job_id}));
    } else {
        println!("queued transfer job {job_id}");
    }
    Ok(())
}

async fn handle_transfer_enqueue_now(ctx: &CliContext<'_>, db: &NodeDatabase, namespace_id: &str) -> Result<()> {
    let (decisions, _unallocated) = run_transfer_allocation(db, Some(namespace_id), None, None, None, false)?;
    if decisions.is_empty() {
        anyhow::bail!("no storage root could accommodate the queued job");
    }
    let summary = materialize_transfer_jobs(ctx, namespace_id).await?;
    if ctx.output_json {
        println!(
            "{}",
            serde_json::json!({"status": "now", "completed": summary.completed, "failed": summary.failed})
        );
    } else {
        println!(
            "fetched and materialized {} transfer jobs ({} failed)",
            summary.completed, summary.failed
        );
    }
    Ok(())
}

async fn materialize_transfer_jobs(
    ctx: &CliContext<'_>,
    namespace_id: &str,
) -> Result<syncweb_core::daemon::TransferJobSummary> {
    if let Some(client) = daemon_client_or_start(ctx.data_dir, ctx.no_daemon, ctx.network).await? {
        let response = client
            .send(IpcRequest::new(IpcCommand::MaterializeTransfers {
                namespace: Some(namespace_id.to_owned()),
            }))
            .await?;
        if let IpcResponse::TransferJobsProcessed { completed, failed } = response {
            return Ok(syncweb_core::daemon::TransferJobSummary::new(completed, failed));
        }
        print_daemon_message(response, ctx.output_json)?;
        anyhow::bail!("daemon returned an unexpected response while materializing transfers");
    }
    let node = open_node(ctx.data_dir).await?;
    let db = open_node_db(ctx.data_dir)?;
    let summary = syncweb_core::daemon::process_transfer_jobs(&node, &db, Some(namespace_id))
        .await
        .map_err(anyhow::Error::from);
    node.stop().await?;
    summary
}

fn handle_transfer_allocate(ctx: &CliContext<'_>, args: &TransferAllocateArgs) -> Result<()> {
    let db = open_node_db(ctx.data_dir)?;
    let (decisions, unallocated) = run_transfer_allocation(
        &db,
        args.namespace.as_deref(),
        args.path_prefix.as_ref(),
        args.min_size,
        args.max_size,
        args.dry_run,
    )?;
    let result = serde_json::json!({
        "dry_run": args.dry_run,
        "allocated": decisions,
        "unallocated": unallocated,
    });
    if ctx.output_json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "{} allocated, {} did not fit{}",
            decisions.len(),
            unallocated.len(),
            if args.dry_run { " (dry run)" } else { "" }
        );
        for decision in decisions {
            println!(
                "{} -> {}",
                decision.candidate.path.display(),
                decision.destination.display()
            );
        }
    }
    Ok(())
}

fn run_transfer_allocation(
    db: &NodeDatabase,
    namespace: Option<&str>,
    path_prefix: Option<&PathBuf>,
    min_size: Option<u64>,
    max_size: Option<u64>,
    dry_run: bool,
) -> Result<(Vec<AllocationDecision>, Vec<AllocationCandidate>)> {
    let roots = db.list_storage_roots()?;
    let mut root_inputs = Vec::new();
    for root in roots {
        let capacity = transfer_root_capacity(db, &root)?;
        root_inputs.push((
            StorageRoot::new(&root.id, &root.path, root.min_free).with_enabled(root.enabled),
            capacity,
        ));
    }
    let jobs = db.list_transfer_jobs(None, Some("queued"))?;
    let mut candidates = Vec::new();
    for job in &jobs {
        if job.root_id.is_some() {
            continue;
        }
        let path = PathBuf::from(
            String::from_utf8(job.entry_key.clone())
                .with_context(|| format!("job {} has a non-UTF-8 entry path", job.id))?,
        );
        if namespace.is_some_and(|ns| ns != job.namespace_id)
            || min_size.is_some_and(|size| job.size < size)
            || max_size.is_some_and(|size| job.size > size)
            || path_prefix.as_ref().is_some_and(|prefix| !path.starts_with(prefix))
        {
            continue;
        }
        let job_namespace = iroh_docs::NamespaceId::from_str(&job.namespace_id)
            .with_context(|| format!("job {} has an invalid namespace", job.id))?;
        candidates.push(AllocationCandidate::new(
            job_namespace,
            path,
            iroh_blobs::Hash::from(job.hash),
            job.size,
            usize::try_from(job.peer_count).unwrap_or(usize::MAX),
            job.state == "completed",
        ));
    }
    let (decisions, unallocated) = allocate(&root_inputs, &candidates);
    if !dry_run {
        let mut assignments = Vec::new();
        for decision in &decisions {
            if let Some(job) = jobs.iter().find(|job| {
                job.root_id.is_none()
                    && job.namespace_id == decision.candidate.namespace.to_string()
                    && job.entry_key == decision.candidate.path.to_string_lossy().as_bytes()
                    && job.hash == *decision.candidate.hash.as_bytes()
            }) {
                let existing_size =
                    existing_destination_size(&decision.destination, decision.candidate.hash.as_bytes())?;
                assignments.push((job.id.clone(), decision, existing_size));
            }
        }
        for (job_id, decision, existing_size) in assignments {
            db.assign_transfer_job(&job_id, &decision.root_id, &decision.destination)?;
            if let Some(size) = existing_size {
                let peer_count = u64::try_from(decision.candidate.peer_count).context("peer count exceeds u64")?;
                db.update_transfer_job_progress(&job_id, size, peer_count, None, 0)?;
                db.update_transfer_job_state(&job_id, "completed", None)?;
            }
        }
    }
    Ok((decisions, unallocated))
}

async fn handle_transfer_materialize(ctx: &CliContext<'_>, args: TransferMaterializeArgs) -> Result<()> {
    let Some(client) = daemon_client_or_start(ctx.data_dir, ctx.no_daemon, ctx.network).await? else {
        anyhow::bail!("transfer materialization requires a running daemon");
    };
    let response = client
        .send(IpcRequest::new(IpcCommand::MaterializeTransfers {
            namespace: args.namespace,
        }))
        .await?;
    if let IpcResponse::TransferJobsProcessed { completed, failed } = response {
        if ctx.output_json {
            println!("{}", serde_json::json!({"completed": completed, "failed": failed}));
        } else {
            println!("materialized {completed} transfer jobs ({failed} failed)");
        }
    } else {
        print_daemon_message(response, ctx.output_json)?;
    }
    Ok(())
}

fn handle_transfer_state(ctx: &CliContext<'_>, args: &TransferJobArgs, state: &str, action: &str) -> Result<()> {
    let db = open_node_db(ctx.data_dir)?;
    db.update_transfer_job_state(&args.id, state, None)?;
    println!(
        "{}",
        if ctx.output_json {
            serde_json::json!({"status": state, "id": args.id}).to_string()
        } else {
            format!("{action} transfer job {}", args.id)
        }
    );
    Ok(())
}

fn handle_transfer_retry(ctx: &CliContext<'_>, args: &TransferJobArgs) -> Result<()> {
    let db = open_node_db(ctx.data_dir)?;
    db.retry_transfer_job(&args.id)?;
    println!(
        "{}",
        if ctx.output_json {
            serde_json::json!({"status": "queued", "id": args.id}).to_string()
        } else {
            format!("requeued transfer job {}", args.id)
        }
    );
    Ok(())
}

#[async_recursion]
async fn handle_verify(ctx: &CliContext<'_>, command: VerifyArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let response = client
            .send(IpcRequest::new(IpcCommand::VerifyIntegrity {
                path: command.path.clone(),
                hash: command.filter.hash.clone(),
                path_filter: command.filter.path_prefix.clone(),
                glob_filter: command.filter.glob.clone(),
                fix: command.fix,
                from: command.providers.from.clone(),
            }))
            .await?;
        return print_daemon_message(response, output_json);
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.resolve(&command.path).await?;
    let checker = IntegrityChecker::new(node.blob_store().clone(), node.docs_engine().clone());

    let filter: Option<syncweb_core::verify::VerifyFilter> = if command.filter.is_empty() {
        None
    } else {
        match syncweb_core::verify::VerifyFilter::try_from(&command.filter) {
            Ok(f) => Some(f),
            Err(e) => {
                node.stop().await?;
                anyhow::bail!("{e}");
            }
        }
    };

    if command.fix {
        let result = checker.verify_folder_filtered(&folder, filter.as_ref()).await?;
        let repair = try_repair(&node, &folder, &result.corrupted, &command.providers.from).await;
        let fixed = syncweb_core::verify::FixedVerifyResult::new(result.clone(), Some(repair.clone()));
        if output_json {
            println!("{}", serde_json::to_string_pretty(&fixed)?);
        } else {
            print_verify_result_text(&result);
            print_repair_result_text(&repair);
        }
        node.stop().await?;
        if repair.repaired != repair.attempted {
            anyhow::bail!("repair could not fix all corrupt blobs");
        }
        return Ok(());
    }

    let result = checker.verify_folder_filtered(&folder, filter.as_ref()).await?;
    if output_json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_verify_result_text(&result);
    }
    node.stop().await?;
    if !result.is_valid() {
        anyhow::bail!("integrity verification failed");
    }
    Ok(())
}

async fn try_repair(
    node: &IrohNode,
    folder: &syncweb_core::folder::SyncwebFolder,
    corrupted: &[syncweb_core::verify::CorruptionInfo],
    tickets: &[String],
) -> syncweb_core::verify::RepairResult {
    let mut result = syncweb_core::verify::RepairResult::default();
    if corrupted.is_empty() {
        return result;
    }

    // Parse provider tickets
    let provider_tickets: Vec<(iroh_blobs::Hash, iroh_blobs::ticket::BlobTicket)> = tickets
        .iter()
        .filter_map(|t| {
            let ticket: iroh_blobs::ticket::BlobTicket = t.parse().ok()?;
            Some((ticket.hash(), ticket))
        })
        .collect();

    // Discover peers in the namespace
    let namespace_peers = node
        .topic_tracker()
        .find_peers(folder.namespace_id())
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
            match node.blob_store().force_fetch(node.endpoint(), ticket).await {
                Ok(()) => {
                    repaired = true;
                    break;
                }
                Err(e) => {
                    result.failed.push(syncweb_core::verify::RepairOutcome::new(
                        item.path.clone(),
                        hash,
                        false,
                        Some(format!("ticket provider: {e}")),
                    ));
                }
            }
        }
        if repaired {
            result.repaired = result.repaired.saturating_add(1);
            continue;
        }

        // Try namespace peers
        let mut any_error = None;
        for peer in &namespace_peers {
            match node
                .blob_store()
                .force_fetch_from_peer(node.endpoint(), peer, hash)
                .await
            {
                Ok(()) => {
                    repaired = true;
                    break;
                }
                Err(e) => {
                    any_error = Some(format!("peer {peer}: {e}"));
                }
            }
        }
        if repaired {
            result.repaired = result.repaired.saturating_add(1);
        } else {
            result.failed.push(syncweb_core::verify::RepairOutcome::new(
                item.path.clone(),
                hash,
                false,
                any_error.or_else(|| Some("no alternative providers available".to_owned())),
            ));
        }
    }
    result
}

fn print_verify_result_text(result: &syncweb_core::verify::VerifyResult) {
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

fn print_repair_result_text(result: &syncweb_core::verify::RepairResult) {
    println!("\nrepair:");
    println!("attempted: {}", result.attempted);
    println!("repaired: {}", result.repaired);
    for item in &result.failed {
        if let Some(ref error) = item.error {
            println!("failed\t{}\t{}\t{}", item.path.display(), item.hash, error);
        } else {
            println!("failed\t{}\t{}", item.path.display(), item.hash);
        }
    }
}

async fn handle_stats(ctx: &CliContext<'_>, command: StatsCommand) -> Result<()> {
    match command {
        StatsCommand::Network(args) => handle_stats_network(ctx, args),
        StatsCommand::Files(args) => handle_stats_files(ctx, args).await,
    }
}

fn handle_stats_network(ctx: &CliContext<'_>, command: StatsNetworkArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    if let Some(period) = command.period {
        parse_period(&period)?;
    }
    let stats_db = open_stats_db(data_dir)?;
    if command.reset {
        stats_db.reset_bandwidth()?;
    }
    let stats = stats_db.current_stats()?;
    if output_json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("total_upload:  {}", format_bytes(stats.total_upload));
    println!("total_download: {}", format_bytes(stats.total_download));
    println!("period_start:   {}", stats.period_start);
    match command.folder {
        Some(folder) => {
            let key = folder.to_string_lossy();
            if let Some(folder_stats) = stats.per_folder.get(key.as_ref()) {
                let mut table = Table::new();
                table.set_header(["Folder", "Upload", "Download", "Files"]);
                table.add_row([
                    key.as_ref(),
                    &format_bytes(folder_stats.upload),
                    &format_bytes(folder_stats.download),
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
                        &format_bytes(folder_stats.upload),
                        &format_bytes(folder_stats.download),
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
                    &format_bytes(peer_stats.upload),
                    &format_bytes(peer_stats.download),
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
                        &format_bytes(peer_stats.upload),
                        &format_bytes(peer_stats.download),
                        &peer_stats.connection_count.to_string(),
                    ]);
                }
                println!("{table}");
            }
        }
    }
    Ok(())
}

#[async_recursion]
async fn handle_stats_files(ctx: &CliContext<'_>, command: StatsFilesArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;

    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let response = client
            .send(IpcRequest::new(IpcCommand::StatsFiles {
                folder: command.path.clone(),
            }))
            .await?;
        let IpcResponse::FileStats(report) = response else {
            return print_daemon_message(response, output_json);
        };
        print_file_stats_report(&report, &command.by, command.top_largest, output_json)?;
        return Ok(());
    }

    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.resolve(&command.path).await?;
    let entries = folder.content_entries().await?;

    let mut collector = syncweb_core::bandwidth_stats::FileStatsCollector::new();
    for entry in entries {
        collector.add_entry_bytes_with_time(entry.key(), entry.content_len(), Some(entry.timestamp()));
    }
    let report = collector.report();

    print_file_stats_report(&report, &command.by, command.top_largest, output_json)?;

    node.stop().await?;
    Ok(())
}

fn print_file_stats_report(
    report: &syncweb_core::bandwidth_stats::FileStatsReport,
    by: &str,
    top_largest: Option<usize>,
    output_json: bool,
) -> Result<()> {
    if output_json {
        let mut trimmed = report.clone();
        if let Some(limit) = top_largest {
            trimmed.largest_files.truncate(limit);
        }
        println!("{}", serde_json::to_string_pretty(&trimmed)?);
        return Ok(());
    }
    println!("total_files: {}", report.total_files);
    println!("total_size:  {}", format_bytes(report.total_size));
    println!();

    if !report.by_extension.is_empty() && matches!(by, "extension" | "all") {
        let mut table = Table::new();
        table.set_header(["Extension", "Files", "Size"]);
        for (ext, group) in &report.by_extension {
            table.add_row([ext.as_str(), &group.count.to_string(), &format_bytes(group.total_size)]);
        }
        println!("{table}");
    }

    if !report.size_buckets.is_empty() && matches!(by, "size" | "all") {
        let mut table = Table::new();
        table.set_header(["Bucket", "Files"]);
        for (label, count) in &report.size_buckets {
            table.add_row([label.as_str(), &count.to_string()]);
        }
        println!("{table}");
    }

    if !report.time_buckets.is_empty() && matches!(by, "time" | "all") {
        println!();
        println!("insertion time distribution:");
        for (label, count) in &report.time_buckets {
            println!("  {label:10} {count:>8} files");
        }
    }

    if let Some(limit) = top_largest
        && !report.largest_files.is_empty()
    {
        println!();
        println!("largest files:");
        let mut table = Table::new();
        table.set_header(["Size", "Path"]);
        for (path, size) in report.largest_files.iter().take(limit) {
            table.add_row([&format_bytes(*size), path.as_str()]);
        }
        println!("{table}");
    }
    Ok(())
}

fn handle_schedule(ctx: &CliContext<'_>, command: Option<ScheduleCommand>) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let node_db = open_node_db(data_dir)?;
    let mut config = node_db.load_app_config()?;
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
            node_db.save_app_config(&config)?;
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
            node_db.save_app_config(&config)?;
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
async fn handle_watch(ctx: &CliContext<'_>, command: WatchArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    let filter_path = command.filters.clone().unwrap_or_else(|| data_dir.join("filters.toml"));
    let engine = if filter_path.exists() {
        FilterEngine::load(&filter_path)?
    } else {
        FilterEngine::new(FilterConfig::default())?
    };
    if command.show_filters {
        return print_filter_config(&engine, output_json);
    }
    if command.dry_run {
        return dry_run_filters(&engine, command.paths, output_json);
    }
    if !command.once && !no_daemon {
        let client = daemon_client_or_start(data_dir, no_daemon, ctx.network)
            .await?
            .ok_or_else(|| anyhow::anyhow!("daemon not available; start with `syncweb start` or pass --no-daemon"))?;

        let namespace = if let Ok(namespace) = command.path.to_string_lossy().parse::<iroh_docs::NamespaceId>() {
            namespace.to_string()
        } else {
            let response = client.send(IpcRequest::new(IpcCommand::ListFolders)).await?;
            let IpcResponse::FolderList(folders) = response else {
                return print_daemon_message(response, output_json);
            };
            match folders.as_slice() {
                [folder] => folder.namespace.clone(),
                [] => anyhow::bail!("{ERR_NO_FOLDERS}"),
                _ => anyhow::bail!(
                    "folder path is not a namespace ID and more than one synchronized folder is available"
                ),
            }
        };

        let response = client
            .send(IpcRequest::new(IpcCommand::AddFolder {
                namespace,
                path: command.path.clone(),
            }))
            .await?;
        return print_daemon_message(response, output_json);
    }
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
    let folder = manager.resolve(&command.path).await?;
    let importer = Importer::new(
        node.blob_store().clone(),
        node.docs_engine().clone(),
        folder.doc().clone(),
        folder.author(),
    )
    .with_root(&root)
    .with_ignore_patterns(command.exclude);
    let watcher = FsWatcher::new(&root)?;
    let filter_folder = folder.namespace_id().to_string();
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
            } else if !changed_path.is_file() {
                continue;
            } else {
                let size = std::fs::metadata(changed_path).map_or(0, |metadata| metadata.len());
                let accepted = engine
                    .evaluate_for_folder(&filter_folder, &FilterEntry::new(relative.to_path_buf(), size))
                    != FilterAction::Reject;
                if !accepted {
                    if output_json {
                        println!(
                            "{}",
                            serde_json::json!({
                                "path": changed_path,
                                "action": "rejected",
                            })
                        );
                    } else {
                        println!("rejected\t{}", changed_path.display());
                    }
                    continue;
                }
                importer.import_path(changed_path).await?;
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

fn print_filter_config(engine: &FilterEngine, output_json: bool) -> Result<()> {
    if output_json {
        println!("{}", serde_json::to_string_pretty(&engine.config())?);
    } else {
        print!("{}", toml::to_string_pretty(&engine.config())?);
    }
    Ok(())
}

fn dry_run_filters(engine: &FilterEngine, paths: Vec<PathBuf>, output_json: bool) -> Result<()> {
    let mut results = Vec::new();
    for path in paths {
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

#[async_recursion]
async fn handle_create(ctx: &CliContext<'_>, command: crate::cli::commands::FolderCreate) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    let do_share = !command.no_share;
    let write = command.write;
    std::fs::create_dir_all(&command.path)
        .with_context(|| format!("failed to create folder path {}", command.path.display()))?;
    let do_import = command.import && !command.no_import;
    let namespace = if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let response = client
            .send(IpcRequest::new(IpcCommand::CreateFolder {
                path: command.path.clone(),
                mode: command.mode.clone(),
                indexing: !command.no_indexing,
            }))
            .await?;
        let created_namespace = if let IpcResponse::Ok { message } = &response {
            message
                .lines()
                .find_map(|l| l.strip_prefix("namespace: "))
                .map(str::to_owned)
        } else {
            None
        };
        if do_import && dir_has_entries(&command.path)? {
            if let Some(namespace) = created_namespace.clone() {
                let import = client
                    .send(IpcRequest::new(IpcCommand::ImportFiles {
                        namespace: Some(namespace),
                        path: command.path.clone(),
                    }))
                    .await?;
                if !matches!(import, IpcResponse::ImportFilesComplete { .. }) {
                    return print_daemon_message(import, output_json);
                }
            } else {
                return print_daemon_message(response, output_json);
            }
        }
        let namespace = created_namespace.ok_or_else(|| anyhow::anyhow!("daemon create did not report a namespace"))?;
        let node_db = open_node_db(data_dir)?;
        node_db.upsert_folder_mount(&namespace, &command.path)?;
        if do_share {
            let share = client
                .send(IpcRequest::new(IpcCommand::Share {
                    namespace: namespace.clone(),
                    blob: None,
                    writable: write,
                    pin: true,
                    persist: true,
                }))
                .await?;
            let url = if let IpcResponse::Ok { message } = &share {
                message.clone()
            } else {
                return print_daemon_message(share, output_json);
            };
            if output_json {
                let ticket = url.split_once("?ticket=").map_or("", |(_, t)| t);
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "path": command.path,
                        "namespace": namespace,
                        "access": if write { "write" } else { "read" },
                        "ticket": ticket,
                        "url": url,
                    }))?
                );
            } else {
                println!("{url}");
            }
        } else if output_json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "path": command.path,
                    "namespace": namespace,
                }))?
            );
        } else {
            println!("{namespace}");
        }
        Some(namespace)
    } else {
        let node = open_node(data_dir).await?;
        let manager = FolderManager::new(&node);
        let folder = manager.create(SyncMode::from_str(&command.mode)?).await?;
        if let Some(network_name) = command.network {
            add_folder_to_network(data_dir, &network_name, folder.namespace_id())?;
        }
        if !command.no_indexing
            && let Ok(indexing) = syncweb_core::indexing::IndexingService::new(data_dir.join("indexing.sqlite"))
            && let Err(error) = indexing.enable_folder(&folder).await
        {
            tracing::warn!(%error, "failed to enable indexing for new folder");
        }
        if do_import && dir_has_entries(&command.path)? {
            let importer = ParallelImporter::new(
                node.blob_store().clone(),
                node.docs_engine().clone(),
                folder.doc().clone(),
                folder.author(),
            )
            .with_root(command.path.clone())
            .with_threads(0);
            importer.import_path(&command.path).await?;
        }
        let namespace = folder.namespace_id().to_string();
        let node_db = open_node_db(data_dir)?;
        node_db.upsert_folder_mount(&namespace, &command.path)?;
        if do_share {
            let db = open_node_db(data_dir)?;
            let result = syncweb_core::folder::share_folder(
                &folder,
                syncweb_core::folder::ShareOptions::new(write)
                    .with_pin(true)
                    .with_persist(true),
                |ns, access, ticket| db.add_share(&ns.to_string(), access, &ticket.to_string()),
            )
            .await?;
            if output_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "path": command.path,
                        "namespace": result.namespace.to_string(),
                        "access": result.access(),
                        "ticket": result.ticket.to_string(),
                        "url": result.url,
                        "pinned": result.pinned,
                    }))?
                );
            } else {
                println!("{}", result.url);
            }
        } else if output_json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "path": command.path,
                    "namespace": namespace,
                }))?
            );
        } else {
            println!("{namespace}");
        }
        node.stop().await?;
        Some(namespace)
    };
    if namespace.is_none() && do_share {
        anyhow::bail!("create succeeded but no folder namespace was reported");
    }
    Ok(())
}

fn dir_has_entries(path: &Path) -> Result<bool> {
    match std::fs::read_dir(path) {
        Ok(mut entries) => Ok(entries.next().is_some()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to read folder path {}", path.display())),
    }
}

#[async_recursion]
async fn handle_join(ctx: &CliContext<'_>, command: crate::cli::commands::FolderJoin) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    let filters = subscribe_filters_from(&command);
    let subscribe = command.effective_subscribe();
    let download_existing = command.download_existing;
    let effective_path = if let Some(prefix) = &command.prefix {
        prefix.join(&command.path)
    } else {
        command.path.clone()
    };

    // `join <folder>` (subscribe is on by default) on an already-tracked folder: idempotent enable.
    let is_new_ticket =
        command.ticket.parse::<iroh_docs::DocTicket>().is_ok() || command.ticket.trim_start().starts_with("syncweb://");
    if !is_new_ticket {
        if !subscribe {
            anyhow::bail!("folder already tracked — re-enable live syncing with `join <folder>`");
        }
        return handle_join_existing(ctx, &command.ticket, &filters).await;
    }

    std::fs::create_dir_all(&effective_path)
        .with_context(|| format!("failed to create folder path {}", effective_path.display()))?;
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let response = client
            .send(IpcRequest::new(IpcCommand::Join {
                ticket: command.ticket.clone(),
                path: effective_path.clone(),
                mode: SyncMode::from_str(&command.mode)?,
                subscribe,
                filters: filters.clone(),
                download: download_existing,
                indexing: !command.no_indexing,
            }))
            .await?;
        return print_daemon_message(response, output_json);
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.join(command.ticket, SyncMode::from_str(&command.mode)?).await?;
    if let Some(network_name) = command.network {
        add_folder_to_network(data_dir, &network_name, folder.namespace_id())?;
    }
    if !command.no_indexing
        && let Ok(indexing) = syncweb_core::indexing::IndexingService::new(data_dir.join("indexing.sqlite"))
        && let Err(error) = indexing.enable_folder(&folder).await
    {
        tracing::warn!(%error, "failed to enable indexing for joined folder");
    }
    let namespace = folder.namespace_id().to_string();
    let node_db = open_node_db(data_dir)?;
    let mut config = node_db.load_app_config()?;
    config.set_subscribe(&namespace, subscribe, &filters);
    node_db.save_app_config(&config)?;
    node_db.upsert_folder_mount(&namespace, &effective_path)?;
    let (downloaded, downloaded_size) = if download_existing {
        download_joined_folder(&node, manager.clone(), &folder, &filters, &effective_path).await?
    } else {
        (0, 0)
    };
    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "status": "joined",
                "namespace": namespace,
                "downloaded": downloaded,
                "size": downloaded_size,
            })
        );
    } else {
        let live = if subscribe { " — live sync on;" } else { "" };
        if download_existing {
            println!("joined: {namespace}{live} downloaded: {downloaded} files ({downloaded_size} bytes)");
        } else {
            println!("joined: {namespace}{live}");
        }
    }
    node.stop().await?;
    Ok(())
}

async fn download_joined_folder(
    node: &IrohNode,
    manager: FolderManager,
    folder: &syncweb_core::folder::SyncwebFolder,
    filters: &SubscribeFilters,
    destination: &Path,
) -> Result<(usize, u64)> {
    const DOWNLOAD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
    let sync = SyncEngine::from_node(node, manager);
    let strategy =
        FetchStrategy::Filter(FetchFilter::new().with_paths(filters.sync_prefix.clone().into_iter().collect()));
    let mut intent = sync.fetch(folder.namespace_id(), strategy).await?;
    // The one-shot download is opportunistic: grab whatever a reachable peer
    // has within a short window, then let live sync (when enabled) continue.
    let _ = tokio::time::timeout(DOWNLOAD_TIMEOUT, async {
        while let Some(event) = intent.next().await {
            match event {
                SyncEvent::Failed(message) => {
                    tracing::warn!(%message, "join download could not reach a peer; existing content will arrive via live sync");
                    break;
                }
                SyncEvent::Finished => break,
                SyncEvent::Started
                | SyncEvent::Progress { .. }
                | SyncEvent::Stats(_)
                | SyncEvent::Paused
                | SyncEvent::Resumed
                | SyncEvent::Cancelled
                | _ => {}
            }
        }
    })
    .await;
    let _ = intent.cancel();
    let area = filters
        .sync_prefix
        .clone()
        .map(AreaFilter::Prefix)
        .or_else(|| filters.glob.clone().map(AreaFilter::Glob))
        .unwrap_or(AreaFilter::All);
    let entries = folder.list_entries().await?;
    let mut count = 0_usize;
    let mut size = 0_u64;
    for entry in entries {
        let rel = Path::new(&entry.path);
        if !area.matches_path(rel) {
            continue;
        }
        let dest = destination.join(rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match node.blob_store().export_to_path(entry.hash, &dest).await {
            Ok(_) => {
                count = count.saturating_add(1);
                size = size.saturating_add(entry.size);
            }
            Err(error) => {
                // A blob may still be en route (or permanently unavailable) after
                // the fetch window; skip it rather than failing the whole join.
                tracing::warn!(
                    %error,
                    path = %rel.display(),
                    "join download could not write entry; it will arrive via live sync"
                );
            }
        }
    }
    Ok((count, size))
}

fn subscribe_filters_from(command: &crate::cli::commands::FolderJoin) -> SubscribeFilters {
    SubscribeFilters::new(
        command.ingest_only,
        command.ignore_self,
        command.sync_prefix.clone(),
        command.glob.clone(),
        command.max_count,
        command.max_size,
    )
}

async fn handle_join_existing(ctx: &CliContext<'_>, selector: &str, filters: &SubscribeFilters) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let namespace = resolve_namespace_via_daemon(&client, selector).await?;
        let response = client
            .send(IpcRequest::new(IpcCommand::SetSubscribe {
                namespace: namespace.clone(),
                enabled: true,
                filters: Some(filters.clone()),
            }))
            .await?;
        if let IpcResponse::Ok { message } = response {
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({"status": "subscribed", "namespace": namespace})
                );
            } else {
                println!("{message}");
            }
            return Ok(());
        }
        return print_daemon_message(response, output_json);
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let namespace = manager.resolve_namespace(selector).await?;
    let namespace_str = namespace.to_string();
    let node_db = open_node_db(data_dir)?;
    let mut config = node_db.load_app_config()?;
    config.set_subscribe(&namespace_str, true, filters);
    node_db.save_app_config(&config)?;
    if let Ok(mount) = std::fs::canonicalize(selector) {
        let _ = node_db.upsert_folder_mount(&namespace_str, &mount);
    }
    if output_json {
        println!(
            "{}",
            serde_json::json!({"status": "subscribed", "namespace": namespace_str})
        );
    } else {
        println!("subscribed: {namespace_str}");
    }
    node.stop().await?;
    Ok(())
}

async fn handle_publish(ctx: &CliContext<'_>, command: PublishCommand) -> Result<()> {
    match command {
        PublishCommand::Catalog(args) => cli::indexing::handle_catalog_publish(ctx, args).await,
    }
}

async fn handle_share(ctx: &CliContext<'_>, args: ShareArgs) -> Result<()> {
    if args.list {
        return handle_share_list(ctx, Some(&args.path)).await;
    }
    handle_share_add(ctx, &args).await
}

async fn handle_share_add(ctx: &CliContext<'_>, args: &ShareArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    let selector = args.path.to_string_lossy();
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let namespace = resolve_namespace_via_daemon(&client, &selector).await?;
        let response = client
            .send(IpcRequest::new(IpcCommand::Share {
                namespace,
                blob: args.blob.clone(),
                writable: args.write,
                pin: !args.no_pin,
                persist: !args.no_persist,
            }))
            .await?;
        return print_daemon_message(response, output_json);
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let namespace = manager.resolve_namespace(&selector).await?;
    let folder = manager.get(namespace).await?;
    if let Some(blob) = &args.blob {
        let hash = blob.parse()?;
        let ticket = folder.publish_blob(node.endpoint().addr(), hash).await?;
        if output_json {
            println!(
                "{}",
                serde_json::json!({"namespace": namespace, "blob": blob, "ticket": ticket.to_string()})
            );
        } else {
            println!("{ticket}");
        }
        node.stop().await?;
        return Ok(());
    }
    let db = open_node_db(data_dir)?;
    let result = syncweb_core::folder::share_folder(
        &folder,
        syncweb_core::folder::ShareOptions::new(args.write)
            .with_pin(!args.no_pin)
            .with_persist(!args.no_persist),
        |ns, access, ticket| db.add_share(&ns.to_string(), access, &ticket.to_string()),
    )
    .await?;
    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "namespace": result.namespace.to_string(),
                "access": result.access(),
                "ticket": result.ticket.to_string(),
                "url": result.url,
                "pinned": result.pinned,
            })
        );
    } else {
        println!("{}", result.url);
    }
    node.stop().await?;
    Ok(())
}

async fn handle_share_list(ctx: &CliContext<'_>, filter: Option<&std::path::Path>) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let response = client.send(IpcRequest::new(IpcCommand::ShareList)).await?;
        return print_daemon_message(response, output_json);
    }
    let mut shares = open_node_db(data_dir)?.list_shares()?;
    if let Some(selector) = filter
        && selector.as_os_str() != "."
    {
        let node = open_node(data_dir).await?;
        let manager = FolderManager::new(&node);
        let wanted = manager
            .resolve_namespace(&selector.to_string_lossy())
            .await?
            .to_string();
        shares.retain(|(namespace, _, _)| namespace == &wanted);
        node.stop().await?;
    }
    if shares.is_empty() {
        println!("no shares");
    } else {
        for (namespace, access, ticket) in shares {
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({"namespace": namespace, "access": access, "ticket": ticket})
                );
            } else {
                println!("namespace: {namespace}");
                println!("access: {access}");
                println!("ticket: {ticket}");
                println!();
            }
        }
    }
    Ok(())
}

async fn handle_unshare(ctx: &CliContext<'_>, args: UnshareArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    let selector = args.path.to_string_lossy();
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let namespace = resolve_namespace_via_daemon(&client, &selector).await?;
        let response = client
            .send(IpcRequest::new(IpcCommand::Unshare {
                namespace,
                blob: args.blob.clone(),
                writable: args.write,
            }))
            .await?;
        return print_daemon_message(response, output_json);
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let namespace = manager.resolve_namespace(&selector).await?;
    if let Some(blob) = &args.blob {
        let hash = blob.parse()?;
        let folder = manager.get(namespace).await?;
        folder.unpublish_blob(hash).await?;
        if output_json {
            println!(
                "{}",
                serde_json::json!({"status": "unshared", "namespace": namespace, "blob": blob})
            );
        } else {
            println!("unshared: {namespace} (blob {blob})");
        }
        node.stop().await?;
        return Ok(());
    }
    let access = if args.write { "write" } else { "read" };
    open_node_db(data_dir)?.remove_share(&namespace.to_string(), access)?;
    if let Ok(folder) = manager.get(namespace).await {
        let _ = folder.unpin_all_content().await;
    }
    if output_json {
        println!(
            "{}",
            serde_json::json!({"status": "unshared", "namespace": namespace, "access": access})
        );
    } else {
        println!("unshared: {namespace} ({access})");
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_package_add(
    ctx: &CliContext<'_>,
    node_db: &NodeDatabase,
    paths: Vec<PathBuf>,
    version: String,
    package_name: Option<String>,
    root_override: Option<PathBuf>,
) -> Result<()> {
    let output_json = ctx.output_json;
    for path in &paths {
        if !path.is_file() {
            std::fs::create_dir_all(path)?;
        }
    }
    let (root, entries) = scan_collection_entries(&paths, root_override)?;
    let source = root.to_string_lossy();
    let existing = node_db.load_workspace_manifest(&source)?;
    let mut manifest = if let Some(bytes) = existing {
        CollectionManifest::from_bytes(bytes)?
    } else {
        let mut created = CollectionManifest::new(uuid::Uuid::new_v4(), version);
        if let Some(name) = package_name {
            created.package = Some(syncweb_core::folder::PackageProfile::new(name));
        }
        created
    };
    manifest.entries = entries;
    node_db.save_workspace_manifest(&source, &manifest.to_bytes()?, &manifest.collection_id.to_string())?;
    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "collection": manifest.collection_id.to_string(),
                "root": source,
                "entries": manifest.entries.len(),
            })
        );
    } else {
        println!("collection: {}", manifest.collection_id);
        println!("root: {source}");
        println!("entries: {}", manifest.entries.len());
    }
    Ok(())
}

fn handle_package_bump(
    ctx: &CliContext<'_>,
    node_db: &NodeDatabase,
    path: &PathBuf,
    version: String,
    changelog: Option<String>,
) -> Result<()> {
    let output_json = ctx.output_json;
    let root = resolve_package_root(std::slice::from_ref(path), None);
    let source = root.to_string_lossy();
    let manifest_bytes = node_db
        .load_workspace_manifest(&source)?
        .ok_or_else(|| anyhow::anyhow!("no workspace manifest found at root {source}; run `package add` first"))?;
    let mut manifest = CollectionManifest::from_bytes(manifest_bytes)?;
    let parent = manifest.blob_id()?;
    manifest.parent = Some(parent);
    manifest.version = version;
    manifest.changelog = changelog;
    node_db.save_workspace_manifest(&source, &manifest.to_bytes()?, &manifest.collection_id.to_string())?;
    if output_json {
        println!("{}", serde_json::json!({"version": manifest.version}));
    } else {
        println!("version: {}", manifest.version);
    }
    Ok(())
}

async fn handle_package_publish(
    ctx: &CliContext<'_>,
    node_db: &NodeDatabase,
    paths: Vec<PathBuf>,
    namespace: Option<String>,
    sequence: u64,
    bootstrap: Vec<String>,
    root_override: Option<PathBuf>,
) -> Result<()> {
    let root = resolve_package_root(&paths, root_override);
    let resolved_namespace = if let Some(provided) = namespace {
        provided
    } else {
        let node = open_node(ctx.data_dir).await?;
        let manager = syncweb_core::folder::FolderManager::new(&node);
        let resolved = manager.resolve_namespace(".").await?;
        node.stop().await?;
        resolved.to_string()
    };
    handle_collection_publish(ctx, node_db, root, resolved_namespace, sequence, bootstrap).await
}

async fn handle_collection_publish(
    ctx: &CliContext<'_>,
    node_db: &NodeDatabase,
    root: std::path::PathBuf,
    namespace: String,
    sequence: u64,
    _bootstrap: Vec<String>,
) -> Result<()> {
    let (data_dir, output_json, no_daemon) = (ctx.data_dir, ctx.output_json, ctx.no_daemon);
    let source = root.to_string_lossy();
    let manifest_bytes = node_db
        .load_workspace_manifest(&source)?
        .ok_or_else(|| anyhow::anyhow!("no workspace manifest found at root {source}; run `package add` first"))?;
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let response = client
            .send(IpcRequest::new(IpcCommand::CollectionPublish {
                path: root,
                namespace,
                sequence,
                bootstrap: Vec::new(),
                manifest_bytes: Some(manifest_bytes.clone()),
            }))
            .await?;
        return print_daemon_message(response, output_json);
    }
    let manifest = syncweb_core::folder::CollectionManifest::from_bytes(manifest_bytes)?;
    let node = open_node(data_dir).await?;
    for entry in &manifest.entries {
        let hash = node.blob_store().add_file(root.join(&entry.logical_path)).await?;
        if hash != entry.content_id {
            anyhow::bail!(
                "collection content changed while publishing: {}",
                entry.logical_path.display()
            );
        }
    }
    let manager = syncweb_core::folder::FolderManager::new(&node);
    let folder = manager.get(namespace.parse()?).await?;
    let store = syncweb_core::folder::CollectionStore::for_node(&node, &folder);
    let head = store.publish(&manifest, sequence).await?;
    let manifest_ticket = node.blob_store().ticket(node.endpoint(), head.manifest);
    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "manifest": head.manifest.to_string(),
                "manifest_ticket": manifest_ticket.to_string(),
                "sequence": head.sequence,
            })
        );
    } else {
        println!("manifest: {}", head.manifest);
        println!("manifest_ticket: {manifest_ticket}");
        println!("sequence: {}", head.sequence);
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_package(ctx: &CliContext<'_>, command: PackageCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let node_db = open_node_db(data_dir)?;
    let packages = PackageManager::new(data_dir.join("packages"), node_db.clone());
    match command {
        PackageCommand::Add {
            paths,
            version,
            name,
            root,
        } => handle_package_add(ctx, &node_db, paths, version, name, root).await?,
        PackageCommand::Bump {
            path,
            version,
            changelog,
        } => handle_package_bump(ctx, &node_db, &path, version, changelog)?,
        PackageCommand::Publish {
            paths,
            namespace,
            sequence,
            bootstrap,
            root,
        } => handle_package_publish(ctx, &node_db, paths, namespace, sequence, bootstrap, root).await?,
        PackageCommand::Export { paths, version, filter } => {
            handle_package_archive_export(ctx, paths, version, filter).await?;
        }
        PackageCommand::Import { archives, filter } => {
            for archive in archives {
                handle_package_archive_import(ctx, archive, filter.clone()).await?;
            }
        }
        PackageCommand::Info { ticket, hash, node_id } => handle_package_info(ctx, ticket, hash, node_id).await?,
        PackageCommand::Install { ticket, path } | PackageCommand::Upgrade { ticket, path } => {
            handle_package_install(ctx, &packages, ticket, path).await?;
        }
        PackageCommand::Remove {
            collection: collection_id,
            version,
        } => handle_package_remove(&packages, &collection_id, &version, output_json)?,
        PackageCommand::Verify { collection, version } => {
            handle_package_verify(&packages, &collection, version.as_deref(), output_json)?;
        }
        PackageCommand::List => handle_package_list(&packages, output_json)?,
        PackageCommand::Versions {
            collection: collection_id,
        } => handle_package_versions(&packages, &collection_id, output_json)?,
        PackageCommand::Switch {
            collection: collection_id,
            version,
        } => handle_package_switch(&packages, &collection_id, &version, output_json)?,
    }
    Ok(())
}

async fn handle_package_info(
    ctx: &CliContext<'_>,
    ticket_value: Option<String>,
    hash: Option<String>,
    node_id: Option<String>,
) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let node = open_node(data_dir).await?;
    let collection_manifest = if let Some(ticket_text) = ticket_value {
        let blob_ticket = ticket_text.parse::<iroh_blobs::ticket::BlobTicket>()?;
        if !node.blob_store().has(blob_ticket.hash()).await? {
            node.blob_store().fetch(node.endpoint(), &blob_ticket).await?;
        }
        let manifest = CollectionManifest::from_bytes(node.blob_store().get(blob_ticket.hash()).await?)?;
        if manifest.blob_id()? != blob_ticket.hash() {
            node.stop().await?;
            anyhow::bail!("manifest ticket hash does not match manifest content");
        }
        manifest
    } else if let (Some(hash_str), Some(node_id_str)) = (hash, node_id) {
        let blob_hash = hash_str.parse::<iroh_blobs::Hash>()?;
        let peer_id = node_id_str.parse::<iroh::PublicKey>()?;
        let ticket = syncweb_core::node::blob_store::raw_blob_ticket(iroh::EndpointAddr::new(peer_id), blob_hash);
        if !node.blob_store().has(blob_hash).await? {
            node.blob_store().fetch(node.endpoint(), &ticket).await?;
        }
        CollectionManifest::from_bytes(node.blob_store().get(blob_hash).await?)?
    } else {
        node.stop().await?;
        anyhow::bail!("provide a blob ticket or hash with --node-id");
    };
    node.stop().await?;
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
    Ok(())
}

async fn handle_package_install(
    ctx: &CliContext<'_>,
    packages: &PackageManager,
    ticket_text: String,
    path: Option<std::path::PathBuf>,
) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let node = open_node(data_dir).await?;
    let blob_ticket = ticket_text.parse::<iroh_blobs::ticket::BlobTicket>()?;
    let _install_path = path.unwrap_or_else(|| data_dir.join("packages"));
    let collection_manifest = packages
        .install_from_ticket(&blob_ticket, node.endpoint(), node.blob_store())
        .await?;
    node.stop().await?;
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
    Ok(())
}

fn handle_package_remove(
    packages: &PackageManager,
    collection_id: &str,
    version: &str,
    output_json: bool,
) -> Result<()> {
    if !confirm_destructive("remove this package version", output_json)? {
        println!("aborted");
        return Ok(());
    }
    let collection = collection_id.parse()?;
    packages.remove(collection, version)?;
    if output_json {
        println!(
            "{}",
            serde_json::json!({"status": "removed", "collection": collection_id, "version": version})
        );
    } else {
        println!("removed: {collection} {version}");
    }
    Ok(())
}

fn handle_package_verify(
    packages: &PackageManager,
    collection: &str,
    version: Option<&str>,
    output_json: bool,
) -> Result<()> {
    let collection_id = collection.parse::<uuid::Uuid>()?;
    let state = packages.state()?;
    let installed = state
        .current(collection_id)
        .ok_or_else(|| anyhow::anyhow!("collection is not installed: {collection}"))?;
    let target_version = version.unwrap_or(&installed.current);
    if !installed.versions.contains_key(target_version) {
        anyhow::bail!("version {target_version} not found for collection {collection}");
    }
    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "status": "verified",
                "collection": collection,
                "version": target_version,
            })
        );
    } else {
        println!("verified: {collection} {target_version}");
    }
    Ok(())
}

fn handle_package_versions(packages: &PackageManager, collection_id: &str, output_json: bool) -> Result<()> {
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
    Ok(())
}

fn handle_package_switch(
    packages: &PackageManager,
    collection_id: &str,
    version: &str,
    output_json: bool,
) -> Result<()> {
    let collection = collection_id.parse()?;
    packages.switch(collection, version)?;
    if output_json {
        println!(
            "{}",
            serde_json::json!({"status": "current", "collection": collection_id, "version": version})
        );
    } else {
        println!("current: {collection} {version}");
    }
    Ok(())
}

#[async_recursion]
async fn handle_package_archive_import(
    ctx: &CliContext<'_>,
    archive: std::path::PathBuf,
    filters: Vec<String>,
) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let node_db = open_node_db(data_dir)?;
    let packages = PackageManager::new(data_dir.join("packages"), node_db);
    let filter = parse_drop_filters(&filters)?;
    let mut options = DropImportOptions::default().with_available_dependencies(packages.available_versions()?);
    if let Some(engine) = filter {
        options = options.with_filter(engine);
    }

    let node = open_node(data_dir).await?;
    let importer = DropImporter::new(node.blob_store().clone());
    let result = importer.import_archive(&archive, options, None).await?;
    let collection = result.collection_id;
    let version = result.version.clone();
    if packages
        .state()?
        .current(collection)
        .is_some_and(|installed| installed.versions.contains_key(&version))
    {
        node.stop().await?;
        anyhow::bail!("collection version {collection} {version} is already installed");
    }

    let source = data_dir.join(format!(
        "{}{}",
        syncweb_core::constants::DROP_SOURCE_STAGING_PREFIX,
        uuid::Uuid::new_v4()
    ));
    let materialize_result = importer.materialize(&result, &source).await;
    if let Err(error) = materialize_result {
        node.stop().await?;
        return Err(error.into());
    }
    let install_result = packages.install(&result.collection_manifest, &source);
    let cleanup_result = std::fs::remove_dir_all(&source)
        .map_err(|error| anyhow::anyhow!("failed to remove temporary drop source: {error}"));
    if let Err(error) = install_result {
        let _ = cleanup_result;
        node.stop().await?;
        return Err(error.into());
    }
    if let Err(error) = cleanup_result {
        node.stop().await?;
        return Err(error);
    }

    let folder = FolderManager::new(&node).create(SyncMode::SendReceive).await?;
    CollectionStore::for_node(&node, &folder)
        .publish(&result.collection_manifest, 1)
        .await?;
    let namespace = folder.namespace_id();
    node.stop().await?;

    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "status": "imported",
                "collection": collection.to_string(),
                "version": version,
                "manifest": result.manifest.to_string(),
                "entries": result.imported_entry_count,
                "skipped_entries": result.skipped_entry_count,
                "namespace": namespace,
            })
        );
    } else {
        println!(
            "imported: {collection} {version} ({} entries)",
            result.imported_entry_count
        );
    }
    Ok(())
}

#[async_recursion]
async fn handle_package_archive_export(
    ctx: &CliContext<'_>,
    paths: Vec<std::path::PathBuf>,
    version: Option<String>,
    filters: Vec<String>,
) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
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
        let entries = scan_single_root_entries(&source)?;
        let collection_id = uuid::Uuid::new_v4();
        let mut manifest =
            CollectionManifest::new(collection_id, version.clone().unwrap_or_else(|| "1.0.0".to_owned()));
        manifest.entries = entries;
        add_drop_content(&node, &manifest, &source).await?;
        let manifests = vec![manifest];
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

#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SearchResult {
    Catalog {
        title: String,
        collection: String,
        hash: String,
        size: u64,
        publisher: String,
        tags: Vec<String>,
    },
    Package {
        name: String,
        version: String,
        collection: String,
        manifest: String,
    },
}

impl SearchResult {
    fn catalog(record: syncweb_core::indexing::CatalogRecord) -> Self {
        Self::Catalog {
            title: record.title,
            collection: record.folder_name,
            hash: record.hash.to_string(),
            size: record.size,
            publisher: record.publisher,
            tags: record.tags,
        }
    }

    fn package(name: &str, version: &str, collection: &str, manifest: &str) -> Self {
        Self::Package {
            name: name.to_string(),
            version: version.to_string(),
            collection: collection.to_string(),
            manifest: manifest.to_string(),
        }
    }
}

fn apply_limit(results: &mut Vec<SearchResult>, limit: usize) {
    results.truncate(limit);
}

async fn handle_search(ctx: &CliContext<'_>, args: SearchArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let query = args.query.as_deref().unwrap_or("");
    let mut all_results: Vec<SearchResult> = Vec::new();

    // Local catalog index (FTS over subscribed catalog docs).
    if matches!(args.kind, SearchKind::All | SearchKind::Catalog)
        && let Ok(indexing) = syncweb_core::indexing::IndexingService::new(data_dir.join("indexing.sqlite"))
        && let Ok(records) = indexing.search(query, args.limit)
    {
        all_results.extend(records.into_iter().map(SearchResult::catalog));
    }

    // Installed packages.
    if matches!(args.kind, SearchKind::All | SearchKind::Package)
        && let Ok(node_db) = open_node_db(data_dir)
    {
        let packages = PackageManager::new(data_dir.join("packages"), node_db);
        if let Ok(state) = packages.state() {
            for (collection, installed) in state.installed {
                let line = format!("{collection}\t{}", installed.current);
                if query.is_empty() || line.contains(query) {
                    all_results.push(SearchResult::package(
                        "",
                        &installed.current,
                        &collection.to_string(),
                        "",
                    ));
                }
            }
        }
    }

    let needs_node = matches!(args.kind, SearchKind::All | SearchKind::Package | SearchKind::Channel)
        && (matches!(args.kind, SearchKind::Package | SearchKind::Channel) || args.channel.is_some());

    if needs_node {
        let node = open_node(data_dir).await?;

        // Editorial channel via catalog-backed persistence when configured.
        let config_path = data_dir.join("config.toml");
        if let Some(ref channel_name) = args.channel
            && let Ok(app_config) = AppConfig::load(&config_path)
            && app_config.channels.contains_key(channel_name)
        {
            let indexing = syncweb_core::indexing::IndexingService::new(data_dir.join("indexing.sqlite"))?;
            let author = node.docs_engine().author().await?;
            let catalog_service = indexing.catalog_service(node.docs_engine(), node.blob_store(), author);
            if let Ok(records) = catalog_service.search(query, 100)
                && let Ok(Some(namespace_id)) = indexing.database().get_channel_namespace(channel_name)
            {
                for record in records.into_iter().filter(|r| r.catalog_namespace_id == namespace_id) {
                    all_results.push(SearchResult::package(
                        &record.title,
                        "",
                        &record.folder_name,
                        &record.hash.to_string(),
                    ));
                }
            }
        }
        node.stop().await?;
    }

    apply_limit(&mut all_results, args.limit);

    if output_json {
        println!("{}", serde_json::to_string_pretty(&all_results)?);
        return Ok(());
    }

    if all_results.is_empty() {
        println!("no results found for query: {query}");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header([
        "Kind",
        "Title",
        "Name",
        "Version",
        "Collection",
        "Hash/Manifest",
        "Size",
    ]);
    for r in &all_results {
        let (kind, title, name, version, collection, id, size) = match r {
            SearchResult::Catalog {
                title,
                collection,
                hash,
                size,
                ..
            } => (
                "catalog",
                title.as_str(),
                title.as_str(),
                "",
                collection.as_str(),
                hash.as_str(),
                *size,
            ),
            SearchResult::Package {
                name,
                version,
                collection,
                manifest,
                ..
            } => (
                "package",
                name.as_str(),
                name.as_str(),
                version.as_str(),
                collection.as_str(),
                manifest.as_str(),
                0_u64,
            ),
        };
        table.add_row([kind, title, name, version, collection, id, &size.to_string()]);
    }
    println!("{table}");
    Ok(())
}

fn handle_db(ctx: &CliContext<'_>, command: cli::commands::DbCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let node_db = match NodeDatabase::open(data_dir.join("node.db")) {
        Ok(db) => db,
        Err(error) => {
            eprintln!("Failed to open node.db: {error}");
            return Ok(());
        }
    };
    let stats_db = match StatsDatabase::open(data_dir.join("stats.db")) {
        Ok(db) => db,
        Err(error) => {
            eprintln!("Failed to open stats.db: {error}");
            return Ok(());
        }
    };
    match command {
        cli::commands::DbCommand::Check => {
            let node_errors = node_db.check_integrity()?;
            let stats_errors = stats_db.check_integrity()?;
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "node_db": node_errors,
                        "stats_db": stats_errors,
                    })
                );
            } else {
                println!("Integrity check:");
                println!("  node.db:  {} errors", node_errors.len());
                for err in &node_errors {
                    println!("    - {err}");
                }
                println!("  stats.db: {} errors", stats_errors.len());
                for err in &stats_errors {
                    println!("    - {err}");
                }
                if node_errors.is_empty() && stats_errors.is_empty() {
                    println!("  all databases healthy!");
                }
            }
        }
        cli::commands::DbCommand::Vacuum => {
            let node_size_before = node_db.size_on_disk().ok();
            let stats_size_before = stats_db.size_on_disk().ok();
            let node_freelist = node_db.freelist_count()?;
            let stats_freelist = stats_db.freelist_count()?;
            node_db.vacuum()?;
            stats_db.vacuum()?;
            let node_size_after = node_db.size_on_disk().ok();
            let stats_size_after = stats_db.size_on_disk().ok();
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "node_db": {
                            "freelist_before": node_freelist,
                            "size_before": node_size_before,
                            "size_after": node_size_after,
                        },
                        "stats_db": {
                            "freelist_before": stats_freelist,
                            "size_before": stats_size_before,
                            "size_after": stats_size_after,
                        },
                    })
                );
            } else {
                println!("VACUUM complete:");
                println!(
                    "  node.db:  {node_freelist} freelist pages → {} bytes",
                    node_size_after.unwrap_or(0)
                );
                println!(
                    "  stats.db: {stats_freelist} freelist pages → {} bytes",
                    stats_size_after.unwrap_or(0)
                );
            }
        }
        cli::commands::DbCommand::Stats => {
            let node_size = node_db.size_on_disk().unwrap_or(0);
            let stats_size = stats_db.size_on_disk().unwrap_or(0);
            let node_freelist = node_db.freelist_count().unwrap_or(0);
            let stats_freelist = stats_db.freelist_count().unwrap_or(0);
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "node_db": {
                            "size_bytes": node_size,
                            "freelist_pages": node_freelist,
                        },
                        "stats_db": {
                            "size_bytes": stats_size,
                            "freelist_pages": stats_freelist,
                        },
                    })
                );
            } else {
                let mut table = Table::new();
                table.set_header(["Database", "Size", "Freelist Pages"]);
                table.add_row(["node.db", &format_bytes(node_size), &node_freelist.to_string()]);
                table.add_row(["stats.db", &format_bytes(stats_size), &stats_freelist.to_string()]);
                println!("{table}");
            }
        }
        cli::commands::DbCommand::Backup { output } => {
            handle_db_backup(data_dir, &output, output_json)?;
        }
    }
    Ok(())
}

fn handle_db_backup(data_dir: &std::path::Path, output: &std::path::Path, output_json: bool) -> Result<()> {
    let backup_dir = output.join(format!(
        "syncweb-db-backup-{}",
        syncweb_core::parsing::current_unix_secs()
    ));
    std::fs::create_dir_all(&backup_dir)?;
    let node_path = data_dir.join("node.db");
    let stats_path = data_dir.join("stats.db");
    if node_path.exists() {
        std::fs::copy(&node_path, backup_dir.join("node.db"))?;
    }
    if stats_path.exists() {
        std::fs::copy(&stats_path, backup_dir.join("stats.db"))?;
    }
    if output_json {
        println!(
            "{}",
            serde_json::json!({
                "path": backup_dir.to_string_lossy(),
                "files": ["node.db", "stats.db"],
            })
        );
    } else {
        println!("Backup created at: {}", backup_dir.display());
    }
    Ok(())
}

fn scan_single_root_entries(path: &std::path::Path) -> Result<Vec<CollectionEntry>> {
    ParallelScanner::new(path, vec![], 0)
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

fn resolve_package_root(paths: &[PathBuf], root_override: Option<PathBuf>) -> PathBuf {
    if let Some(root) = root_override {
        return root;
    }
    if paths.len() == 1 {
        let parent = paths
            .first()
            .and_then(|p| p.parent())
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        return parent.to_path_buf();
    }
    longest_common_ancestor(paths)
}

fn longest_common_ancestor(paths: &[PathBuf]) -> PathBuf {
    let Some((first, rest)) = paths.split_first() else {
        return PathBuf::new();
    };
    let mut common: Vec<_> = first.components().collect();
    for path in rest {
        let comps: Vec<_> = path.components().collect();
        let shared = common.iter().zip(comps.iter()).take_while(|(a, b)| a == b).count();
        common.truncate(shared);
    }
    common.into_iter().collect()
}

fn scan_collection_entries(
    paths: &[PathBuf],
    root_override: Option<PathBuf>,
) -> Result<(PathBuf, Vec<CollectionEntry>)> {
    let root = resolve_package_root(paths, root_override);
    let mut entries: Vec<CollectionEntry> = Vec::new();
    for input in paths {
        let rel = input.strip_prefix(&root).unwrap_or(input.as_path()).to_path_buf();
        let is_file = input.is_file();
        let scanned = ParallelScanner::new(input, vec![], 0).scan()?;
        for entry in scanned {
            if entry.file_type != FileType::File {
                continue;
            }
            let logical = if is_file {
                rel.clone()
            } else {
                rel.join(&entry.relative_path)
            };
            entries.push(
                CollectionEntry::new(
                    iroh_blobs::Hash::from_bytes(*entry.hash.as_bytes()),
                    logical,
                    entry.size,
                )
                .map_err(anyhow::Error::from)?,
            );
        }
    }
    entries.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    let mut deduped: Vec<CollectionEntry> = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Some(last) = deduped.last()
            && last.logical_path == entry.logical_path
        {
            if last.content_id != entry.content_id {
                anyhow::bail!(
                    "conflicting inputs resolve to the same logical path: {}",
                    entry.logical_path.display()
                );
            }
            continue;
        }
        deduped.push(entry);
    }
    Ok((root, deduped))
}

#[async_recursion]
async fn handle_network(ctx: &CliContext<'_>, command: NetworkCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
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
            if let Some(network) = manager.get(&id).cloned()
                && let Err(error) = syncweb_core::net::membership::provision(data_dir, &network, &mut manager).await
            {
                tracing::warn!(%error, "network created without a membership doc (doc_ticket)");
            }
            if output_json {
                println!(
                    "{}",
                    serde_json::json!({"status": "created", "name": name, "id": id.to_string()})
                );
            } else {
                println!("created: {name}\t{id}");
            }
        }
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
            if !confirm_destructive("leave this network", output_json)? {
                println!("aborted");
                return Ok(());
            }
            let id = network_id_by_name(&manager, &name)?;
            manager.leave(id)?;
            if output_json {
                println!("{}", serde_json::json!({"status": "left", "name": name}));
            } else {
                println!("left: {name}");
            }
        }
        NetworkCommand::List { name } => handle_network_list(&manager, name, output_json)?,
        NetworkCommand::Invite { name, device } => {
            let id = network_id_by_name(&manager, &name)?;
            let ticket = if let Some(node_id) = device {
                manager.invite(id, node_id.parse()?)?
            } else {
                manager.invite_any(id)?
            };
            if let Some(network) = manager.get(&id).cloned()
                && network.doc_ticket.is_some()
                && let Err(error) = syncweb_core::net::membership::refresh(data_dir, &network).await
            {
                tracing::warn!(%error, "failed to refresh network membership doc");
            }
            if output_json {
                println!("{}", serde_json::json!({"ticket": ticket.to_string()}));
            } else {
                println!("{ticket}");
            }
        }
        NetworkCommand::Kick { name, device } => {
            if !confirm_destructive("kick this device from the network", output_json)? {
                println!("aborted");
                return Ok(());
            }
            let id = network_id_by_name(&manager, &name)?;
            manager.kick(id, &device.parse()?)?;
            if let Some(network) = manager.get(&id).cloned()
                && network.doc_ticket.is_some()
                && let Err(error) = syncweb_core::net::membership::refresh(data_dir, &network).await
            {
                tracing::warn!(%error, "failed to refresh network membership doc");
            }
            if output_json {
                println!("{}", serde_json::json!({"status": "kicked", "device": device}));
            } else {
                println!("kicked: {device}");
            }
        }
        NetworkCommand::Events { network_id, limit } => {
            handle_network_events(data_dir, &network_id, limit, output_json)?;
        }
        NetworkCommand::TestRelay { relay_url } => {
            let identity = IdentityManager::new(data_dir.join("identity.key"))?;
            let app_config = open_node_db(data_dir)?.load_app_config()?;
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

fn handle_network_events(data_dir: &std::path::Path, network_id: &str, limit: usize, output_json: bool) -> Result<()> {
    let stats_db = open_stats_db(data_dir)?;
    let events = stats_db.recent_network_events(network_id, limit)?;
    if output_json {
        let json_events: Vec<_> = events
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "timestamp": e.timestamp,
                    "network_id": e.network_id,
                    "event_type": e.event_type,
                    "peer": e.peer,
                    "details": e.details,
                })
            })
            .collect();
        println!("{}", serde_json::to_string(&json_events)?);
    } else {
        println!("Events for network {network_id}:");
        for event in &events {
            println!(
                "  [{}] {} {}",
                event.timestamp,
                event.event_type,
                event.details.as_deref().unwrap_or("")
            );
        }
        if events.is_empty() {
            println!("  no events");
        }
    }
    Ok(())
}

fn handle_network_health(
    data_dir: &std::path::Path,
    manager: &NetworkManager,
    network: Option<String>,
    output_json: bool,
) -> Result<()> {
    let stats_db = open_stats_db(data_dir)?;
    if let Some(network_id) = network {
        let events = stats_db.recent_network_events(&network_id, 100)?;
        let sessions = stats_db.recent_sync_sessions(&network_id, 100)?;
        if output_json {
            println!(
                "{}",
                serde_json::json!({
                    "network_id": network_id,
                    "events": events.len(),
                    "sessions": sessions.len(),
                })
            );
        } else {
            println!("Network: {network_id}");
            println!("  events:   {}", events.len());
            println!("  sessions: {}", sessions.len());
        }
    } else {
        let networks = manager.list();
        if output_json {
            let summary: Vec<_> = networks
                .iter()
                .map(|n| {
                    let id = n.id.to_string();
                    let events = stats_db.recent_network_events(&id, 1).unwrap_or_default();
                    serde_json::json!({
                        "name": n.name,
                        "id": id,
                        "member_count": n.members.len(),
                        "folder_count": n.folders.len(),
                        "last_event": events.first().map(|e| e.timestamp),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string(&summary)?);
        } else {
            for net in &networks {
                let id = net.id.to_string();
                let events = stats_db.recent_network_events(&id, 1).unwrap_or_default();
                let last_event = events
                    .first()
                    .map_or_else(|| "never".to_owned(), |e| e.timestamp.to_string());
                println!(
                    "{}  members={}  folders={}  last_event={last_event}",
                    net.name,
                    net.members.len(),
                    net.folders.len(),
                );
            }
        }
    }
    Ok(())
}

fn handle_status_networks(data_dir: &std::path::Path, name: Option<&str>, output_json: bool) -> Result<()> {
    let manager = open_network_manager(data_dir)?;
    if let Some(selected) = name {
        let network = manager
            .get_by_name(selected)
            .or_else(|| manager.list().into_iter().find(|n| n.id.to_string() == selected))
            .with_context(|| format!("network not found: {selected}"))?;
        let network_name = network.name.clone();
        let network_id = network.id.to_string();
        handle_network_list(&manager, Some(network_name), output_json)?;
        handle_network_health(data_dir, &manager, Some(network_id), output_json)?;
    } else {
        handle_network_health(data_dir, &manager, None, output_json)?;
    }
    Ok(())
}

fn open_network_manager(data_dir: &std::path::Path) -> Result<NetworkManager> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let db = open_node_db(data_dir)?;
    let empty_keys = std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashSet::new()));
    let logger = NetworkLogger::new(open_stats_db(data_dir)?);
    Ok(NetworkManager::with_logger(db, identity.node_id(), logger, empty_keys)?)
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
async fn handle_leave(ctx: &CliContext<'_>, command: crate::cli::commands::LeaveArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let namespace = resolve_namespace_via_daemon(&client, &command.folder).await?;
        let response = client
            .send(IpcRequest::new(IpcCommand::LeaveFolder {
                namespace,
                delete_files: command.delete_files,
            }))
            .await?;
        return print_daemon_message(response, output_json);
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let namespace = manager.resolve_namespace(&command.folder).await?;
    let _ = cancel_session(namespace);
    manager.drop_when_ready(namespace).await?;
    let namespace_str = namespace.to_string();
    let node_db = open_node_db(data_dir)?;
    node_db.remove_folder_mount(&namespace_str)?;
    if command.delete_files {
        let path = std::path::Path::new(&command.folder);
        if path.exists() {
            FolderManager::delete_folder_files(path).await?;
        }
    }
    if output_json {
        println!(
            "{}",
            serde_json::json!({"status": "left", "namespace": namespace.to_string()})
        );
    } else {
        println!("left: {namespace}");
    }
    node.stop().await?;
    Ok(())
}

#[async_recursion]
async fn handle_folders(ctx: &CliContext<'_>) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let no_daemon = ctx.no_daemon;
    if let Some(client) = daemon_client_or_start(data_dir, no_daemon, ctx.network).await? {
        let response = client.send(IpcRequest::new(IpcCommand::ListFolders)).await?;
        match response {
            IpcResponse::FolderList(folders) => {
                if output_json {
                    println!("{}", serde_json::to_string_pretty(&folders)?);
                } else {
                    let mut table = Table::new();
                    table.set_header(["Namespace", "Path", "Active", "Last Sync", "Entries", "Errors"]);
                    for folder in &folders {
                        let active_label = if folder.session_active { "yes" } else { "no" };
                        table.add_row([
                            &folder.namespace,
                            &folder.path.display().to_string(),
                            &active_label.to_string(),
                            &folder.last_sync_at.map_or_else(|| "-".to_owned(), |v| v.to_string()),
                            &folder.entries_synced.to_string(),
                            &folder.errors.len().to_string(),
                        ]);
                    }
                    println!("{table}");
                }
                return Ok(());
            }
            IpcResponse::Ok { .. }
            | IpcResponse::Status(_)
            | IpcResponse::DownloadComplete { .. }
            | IpcResponse::ImportFilesComplete { .. }
            | IpcResponse::ImportComplete(_)
            | IpcResponse::ExportComplete(_)
            | IpcResponse::Error { .. }
            | _ => return print_daemon_message(response, output_json),
        }
    }
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folders = manager.list().await?;
    if folders.is_empty() {
        if output_json {
            println!("[]");
        } else {
            println!("No folders found. Create one with `syncweb create [path]`");
        }
        node.stop().await?;
        return Ok(());
    }
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

fn handle_devices(ctx: &CliContext<'_>) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
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

fn handle_networks(ctx: &CliContext<'_>, args: &NetworkListArgs) -> Result<()> {
    handle_status_networks(ctx.data_dir, args.name.as_deref(), ctx.output_json)
}

// ---------------------------------------------------------------------------
// Metadata-first `ls`/`find`/`sort`
// ---------------------------------------------------------------------------

/// A single folder entry with disk metadata overlaid when the blob is local.
#[derive(Clone, Debug)]
struct LocalEntry {
    path: String,
    hash: BlobHash,
    size: u64,
    local: bool,
    modified: Option<SystemTime>,
}

/// How a `ls`/`find`/`sort` selector resolved to a folder.
struct ResolvedFolder {
    namespace: String,
    mount_root: Option<PathBuf>,
    remainder: PathBuf,
}

/// The entry source for a listing: either the daemon (IPC) or an embedded node.
enum ListingMode {
    Daemon { client: IpcClient },
    Embedded(Box<EmbeddedListing>),
}

/// An embedded node + its resolved folder, boxed to keep the listing mode small.
struct EmbeddedListing {
    node: IrohNode,
    folder: SyncwebFolder,
}

/// Sort vocabulary for the metadata table (step 3 of plan 01).
#[derive(Clone, Copy, Debug)]
enum MetaSort {
    Name,
    Size,
    Modified,
    State,
}

fn parse_meta_sort(by: &str) -> Result<MetaSort> {
    match by.to_ascii_lowercase().as_str() {
        "name" => Ok(MetaSort::Name),
        "size" => Ok(MetaSort::Size),
        "modified" => Ok(MetaSort::Modified),
        "state" => Ok(MetaSort::State),
        other => anyhow::bail!(
            "sort --by '{other}' is only valid with --local-only; on a synchronized folder \
             use name, size, modified, or state"
        ),
    }
}

/// The metadata listing's shared entry walk: read doc entries, compute the
/// local/remote `State`, and overlay the real disk `size`/`modified` on local
/// rows. Never materializes blobs and never scans the directory.
async fn enumerate_folder_entries(
    folder: &SyncwebFolder,
    mount_root: Option<&Path>,
    enrich: bool,
) -> Result<Vec<LocalEntry>> {
    let entries = folder.list_entries().await?;
    let mut rows = Vec::with_capacity(entries.len());
    for entry in entries {
        let local = folder.has_local(entry.hash).await?;
        let (size, modified) = if enrich
            && local
            && let Some(root) = mount_root
            && let Ok(metadata) = std::fs::metadata(root.join(&entry.path))
        {
            (metadata.len(), metadata.modified().ok())
        } else {
            (entry.size, None)
        };
        rows.push(LocalEntry {
            path: entry.path,
            hash: entry.hash,
            size,
            local,
            modified,
        });
    }
    Ok(rows)
}

fn entry_row_to_local(row: EntryRow) -> LocalEntry {
    LocalEntry {
        path: row.path,
        hash: row.hash,
        size: row.size,
        local: row.local,
        modified: row.modified.map(|seconds| {
            UNIX_EPOCH
                .checked_add(Duration::from_secs(seconds))
                .unwrap_or(UNIX_EPOCH)
        }),
    }
}

/// Fetch the entry rows for a resolved folder, stopping the embedded node when
/// one was opened.
async fn fetch_listing_rows(mode: ListingMode, resolved: &ResolvedFolder, enrich: bool) -> Result<Vec<LocalEntry>> {
    match mode {
        ListingMode::Daemon { client } => {
            let response = client
                .send(IpcRequest::new(IpcCommand::ListEntries {
                    folder: resolved.namespace.clone(),
                    enrich,
                }))
                .await?;
            let IpcResponse::Entries(rows) = response else {
                anyhow::bail!("daemon returned an unexpected response while listing entries");
            };
            Ok(rows.into_iter().map(entry_row_to_local).collect())
        }
        ListingMode::Embedded(listing) => {
            let rows = enumerate_folder_entries(&listing.folder, resolved.mount_root.as_deref(), enrich).await?;
            listing.node.stop().await?;
            Ok(rows)
        }
    }
}

fn canonical_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        }
    })
}

/// Registered mount paths for live folders only (stale rows never resolve).
async fn live_folder_mounts(data_dir: &Path, manager: &FolderManager) -> Result<Vec<(String, PathBuf)>> {
    let live: std::collections::HashSet<String> = manager
        .list()
        .await?
        .into_iter()
        .map(|folder| folder.namespace_id().to_string())
        .collect();
    let node_db = open_node_db(data_dir)?;
    Ok(node_db
        .load_folder_mounts()?
        .into_iter()
        .filter(|(namespace, _)| live.contains(namespace))
        .collect())
}

async fn resolve_selector_embedded(
    data_dir: &Path,
    manager: &FolderManager,
    selector: &Path,
) -> Result<(ResolvedFolder, SyncwebFolder)> {
    let selector_str = selector.to_string_lossy();
    if let Ok(namespace_id) = selector_str.parse::<iroh_docs::NamespaceId>() {
        let folder = manager.get(namespace_id).await?;
        let mount_root = live_folder_mounts(data_dir, manager)
            .await?
            .into_iter()
            .find_map(|(namespace, path)| (namespace == selector_str).then_some(path));
        return Ok((
            ResolvedFolder {
                namespace: namespace_id.to_string(),
                mount_root,
                remainder: PathBuf::new(),
            },
            folder,
        ));
    }
    let mounts = live_folder_mounts(data_dir, manager).await?;
    let canonical = canonical_path(selector);
    let mut best: Option<(PathBuf, PathBuf)> = None;
    for (_, root) in &mounts {
        let root_canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.clone());
        if let Ok(remainder) = canonical.strip_prefix(&root_canonical) {
            let better = best
                .as_ref()
                .is_none_or(|(best_root, _)| root_canonical.as_os_str().len() > best_root.as_os_str().len());
            if better {
                best = Some((root_canonical, remainder.to_path_buf()));
            }
        }
    }
    if let Some((root, remainder)) = best {
        let Some(namespace) = mounts
            .iter()
            .find(|(_, mount)| std::fs::canonicalize(mount).unwrap_or_else(|_| mount.clone()) == root)
            .map(|(namespace, _)| namespace.clone())
        else {
            anyhow::bail!("folder mount root resolved to a departed folder");
        };
        let namespace_id = namespace
            .parse::<iroh_docs::NamespaceId>()
            .map_err(|error| anyhow::anyhow!("invalid stored namespace {namespace:?}: {error}"))?;
        let folder = manager.get(namespace_id).await?;
        return Ok((
            ResolvedFolder {
                namespace,
                mount_root: Some(root),
                remainder,
            },
            folder,
        ));
    }
    anyhow::bail!(
        "{} is not inside of a Syncweb folder — run `syncweb ls --local-only {}` to list the disk directly",
        selector.display(),
        selector.display()
    );
}

async fn resolve_selector_via_daemon(client: &IpcClient, selector: &Path) -> Result<ResolvedFolder> {
    let response = client.send(IpcRequest::new(IpcCommand::ListFolders)).await?;
    let IpcResponse::FolderList(folders) = response else {
        anyhow::bail!("unexpected response from daemon while resolving folder");
    };
    let selector_str = selector.to_string_lossy();
    if let Ok(_namespace_id) = selector_str.parse::<iroh_docs::NamespaceId>() {
        let mount_root = folders
            .iter()
            .find(|folder| folder.namespace == selector_str)
            .map(|folder| folder.path.clone());
        return Ok(ResolvedFolder {
            namespace: selector_str.into_owned(),
            mount_root,
            remainder: PathBuf::new(),
        });
    }
    let canonical = canonical_path(selector);
    let mut best: Option<(String, PathBuf, PathBuf)> = None;
    for folder in &folders {
        if folder.path.as_os_str().is_empty() {
            continue;
        }
        let root = std::fs::canonicalize(&folder.path).unwrap_or_else(|_| folder.path.clone());
        if let Ok(remainder) = canonical.strip_prefix(&root) {
            let better = best
                .as_ref()
                .is_none_or(|(_, best_root, _)| root.as_os_str().len() > best_root.as_os_str().len());
            if better {
                best = Some((folder.namespace.clone(), root, remainder.to_path_buf()));
            }
        }
    }
    if let Some((namespace, root, remainder)) = best {
        return Ok(ResolvedFolder {
            namespace,
            mount_root: Some(root),
            remainder,
        });
    }
    if !selector.exists() {
        let matched = folders
            .iter()
            .find(|folder| folder.namespace.starts_with(&*selector_str))
            .or_else(|| folders.first().filter(|_| folders.len() == 1))
            .map(|folder| ResolvedFolder {
                namespace: folder.namespace.clone(),
                mount_root: (!folder.path.as_os_str().is_empty()).then(|| folder.path.clone()),
                remainder: PathBuf::new(),
            });
        if let Some(resolved) = matched {
            return Ok(resolved);
        }
    }
    anyhow::bail!(
        "{} is not inside of a Syncweb folder — run `syncweb ls --local-only {}` to list the disk directly",
        selector.display(),
        selector.display()
    );
}

/// Resolve a selector to a folder listing source. Uses the daemon when one is
/// running (no second embedded node is opened on the same `data_dir`); falls
/// back to an embedded node otherwise.
async fn resolve_entry_source(ctx: &CliContext<'_>, selector: &Path) -> Result<(ListingMode, ResolvedFolder)> {
    if let Some(client) = syncweb_core::daemon::daemon_client(ctx.data_dir)? {
        let resolved = resolve_selector_via_daemon(&client, selector).await?;
        return Ok((ListingMode::Daemon { client }, resolved));
    }
    let node = open_node(ctx.data_dir).await?;
    let manager = FolderManager::new(&node);
    let (resolved, folder) = resolve_selector_embedded(ctx.data_dir, &manager, selector).await?;
    Ok((
        ListingMode::Embedded(Box::new(EmbeddedListing { node, folder })),
        resolved,
    ))
}

fn entry_matches_path_filters(entry_path: &str, remainder: &Path, prefix: Option<&str>, glob: Option<&str>) -> bool {
    if !remainder.as_os_str().is_empty() {
        let remainder_str = remainder
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if entry_path != remainder_str && !entry_path.starts_with(&format!("{remainder_str}/")) {
            return false;
        }
    }
    if let Some(path_prefix) = prefix
        && entry_path != path_prefix
        && !entry_path.starts_with(&format!("{path_prefix}/"))
    {
        return false;
    }
    if let Some(glob_pattern) = glob
        && !globset::Glob::new(glob_pattern).is_ok_and(|compiled| compiled.compile_matcher().is_match(entry_path))
    {
        return false;
    }
    true
}

fn apply_listing_filters(rows: Vec<LocalEntry>, remainder: &Path, flags: &ListingFlags) -> Vec<LocalEntry> {
    rows.into_iter()
        .filter(|row| {
            entry_matches_path_filters(
                &row.path,
                remainder,
                flags.path_prefix.as_deref(),
                flags.path_glob.as_deref(),
            )
        })
        .filter(|row| !flags.remote_only || !row.local)
        .collect()
}

fn sort_local_entries(rows: &mut [LocalEntry], by: MetaSort) {
    match by {
        MetaSort::Name => rows.sort_by_key(|row| row.path.clone()),
        MetaSort::Size => rows.sort_by_key(|row| row.size),
        MetaSort::Modified => rows.sort_by(|left, right| {
            let left_key = left
                .modified
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(left.size, |duration| duration.as_secs());
            let right_key = right
                .modified
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(right.size, |duration| duration.as_secs());
            left_key.cmp(&right_key)
        }),
        MetaSort::State => rows.sort_by_key(|row| (!row.local, row.path.clone())),
    }
}

fn local_entry_json(row: &LocalEntry) -> serde_json::Value {
    serde_json::json!({
        "path": row.path,
        "size": row.size,
        "hash": row.hash.to_string(),
        "local": row.local,
        "modified": row
            .modified
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs()),
    })
}

fn render_entries(rows: &[LocalEntry], namespace: &str, selector: &Path, output_json: bool) -> Result<()> {
    if output_json {
        let entries: Vec<serde_json::Value> = rows.iter().map(local_entry_json).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "folder": namespace,
                "path": selector.display().to_string(),
                "entries": entries,
            }))?
        );
        return Ok(());
    }
    let mut table = Table::new();
    table.set_header(["Path", "Size", "Modified", "State"]);
    for row in rows {
        let modified = row
            .modified
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or_else(|| "-".to_owned(), |duration| duration.as_secs().to_string());
        table.add_row([
            row.path.as_str(),
            &format_bytes(row.size),
            &modified,
            if row.local { "local" } else { "remote" },
        ]);
    }
    println!("{table}");
    Ok(())
}

fn print_no_entries_nudge(namespace: &str) {
    println!(
        "folder {namespace} has no remote entries yet; files will appear as they sync — \
         fetch current content with `syncweb download <path>`"
    );
}

fn handle_ls_disk(path: &Path, threads: usize, output_json: bool) -> Result<()> {
    let entries = ParallelScanner::new(path, Vec::<String>::new(), threads).scan()?;
    if entries.is_empty() && !output_json {
        println!("No files found in {}", path.display());
        return Ok(());
    }
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

async fn handle_ls(ctx: &CliContext<'_>, command: crate::cli::commands::LocalPathArgs) -> Result<()> {
    let output_json = ctx.output_json;
    if command.listing.local_only {
        if let Some(criteria) = command.sort {
            let sort_args = crate::cli::commands::SortArgs {
                path: command.path,
                by: criteria,
                min_seeders: None,
                max_seeders: None,
                niche: None,
                frecency_weight: None,
                limit_size: None,
                depth: Vec::new(),
                min_depth: None,
                max_depth: None,
                threads: command.threads,
                enrich: false,
                listing: command.listing,
            };
            return handle_sort(ctx, &sort_args).await;
        }
        return handle_ls_disk(&command.path, command.threads, output_json);
    }
    let (mode, resolved) = resolve_entry_source(ctx, &command.path).await?;
    let mut rows = fetch_listing_rows(mode, &resolved, !command.listing.no_enrich).await?;
    if rows.is_empty() {
        if output_json {
            render_entries(&rows, &resolved.namespace, &command.path, output_json)?;
        } else {
            print_no_entries_nudge(&resolved.namespace);
        }
        return Ok(());
    }
    rows = apply_listing_filters(rows, &resolved.remainder, &command.listing);
    if let Some(criteria) = command.sort {
        let by = parse_meta_sort(&criteria)?;
        sort_local_entries(&mut rows, by);
    }
    render_entries(&rows, &resolved.namespace, &command.path, output_json)?;
    Ok(())
}

fn build_find_query(command: &crate::cli::commands::FindArgs) -> Result<FindQuery> {
    let mut query = match command.kind.as_str() {
        "exact" => FindQuery::exact(&command.pattern),
        "regex" => FindQuery::regex(&command.pattern),
        _ => FindQuery::glob(&command.pattern),
    };

    if command.ignore_case {
        query.case_sensitive = Some(false);
    }
    if command.case_sensitive && !command.ignore_case {
        query.case_sensitive = Some(true);
    }

    query.fixed_strings = command.fixed_strings;
    query.full_path = command.full_path;
    query.hidden = command.hidden;
    query.follow_links = command.follow_links;
    query.absolute_path = command.absolute_path;
    query.downloadable = command.downloadable;

    let (min_depth, max_depth) = syncweb_core::parsing::parse_depth_constraints(
        &command.depth,
        command.min_depth.unwrap_or(0),
        command.max_depth,
    );
    query.min_depth = Some(min_depth);
    query.max_depth = max_depth;

    let (min_size, max_size) = FindQuery::parse_size_constraints(&command.sizes)?;
    query.min_size = min_size;
    query.max_size = max_size;

    let (after, before) = FindQuery::parse_time_constraints(
        &command.modified_within,
        &command.modified_before,
        &command.time_modified,
    )?;
    query.modified_after = after;
    query.modified_before = before;

    if !command.extension.is_empty() {
        query.extensions.clone_from(&command.extension);
    }
    if let Some(ref ext) = query.extension
        && !ext.is_empty()
    {
        query.extensions.push(ext.trim_start_matches('.').to_lowercase());
    }

    query.file_type = command.file_type.clone().map(|kind| match kind.as_str() {
        "d" => FileType::Directory,
        "l" => FileType::Symlink,
        _ => FileType::File,
    });
    Ok(query)
}

fn local_entry_to_file_entry(
    entry: &LocalEntry,
    mount_root: Option<&Path>,
) -> std::result::Result<FileEntry, &'static str> {
    let relative = PathBuf::from(&entry.path);
    let path = mount_root.map_or_else(|| relative.clone(), |root| root.join(&entry.path));
    FileEntry::builder()
        .path(path)
        .relative_path(relative)
        .size(entry.size)
        .modified(entry.modified.unwrap_or(UNIX_EPOCH))
        .hash(blake3::Hash::from_bytes(*entry.hash.as_bytes()))
        .file_type(FileType::File)
        .build()
}

fn handle_find_disk(command: &crate::cli::commands::FindArgs, output_json: bool) -> Result<()> {
    let query = build_find_query(command)?;
    let entries = FindEngine::new(&command.path)
        .with_threads(command.threads)
        .find(&query)?;

    if entries.is_empty() && !output_json {
        println!(
            "No files matching '{}' found in {}",
            command.pattern,
            command.path.display()
        );
        return Ok(());
    }

    if output_json {
        let paths = entries
            .iter()
            .map(|entry| {
                if command.absolute_path {
                    entry.path.display().to_string()
                } else {
                    entry.relative_path.display().to_string()
                }
            })
            .collect::<Vec<_>>();
        println!("{}", serde_json::to_string_pretty(&paths)?);
    } else {
        for entry in entries {
            if command.absolute_path {
                println!("{}", entry.path.display());
            } else {
                println!("{}", entry.relative_path.display());
            }
        }
    }
    Ok(())
}

async fn handle_find(ctx: &CliContext<'_>, command: crate::cli::commands::FindArgs) -> Result<()> {
    let output_json = ctx.output_json;
    if command.listing.local_only {
        return handle_find_disk(&command, output_json);
    }
    let query = build_find_query(&command)?;
    let (mode, resolved) = resolve_entry_source(ctx, &command.path).await?;
    let mut rows = fetch_listing_rows(mode, &resolved, !command.listing.no_enrich).await?;
    if rows.is_empty() {
        if output_json {
            render_entries(&rows, &resolved.namespace, &command.path, output_json)?;
        } else {
            print_no_entries_nudge(&resolved.namespace);
        }
        return Ok(());
    }
    rows = apply_listing_filters(rows, &resolved.remainder, &command.listing);

    let file_entries: Vec<FileEntry> = rows
        .iter()
        .filter_map(|entry| local_entry_to_file_entry(entry, resolved.mount_root.as_deref()).ok())
        .collect();
    let matched = filter_entries(&file_entries, &query);
    let matched_rows: Vec<LocalEntry> = matched
        .iter()
        .filter_map(|file_entry| {
            let relative = file_entry.relative_path.to_string_lossy().into_owned();
            rows.iter().find(|row| row.path == relative).cloned()
        })
        .collect();

    if matched_rows.is_empty() && !output_json {
        println!(
            "No files matching '{}' found in {}",
            command.pattern,
            command.path.display()
        );
        return Ok(());
    }

    if output_json {
        let entries: Vec<serde_json::Value> = matched_rows.iter().map(local_entry_json).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "folder": resolved.namespace,
                "path": command.path.display().to_string(),
                "entries": entries,
            }))?
        );
    } else {
        for row in &matched_rows {
            if command.absolute_path {
                println!(
                    "{}",
                    resolved.mount_root.as_ref().map_or_else(
                        || row.path.clone(),
                        |root| root.join(&row.path).to_string_lossy().into_owned()
                    )
                );
            } else {
                println!("{}", row.path);
            }
        }
    }
    Ok(())
}

fn handle_sort_disk(ctx: &CliContext<'_>, command: &crate::cli::commands::SortArgs) -> Result<()> {
    let output_json = ctx.output_json;
    let data_dir = ctx.data_dir;
    let entries = ParallelScanner::new(&command.path, Vec::<String>::new(), command.threads).scan()?;
    let mut sortable: Vec<SortEntry> = entries.into_iter().map(sort_entry).collect();

    // Build sort config from CLI args
    let mut criteria = SortConfig::parse_criteria(std::slice::from_ref(&command.by));
    if criteria.is_empty() {
        criteria = SortConfig::default().criteria;
    }

    let mut config = SortConfig::default();
    config.criteria = criteria;
    config.niche = command.niche.unwrap_or(3);
    config.frecency_weight = command.frecency_weight.unwrap_or(3);
    config.min_seeders = command.min_seeders;
    config.max_seeders = command.max_seeders;
    config.limit_size = command
        .limit_size
        .as_deref()
        .map(SortConfig::parse_limit_size)
        .transpose()?;
    config.min_depth = command.min_depth;
    config.max_depth = command.max_depth;
    config.enrich = command.enrich;

    // Parse depth constraints
    if !command.depth.is_empty() {
        let (min_depth, max_depth) = syncweb_core::parsing::parse_depth_constraints(
            &command.depth,
            config.min_depth.unwrap_or(0),
            config.max_depth,
        );
        config.min_depth = Some(min_depth);
        config.max_depth = max_depth;
    }

    let sorter = Sorter::new(config);

    // Enrich with daemon data if --enrich is set
    if command.enrich {
        if let Some(client) = syncweb_core::daemon::daemon_client(data_dir)? {
            let response = tokio::runtime::Runtime::new()
                .map_err(|e| anyhow::anyhow!("failed to create runtime for enrich sort: {e}"))?
                .block_on(async {
                    client
                        .send(IpcRequest::new(IpcCommand::EnrichSort {
                            path: command.path.clone(),
                        }))
                        .await
                })?;
            match response {
                IpcResponse::EnrichData(peer_map) => {
                    let needs_niche = command.by.eq_ignore_ascii_case("niche");
                    let needs_frecency = command.by.eq_ignore_ascii_case("frecency");
                    let needs_peers = command.by.eq_ignore_ascii_case("peers");
                    if needs_peers || needs_niche || needs_frecency {
                        sorter.enrich_peers(&mut sortable, &peer_map);
                    }
                    if needs_niche {
                        sorter.enrich_niche(&mut sortable);
                    }
                }
                IpcResponse::Error { message } => {
                    eprintln!("warning: daemon enrichment failed: {message}");
                }
                IpcResponse::Ok { .. }
                | IpcResponse::Status(_)
                | IpcResponse::FolderList(_)
                | IpcResponse::DownloadComplete { .. }
                | IpcResponse::ImportFilesComplete { .. }
                | IpcResponse::ImportComplete(_)
                | IpcResponse::ExportComplete(_)
                | _ => {
                    eprintln!("warning: daemon returned unexpected response; enrichment skipped");
                }
            }
        } else {
            eprintln!("warning: daemon is not running; enrichment requires a running daemon");
        }
    }

    // Filter by seeders first
    sortable = sorter.filter_seeders(sortable);

    // Sort
    let result = sorter.sort(&mut sortable);

    if result.iter().next().is_none() && !output_json {
        println!("No entries match the sorting criteria");
        return Ok(());
    }

    if output_json {
        let paths: Vec<_> = result.iter().map(|entry| entry.path.display().to_string()).collect();
        println!("{}", serde_json::to_string_pretty(&paths)?);
    } else {
        for entry in &result {
            println!("{}", entry.path.display());
        }
    }
    Ok(())
}

async fn handle_sort(ctx: &CliContext<'_>, command: &crate::cli::commands::SortArgs) -> Result<()> {
    let output_json = ctx.output_json;
    if command.listing.local_only {
        return handle_sort_disk(ctx, command);
    }
    let (mode, resolved) = resolve_entry_source(ctx, &command.path).await?;
    let mut rows = fetch_listing_rows(mode, &resolved, !command.listing.no_enrich).await?;
    if rows.is_empty() {
        if output_json {
            render_entries(&rows, &resolved.namespace, &command.path, output_json)?;
        } else {
            print_no_entries_nudge(&resolved.namespace);
        }
        return Ok(());
    }
    rows = apply_listing_filters(rows, &resolved.remainder, &command.listing);
    let by = parse_meta_sort(&command.by)?;
    sort_local_entries(&mut rows, by);
    render_entries(&rows, &resolved.namespace, &command.path, output_json)?;
    Ok(())
}

fn print_config(config: &AppConfig) -> Result<()> {
    print!("{}", toml::to_string_pretty(config)?);
    Ok(())
}

fn sort_entry(entry: FileEntry) -> SortEntry {
    SortEntry::new(entry.relative_path)
        .with_modified(entry.modified)
        .with_size(entry.size)
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

fn handle_stat(ctx: &CliContext<'_>, command: crate::cli::commands::StatArgs) -> Result<()> {
    let output_json = ctx.output_json;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, size: u64, local: bool, modified: Option<u64>) -> LocalEntry {
        LocalEntry {
            path: path.to_owned(),
            hash: BlobHash::from_bytes(blake3::hash(path.as_bytes()).into()),
            size,
            local,
            modified: modified.map(|seconds| {
                UNIX_EPOCH
                    .checked_add(Duration::from_secs(seconds))
                    .unwrap_or(UNIX_EPOCH)
            }),
        }
    }

    #[test]
    fn parse_meta_sort_accepts_folder_vocabulary() {
        assert!(matches!(parse_meta_sort("name").unwrap(), MetaSort::Name));
        assert!(matches!(parse_meta_sort("size").unwrap(), MetaSort::Size));
        assert!(matches!(parse_meta_sort("modified").unwrap(), MetaSort::Modified));
        assert!(matches!(parse_meta_sort("state").unwrap(), MetaSort::State));
        assert!(matches!(parse_meta_sort("SIZE").unwrap(), MetaSort::Size));
        let error = parse_meta_sort("niche").unwrap_err();
        assert!(error.to_string().contains("--local-only"), "{error}");
    }

    #[test]
    fn entry_matches_path_filters_applies_remainder_prefix_glob() {
        let remainder = PathBuf::from("sub");
        assert!(entry_matches_path_filters("sub/a.txt", &remainder, None, None));
        assert!(entry_matches_path_filters("sub", &remainder, None, None));
        assert!(!entry_matches_path_filters("other/a.txt", &remainder, None, None));
        assert!(entry_matches_path_filters(
            "sub/a.txt",
            &PathBuf::new(),
            Some("sub"),
            None
        ));
        assert!(!entry_matches_path_filters(
            "subtle/a.txt",
            &PathBuf::new(),
            Some("sub"),
            None
        ));
        assert!(entry_matches_path_filters(
            "a.txt",
            &PathBuf::new(),
            None,
            Some("*.txt")
        ));
        assert!(!entry_matches_path_filters(
            "a.md",
            &PathBuf::new(),
            None,
            Some("*.txt")
        ));
    }

    #[test]
    fn apply_listing_filters_honors_remote_only() {
        let rows = vec![
            entry("a.txt", 1, true, None),
            entry("b.txt", 2, false, None),
            entry("c.txt", 3, true, None),
        ];
        let flags = ListingFlags {
            remote_only: true,
            ..ListingFlags::default()
        };
        let filtered = apply_listing_filters(rows, &PathBuf::new(), &flags);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.first().unwrap().path, "b.txt");
    }

    #[test]
    fn sort_local_entries_orders_by_name_size_state() {
        let mut rows = vec![
            entry("b.txt", 10, false, None),
            entry("a.txt", 5, true, None),
            entry("c.txt", 7, true, None),
        ];
        sort_local_entries(&mut rows, MetaSort::Name);
        assert_eq!(rows.first().unwrap().path, "a.txt");
        assert_eq!(rows.get(2).map(|row| row.path.as_str()), Some("c.txt"));

        sort_local_entries(&mut rows, MetaSort::Size);
        assert_eq!(rows.first().unwrap().path, "a.txt");
        assert_eq!(rows.get(2).map(|row| row.path.as_str()), Some("b.txt"));

        sort_local_entries(&mut rows, MetaSort::State);
        // local rows sort before remote rows
        assert_eq!(rows.first().unwrap().path, "a.txt");
        assert_eq!(rows.get(2).map(|row| row.path.as_str()), Some("b.txt"));
    }

    #[test]
    fn sort_local_entries_modified_falls_back_to_doc_size_for_remote() {
        let mut rows = vec![
            entry("old.txt", 100, true, Some(1000)),
            entry("remote.txt", 5, false, None),
        ];
        sort_local_entries(&mut rows, MetaSort::Modified);
        assert_eq!(rows.first().unwrap().path, "remote.txt");
        assert_eq!(rows.get(1).map(|row| row.path.as_str()), Some("old.txt"));
    }

    #[test]
    fn local_entry_json_emits_stable_envelope_fields() {
        let row = entry("docs/a.txt", 42, true, Some(1234));
        let value = local_entry_json(&row);
        assert_eq!(value.get("path"), Some(&serde_json::json!("docs/a.txt")));
        assert_eq!(value.get("size"), Some(&serde_json::json!(42)));
        assert_eq!(value.get("local"), Some(&serde_json::json!(true)));
        assert_eq!(value.get("modified"), Some(&serde_json::json!(1234)));
        assert_eq!(
            value.get("hash").and_then(serde_json::Value::as_str).map(str::len),
            Some(64)
        );

        let remote = entry("docs/b.txt", 7, false, None);
        assert_eq!(
            local_entry_json(&remote).get("modified"),
            Some(&serde_json::Value::Null)
        );
    }

    #[test]
    fn render_entries_json_wraps_entries_in_envelope() {
        let row = entry("a.txt", 1, true, None);
        let rows = std::slice::from_ref(&row);
        let entries: Vec<serde_json::Value> = rows.iter().map(local_entry_json).collect();
        let envelope = serde_json::json!({
            "folder": "ns1",
            "path": "/tmp/folder",
            "entries": entries,
        });
        let parsed: serde_json::Value = serde_json::from_str(&serde_json::to_string(&envelope).unwrap()).unwrap();
        assert_eq!(parsed.get("folder"), Some(&serde_json::json!("ns1")));
        assert_eq!(parsed.get("path"), Some(&serde_json::json!("/tmp/folder")));
        let parsed_entries = parsed.get("entries").and_then(serde_json::Value::as_array).unwrap();
        assert_eq!(parsed_entries.len(), 1);
        assert_eq!(
            parsed_entries.first().unwrap().get("path"),
            Some(&serde_json::json!("a.txt"))
        );
    }

    #[test]
    fn canonical_path_resolves_relative_to_cwd() {
        let absolute = canonical_path(Path::new("."));
        assert!(absolute.is_absolute());
    }
}
