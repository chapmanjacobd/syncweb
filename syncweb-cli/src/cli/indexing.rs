use std::{
    fs,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

use anyhow::{Context, Result, ensure};
use async_recursion::async_recursion;
use serde::Serialize;

use super::args::CliContext;
use super::commands::{FilterCommand, IndexingCommand, LinkCommand, ProviderCommand, PublishCatalogArgs};
use syncweb_core::{
    folder::FolderManager,
    indexing::{DenylistRule, FilterList, IndexingService, Link, LinkResolver, LinkStore, PrivateLink},
    node::identity::IdentityManager,
};

use iroh_blobs::{Hash, ticket::BlobTicket};
use syncweb_core::init::open_node;

use super::output::confirm_destructive;

#[async_recursion]
pub async fn handle_indexing(ctx: &CliContext<'_>, command: IndexingCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        IndexingCommand::Enable { folder } => {
            let node = open_node(data_dir).await?;
            let manager = FolderManager::new(&node);
            let selected = manager.resolve(&folder).await?;
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
            let selected = manager.resolve(&folder).await?;
            let selected_namespace = selected.namespace_id();
            open_indexing(data_dir)?.disable_folder(selected_namespace).await?;
            print_status(
                output_json,
                serde_json::json!({"status": "disabled", "namespace": selected_namespace.to_string()}),
                format!("disabled: {selected_namespace}"),
            )?;
            node.stop().await?;
        }
        IndexingCommand::Filter {
            command: filter_command,
        } => handle_filter(ctx, filter_command)?,
    }
    Ok(())
}

pub async fn handle_catalog_publish(ctx: &CliContext<'_>, args: PublishCatalogArgs) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let PublishCatalogArgs { folder, catalog, tags } = args;
    let node = open_node(data_dir).await?;
    let manager = FolderManager::new(&node);
    let selected = manager.resolve(&folder).await?;
    let indexing = open_indexing(data_dir)?;
    indexing.enable_folder(&selected).await?;
    let catalog_service = indexing.catalog_service(
        node.docs_engine(),
        node.blob_store(),
        node.docs_engine().author().await?,
    );
    let catalog_handle = catalog_service.get_or_create_catalog(&catalog).await?;
    let published = catalog_service
        .publish_folder_with_metadata(&catalog_handle, &selected, selected.namespace_id().to_string(), &tags)
        .await?;
    let ticket = catalog_service.ticket(&catalog_handle, false).await?;
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
        LinkCommand::Resolve {
            link,
            version,
            no_fetch,
        } => {
            let store = open_link_store(data_dir)?;
            let resolver: LinkResolver = store.resolver().clone();
            let parsed = link.parse::<Link>()?;
            let resolution = if let Some(version_value) = version {
                let Link::Name(name) = &parsed else {
                    anyhow::bail!("--version is only valid for mutable name links");
                };
                resolver.resolve_version(name, &version_value)?
            } else {
                resolver.resolve(&parsed)?
            };
            let fetched = if no_fetch {
                false
            } else {
                let node = open_node(data_dir).await?;
                let fetch_result = fetch_and_pin_blob(&node, resolution.manifest, &resolution.tickets).await;
                node.stop().await?;
                fetch_result?;
                true
            };
            print_status(
                output_json,
                serde_json::json!({
                    "link": link,
                    "manifest": resolution.manifest.to_string(),
                    "version": resolution.version,
                    "sequence": resolution.sequence,
                    "fetched": fetched,
                    "tickets": resolution.tickets.iter().map(ToString::to_string).collect::<Vec<_>>(),
                }),
                format!(
                    "manifest: {}\nversion: {}\nsequence: {}\nfetched: {}",
                    resolution.manifest,
                    resolution.version.as_deref().unwrap_or("-"),
                    resolution
                        .sequence
                        .map_or_else(|| "-".to_owned(), |value| value.to_string()),
                    fetched,
                ),
            )?;
        }
        LinkCommand::Revoke { link } => {
            if !confirm_destructive("revoke this link", output_json)? {
                println!("aborted");
                return Ok(());
            }
            let parsed = link.parse::<PrivateLink>()?;
            open_link_store(data_dir)?.revoke(&parsed)?;
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

async fn handle_link_create(ctx: &CliContext<'_>, opts: LinkCreateOptions) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let store = open_link_store(data_dir)?;
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let hash = syncweb_core::indexing::hash_source(&opts.source)?;
    let link = if opts.private {
        store.create_private_link(hash, opts.expires)?
    } else if let Some(alias) = opts.name {
        store.create_named_link(&identity, alias, hash, opts.version, opts.sequence)?
    } else {
        ensure!(opts.version.is_none(), "--version requires --name");
        ensure!(opts.sequence == 0, "--sequence requires --name");
        store.create_content_link(hash)
    };

    if let Some(namespace_str) = opts.publish {
        let node = open_node(data_dir).await?;
        let publish_result = store.publish_link(&node, &namespace_str, &link).await;
        node.stop().await?;
        publish_result?;
    }

    print_status(
        output_json,
        serde_json::json!({"status": "created", "link": link.to_string(), "hash": hash.to_string()}),
        format!("link: {link}\nhash: {hash}"),
    )?;
    Ok(())
}

pub fn handle_provider(ctx: &CliContext<'_>, command: ProviderCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        ProviderCommand::Add { collection, provider } => {
            let ticket = provider.parse::<BlobTicket>()?;
            let store = open_link_store(data_dir)?;
            let expected_hash = store.collection_hash(&collection)?;
            ensure!(
                expected_hash.is_none_or(|hash| hash == ticket.hash()),
                "provider hash does not match collection"
            );
            store.register_mirror(ticket.clone())?;
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
    _min_providers: usize,
    _no_sharing: bool,
    export_path: Option<&Path>,
) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    let first_ticket = tickets.first().context("tickets list is empty")?;
    let content_hash = first_ticket.hash();

    let node = open_node(data_dir).await?;
    fetch_and_pin_blob(&node, content_hash, tickets).await?;

    if let Some(path) = export_path {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        node.blob_store()
            .export_to_path_with_mode(content_hash, path, iroh_blobs::api::blobs::ExportMode::Copy)
            .await?;
    }

    if output_json {
        println!(
            "{}",
            serde_json::json!({"hash": content_hash.to_string(), "pinned": true})
        );
    } else {
        println!("hash: {content_hash}");
        println!("pinned: true");
    }
    node.stop().await?;
    Ok(())
}

/// Fetch a content hash through the first working provider ticket and pin it,
/// skipping the fetch when the blob is already present locally.
pub async fn fetch_and_pin_blob(
    node: &syncweb_core::node::iroh_node::IrohNode,
    content_hash: Hash,
    tickets: &[BlobTicket],
) -> Result<()> {
    node.blob_store()
        .fetch_and_pin(node.endpoint(), content_hash, tickets)
        .await
        .map_err(anyhow::Error::from)
}

fn handle_filter(ctx: &CliContext<'_>, command: FilterCommand) -> Result<()> {
    let data_dir = ctx.data_dir;
    let output_json = ctx.output_json;
    match command {
        FilterCommand::Add { rule_type, value } => {
            let rule = parse_denylist_rule(&rule_type, &value)?;
            let indexing = open_indexing(data_dir)?;
            indexing.denylist_service().add(rule)?;
            print_status(
                output_json,
                serde_json::json!({"status": "added", "type": rule_type, "value": value}),
                format!("filter added: {rule_type}\t{value}"),
            )?;
        }
        FilterCommand::Subscribe { source } => {
            let bytes = read_filter_source(&source)?;
            let list = FilterList::from_bytes(bytes)?;
            list.verify_signature()?;
            let indexing = open_indexing(data_dir)?;
            let changed = indexing.denylist_service().sync_filter_list(&list)?;
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

fn open_indexing(data_dir: &Path) -> Result<IndexingService> {
    Ok(IndexingService::new(data_dir.join("indexing.sqlite"))?)
}

fn open_link_store(data_dir: &Path) -> Result<LinkStore> {
    Ok(LinkStore::open(data_dir.join("indexing.sqlite"))?)
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
