use std::{
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use async_recursion::async_recursion;
use ed25519_dalek::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::commands::{
    AttestArgs, FilterCommand, IndexingCommand, LinkCommand, MetaCommand, MirrorCommand, ModerationCommand, ReportArgs,
    TrustCommand,
};
use syncweb_core::{
    folder::{FolderManager, SyncwebFolder},
    indexing::{
        Attestation, AttestationKind, CatalogRecord, ContentLink, DenylistRule, FilterList, IndexingService, Link,
        LinkResolver, MetadataEntry, ModerationAction, ModerationContext, ModerationRecord, MutablePointer,
        PrivateLink, ReplicationBudget, ResilienceConfig, TrustDecision, TrustDelegation, TrustPolicy, WotService,
    },
    node::{
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};

use iroh::PublicKey;
use iroh_blobs::{Hash, ticket::BlobTicket};
use iroh_docs::NamespaceId;

const DEFAULT_PRIVATE_LINK_TTL: u64 = 30 * 24 * 60 * 60;

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
pub async fn handle_indexing(data_dir: &Path, command: IndexingCommand, output_json: bool) -> Result<()> {
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
        IndexingCommand::Meta { command: meta_command } => handle_meta(data_dir, meta_command, output_json)?,
        IndexingCommand::Filter {
            command: filter_command,
        } => handle_filter(data_dir, filter_command, output_json)?,
    }
    Ok(())
}

pub fn handle_link(data_dir: &Path, command: LinkCommand, output_json: bool) -> Result<()> {
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

pub fn handle_mirror(data_dir: &Path, command: MirrorCommand, output_json: bool) -> Result<()> {
    match command {
        MirrorCommand::Add { collection, provider } => {
            let ticket = provider.parse::<BlobTicket>()?;
            let expected_hash = collection_hash(data_dir, &collection)?;
            ensure!(
                expected_hash.is_none_or(|hash| hash == ticket.hash()),
                "mirror provider hash does not match collection"
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
                format!("mirror added: {}\nprovider: {provider}", ticket.hash()),
            )?;
        }
    }
    Ok(())
}

pub fn handle_trust(data_dir: &Path, command: TrustCommand, output_json: bool) -> Result<()> {
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
    }
    Ok(())
}

pub fn handle_attest(data_dir: &Path, command: AttestArgs, output_json: bool) -> Result<()> {
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

pub fn handle_report(data_dir: &Path, command: ReportArgs, output_json: bool) -> Result<()> {
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

pub fn handle_moderation(data_dir: &Path, command: ModerationCommand, output_json: bool) -> Result<()> {
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

fn handle_meta(data_dir: &Path, command: MetaCommand, output_json: bool) -> Result<()> {
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

fn handle_filter(data_dir: &Path, command: FilterCommand, output_json: bool) -> Result<()> {
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
    Ok(wot)
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

async fn open_node(data_dir: &Path) -> Result<IrohNode> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    Ok(IrohNode::new(identity, data_dir.join("data"), RelayMode::Default).await?)
}

async fn resolve_folder(manager: &FolderManager, selector: &Path) -> Result<SyncwebFolder> {
    if let Ok(namespace) = selector.to_string_lossy().parse::<NamespaceId>() {
        return Ok(manager.get(namespace).await?);
    }
    let folders = manager.list().await?;
    match folders.as_slice() {
        [folder] => Ok(folder.clone()),
        [] => anyhow::bail!("no synchronized folders are available"),
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
