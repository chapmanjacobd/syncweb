mod test_utils;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use iroh::SecretKey;
use iroh::address_lookup::memory::MemoryLookup;
use iroh_blobs::Hash;
use n0_future::StreamExt;
use syncweb_core::{
    folder::{FolderManager, SyncMode},
    gossip::TopicChannel,
    indexing::{
        ContentLink, Link, LinkResolution, LinkResolver, MutablePointer, PrivateLink, REVOCATION_GOSSIP_TOPIC,
        revocation_topic_id,
    },
    node::{
        gossip_service::GossipService,
        identity::IdentityManager,
        iroh_node::{IrohNode, RelayMode},
    },
};

const fn test_hash(byte: u8) -> Hash {
    Hash::from_bytes([byte; 32])
}

fn test_pointer(seed: u8, alias: &str, hash: Hash, sequence: u64) -> anyhow::Result<MutablePointer> {
    let secret = SecretKey::from_bytes(&[seed; 32]);
    MutablePointer::signed_with_secret_key(secret.public(), alias, hash, sequence, &secret).map_err(anyhow::Error::from)
}

async fn test_node(
    directory: &test_utils::TestDirectory,
    name: &str,
    relay_map: Option<iroh::RelayMap>,
    address_lookup: Option<MemoryLookup>,
) -> anyhow::Result<IrohNode> {
    let root = directory.path().join(name);
    let identity = IdentityManager::new(root.join("identity.key"))?;
    let relay_mode = relay_map.map_or(RelayMode::Default, |map| RelayMode::Custom { map, insecure: true });
    match address_lookup {
        Some(lookup) => Ok(IrohNode::new_with_address_lookup(
            identity,
            root.join("data"),
            relay_mode,
            lookup,
            test_utils::empty_member_keys(),
        )
        .await?),
        None => Ok(IrohNode::new(identity, root.join("data"), relay_mode, test_utils::empty_member_keys()).await?),
    }
}

#[test]
fn test_link_create_is_local_only() {
    let hash = test_hash(1);
    let resolver = LinkResolver::new();
    let secret = SecretKey::from_bytes(&[42; 32]);
    let pointer = MutablePointer::signed_with_secret_key(secret.public(), "my-alias", hash, 1, &secret)
        .expect("pointer should sign");
    resolver.publish(&pointer).expect("pointer should publish");
    let name = pointer.link().expect("pointer should yield a link");

    let resolution = resolver
        .resolve(&Link::Name(name))
        .expect("name should resolve locally");
    assert_eq!(resolution.manifest, hash);
}

#[test]
fn test_link_resolve_never_fetches_network() {
    let resolver = LinkResolver::new();
    let hash = test_hash(0);
    let link = ContentLink::new(hash);
    let resolution = resolver
        .resolve(&Link::Content(link))
        .expect("content link should resolve from in-memory state");
    assert_eq!(resolution.manifest, hash);
}

#[test]
fn test_link_revoke_does_not_propagate() {
    let hash = test_hash(2);
    let far_future = 4_000_000_000;
    let link = PrivateLink::generate(hash, far_future).expect("private link should generate");

    let alice_resolver = LinkResolver::new();
    alice_resolver.revoke(&link).expect("alice should revoke");

    let bob_resolver = LinkResolver::new();
    let bob_resolution = bob_resolver
        .resolve_at(&Link::Private(link), far_future - 1)
        .expect("bob should still resolve — revocation not propagated");
    assert_eq!(bob_resolution.manifest, hash);
}

/// Two-node test: Alice creates a folder, publishes a mutable pointer to
/// the folder doc, Bob joins the folder.
#[tokio::test]
async fn test_link_publish_mutable_to_folder_doc() -> anyhow::Result<()> {
    let directory = test_utils::TestDirectory::new("link-publish")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();
    let alice = test_node(
        &directory,
        "alice",
        Some(relay_map.clone()),
        Some(memory_lookup.clone()),
    )
    .await?;
    let bob = test_node(&directory, "bob", Some(relay_map), Some(memory_lookup.clone())).await?;
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(alice.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(bob.endpoint().id()).with_relay_url(relay_url));

    let alice_manager = FolderManager::new(&alice);
    let folder = alice_manager.create(SyncMode::SendReceive).await?;

    let hash = alice.blob_store().add_bytes(b"hello").await?;
    let pointer =
        MutablePointer::signed_with_secret_key(alice.endpoint().id(), "docs", hash, 1, alice.endpoint().secret_key())?;
    let payload = serde_json::to_vec(&pointer)?;
    folder.set_blob("sys/links/mutable/docs", payload).await?;

    let ticket = folder.ticket(alice.endpoint().addr(), true).await?;
    let bob_manager = FolderManager::new(&bob);
    let bob_folder = bob_manager.join(ticket.to_string(), SyncMode::ReceiveOnly).await?;

    // Trigger sync so Bob receives Alice's entry
    alice
        .docs_engine()
        .start_sync(folder.doc(), vec![bob.endpoint().addr()])
        .await?;
    bob.docs_engine()
        .start_sync(bob_folder.doc(), vec![alice.endpoint().addr()])
        .await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let entry = bob
        .docs_engine()
        .get_any(bob_folder.doc(), "sys/links/mutable/docs")
        .await?;
    anyhow::ensure!(entry.is_some(), "Bob should see the pointer entry after sync");

    alice.stop().await?;
    bob.stop().await?;
    Ok(())
}

/// Two-node test: Alice creates a private link and publishes it to a folder doc.
#[tokio::test]
async fn test_link_publish_private_to_folder_doc() -> anyhow::Result<()> {
    let directory = test_utils::TestDirectory::new("link-publish-private")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();
    let alice = test_node(
        &directory,
        "alice",
        Some(relay_map.clone()),
        Some(memory_lookup.clone()),
    )
    .await?;
    let bob = test_node(&directory, "bob", Some(relay_map), Some(memory_lookup.clone())).await?;
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(alice.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(bob.endpoint().id()).with_relay_url(relay_url));

    let alice_manager = FolderManager::new(&alice);
    let folder = alice_manager.create(SyncMode::SendReceive).await?;

    let hash = alice.blob_store().add_bytes(b"secret").await?;
    let link = PrivateLink::generate(hash, 4_000_000_000)?;
    let payload = serde_json::to_vec(&link)?;
    let link_key = format!("sys/links/private/{}", hex::encode(link.capability));
    folder.set_blob(&link_key, payload).await?;

    let ticket = folder.ticket(alice.endpoint().addr(), true).await?;
    let bob_manager = FolderManager::new(&bob);
    let bob_folder = bob_manager.join(ticket.to_string(), SyncMode::ReceiveOnly).await?;

    alice
        .docs_engine()
        .start_sync(folder.doc(), vec![bob.endpoint().addr()])
        .await?;
    bob.docs_engine()
        .start_sync(bob_folder.doc(), vec![alice.endpoint().addr()])
        .await?;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let entry = bob.docs_engine().get_any(bob_folder.doc(), &link_key).await?;
    anyhow::ensure!(entry.is_some(), "Bob should see the private link entry after sync");

    alice.stop().await?;
    bob.stop().await?;
    Ok(())
}

/// Two-node gossip test: Alice publishes a revocation via gossip.
/// Bob receives it after subscribing to the same topic.
#[tokio::test]
async fn test_link_revoke_propagates_via_gossip() -> anyhow::Result<()> {
    let directory = test_utils::TestDirectory::new("link-revoke-gossip")?;
    let (relay_map, relay_url, _server) = iroh::test_utils::run_relay_server().await?;
    let memory_lookup = MemoryLookup::new();
    let alice = test_node(
        &directory,
        "alice",
        Some(relay_map.clone()),
        Some(memory_lookup.clone()),
    )
    .await?;
    let bob = test_node(&directory, "bob", Some(relay_map), Some(memory_lookup.clone())).await?;
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(alice.endpoint().id()).with_relay_url(relay_url.clone()));
    memory_lookup.add_endpoint_info(iroh::EndpointAddr::new(bob.endpoint().id()).with_relay_url(relay_url));

    let hash = alice.blob_store().add_bytes(b"shared").await?;
    let link = PrivateLink::generate(hash, 4_000_000_000)?;

    let topic_id = revocation_topic_id();

    // Alice subscribes to the revocation topic
    let alice_topic = alice.gossip_service().subscribe(topic_id, Vec::new()).await?;
    let (alice_sender, _alice_receiver) = GossipService::split(alice_topic);
    let alice_channel = TopicChannel::<PrivateLink>::new(
        Arc::new(alice.gossip_service().inner().clone()),
        REVOCATION_GOSSIP_TOPIC,
        alice_sender,
    );

    // Bob subscribes to receive revocations, bootstrapping to Alice
    let mut bob_topic = bob
        .gossip_service()
        .subscribe(topic_id, vec![alice.endpoint().id()])
        .await?;
    tokio::time::timeout(Duration::from_secs(30), bob_topic.joined())
        .await
        .context("Bob's gossip join timed out")?
        .context("Bob's gossip join failed")?;
    let (bob_sender, bob_receiver) = GossipService::split(bob_topic);
    let bob_channel = TopicChannel::<PrivateLink>::new(
        Arc::new(bob.gossip_service().inner().clone()),
        REVOCATION_GOSSIP_TOPIC,
        bob_sender,
    );
    let mut bob_stream = bob_channel.receive_from(bob_receiver);

    // Alice publishes the revocation
    alice_channel.publish(&link).await?;

    // Bob should receive it within a timeout
    let received = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Some(msg) = bob_stream.next().await {
                return msg;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .context("Bob did not receive revocation via gossip")?;

    anyhow::ensure!(received.manifest == link.manifest, "manifest mismatch");
    anyhow::ensure!(received.capability == link.capability, "capability mismatch");

    alice.stop().await?;
    bob.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_resolve_remote_with_provider_fetch() -> anyhow::Result<()> {
    let fetch: syncweb_core::indexing::ProviderFetch = Arc::new(|name_link| {
        let alias = name_link.alias;
        Box::pin(async move {
            let mut r = LinkResolution::new(test_hash(9));
            r.version = Some(alias);
            Ok(r)
        })
    });

    let resolver = LinkResolver::with_provider_fetch(fetch);
    let pointer = test_pointer(9, "fetch-test", test_hash(9), 1)?;
    let name = pointer.link()?;

    let result = resolver.resolve_remote(&Link::Name(name)).await;
    anyhow::ensure!(result.is_ok(), "expected ok result");
    let resolution = result?.context("provider fetch should return resolution")?;
    anyhow::ensure!(resolution.manifest == test_hash(9), "manifest mismatch");
    anyhow::ensure!(resolution.version == Some("fetch-test".to_string()), "version mismatch");
    Ok(())
}
