mod cli;

use std::str::FromStr;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use cli::{
    args::Cli,
    commands::{CollectionCommand, Command, ConfigCommand, NetworkCommand, PackageCommand},
    output::{init_tracing, print_version, run_repl},
};
use rayon::prelude::*;
use syncweb_core::{
    filter::{FilterAction, FilterConfig, FilterEngine, FilterEntry},
    folder::{
        CollectionEntry, CollectionManifest, CollectionStore, FolderManager, PackageAnnouncement, PackageCatalog,
        PackageManager, SyncMode,
    },
    fs::{FileEntry, FileType, ParallelScanner},
    init::InitResult,
    net::{NetworkManager, NetworkOptions, TransportFallback},
    node::{
        identity::{DeviceId, IdentityManager},
        iroh_node::{IrohNode, RelayMode},
    },
    search::{FindEngine, FindQuery},
    sort::{SortCriterion, SortEntry, Sorter},
    stat::{StatFormat, StatOutput},
    storage::Config as AppConfig,
    sync::{AreaFilter, AreaOfInterest, SessionMode, SubscribeParams, SyncEngine},
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;
    tracing::debug!(command = ?cli.command, "cli initialized");
    execute_cli(cli).await
}

async fn execute_cli(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Version => print_version(),
        Command::Repl => run_repl()?,
        Command::Create(command) => handle_create(&cli.data_dir, command).await?,
        Command::Join(command) => handle_join(&cli.data_dir, command).await?,
        Command::Accept { namespace } => handle_accept(&cli.data_dir, namespace).await?,
        Command::Drop { namespace } => handle_drop(&cli.data_dir, namespace).await?,
        Command::Folders => handle_folders(&cli.data_dir).await?,
        Command::Devices => handle_devices(&cli.data_dir)?,
        Command::Ls(command) => handle_ls(command)?,
        Command::Find(command) => handle_find(command)?,
        Command::Sort(command) => handle_sort(&command)?,
        Command::Stat(command) => handle_stat(command)?,
        Command::Download(command) => {
            copy_path(&command.source, &command.destination, command.threads)?;
            println!("{}", command.destination.display());
        }
        Command::Init(command) => {
            std::fs::create_dir_all(&command.path)?;
            let node = open_node(&cli.data_dir).await?;
            let manager = FolderManager::new(&node);
            let folder = manager.create(SyncMode::from_str(&command.mode)?).await?;
            let ticket = folder.ticket(node.endpoint().addr(), true).await?;
            let result = InitResult::new(&command.path, folder.namespace_id(), ticket);
            println!("path: {}", result.path.display());
            println!("namespace: {}", result.namespace);
            println!("ticket: {}", result.ticket);
            println!("share_url: {}", result.share_url);
            node.stop().await?;
        }
        Command::Automatic(command) => handle_automatic(&cli.data_dir, command).await?,
        Command::Subscribe(command) => handle_subscribe(&cli.data_dir, command).await?,
        Command::Publish(command) => handle_publish(&cli.data_dir, command).await?,
        Command::Unpublish(command) => handle_unpublish(&cli.data_dir, command).await?,
        Command::Collection { command } => handle_collection(&cli.data_dir, command).await?,
        Command::Package { command } => handle_package(&cli.data_dir, command).await?,
        Command::Config { command } => {
            let config_path = cli.data_dir.join("config.toml");
            let mut config = AppConfig::load(&config_path)?;
            match command {
                None | Some(ConfigCommand::Show { section: None }) => {
                    print_config(&config)?;
                }

                Some(ConfigCommand::Show { section: Some(section) }) => {
                    if section != "bep" {
                        anyhow::bail!("unsupported config section {section:?}; supported section: bep");
                    }
                    println!("{}", toml::to_string_pretty(&config.bep)?);
                }
                Some(ConfigCommand::Set { key, value }) => {
                    config.set(&key, &value)?;
                    config.save(&config_path)?;
                    println!("{key} updated");
                }
            }
        }
        Command::Network { command } => handle_network(&cli.data_dir, command).await?,
        Command::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "syncweb", &mut std::io::stdout());
        }
        Command::Manpages { dir } => {
            std::fs::create_dir_all(&dir)?;
            let cmd = Cli::command();

            // Generate main man page
            let man = clap_mangen::Man::new(cmd.clone());
            let mut buffer = Vec::default();
            man.render(&mut buffer)?;
            std::fs::write(dir.join("syncweb.1"), buffer)?;
            println!("Generated: syncweb.1");

            // Generate subcommand pages
            for subcmd in cmd.get_subcommands() {
                let name = subcmd.get_name();
                if name == "help" || name == "completions" || name == "manpages" {
                    continue;
                }

                // clap_mangen uses the subcommand's name directly in the man page.
                // We should build a new command for the subpage, or just render it.
                // Wait, in nofs, we did `let man = clap_mangen::Man::new(subcmd.clone());`
                // But we also need to change the file name to syncweb-{name}.1
                let subcmd_man = clap_mangen::Man::new(subcmd.clone());
                let mut subcmd_buffer = Vec::default();
                subcmd_man.render(&mut subcmd_buffer)?;
                let filename = format!("syncweb-{name}.1");
                std::fs::write(dir.join(&filename), subcmd_buffer)?;
                println!("Generated: {filename}");
            }

            println!("manpages generated in {}", dir.display());
        }
    }
    Ok(())
}

async fn handle_create(data_dir: &std::path::Path, command: crate::cli::commands::FolderCreate) -> Result<()> {
    std::fs::create_dir_all(&command.path)
        .with_context(|| format!("failed to create folder path {}", command.path.display()))?;
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::from_str(&command.mode)?).await?;
    if let Some(network_name) = command.network {
        add_folder_to_network(data_dir, &network_name, folder.namespace_id())?;
    }
    let ticket = folder.ticket(node.endpoint().addr(), true).await?;
    println!("namespace: {}", folder.namespace_id());
    println!("ticket: {ticket}");
    node.stop().await?;
    Ok(())
}

async fn handle_join(data_dir: &std::path::Path, command: crate::cli::commands::FolderJoin) -> Result<()> {
    std::fs::create_dir_all(&command.path)
        .with_context(|| format!("failed to create folder path {}", command.path.display()))?;
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.join(command.ticket, SyncMode::from_str(&command.mode)?).await?;
    if let Some(network_name) = command.network {
        add_folder_to_network(data_dir, &network_name, folder.namespace_id())?;
    }
    println!("joined: {}", folder.namespace_id());
    node.stop().await?;
    Ok(())
}

async fn handle_automatic(data_dir: &std::path::Path, command: crate::cli::commands::AutomaticArgs) -> Result<()> {
    let filter_path = command.filters.unwrap_or_else(|| data_dir.join("filters.toml"));
    let engine = if filter_path.exists() {
        FilterEngine::load(&filter_path)?
    } else {
        FilterEngine::new(FilterConfig::default())?
    };
    if command.show_filters {
        print!("{}", toml::to_string_pretty(&engine.config())?);
        return Ok(());
    }
    if command.dry_run {
        for path in command.paths {
            for entry in ParallelScanner::new(&path, Vec::<String>::new(), 0).scan()? {
                let filter_entry = FilterEntry::from_file(&entry);
                let action = engine.evaluate(&filter_entry);
                let label = match action {
                    FilterAction::Accept => "accept",
                    FilterAction::Reject => "reject",
                    _ => "unknown",
                };
                println!("{label}\t{}", entry.path.display());
            }
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
    println!("automatic synchronization running: {} folders", intents.len());
    tokio::signal::ctrl_c().await?;
    for intent in &intents {
        let _result = intent.cancel();
    }
    drop(network_topics);
    node.stop().await?;
    Ok(())
}

async fn handle_subscribe(data_dir: &std::path::Path, command: crate::cli::commands::SubscribeArgs) -> Result<()> {
    std::fs::create_dir_all(&command.path)?;
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    if let Ok(ticket) = command.ticket.parse::<iroh_blobs::ticket::BlobTicket>() {
        let folder = manager.subscribe_public(&ticket).await?;
        println!("subscribed: {}", folder.namespace_id());
        println!("blob: {}", ticket.hash());
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
    println!("subscribed: {}", folder.namespace_id());
    println!("ingest_only: {}", params.ingest_only);
    tokio::signal::ctrl_c().await?;
    let _result = intent.cancel();
    node.stop().await?;
    Ok(())
}

async fn handle_publish(data_dir: &std::path::Path, command: crate::cli::commands::PublishArgs) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.get(command.namespace.parse()?).await?;
    if let Some(blob) = command.blob {
        let hash = blob.parse()?;
        println!(
            "blob_ticket: {}",
            folder.publish_blob(node.endpoint().addr(), hash).await?
        );
    } else {
        println!("ticket: {}", folder.ticket(node.endpoint().addr(), false).await?);
    }
    node.stop().await?;
    Ok(())
}

async fn handle_unpublish(data_dir: &std::path::Path, command: crate::cli::commands::UnpublishArgs) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.get(command.namespace.parse()?).await?;
    folder.unpublish_blob(command.blob.parse()?).await?;
    println!("unpublished: {}", command.blob);
    node.stop().await?;
    Ok(())
}

async fn handle_collection(data_dir: &std::path::Path, command: CollectionCommand) -> Result<()> {
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
            println!("collection: {}", manifest.collection_id);
        }
        CollectionCommand::Add { path } => {
            let mut manifest = load_manifest(&path)?;
            manifest.entries = scan_collection_entries(&path)?;
            save_manifest(&path, &manifest)?;
            println!("entries: {}", manifest.entries.len());
        }
        CollectionCommand::Versions {
            path,
            version,
            changelog,
        } => {
            let mut manifest = load_manifest(&path)?;
            let parent = manifest.content_id()?;
            manifest.parent = Some(parent);
            manifest.version = version;
            manifest.changelog = changelog;
            save_manifest(&path, &manifest)?;
            println!("version: {}", manifest.version);
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
            println!("manifest: {}", head.manifest);
            println!("manifest_ticket: {}", announcement.manifest_ticket);
            println!("sequence: {}", head.sequence);
            node.stop().await?;
        }
    }
    Ok(())
}

async fn handle_package(data_dir: &std::path::Path, command: PackageCommand) -> Result<()> {
    let packages = PackageManager::new(data_dir.join("packages"));
    match command {
        PackageCommand::Search {
            query,
            bootstrap: bootstrap_values,
            timeout_ms,
        } => handle_package_search(data_dir, query, bootstrap_values, timeout_ms, &packages).await?,
        PackageCommand::Info {
            manifest: manifest_path,
            ticket: ticket_value,
        } => {
            let collection_manifest = load_package_manifest(data_dir, manifest_path, ticket_value).await?;
            println!("{}", serde_json::to_string_pretty(&collection_manifest)?);
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
            println!(
                "installed: {} {}",
                collection_manifest.collection_id, collection_manifest.version
            );
        }
        PackageCommand::Remove {
            collection: collection_id,
            version,
        } => {
            let collection = collection_id.parse()?;
            packages.remove(collection, &version)?;
            println!("removed: {collection} {version}");
        }
        PackageCommand::Verify {
            manifest: manifest_path,
        } => {
            let collection_manifest = load_manifest_file(&manifest_path)?;
            packages.verify(&collection_manifest)?;
            println!(
                "verified: {} {}",
                collection_manifest.collection_id, collection_manifest.version
            );
        }
        PackageCommand::List => {
            for (collection, installed) in packages.state()?.installed {
                println!("{collection}\t{}", installed.current);
            }
        }
        PackageCommand::Versions {
            collection: collection_id,
        } => {
            let collection = collection_id.parse()?;
            let state = packages.state()?;
            let installed = state
                .current(collection)
                .ok_or_else(|| anyhow::anyhow!("collection is not installed: {collection}"))?;
            for version in installed.versions.keys() {
                println!("{version}");
            }
        }
        PackageCommand::Switch {
            collection: collection_id,
            version,
        } => {
            let collection = collection_id.parse()?;
            packages.switch(collection, &version)?;
            println!("current: {collection} {version}");
        }
    }
    Ok(())
}

async fn handle_package_search(
    data_dir: &std::path::Path,
    query: Option<String>,
    bootstrap_values: Vec<String>,
    timeout_ms: u64,
    packages: &PackageManager,
) -> Result<()> {
    for (collection, installed) in packages.state()?.installed {
        let line = format!("{collection}\t{}", installed.current);
        if query.as_ref().is_none_or(|value| line.contains(value)) {
            println!("{line}");
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
        println!(
            "{}\t{}\t{}\t{}",
            announcement.name, announcement.version, announcement.collection_id, announcement.manifest
        );
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
        if manifest.content_id()? != blob_ticket.hash() {
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

async fn handle_network(data_dir: &std::path::Path, command: NetworkCommand) -> Result<()> {
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
            println!("created: {name}\t{id}");
        }
        NetworkCommand::List { name } => {
            if let Some(network_name) = name {
                let network = manager
                    .get_by_name(&network_name)
                    .with_context(|| format!("network not found: {network_name}"))?;
                println!(
                    "{}\t{}\t{} members\t{} folders",
                    network.name,
                    network.id,
                    network.members.len(),
                    network.folders.len()
                );
            } else {
                for network in manager.list() {
                    println!(
                        "{}\t{}\t{} members\t{} folders",
                        network.name,
                        network.id,
                        network.members.len(),
                        network.folders.len()
                    );
                }
            }
        }
        NetworkCommand::Join { ticket } => {
            let parsed = ticket.parse()?;
            let id = manager.join(parsed)?;
            println!("joined: {id}");
        }
        NetworkCommand::Leave { name } => {
            let id = network_id_by_name(&manager, &name)?;
            manager.leave(id)?;
            println!("left: {name}");
        }
        NetworkCommand::Invite { name, device } => {
            let id = network_id_by_name(&manager, &name)?;
            let ticket = if let Some(node_id) = device {
                manager.invite(id, node_id.parse()?)?
            } else {
                manager.invite_any(id)?
            };
            println!("{ticket}");
        }
        NetworkCommand::Kick { name, device } => {
            let id = network_id_by_name(&manager, &name)?;
            manager.kick(id, &device.parse()?)?;
            println!("kicked: {device}");
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
            println!("relay reachable: {relay_url}");
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

async fn handle_accept(data_dir: &std::path::Path, namespace: String) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let folder = manager.accept(namespace.parse()?).await?;
    println!("accepted: {}", folder.namespace_id());
    node.stop().await?;
    Ok(())
}

async fn handle_drop(data_dir: &std::path::Path, namespace: String) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    manager.drop(namespace.parse()?).await?;
    println!("dropped: {namespace}");
    node.stop().await?;
    Ok(())
}

async fn handle_folders(data_dir: &std::path::Path) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    for folder in manager.list().await? {
        println!("{}\t{}", folder.namespace_id(), folder.mode());
    }
    node.stop().await?;
    Ok(())
}

fn handle_devices(data_dir: &std::path::Path) -> Result<()> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let device_id = DeviceId::from_node_id(identity.node_id());
    println!("iroh: {}", identity.node_id());
    println!("syncthing: {}", device_id.to_syncthing());
    Ok(())
}

fn handle_ls(command: crate::cli::commands::LocalPathArgs) -> Result<()> {
    let entries = ParallelScanner::new(&command.path, Vec::<String>::new(), command.threads).scan()?;
    if let Some(criteria) = command.sort {
        let mut sortable = entries.into_iter().map(sort_entry).collect::<Vec<_>>();
        Sorter::new(parse_sort_criterion(&criteria)?).sort(&mut sortable);
        for entry in sortable {
            println!("{}", entry.path.display());
        }
    } else {
        for entry in entries {
            println!("{}", entry.relative_path.display());
        }
    }
    Ok(())
}

fn handle_find(command: crate::cli::commands::FindArgs) -> Result<()> {
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
    for entry in FindEngine::new(&command.path)
        .with_threads(command.threads)
        .find(&query)?
    {
        println!("{}", entry.relative_path.display());
    }
    Ok(())
}

fn handle_sort(command: &crate::cli::commands::SortArgs) -> Result<()> {
    let entries = ParallelScanner::new(&command.path, Vec::<String>::new(), command.threads).scan()?;
    let mut sortable = entries.into_iter().map(sort_entry).collect::<Vec<_>>();
    Sorter::new(parse_sort_criterion(&command.by)?).sort(&mut sortable);
    for entry in sortable {
        println!("{}", entry.path.display());
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

fn handle_stat(command: crate::cli::commands::StatArgs) -> Result<()> {
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
    println!("{}", output.display(format));
    Ok(())
}
