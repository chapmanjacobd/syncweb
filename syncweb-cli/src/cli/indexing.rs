use std::{
    fs,
    io::IsTerminal,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use async_recursion::async_recursion;
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};

use super::args::CliContext;
use super::commands::{FilterCommand, IndexingCommand, LinkCommand, MetaCommand, ProviderCommand, PublishCatalogArgs};
use syncweb_core::{
    constants::{DOWNLOAD_PIN_PREFIX, RESILIENCE_TOPIC},
    folder::{FolderManager, SyncwebFolder},
    gossip::gossip_topic_id,
    indexing::{
        ContentLink, DenylistRule, FilterList, IndexingDatabase, IndexingService, Link, LinkResolver, MetadataEntry,
        MutablePointer, NameLink, PrivateLink, ProviderLease, ReplicationBudget, ResilienceConfig, ResilienceService,
        TrustPolicy, WotService,
    },
    node::identity::IdentityManager,
};

use dialoguer::Confirm;
use iroh::EndpointAddr;
use iroh_blobs::{
    BlobFormat, Hash,
    api::blobs::ExportMode,
    get::fsm::{self, ConnectedNext, EndBlobNext},
    protocol::GetRequest,
    ticket::BlobTicket,
};
use iroh_docs::NamespaceId;
use syncweb_core::{gossip::TopicChannel, init::open_node, node::iroh_node::IrohNode};

const DEFAULT_PRIVATE_LINK_TTL: u64 = 30 * 24 * 60 * 60;

fn confirm_destructive(operation: &str, output_json: bool) -> Result<bool> {
    if output_json {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        return Ok(true);
    }
    Ok(Confirm::new()
        .with_prompt(format!("Are you sure you want to {operation}?"))
        .default(false)
        .show_default(true)
        .interact()?)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct IndexingState {
    #[serde(default)]
    catalogs: Vec<CatalogState>,
    #[serde(default)]
    federated_filters: Vec<FederatedFilterState>,
    #[serde(default)]
    denylist: Vec<DenylistRule>,
    #[serde(default)]
    links: LinkState,
    #[serde(default)]
    leases: Vec<syncweb_core::indexing::ProviderLease>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CatalogState {
    name: String,
    namespace: NamespaceId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FederatedFilterState {
    namespace: NamespaceId,
    sequence: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct LinkState {
    #[serde(default)]
    pointers: Vec<MutablePointer>,
    #[serde(default)]
    mirrors: Vec<String>,
    #[serde(default)]
    revoked: Vec<PrivateLink>,
}

#[async_recursion]
pub async fn handle_indexing(ctx: &CliContext<'_>, command: IndexingCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        IndexingCommand::Enable { folder, namespace } => {
            let node = open_node(data_dir).await?;
            let manager = FolderManager::new(&node);
            let selector = namespace.map_or(folder, PathBuf::from);
            let selected = manager.resolve(&selector).await?;
            let indexing = open_indexing(data_dir)?;
            let handle = indexing.enable_folder(&selected).await?;
            print_status(
                output_json,
                serde_json::json!({
                    "status": "enabled",
                    "namespace": handle.namespace_id().to_string(),
                }),
                format!("enabled: {}", handle.namespace_id()),
            )?;
            node.stop().await?;
        }
        IndexingCommand::Disable { folder, namespace } => {
            let node = open_node(data_dir).await?;
            let manager = FolderManager::new(&node);
            let selector = namespace.map_or(folder, PathBuf::from);
            let selected = manager.resolve(&selector).await?;
            let selected_namespace = selected.namespace_id();
            open_indexing(data_dir)?.disable_folder(selected_namespace).await?;
            print_status(
                output_json,
                serde_json::json!({"status": "disabled", "namespace": selected_namespace.to_string()}),
                format!("disabled: {selected_namespace}"),
            )?;
            node.stop().await?;
        }
        IndexingCommand::Health { hash } => {
            let content_hash = parse_hash(&hash)?;
            let (_, state) = open_indexing_state(data_dir)?;
            let resilience =
                open_indexing(data_dir)?.resilience_service(ResilienceConfig::new(ReplicationBudget::default()));
            let now = epoch_seconds();
            for lease in state.leases {
                if !lease.is_expired_at(now) {
                    resilience.record_lease(&lease)?;
                }
            }
            let health = resilience.health(&content_hash)?;
            print_status(
                output_json,
                serde_json::json!({
                    "hash": content_hash.to_string(),
                    "verified": health.verified,
                    "local": health.local,
                    "verified_providers": health.verified_providers.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    "local_providers": health.local_providers.iter().map(ToString::to_string).collect::<Vec<_>>(),
                }),
                format!(
                    "hash: {content_hash}\nverified providers: {}\nlocal providers: {}",
                    health.verified, health.local
                ),
            )?;
        }
        IndexingCommand::Meta { command: meta_command } => handle_meta(ctx, meta_command)?,
        IndexingCommand::Filter {
            command: filter_command,
        } => handle_filter(ctx, filter_command)?,
    }
    Ok(())
}

pub async fn handle_catalog_publish(ctx: &CliContext<'_>, args: PublishCatalogArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let PublishCatalogArgs {
        folder,
        namespace,
        catalog,
        tags,
    } = args;
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let selector = namespace.map_or_else(|| folder.clone(), PathBuf::from);
    let selected = manager.resolve(&selector).await?;
    let indexing = open_indexing(data_dir)?;
    let catalog_service = indexing.catalog_service(
        node.docs_engine(),
        node.blob_store(),
        node.docs_engine().author().await?,
    );
    let (_, mut state) = open_indexing_state(data_dir)?;
    let catalog_handle = if let Some(existing) = state.catalogs.iter().find(|item| item.name == catalog).cloned() {
        catalog_service.subscribe_namespace(existing.namespace).await?
    } else {
        let created = catalog_service.create_catalog(&catalog).await?;
        state.catalogs.push(CatalogState {
            name: catalog.clone(),
            namespace: created.namespace_id(),
        });
        created
    };
    let published = catalog_service
        .publish_folder_with_metadata(&catalog_handle, &selected, selected.namespace_id().to_string(), &tags)
        .await?;
    let ticket = catalog_service
        .ticket(&catalog_handle, node.endpoint().addr(), false)
        .await?;
    print_status(
        output_json,
        serde_json::json!({
            "status": "published",
            "catalog": catalog,
            "catalog_namespace": catalog_handle.namespace_id().to_string(),
            "records": published,
            "ticket": ticket.to_string(),
        }),
        format!(
            "published: {published}\ncatalog: {catalog}\nnamespace: {}\nticket: {ticket}",
            catalog_handle.namespace_id()
        ),
    )?;
    node.stop().await?;
    Ok(())
}

pub async fn handle_link(ctx: &CliContext<'_>, command: LinkCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        LinkCommand::Create {
            source,
            name,
            version,
            sequence,
            private,
            expires,
            publish,
        } => {
            let opts = LinkCreateOptions {
                source,
                name,
                version,
                sequence,
                private,
                expires,
                publish,
            };
            handle_link_create(ctx, opts).await?;
        }
        LinkCommand::Resolve { link, version } => {
            let (_, state) = open_indexing_state(data_dir)?;
            let resolver = load_resolver(&state)?;
            let parsed = link.parse::<Link>()?;
            let resolution = if let Some(version_value) = version {
                let Link::Name(name) = &parsed else {
                    anyhow::bail!("--version is only valid for mutable name links");
                };
                resolver.resolve_version(name, &version_value)?
            } else {
                resolver.resolve(&parsed)?
            };
            print_status(
                output_json,
                serde_json::json!({
                    "link": link,
                    "manifest": resolution.manifest.to_string(),
                    "version": resolution.version,
                    "sequence": resolution.sequence,
                    "providers": resolution.providers.iter().map(|provider| serde_json::json!({
                        "provider": provider.provider.to_string(),
                        "ticket": provider.ticket,
                        "expires_at": provider.expires_at,
                    })).collect::<Vec<_>>(),
                    "tickets": resolution.tickets.iter().map(ToString::to_string).collect::<Vec<_>>(),
                }),
                format!(
                    "manifest: {}\nversion: {}\nsequence: {}\nproviders: {}",
                    resolution.manifest,
                    resolution.version.as_deref().unwrap_or("-"),
                    resolution
                        .sequence
                        .map_or_else(|| "-".to_owned(), |value| value.to_string()),
                    resolution.providers.len()
                ),
            )?;
        }
        LinkCommand::Revoke { link } => {
            if !confirm_destructive("revoke this link", output_json)? {
                println!("aborted");
                return Ok(());
            }
            let parsed = link.parse::<PrivateLink>()?;
            let (db, mut state) = open_indexing_state(data_dir)?;
            let resolver = load_resolver(&state)?;
            resolver.revoke(&parsed)?;
            if !state.links.revoked.contains(&parsed) {
                state.links.revoked.push(parsed);
            }
            db.save_links(&state.links.pointers, &state.links.mirrors, &state.links.revoked)?;

            print_status(
                output_json,
                serde_json::json!({"status": "revoked", "link": link}),
                format!("revoked: {link}"),
            )?;
        }
    }
    Ok(())
}

struct LinkCreateOptions {
    source: PathBuf,
    name: Option<String>,
    version: Option<String>,
    sequence: u64,
    private: bool,
    expires: Option<u64>,
    publish: Option<String>,
}

fn create_private_link(hash: Hash, expires: Option<u64>, state: &mut IndexingState) -> Result<Link> {
    let expires_at = expires.unwrap_or_else(|| epoch_seconds().saturating_add(DEFAULT_PRIVATE_LINK_TTL));
    let link = PrivateLink::generate(hash, expires_at)?;
    state.links.revoked.retain(|existing| existing != &link);
    Ok(Link::Private(link))
}

fn create_named_link(
    alias: String,
    hash: Hash,
    version: Option<String>,
    sequence: u64,
    state: &mut IndexingState,
    data_dir: &Path,
) -> Result<Link> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let signing_key = signing_key(&identity);
    let pointer_sequence = if sequence == 0 {
        state
            .links
            .pointers
            .iter()
            .filter(|pointer| pointer.publisher == identity.node_id() && pointer.alias == alias)
            .map(|pointer| pointer.sequence)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    } else {
        sequence
    };
    let mut pointer = MutablePointer::signed_with_secret_key(
        identity.node_id(),
        alias,
        hash,
        pointer_sequence,
        identity.secret_key(),
    )?;
    if let Some(version_value) = version {
        pointer = pointer.with_version(version_value);
        pointer.sign(&signing_key)?;
    }
    let link = pointer.link()?;
    state.links.pointers.push(pointer);
    Ok(Link::Name(link))
}

async fn publish_name_link(
    name_link: &NameLink,
    folder: &SyncwebFolder,
    state: &IndexingState,
    namespace: NamespaceId,
) -> Result<()> {
    let pointer = state
        .links
        .pointers
        .iter()
        .find(|p| p.publisher == name_link.publisher && p.alias == name_link.alias);
    if let Some(p) = pointer {
        let payload = serde_json::to_vec(p)?;
        folder
            .set_blob(format!("sys/links/mutable/{}", p.alias), payload)
            .await?;
        tracing::info!(alias = %p.alias, namespace = %namespace, "published mutable pointer to folder");
    }
    Ok(())
}

async fn publish_link(namespace_str: String, link: &Link, state: &IndexingState, data_dir: &Path) -> Result<()> {
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let namespace = namespace_str.parse::<NamespaceId>()?;
    let folder = manager.get(namespace).await?;
    match link {
        Link::Name(name_link) => publish_name_link(name_link, &folder, state, namespace).await?,
        Link::Private(private_link) => {
            let payload = serde_json::to_vec(private_link)?;
            folder
                .set_blob(
                    format!("sys/links/private/{}", hex::encode(private_link.capability)),
                    payload,
                )
                .await?;
            tracing::info!(namespace = %namespace, "published private link to folder");
        }
        Link::Content(_) => {
            tracing::warn!("--publish has no effect on immutable content links");
        }
        _ => {}
    }
    node.stop().await?;
    Ok(())
}

async fn handle_link_create(ctx: &CliContext<'_>, opts: LinkCreateOptions) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let hash = hash_source(&opts.source)?;
    let (db, mut state) = open_indexing_state(data_dir)?;
    let link = if opts.private {
        create_private_link(hash, opts.expires, &mut state)?
    } else if let Some(alias) = opts.name {
        create_named_link(alias, hash, opts.version, opts.sequence, &mut state, data_dir)?
    } else {
        ensure!(opts.version.is_none(), "--version requires --name");
        ensure!(opts.sequence == 0, "--sequence requires --name");
        Link::Content(ContentLink::new(hash))
    };
    db.save_links(&state.links.pointers, &state.links.mirrors, &state.links.revoked)?;

    if let Some(namespace_str) = opts.publish {
        publish_link(namespace_str, &link, &state, data_dir).await?;
    }

    print_status(
        output_json,
        serde_json::json!({"status": "created", "link": link.to_string(), "hash": hash.to_string()}),
        format!("link: {link}\nhash: {hash}"),
    )?;
    Ok(())
}

pub async fn handle_provider(ctx: &CliContext<'_>, command: ProviderCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        ProviderCommand::Add { collection, provider } => {
            let ticket = provider.parse::<BlobTicket>()?;
            let expected_hash = collection_hash(data_dir, &collection)?;
            ensure!(
                expected_hash.is_none_or(|hash| hash == ticket.hash()),
                "provider hash does not match collection"
            );
            let (db, mut state) = open_indexing_state(data_dir)?;
            let resolver = load_resolver(&state)?;
            resolver.register_mirror(ticket.clone())?;
            if !state.links.mirrors.contains(&provider) {
                state.links.mirrors.push(provider.clone());
            }
            db.save_links(&state.links.pointers, &state.links.mirrors, &state.links.revoked)?;
            let node = open_node(data_dir).await?;
            if ticket.addr().id == node.endpoint().id() {
                announce_self_lease(data_dir, &node, ticket.hash()).await?;
            }
            node.stop().await?;
            print_status(
                output_json,
                serde_json::json!({"status": "added", "hash": ticket.hash().to_string(), "provider": provider}),
                format!("provider added: {}\nprovider: {provider}", ticket.hash()),
            )?;
        }
    }
    Ok(())
}

pub async fn download_blob(
    ctx: &CliContext<'_>,
    tickets: &[BlobTicket],
    min_providers: usize,
    no_sharing: bool,
    export_path: Option<&Path>,
) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let first_ticket = tickets.first().context("tickets list is empty")?;
    let content_hash = first_ticket.hash();

    if no_sharing {
        if let Some(path) = export_path {
            direct_fetch_to_path(data_dir, first_ticket, content_hash, path).await?;
        } else {
            let node = open_node(data_dir).await?;
            node.blob_store()
                .fetch(node.endpoint(), first_ticket)
                .await
                .context("failed to fetch blob")?;
            let pin_name = format!("{DOWNLOAD_PIN_PREFIX}{content_hash}");
            node.blob_store().pin(&pin_name, content_hash).await?;
            announce_self_lease(data_dir, &node, content_hash).await?;
            node.stop().await?;
        }
    } else {
        let node = open_node(data_dir).await?;
        let resilience = ResilienceService::new(ResilienceConfig::new(ReplicationBudget::new(min_providers)));
        for ticket in tickets {
            let expires_at = epoch_seconds().saturating_add(365 * 24 * 60 * 60);
            let lease = ProviderLease::new(ticket.hash(), ticket.to_string(), 0, expires_at)?;
            resilience.record_lease(&lease)?;
        }

        let result = resilience
            .ensure_replication(node.endpoint(), node.blob_store(), content_hash)
            .await?;

        if result.pinned {
            announce_self_lease(data_dir, &node, content_hash).await?;
        }

        if output_json {
            println!(
                "{}",
                serde_json::json!({
                    "hash": result.hash.to_string(),
                    "pinned": result.pinned,
                    "short_circuited": result.short_circuited,
                    "fetched_from": result.fetched_from.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    "failed_from": result.failed_from.iter().map(|(p, k)| serde_json::json!({
                        "provider": p.to_string(),
                        "kind": format!("{k:?}"),
                    })).collect::<Vec<_>>(),
                    "providers_before": result.health_before.verified,
                    "providers_after": result.health_after.verified,
                })
            );
        } else {
            println!("hash: {}", result.hash);
            println!("pinned: {}", result.pinned);
            println!("short-circuited: {}", result.short_circuited);
            if !result.fetched_from.is_empty() {
                println!(
                    "fetched from: {}",
                    result
                        .fetched_from
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            if !result.failed_from.is_empty() {
                for (provider, kind) in &result.failed_from {
                    println!("failed: {provider} ({kind:?})");
                }
            }
            println!("providers before: {}", result.health_before.verified);
            println!("providers after: {}", result.health_after.verified);
        }

        if let Some(path) = export_path {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            node.blob_store()
                .export_to_path_with_mode(content_hash, path, ExportMode::TryReference)
                .await?;
        }

        node.stop().await?;
    }

    Ok(())
}

/// Publish a provider lease over the resilience gossip topic.
///
/// Remote peers hydrate their mirror/replication provider sets from these
/// time-bound announcements (see the daemon's lease listener).
///
/// # Errors
///
/// Returns an error if the gossip channel cannot be opened or the lease cannot
/// be published.
async fn announce_lease(node: &IrohNode, lease: &ProviderLease) -> Result<()> {
    let (channel, _receiver) =
        TopicChannel::<ProviderLease>::open(node.gossip_service(), gossip_topic_id(RESILIENCE_TOPIC), Vec::new())
            .await?;
    channel.publish(lease).await?;
    Ok(())
}

/// Sign and announce a lease advertising that this node serves `hash`.
///
/// The lease uses a self-ticket so the node's own identity key is the signing
/// key, which is what gossip receivers verify against.
///
/// # Errors
///
/// Returns an error if the lease cannot be signed or published.
async fn announce_self_lease(data_dir: &Path, node: &IrohNode, hash: Hash) -> Result<()> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let ticket = BlobTicket::new(EndpointAddr::new(node.endpoint().id()), hash, BlobFormat::Raw).to_string();
    let expires_at = epoch_seconds().saturating_add(365 * 24 * 60 * 60);
    let lease = ProviderLease::signed(hash, ticket, epoch_seconds(), expires_at, identity.secret_key())?;
    announce_lease(node, &lease).await
}

async fn direct_fetch_to_path(data_dir: &Path, ticket: &BlobTicket, hash: Hash, path: &Path) -> Result<()> {
    let node = open_node(data_dir).await?;

    let connection = node
        .endpoint()
        .connect(ticket.addr().clone(), iroh_blobs::ALPN)
        .await
        .context("failed to connect to provider")?;

    let request = GetRequest::blob(hash);
    let at_connected = fsm::start(connection, request, fsm::RequestCounters::default())
        .next()
        .await
        .map_err(|e| anyhow::anyhow!("get negotiation failed: {e}"))?;
    let ConnectedNext::StartRoot(at_start_root) = at_connected
        .next()
        .await
        .map_err(|e| anyhow::anyhow!("get request failed: {e}"))?
    else {
        anyhow::bail!("unexpected provider response");
    };
    let at_blob_header = at_start_root.next();

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let file = std::fs::File::create(path).context("failed to create destination file")?;
    let writer = iroh_io::File::from_std(file);

    let at_end_blob = at_blob_header
        .write_all(writer)
        .await
        .map_err(|e| anyhow::anyhow!("download failed: {e}"))?;

    let EndBlobNext::Closing(closing) = at_end_blob.next() else {
        anyhow::bail!("unexpected end of blob stream");
    };
    closing
        .next()
        .await
        .map_err(|e| anyhow::anyhow!("connection close failed: {e}"))?;

    node.stop().await?;
    Ok(())
}

fn handle_meta(ctx: &CliContext<'_>, command: MetaCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        MetaCommand::Add {
            hash,
            key,
            value,
            sequence,
        } => {
            let identity = IdentityManager::new(data_dir.join("identity.key"))?;
            let entry = MetadataEntry::new(parse_hash(&hash)?, key, value, sequence, &signing_key(&identity))?;
            let inserted = open_wot_for_state(data_dir)?.append_metadata(&entry)?;
            print_status(
                output_json,
                serde_json::json!({
                    "status": if inserted { "added" } else { "unchanged" },
                    "content": entry.content.to_string(),
                    "key": entry.key,
                    "value": entry.value,
                    "sequence": entry.sequence,
                }),
                format!("metadata: {}\t{}\t{}", entry.content, entry.key, entry.value),
            )?;
        }
        MetaCommand::List { hash } => {
            let content = parse_hash(&hash)?;
            let (_, state) = open_indexing_state(data_dir)?;
            let indexing = open_indexing(data_dir)?;
            let wot = load_wot(&indexing, &state)?;
            let entries = wot
                .search("", 10_000)?
                .into_iter()
                .filter(|entry| entry.content == content)
                .collect::<Vec<_>>();
            if output_json {
                println!("{}", serde_json::to_string_pretty(&entries)?);
            } else {
                for entry in entries {
                    println!("{}\t{}\t{}", entry.key, entry.value, entry.sequence);
                }
            }
        }
    }
    Ok(())
}

fn handle_filter(ctx: &CliContext<'_>, command: FilterCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        FilterCommand::Add { rule_type, value } => {
            let rule = parse_denylist_rule(&rule_type, &value)?;
            let indexing = open_indexing(data_dir)?;
            let service = indexing.denylist_service();
            service.add(rule.clone())?;
            let (db, mut state) = open_indexing_state(data_dir)?;
            if !state.denylist.contains(&rule) {
                state.denylist.push(rule);
                db.save_denylist_rules(&state.denylist)?;
            }
            print_status(
                output_json,
                serde_json::json!({"status": "added", "type": rule_type, "value": value}),
                format!("filter added: {rule_type}\t{value}"),
            )?;
        }
        FilterCommand::Subscribe { source } => {
            let bytes = read_filter_source(&source)?;
            let list = FilterList::from_bytes(bytes)?;
            let (db, mut state) = open_indexing_state(data_dir)?;
            list.verify_signature()?;
            let current_sequence = state
                .federated_filters
                .iter()
                .find(|filter| filter.namespace == list.namespace_id)
                .map_or(0, |filter| filter.sequence);
            let changed = if list.sequence <= current_sequence {
                false
            } else {
                let indexing = open_indexing(data_dir)?;
                indexing.denylist_service().subscribe(&list)?
            };
            if changed {
                for rule in &list.entries {
                    if !state.denylist.contains(rule) {
                        state.denylist.push(rule.clone());
                    }
                }
                if let Some(filter) = state
                    .federated_filters
                    .iter_mut()
                    .find(|filter| filter.namespace == list.namespace_id)
                {
                    filter.sequence = list.sequence;
                } else {
                    state.federated_filters.push(FederatedFilterState {
                        namespace: list.namespace_id,
                        sequence: list.sequence,
                    });
                }
                db.save_denylist_rules(&state.denylist)?;
            }
            print_status(
                output_json,
                serde_json::json!({
                    "status": if changed { "subscribed" } else { "unchanged" },
                    "namespace": list.namespace_id.to_string(),
                    "sequence": list.sequence,
                    "entries": list.entries.len(),
                }),
                format!(
                    "filter list: {}\nsequence: {}\nentries: {}",
                    list.namespace_id,
                    list.sequence,
                    list.entries.len()
                ),
            )?;
        }
    }
    Ok(())
}

fn open_wot_for_state(data_dir: &Path) -> Result<WotService> {
    let indexing = open_indexing(data_dir)?;
    let (_, state) = open_indexing_state(data_dir)?;
    load_wot(&indexing, &state)
}

fn open_indexing(data_dir: &Path) -> Result<IndexingService> {
    Ok(IndexingService::new(data_dir.join("indexing.sqlite"))?)
}

fn load_wot(indexing: &IndexingService, _state: &IndexingState) -> Result<WotService> {
    let data_dir = indexing.database().path().parent().unwrap_or_else(|| Path::new("."));
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let signing = signing_key(&identity);
    Ok(indexing.wot_service(TrustPolicy::with_root(&signing)))
}

fn load_resolver(state: &IndexingState) -> Result<LinkResolver> {
    let resolver = LinkResolver::new();
    let mut pointers = state.links.pointers.clone();
    pointers.sort_by(|left, right| {
        left.publisher
            .to_string()
            .cmp(&right.publisher.to_string())
            .then_with(|| left.alias.cmp(&right.alias))
            .then_with(|| left.sequence.cmp(&right.sequence))
    });
    for pointer in pointers {
        resolver.publish(&pointer)?;
    }
    for mirror in &state.links.mirrors {
        resolver.register_mirror(mirror.parse()?)?;
    }
    for revoked in &state.links.revoked {
        resolver.revoke(revoked)?;
    }
    Ok(resolver)
}

fn open_indexing_state(data_dir: &Path) -> Result<(IndexingDatabase, IndexingState)> {
    let db = IndexingDatabase::open(data_dir.join("indexing.sqlite"))?;
    let catalogs = db
        .load_catalogs()?
        .into_iter()
        .map(|(namespace, name)| CatalogState { name, namespace })
        .collect();
    let denylist = db.load_denylist_rules()?;
    let (pointers, mirrors, revoked) = db.load_links()?;
    let leases = db.load_leases()?;
    let state = IndexingState {
        catalogs,
        federated_filters: Vec::new(),
        denylist,
        links: LinkState {
            pointers,
            mirrors,
            revoked,
        },
        leases,
    };
    Ok((db, state))
}

fn signing_key(identity: &IdentityManager) -> SigningKey {
    SigningKey::from_bytes(&identity.secret_key().to_bytes())
}

fn parse_hash(value: &str) -> Result<Hash> {
    value
        .parse()
        .map_err(|error| anyhow::anyhow!("invalid content hash {value:?}: {error}"))
}

fn parse_denylist_rule(rule_type: &str, value: &str) -> Result<DenylistRule> {
    match rule_type {
        "device" => Ok(DenylistRule::device(value)),
        "file" => Ok(DenylistRule::file(value)),
        "hash" => Ok(DenylistRule::hash(parse_hash(value)?)),
        _ => anyhow::bail!("unsupported denylist rule type: {rule_type}"),
    }
}

fn read_filter_source(source: &str) -> Result<Vec<u8>> {
    if let Some(path) = source.strip_prefix("file://") {
        return Ok(fs::read(path)?);
    }
    if source.starts_with("http://") || source.starts_with("https://") {
        let output = ProcessCommand::new("curl")
            .args(["--fail", "--silent", "--show-error", source])
            .output()
            .context("failed to run curl for filter list")?;
        ensure!(
            output.status.success(),
            "filter list download failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Ok(output.stdout);
    }
    Ok(fs::read(source)?)
}

fn collection_hash(data_dir: &Path, collection: &str) -> Result<Option<Hash>> {
    if let Ok(hash) = collection.parse() {
        return Ok(Some(hash));
    }
    if let Ok(parsed_link) = collection.parse::<Link>() {
        return Ok(Some(match parsed_link {
            Link::Content(content_link) => content_link.hash,
            Link::Private(private_link) => private_link.manifest,
            Link::Name(_) => {
                load_resolver(&open_indexing_state(data_dir)?.1)?
                    .resolve(&parsed_link)?
                    .manifest
            }
            _ => anyhow::bail!("unsupported link type"),
        }));
    }
    if Path::new(collection).exists() {
        return Ok(Some(hash_source(Path::new(collection))?));
    }
    Ok(None)
}

fn hash_source(source: &Path) -> Result<Hash> {
    if source.is_file() {
        return Ok(Hash::from_bytes(*blake3::hash(&fs::read(source)?).as_bytes()));
    }
    if let Ok(hash) = source.to_string_lossy().parse() {
        return Ok(hash);
    }
    ensure!(
        source.is_dir(),
        "link source does not exist or is not a file/directory: {}",
        source.display()
    );
    let mut files = Vec::new();
    collect_files(source, source, &mut files)?;
    files.sort();
    let mut hasher = blake3::Hasher::new();
    for relative in files {
        let bytes = fs::read(source.join(&relative))?;
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(&[0]);
        hasher.update(&bytes);
        hasher.update(&[0]);
    }
    Ok(Hash::from_bytes(*hasher.finalize().as_bytes()))
}

fn collect_files(root: &Path, current: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    for directory_entry_result in fs::read_dir(current)? {
        let directory_entry = directory_entry_result?;
        let path = directory_entry.path();
        if path.is_dir() {
            collect_files(root, &path, output)?;
        } else {
            if path.is_file() {
                output.push(path.strip_prefix(root)?.to_path_buf());
            }
        }
    }
    Ok(())
}

fn print_status<T, S>(output_json: bool, json: T, text: S) -> Result<()>
where
    T: Serialize,
    S: std::fmt::Display,
{
    if output_json {
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("{text}");
    }
    Ok(())
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
