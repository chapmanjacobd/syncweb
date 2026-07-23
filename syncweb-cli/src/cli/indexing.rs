use std::{
    fs,
    io::IsTerminal,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use async_recursion::async_recursion;
use ed25519_dalek::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::args::CliContext;
use super::commands::{
    AttestArgs, FilterCommand, IndexingCommand, LinkCommand, MetaCommand, ModerationCommand, ProviderCommand,
    ProviderTrustCommand, ReportArgs, TrustCommand, TrustStreamCommand,
};
use syncweb_core::{
    folder::{FolderManager, SyncwebFolder},
    indexing::{
        Attestation, AttestationKind, BanRecord, CatalogRecord, ContentLink, DenylistRule, FilterList, IndexingService,
        Link, LinkResolver, MetadataEntry, ModerationAction, ModerationContext, ModerationRecord, MutablePointer,
        PrivateLink, ProviderLease, ProviderReputationStore, ProviderTrustAction, ProviderTrustDecision,
        ProviderTrustRecord, ProviderTrustSignal, ReplicationBudget, ReputationConfig, ResilienceConfig,
        ResilienceService, TrustDecision, TrustDelegation, TrustPolicy, TrustSignalKind, WotService,
    },
    node::identity::IdentityManager,
};

use dialoguer::Confirm;
use iroh::PublicKey;
use iroh_blobs::{
    Hash,
    api::blobs::ExportMode,
    get::fsm::{self, ConnectedNext, EndBlobNext},
    protocol::GetRequest,
    ticket::BlobTicket,
};
use iroh_docs::NamespaceId;
use iroh_gossip::api::Event;
use n0_future::StreamExt;
use syncweb_core::init::open_node;

const DEFAULT_PRIVATE_LINK_TTL: u64 = 30 * 24 * 60 * 60;
const TRUST_SIGNAL_TTL_SECONDS: u64 = 7 * 24 * 60 * 60;
const ERR_NO_FOLDERS: &str = "no synchronized folders are available; use `syncweb folders` to list available folders";

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
    #[serde(default)]
    delegations: Vec<TrustDelegation>,
    #[serde(default)]
    moderation: Vec<ModerationRecord>,
    #[serde(default)]
    attestations: Vec<Attestation>,
    #[serde(default)]
    reports: Vec<ReportRecord>,
    #[serde(default)]
    provider_bans: Vec<BanRecord>,
    #[serde(default)]
    provider_trust: Vec<ProviderTrustRecord>,
    #[serde(default)]
    trust_signals: Vec<ProviderTrustSignal>,
    #[serde(default)]
    trust_streams: Vec<String>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ReportRecord {
    content: Hash,
    reason: String,
    created_at: u64,
}

#[async_recursion]
pub async fn handle_indexing(ctx: &CliContext<'_>, command: IndexingCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        IndexingCommand::Enable { folder } => {
            let node = open_node(data_dir).await?;
            let manager = FolderManager::new(&node);
            let selected = resolve_folder(&manager, &folder).await?;
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
        IndexingCommand::Disable { folder } => {
            let node = open_node(data_dir).await?;
            let manager = FolderManager::new(&node);
            let selected = resolve_folder(&manager, &folder).await?;
            let namespace = selected.namespace_id();
            open_indexing(data_dir)?.disable_folder(namespace).await?;
            print_status(
                output_json,
                serde_json::json!({"status": "disabled", "namespace": namespace.to_string()}),
                format!("disabled: {namespace}"),
            )?;
            node.stop().await?;
        }
        IndexingCommand::Publish { folder, catalog, tags } => {
            let node = open_node(data_dir).await?;
            let manager = FolderManager::new(&node);
            let selected = resolve_folder(&manager, &folder).await?;
            let indexing = open_indexing(data_dir)?;
            let catalog_service = indexing.catalog_service(
                node.docs_engine(),
                node.blob_store(),
                node.docs_engine().author().await?,
            );
            let mut state = load_state(data_dir)?;
            let catalog_handle =
                if let Some(existing) = state.catalogs.iter().find(|item| item.name == catalog).cloned() {
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
            save_state(data_dir, &state)?;
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
        }
        IndexingCommand::Search { query, limit } => {
            let results = open_indexing(data_dir)?.search(&query, limit)?;
            if results.is_empty() {
                println!("no results found for query: {query}");
                return Ok(());
            }
            print_catalog_results(&results, output_json)?;
        }
        IndexingCommand::Health { hash } => {
            let content_hash = parse_hash(&hash)?;
            let state = load_state(data_dir)?;
            let resilience =
                open_indexing(data_dir)?.resilience_service(ResilienceConfig::new(ReplicationBudget::default()));
            let now = epoch_seconds();
            for lease in state.leases {
                if !lease.is_expired_at(now) {
                    resilience.record_lease(lease)?;
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

pub fn handle_link(ctx: &CliContext<'_>, command: LinkCommand) -> Result<()> {
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
        } => {
            let hash = hash_source(&source)?;
            let mut state = load_state(data_dir)?;
            let link = if private {
                let expires_at = expires.unwrap_or_else(|| epoch_seconds().saturating_add(DEFAULT_PRIVATE_LINK_TTL));
                let link = PrivateLink::generate(hash, expires_at)?;
                state.links.revoked.retain(|existing| existing != &link);
                Link::Private(link)
            } else if let Some(alias) = name {
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
                Link::Name(link)
            } else {
                ensure!(version.is_none(), "--version requires --name");
                ensure!(sequence == 0, "--sequence requires --name");
                Link::Content(ContentLink::new(hash))
            };
            save_state(data_dir, &state)?;
            print_status(
                output_json,
                serde_json::json!({"status": "created", "link": link.to_string(), "hash": hash.to_string()}),
                format!("link: {link}\nhash: {hash}"),
            )?;
        }
        LinkCommand::Resolve { link, version } => {
            let state = load_state(data_dir)?;
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
            let mut state = load_state(data_dir)?;
            let resolver = load_resolver(&state)?;
            resolver.revoke(&parsed)?;
            if !state.links.revoked.contains(&parsed) {
                state.links.revoked.push(parsed);
            }
            save_state(data_dir, &state)?;
            print_status(
                output_json,
                serde_json::json!({"status": "revoked", "link": link}),
                format!("revoked: {link}"),
            )?;
        }
    }
    Ok(())
}

pub fn handle_provider(ctx: &CliContext<'_>, command: ProviderCommand) -> Result<()> {
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
            let mut state = load_state(data_dir)?;
            let resolver = load_resolver(&state)?;
            resolver.register_mirror(ticket.clone())?;
            if !state.links.mirrors.contains(&provider) {
                state.links.mirrors.push(provider.clone());
            }
            save_state(data_dir, &state)?;
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
            let pin_name = format!("syncweb/download/{content_hash}");
            node.blob_store().pin(&pin_name, content_hash).await?;
            node.stop().await?;
        }
    } else {
        let node = open_node(data_dir).await?;
        let resilience = ResilienceService::new(ResilienceConfig::new(ReplicationBudget::new(min_providers)));
        for ticket in tickets {
            let expires_at = epoch_seconds().saturating_add(365 * 24 * 60 * 60);
            let lease = ProviderLease::new(ticket.hash(), ticket.to_string(), 0, expires_at)?;
            resilience.record_lease(lease)?;
        }

        let result = resilience
            .ensure_replication(node.endpoint(), node.blob_store(), content_hash)
            .await?;

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

#[async_recursion]
pub async fn handle_trust(ctx: &CliContext<'_>, command: TrustCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        TrustCommand::Show { subject } => {
            let state = load_state(data_dir)?;
            let indexing = open_indexing(data_dir)?;
            let wot = load_wot(&indexing, &state)?;
            let identity = IdentityManager::new(data_dir.join("identity.key"))?;
            let own_author = author_id(&signing_key(&identity).verifying_key());
            let (trust, content, moderation, metadata, attestations) = if let Ok(hash) = subject.parse::<Hash>() {
                let metadata = wot
                    .search("", 10_000)?
                    .into_iter()
                    .filter(|entry| entry.content == hash)
                    .collect::<Vec<_>>();
                let _moderation = wot.moderation(&hash)?;
                let decision = wot.moderation_decision(&ModerationContext::new(hash))?;
                let attestations = state
                    .attestations
                    .iter()
                    .filter(|entry| entry.content == hash)
                    .cloned()
                    .collect::<Vec<_>>();
                (
                    wot.policy()?.evaluate_for(&own_author, Some(&hash)),
                    Some(hash),
                    Some(decision),
                    metadata,
                    attestations,
                )
            } else {
                let publisher = parse_verifying_key(&subject)?;
                (
                    wot.policy()?.evaluate(&author_id(&publisher)),
                    None,
                    None,
                    Vec::new(),
                    Vec::new(),
                )
            };
            print_status(
                output_json,
                serde_json::json!({
                    "subject": subject,
                    "trust": trust_label(trust),
                    "content": content.map(|hash| hash.to_string()),
                    "moderation": moderation.as_ref().map(moderation_label),
                    "metadata": metadata,
                    "attestations": attestations,
                }),
                format!(
                    "subject: {subject}\ntrust: {}\nmoderation: {}",
                    trust_label(trust),
                    moderation.as_ref().map_or("-", moderation_label)
                ),
            )?;
        }
        TrustCommand::Delegate {
            publisher,
            expires,
            scope: requested_scope,
            sequence,
        } => {
            let identity = IdentityManager::new(data_dir.join("identity.key"))?;
            let signing = signing_key(&identity);
            let delegate = parse_verifying_key(&publisher)?;
            let scope = requested_scope.map(|value| parse_hash(&value)).transpose()?;
            let expires_at = expires.unwrap_or_else(|| epoch_seconds().saturating_add(365 * 24 * 60 * 60));
            let delegation = TrustDelegation::new(&delegate, scope, sequence, expires_at, &signing)?;
            let indexing = open_indexing(data_dir)?;
            let mut state = load_state(data_dir)?;
            let wot = load_wot(&indexing, &state)?;
            let inserted = wot.add_delegation(delegation.clone())?;
            if inserted {
                state.delegations.push(delegation.clone());
                save_state(data_dir, &state)?;
            }
            print_status(
                output_json,
                serde_json::json!({
                    "status": if inserted { "delegated" } else { "unchanged" },
                    "publisher": delegation.delegate,
                    "expires_at": delegation.expires_at,
                    "scope": delegation.scope.map(|hash| hash.to_string()),
                }),
                format!(
                    "delegated: {}\nexpires_at: {}",
                    delegation.delegate, delegation.expires_at
                ),
            )?;
        }
        TrustCommand::Provider {
            command: provider_command,
        } => handle_provider_trust(ctx, provider_command)?,
        TrustCommand::Stream {
            command: stream_command,
        } => handle_trust_stream(ctx, stream_command).await?,
    }
    Ok(())
}

fn handle_provider_trust(ctx: &CliContext<'_>, command: ProviderTrustCommand) -> Result<()> {
    match command {
        ProviderTrustCommand::Show { provider, hash } => {
            handle_provider_show(ctx, &provider, hash.as_deref())?;
        }
        ProviderTrustCommand::List { hash } => {
            handle_provider_list(ctx, hash.as_deref())?;
        }
        ProviderTrustCommand::Ban {
            provider,
            hash,
            reason,
            duration,
        } => handle_provider_ban(ctx, &provider, hash.as_deref(), reason, duration)?,
        ProviderTrustCommand::Unban { provider } => {
            handle_provider_unban(ctx, &provider)?;
        }
        ProviderTrustCommand::Vouch {
            provider,
            scope,
            reason,
        } => handle_provider_trust_record(ctx, &provider, scope.as_deref(), reason, ProviderTrustAction::Vouch)?,
        ProviderTrustCommand::Distrust {
            provider,
            scope,
            reason,
        } => handle_provider_trust_record(ctx, &provider, scope.as_deref(), reason, ProviderTrustAction::Distrust)?,
    }
    Ok(())
}

fn handle_provider_show(ctx: &CliContext<'_>, provider: &str, hash: Option<&str>) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let provider_key = parse_provider(provider)?;
    let scope = hash.map(parse_hash).transpose()?;
    let state = load_state(data_dir)?;
    let indexing = open_indexing(data_dir)?;
    let wot = load_wot(&indexing, &state)?;
    let reputation = load_reputation(&wot, &state)?;
    let resilience = load_resilience(&state)?;
    let now = epoch_seconds();
    let records = wot.provider_trust_records(provider_key)?;
    let bans = active_provider_bans(&state, provider_key, scope.as_ref(), now);
    let decision = wot.evaluate_provider_trust(provider_key, scope.as_ref(), now)?;
    let health = scope
        .as_ref()
        .map(|content_hash| resilience.health(content_hash))
        .transpose()?
        .map(|health| {
            serde_json::json!({
                "verified": health.verified,
                "local": health.local,
                "verified_providers": health.verified_providers,
                "local_providers": health.local_providers,
            })
        });
    let report = serde_json::json!({
        "provider": provider_key,
        "trust": provider_trust_label(decision),
        "score": reputation.score(provider_key, now),
        "reputation": reputation.reputation(provider_key),
        "bans": bans,
        "records": records,
        "health": health,
    });
    print_status(
        output_json,
        report,
        format!(
            "provider: {provider_key}\ntrust: {}\nscore: {:.3}\nbans: {}",
            provider_trust_label(decision),
            reputation.score(provider_key, now),
            bans.len()
        ),
    )
}

fn handle_provider_list(ctx: &CliContext<'_>, hash: Option<&str>) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let scope = hash.map(parse_hash).transpose()?;
    let state = load_state(data_dir)?;
    let indexing = open_indexing(data_dir)?;
    let wot = load_wot(&indexing, &state)?;
    let reputation = load_reputation(&wot, &state)?;
    let now = epoch_seconds();
    let mut providers = state
        .leases
        .iter()
        .map(|lease| lease.provider)
        .chain(state.provider_bans.iter().map(|ban| ban.provider))
        .chain(state.provider_trust.iter().map(|record| record.provider))
        .chain(state.trust_signals.iter().map(|signal| signal.provider))
        .collect::<Vec<_>>();
    providers.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    providers.dedup();
    let reports = providers
        .into_iter()
        .map(|provider_key| {
            let decision = wot.evaluate_provider_trust(provider_key, scope.as_ref(), now)?;
            let records = wot.provider_trust_records(provider_key)?;
            Ok(serde_json::json!({
                "provider": provider_key,
                "trust": provider_trust_label(decision),
                "score": reputation.score(provider_key, now),
                "bans": active_provider_bans(&state, provider_key, scope.as_ref(), now),
                "records": records.len(),
            }))
        })
        .collect::<Result<Vec<_>>>()?;
    if output_json {
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        for report in reports {
            let provider = report
                .get("provider")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("-");
            let trust = report.get("trust").and_then(serde_json::Value::as_str).unwrap_or("-");
            let provider_score = report.get("score").and_then(serde_json::Value::as_f64).unwrap_or(0.5);
            let bans = report
                .get("bans")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len);
            println!("{provider}\t{trust}\t{provider_score:.3}\t{bans}");
        }
    }
    Ok(())
}

fn handle_provider_ban(
    ctx: &CliContext<'_>,
    provider: &str,
    hash: Option<&str>,
    reason: String,
    duration: Option<u64>,
) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    if !confirm_destructive("ban this provider", output_json)? {
        println!("aborted");
        return Ok(());
    }
    let provider_key = parse_provider(provider)?;
    let scope = hash.map(parse_hash).transpose()?;
    let ban_duration = duration.map(Duration::from_secs);
    let mut state = load_state(data_dir)?;
    let resilience = load_resilience(&state)?;
    let ban = resilience.ban_provider(provider_key, reason, scope, ban_duration)?;
    state
        .provider_bans
        .retain(|existing| !(existing.provider == provider_key && existing.hash == ban.hash));
    state.provider_bans.push(ban.clone());
    save_state(data_dir, &state)?;
    print_status(
        output_json,
        serde_json::json!({"status": "banned", "ban": ban}),
        format!("banned: {provider_key}"),
    )
}

fn handle_provider_unban(ctx: &CliContext<'_>, provider: &str) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    if !confirm_destructive("unban this provider", output_json)? {
        println!("aborted");
        return Ok(());
    }
    let provider_key = parse_provider(provider)?;
    let mut state = load_state(data_dir)?;
    let removed = state.provider_bans.iter().any(|ban| ban.provider == provider_key);
    state.provider_bans.retain(|ban| ban.provider != provider_key);
    if removed {
        save_state(data_dir, &state)?;
    }
    print_status(
        output_json,
        serde_json::json!({"status": if removed { "unbanned" } else { "unchanged" }, "provider": provider_key}),
        format!("{}: {provider_key}", if removed { "unbanned" } else { "unchanged" }),
    )
}

fn handle_provider_trust_record(
    ctx: &CliContext<'_>,
    provider: &str,
    scope: Option<&str>,
    reason: String,
    action: ProviderTrustAction,
) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let provider_key = parse_provider(provider)?;
    let scope_hash = scope.map(parse_hash).transpose()?;
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let signing = signing_key(&identity);
    let issuer = author_id(&signing.verifying_key());
    let mut state = load_state(data_dir)?;
    let sequence = state
        .provider_trust
        .iter()
        .filter(|record| record.provider == provider_key && record.issuer == issuer && record.scope == scope_hash)
        .map(|record| record.sequence)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let record = ProviderTrustRecord::new(provider_key, action, scope_hash, sequence, None, reason, &signing)?;
    let indexing = open_indexing(data_dir)?;
    let wot = load_wot(&indexing, &state)?;
    let inserted = wot.apply_provider_trust(record.clone())?;
    if inserted {
        state.provider_trust.push(record.clone());
        save_state(data_dir, &state)?;
    }
    print_status(
        output_json,
        serde_json::json!({
            "status": if inserted { "updated" } else { "unchanged" },
            "provider": provider_key,
            "action": provider_trust_action_label(&record.action),
            "scope": record.scope.map(|value| value.to_string()),
            "sequence": record.sequence,
        }),
        format!("{}: {provider_key}", provider_trust_action_label(&record.action)),
    )
}

#[async_recursion]
async fn handle_trust_stream(ctx: &CliContext<'_>, command: TrustStreamCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        TrustStreamCommand::Publish {
            provider,
            signal,
            hash,
            sequence,
        } => {
            let provider_key = parse_provider(&provider)?;
            let signal_kind = parse_signal_kind(&signal)?;
            let scope = hash.map(|value| parse_hash(&value)).transpose()?;
            let identity = IdentityManager::new(data_dir.join("identity.key"))?;
            let signing = signing_key(&identity);
            let reporter = PublicKey::from_bytes(&signing.verifying_key().to_bytes())?;
            let mut state = load_state(data_dir)?;
            let next_sequence = sequence.unwrap_or_else(|| {
                state
                    .trust_signals
                    .iter()
                    .filter(|existing| existing.provider == provider_key && existing.reporter == reporter)
                    .map(|existing| existing.sequence)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1)
            });
            let trust_signal =
                ProviderTrustSignal::new_with_time(provider_key, signal_kind, scope, next_sequence, &signing)?;

            let node = open_node(data_dir).await?;
            let gossip_store = ProviderReputationStore::default();
            let result = async {
                let topic = gossip_store
                    .subscribe_trust_stream(node.gossip_service(), Vec::new())
                    .await?;
                let (sender, _receiver) = syncweb_core::node::gossip_service::GossipService::split(topic);
                gossip_store
                    .publish_signal(node.gossip_service(), &sender, &trust_signal)
                    .await
            }
            .await;
            node.stop().await?;
            result?;

            if !state.trust_signals.contains(&trust_signal) {
                state.trust_signals.push(trust_signal.clone());
            }
            let stream_path = data_dir.join("trust-stream.json");
            fs::write(&stream_path, serde_json::to_vec_pretty(&state.trust_signals)?)?;
            save_state(data_dir, &state)?;
            print_status(
                output_json,
                serde_json::json!({
                    "status": "published",
                    "provider": provider_key,
                    "signal": trust_signal_label(signal_kind),
                    "sequence": trust_signal.sequence,
                    "ticket": format!("file://{}", stream_path.display()),
                    "bootstrap": node.endpoint().addr().id,
                }),
                format!(
                    "published: {}\nprovider: {provider_key}\nticket: file://{}",
                    trust_signal_label(signal_kind),
                    stream_path.display()
                ),
            )?;
        }
        TrustStreamCommand::Subscribe { ticket } => {
            let imported = if let Some(bytes) = read_trust_stream_source(&ticket)? {
                parse_trust_signals(&bytes)?
            } else if let Ok(bootstrap) = parse_provider(&ticket) {
                receive_trust_signals(data_dir, bootstrap).await?
            } else {
                anyhow::bail!("invalid trust stream ticket or source: {ticket}");
            };
            let mut state = load_state(data_dir)?;
            let indexing = open_indexing(data_dir)?;
            let wot = load_wot(&indexing, &state)?;
            let mut reputation = load_reputation(&wot, &state)?;
            let mut accepted = 0_usize;
            for signal in imported {
                if state.trust_signals.contains(&signal) {
                    continue;
                }
                if reputation.ingest_trust_signal(signal.clone())? {
                    state.trust_signals.push(signal);
                    accepted = accepted.saturating_add(1);
                }
            }
            if !state.trust_streams.contains(&ticket) {
                state.trust_streams.push(ticket.clone());
            }
            save_state(data_dir, &state)?;
            print_status(
                output_json,
                serde_json::json!({
                    "status": "subscribed",
                    "ticket": ticket,
                    "accepted": accepted,
                    "signals": state.trust_signals.len(),
                }),
                format!("subscribed: {ticket}\naccepted: {accepted}"),
            )?;
        }
    }
    Ok(())
}

async fn receive_trust_signals(data_dir: &Path, bootstrap: PublicKey) -> Result<Vec<ProviderTrustSignal>> {
    let state = load_state(data_dir)?;
    let indexing = open_indexing(data_dir)?;
    let wot = load_wot(&indexing, &state)?;
    let reputation = load_reputation(&wot, &state)?;
    let node = open_node(data_dir).await?;
    let mut topic = reputation
        .subscribe_trust_stream(node.gossip_service(), vec![bootstrap])
        .await?;
    let mut signals = Vec::new();
    loop {
        let timed_event = tokio::time::timeout(Duration::from_millis(250), topic.next()).await;
        let Ok(Some(event)) = timed_event else {
            break;
        };
        if let Event::Received(message) =
            event.map_err(|error| anyhow::anyhow!("trust stream event failed: {error}"))?
        {
            let signal = ProviderTrustSignal::from_bytes(message.content)?;
            signal.verify()?;
            signals.push(signal);
        }
    }
    node.stop().await?;
    Ok(signals)
}

fn parse_trust_signals(bytes: &[u8]) -> Result<Vec<ProviderTrustSignal>> {
    let signals = serde_json::from_slice::<Vec<ProviderTrustSignal>>(bytes)
        .or_else(|_| serde_json::from_slice::<ProviderTrustSignal>(bytes).map(|signal| vec![signal]))
        .context("trust stream must contain a signed signal or signal array")?;
    for signal in &signals {
        signal.verify()?;
    }
    Ok(signals)
}

fn read_trust_stream_source(source: &str) -> Result<Option<Vec<u8>>> {
    let path = source.strip_prefix("file://").unwrap_or(source);
    if Path::new(path).is_file() {
        return Ok(Some(fs::read(path)?));
    }
    if source.trim_start().starts_with('{') || source.trim_start().starts_with('[') {
        return Ok(Some(source.as_bytes().to_vec()));
    }
    Ok(None)
}

pub fn handle_attest(ctx: &CliContext<'_>, command: AttestArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let value = command
        .license
        .map(|value| (AttestationKind::License, value))
        .or_else(|| command.provenance.map(|value| (AttestationKind::Provenance, value)))
        .or_else(|| command.derivative.map(|value| (AttestationKind::Derivative, value)))
        .context("one of --license, --provenance, or --derivative is required")?;
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let attestation = Attestation::new(
        parse_hash(&command.content)?,
        value.0,
        value.1,
        command.sequence,
        &signing_key(&identity),
    )?;
    let indexing = open_indexing(data_dir)?;
    let mut state = load_state(data_dir)?;
    let inserted = if state.attestations.contains(&attestation) {
        false
    } else {
        load_wot(&indexing, &state)?.append_attestation(attestation.clone())?
    };
    if inserted {
        state.attestations.push(attestation.clone());
        save_state(data_dir, &state)?;
    }
    print_status(
        output_json,
        serde_json::json!({
            "status": if inserted { "attested" } else { "unchanged" },
            "content": attestation.content.to_string(),
            "issuer": attestation.issuer,
            "value": attestation.value,
        }),
        format!("attested: {}\nvalue: {}", attestation.content, attestation.value),
    )
}

pub fn handle_report(ctx: &CliContext<'_>, command: ReportArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let content = parse_hash(&command.record)?;
    let mut state = load_state(data_dir)?;
    let report = ReportRecord {
        content,
        reason: command.reason,
        created_at: epoch_seconds(),
    };
    state.reports.push(report.clone());
    save_state(data_dir, &state)?;
    print_status(
        output_json,
        serde_json::json!({"status": "reported", "content": content.to_string(), "reason": report.reason}),
        format!("reported: {content}\nreason: {}", report.reason),
    )
}

pub fn handle_moderation(ctx: &CliContext<'_>, command: ModerationCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        ModerationCommand::List { content } => {
            let state = load_state(data_dir)?;
            let filter = content.map(|value| parse_hash(&value)).transpose()?;
            let records = state
                .moderation
                .iter()
                .filter(|record| filter.is_none_or(|hash| hash == record.content))
                .cloned()
                .collect::<Vec<_>>();
            if output_json {
                println!("{}", serde_json::to_string_pretty(&records)?);
            } else {
                for record in records {
                    println!(
                        "{}\t{}\t{}\t{}",
                        record.content,
                        moderation_label(&record.action),
                        record.sequence,
                        record.reason
                    );
                }
            }
        }
        ModerationCommand::Hide { record, reason } => {
            let content = parse_hash(&record)?;
            let identity = IdentityManager::new(data_dir.join("identity.key"))?;
            let signing = signing_key(&identity);
            let mut state = load_state(data_dir)?;
            let sequence = state
                .moderation
                .iter()
                .filter(|existing| existing.content == content)
                .map(|existing| existing.sequence)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            let moderation = ModerationRecord::new(content, ModerationAction::Hide, sequence, reason, &signing)?;
            let inserted = load_wot(&open_indexing(data_dir)?, &state)?.apply_moderation(moderation.clone())?;
            if inserted {
                state.moderation.push(moderation.clone());
                save_state(data_dir, &state)?;
            }
            print_status(
                output_json,
                serde_json::json!({
                    "status": if inserted { "hidden" } else { "unchanged" },
                    "content": content.to_string(),
                    "sequence": moderation.sequence,
                }),
                format!("hidden: {content}"),
            )?;
        }
    }
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
            let mut state = load_state(data_dir)?;
            if !state.denylist.contains(&rule) {
                state.denylist.push(rule);
                save_state(data_dir, &state)?;
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
            let mut state = load_state(data_dir)?;
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
                save_state(data_dir, &state)?;
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
    let state = load_state(data_dir)?;
    load_wot(&indexing, &state)
}

fn open_indexing(data_dir: &Path) -> Result<IndexingService> {
    Ok(IndexingService::new(data_dir.join("indexing.sqlite"))?)
}

fn load_wot(indexing: &IndexingService, state: &IndexingState) -> Result<WotService> {
    let data_dir = indexing.database().path().parent().unwrap_or_else(|| Path::new("."));
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let signing = signing_key(&identity);
    let wot = indexing.wot_service(TrustPolicy::with_root(&signing));
    for delegation in &state.delegations {
        wot.add_delegation(delegation.clone())?;
    }
    for moderation in &state.moderation {
        wot.apply_moderation(moderation.clone())?;
    }
    let now = epoch_seconds();
    for record in &state.provider_trust {
        if record.expires_at.is_none_or(|expires_at| expires_at > now) {
            wot.apply_provider_trust(record.clone())?;
        }
    }
    Ok(wot)
}

fn load_resilience(state: &IndexingState) -> Result<ResilienceService> {
    let resilience = ResilienceService::new(ResilienceConfig::new(ReplicationBudget::default()));
    let now = epoch_seconds();
    for lease in &state.leases {
        if !lease.is_expired_at(now) {
            resilience.record_lease(lease.clone())?;
        }
    }
    for ban in &state.provider_bans {
        let Some(expires_at) = ban.expires_at else {
            resilience.ban_provider(ban.provider, ban.reason.clone(), ban.hash, None)?;
            continue;
        };
        if expires_at > now {
            resilience.ban_provider(
                ban.provider,
                ban.reason.clone(),
                ban.hash,
                Some(Duration::from_secs(expires_at.saturating_sub(now))),
            )?;
        }
    }
    Ok(resilience)
}

fn load_reputation(wot: &WotService, state: &IndexingState) -> Result<ProviderReputationStore> {
    let mut reputation = ProviderReputationStore::with_policy(ReputationConfig::default(), wot.policy()?);
    let now = epoch_seconds();
    for signal in &state.trust_signals {
        if signal.timestamp <= now && now.saturating_sub(signal.timestamp) > TRUST_SIGNAL_TTL_SECONDS {
            continue;
        }
        signal.verify()?;
        reputation.ingest_trust_signal(signal.clone())?;
    }
    Ok(reputation)
}

fn active_provider_bans(state: &IndexingState, provider: PublicKey, scope: Option<&Hash>, now: u64) -> Vec<BanRecord> {
    state
        .provider_bans
        .iter()
        .filter(|ban| {
            ban.provider == provider
                && ban.expires_at.is_none_or(|expires_at| expires_at > now)
                && ban.hash.as_ref().is_none_or(|hash| Some(hash) == scope)
        })
        .cloned()
        .collect()
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
        resolver.publish(pointer)?;
    }
    for mirror in &state.links.mirrors {
        resolver.register_mirror(mirror.parse()?)?;
    }
    for revoked in &state.links.revoked {
        resolver.revoke(revoked)?;
    }
    Ok(resolver)
}

fn load_state(data_dir: &Path) -> Result<IndexingState> {
    let path = data_dir.join("indexing-state.json");
    if !path.exists() {
        return Ok(IndexingState::default());
    }
    serde_json::from_slice(&fs::read(&path)?).with_context(|| format!("invalid indexing state {}", path.display()))
}

fn save_state(data_dir: &Path, state: &IndexingState) -> Result<()> {
    fs::create_dir_all(data_dir)?;
    let path = data_dir.join("indexing-state.json");
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(state)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn signing_key(identity: &IdentityManager) -> SigningKey {
    SigningKey::from_bytes(&identity.secret_key().to_bytes())
}

fn author_id(key: &VerifyingKey) -> String {
    hex::encode(key.to_bytes())
}

fn parse_verifying_key(value: &str) -> Result<VerifyingKey> {
    if let Ok(decoded_bytes) = hex::decode(value) {
        let key_bytes: [u8; 32] = decoded_bytes
            .try_into()
            .map_err(|error| anyhow::anyhow!("publisher identity must contain 32 bytes: {error:?}"))?;
        return VerifyingKey::from_bytes(&key_bytes)
            .map_err(|error| anyhow::anyhow!("invalid publisher identity: {error}"));
    }
    let public_key = value
        .parse::<PublicKey>()
        .map_err(|error| anyhow::anyhow!("invalid publisher identity: {error}"))?;
    VerifyingKey::from_bytes(public_key.as_bytes())
        .map_err(|error| anyhow::anyhow!("invalid publisher identity: {error}"))
}

fn parse_provider(value: &str) -> Result<PublicKey> {
    if let Ok(provider) = value.parse::<PublicKey>() {
        return Ok(provider);
    }
    let verifying_key = parse_verifying_key(value)?;
    PublicKey::from_bytes(&verifying_key.to_bytes())
        .map_err(|error| anyhow::anyhow!("invalid provider identity: {error}"))
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
            Link::Name(_) => load_resolver(&load_state(data_dir)?)?.resolve(&parsed_link)?.manifest,
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

async fn resolve_folder(manager: &FolderManager, selector: &Path) -> Result<SyncwebFolder> {
    if let Ok(namespace) = selector.to_string_lossy().parse::<NamespaceId>() {
        return Ok(manager.get(namespace).await?);
    }
    let folders = manager.list().await?;
    match folders.as_slice() {
        [folder] => Ok(folder.clone()),
        [] => anyhow::bail!(ERR_NO_FOLDERS),
        _ => anyhow::bail!("folder selector is not a namespace ID and more than one synchronized folder is available"),
    }
}

fn print_catalog_results(results: &[CatalogRecord], output_json: bool) -> Result<()> {
    if output_json {
        println!("{}", serde_json::to_string_pretty(results)?);
    } else {
        for result in results {
            println!(
                "{}\t{}\t{}\t{}\t{}",
                result.title,
                result.folder_name,
                result.hash,
                result.size,
                String::from_utf8_lossy(&result.key)
            );
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

fn parse_signal_kind(value: &str) -> Result<TrustSignalKind> {
    match value.to_ascii_lowercase().replace('_', "-").as_str() {
        "success" | "observed-success" => Ok(TrustSignalKind::ObservedSuccess),
        "failure" | "observed-failure" => Ok(TrustSignalKind::ObservedFailure),
        "corruption" | "observed-corruption" => Ok(TrustSignalKind::ObservedCorruption),
        _ => anyhow::bail!("unsupported trust signal {value:?}; use success, failure, or corruption"),
    }
}

const fn trust_signal_label(signal: TrustSignalKind) -> &'static str {
    match signal {
        TrustSignalKind::ObservedSuccess => "success",
        TrustSignalKind::ObservedFailure => "failure",
        TrustSignalKind::ObservedCorruption => "corruption",
        _ => "unknown",
    }
}

const fn provider_trust_action_label(action: &ProviderTrustAction) -> &'static str {
    match action {
        ProviderTrustAction::Trust => "trust",
        ProviderTrustAction::Distrust => "distrust",
        ProviderTrustAction::Vouch => "vouch",
        ProviderTrustAction::Warn => "warn",
        _ => "unknown",
    }
}

const fn provider_trust_label(decision: ProviderTrustDecision) -> &'static str {
    match decision {
        ProviderTrustDecision::Trusted => "trusted",
        ProviderTrustDecision::Distrusted => "distrusted",
        ProviderTrustDecision::Conflicting => "conflicting",
        ProviderTrustDecision::Unknown | _ => "unknown",
    }
}

const fn trust_label(decision: TrustDecision) -> &'static str {
    match decision {
        TrustDecision::TrustedRoot => "trusted-root",
        TrustDecision::TrustedDelegation => "trusted-delegation",
        TrustDecision::Untrusted => "untrusted",
        TrustDecision::Revoked => "revoked",
        _ => "unknown",
    }
}

const fn moderation_label(action: &ModerationAction) -> &'static str {
    match action {
        ModerationAction::Show => "show",
        ModerationAction::Hide => "hide",
        ModerationAction::Warn => "warn",
        ModerationAction::Quarantine => "quarantine",
        ModerationAction::Restore => "restore",
        _ => "unknown",
    }
}
