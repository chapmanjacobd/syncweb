use std::time::Duration;

use anyhow::{Context, ensure};
use ed25519_dalek::SigningKey;
use iroh::SecretKey;
use iroh::address_lookup::memory::MemoryLookup;
use n0_future::StreamExt;
use syncweb_core::constants::TRUST_STREAM_TOPIC;
use syncweb_core::gossip::{TopicChannel, gossip_topic_id};
use syncweb_core::indexing::{
    ProviderReputationStore, ProviderTrustAction, ProviderTrustRecord, ProviderTrustSignal, TrustSignalKind,
    trust_stream_topic,
};
use syncweb_core::node::gossip_service::GossipService;
use syncweb_core::node::identity::IdentityManager;
use syncweb_core::node::iroh_node::{DiscoveryConfig, IrohNode, RelayMode};

use crate::test_utils::TestDirectory;

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn provider(seed: u8) -> iroh::PublicKey {
    SecretKey::from_bytes(&[seed; 32]).public()
}

async fn test_node(
    directory: &TestDirectory,
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
            DiscoveryConfig::disabled(),
            crate::test_utils::empty_member_keys(),
        )
        .await?),
        None => Ok(IrohNode::new(
            identity,
            root.join("data"),
            relay_mode,
            crate::test_utils::empty_member_keys(),
            DiscoveryConfig::disabled(),
        )
        .await?),
    }
}

#[test]
fn test_from_trust_record_vouch_converts_to_signal() -> anyhow::Result<()> {
    let key = signing_key(1);
    let provider_key = provider(2);
    let record = ProviderTrustRecord::new(
        provider_key,
        ProviderTrustAction::Vouch,
        None,
        1,
        None,
        "good provider",
        &key,
    )?;
    let signal = ProviderTrustSignal::from_trust_record(&record, &key)?;
    ensure!(signal.provider == provider_key, "provider mismatch");
    ensure!(
        signal.signal == TrustSignalKind::ObservedSuccess,
        "signal kind mismatch"
    );
    ensure!(signal.sequence == 1, "sequence mismatch");
    ensure!(signal.verify().is_ok(), "signal verification failed");
    Ok(())
}

#[test]
fn test_from_trust_record_distrust_converts_to_signal() -> anyhow::Result<()> {
    let key = signing_key(3);
    let provider_key = provider(4);
    let record = ProviderTrustRecord::new(
        provider_key,
        ProviderTrustAction::Distrust,
        None,
        1,
        None,
        "unreliable",
        &key,
    )?;
    let signal = ProviderTrustSignal::from_trust_record(&record, &key)?;
    ensure!(signal.provider == provider_key, "provider mismatch");
    ensure!(
        signal.signal == TrustSignalKind::ObservedFailure,
        "signal kind mismatch"
    );
    ensure!(signal.verify().is_ok(), "signal verification failed");
    Ok(())
}

#[test]
fn test_from_trust_record_rejects_non_vouch_distrust() -> anyhow::Result<()> {
    let key = signing_key(5);
    let provider_key = provider(6);
    let record = ProviderTrustRecord::new(provider_key, ProviderTrustAction::Trust, None, 1, None, "trusted", &key)?;
    let result = ProviderTrustSignal::from_trust_record(&record, &key);
    ensure!(result.is_err(), "Trust action should not be convertible");
    Ok(())
}

#[test]
fn test_from_trust_record_rejects_warn_action() -> anyhow::Result<()> {
    let key = signing_key(7);
    let provider_key = provider(8);
    let record = ProviderTrustRecord::new(provider_key, ProviderTrustAction::Warn, None, 1, None, "warning", &key)?;
    let result = ProviderTrustSignal::from_trust_record(&record, &key);
    ensure!(result.is_err(), "Warn action should not be convertible");
    Ok(())
}

#[tokio::test]
async fn test_vouch_signal_published_via_gossip_reaches_subscriber() -> anyhow::Result<()> {
    let directory = TestDirectory::new("vouch-gossip")?;
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

    let provider_key = provider(10);
    let alice_key = signing_key(1);
    let record = ProviderTrustRecord::new(
        provider_key,
        ProviderTrustAction::Vouch,
        None,
        1,
        None,
        "good",
        &alice_key,
    )?;
    let signal = ProviderTrustSignal::from_trust_record(&record, &alice_key)?;

    let topic_id = trust_stream_topic();
    let alice_topic = alice
        .gossip_service()
        .subscribe(topic_id, vec![])
        .await
        .context("alice subscribe")?;
    let (alice_sender, _) = GossipService::split(alice_topic);

    let channel = TopicChannel::<ProviderTrustSignal>::new(
        std::sync::Arc::new(alice.gossip_service().inner().clone()),
        gossip_topic_id(TRUST_STREAM_TOPIC),
        alice_sender,
    );

    let mut bob_topic = bob
        .gossip_service()
        .subscribe(topic_id, vec![alice.endpoint().id()])
        .await
        .context("bob subscribe")?;

    tokio::time::timeout(Duration::from_secs(30), bob_topic.joined())
        .await
        .context("bob topic join timed out")?
        .context("bob topic join failed")?;

    let (_bob_sender, mut bob_receiver) = GossipService::split(bob_topic);

    channel.publish(&signal).await.context("alice publish")?;

    let received = tokio::time::timeout(Duration::from_secs(15), async {
        while let Some(event) = bob_receiver.next().await {
            if let Ok(iroh_gossip::api::Event::Received(msg)) = event
                && let Ok(decoded) = ProviderTrustSignal::from_bytes(&msg.content)
            {
                return Some(decoded);
            }
        }
        None
    })
    .await
    .context("receive timed out")?
    .context("stream closed before receiving")?;

    ensure!(
        received.provider == provider_key,
        "provider mismatch in received signal"
    );
    ensure!(
        received.signal == TrustSignalKind::ObservedSuccess,
        "signal kind mismatch in received signal"
    );
    ensure!(received.verify().is_ok(), "received signal verification failed");

    alice.stop().await?;
    bob.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_incoming_trust_signal_applied_to_wot() -> anyhow::Result<()> {
    let directory = TestDirectory::new("signal-wot")?;
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

    let provider_key = provider(20);
    let alice_key = signing_key(11);
    let record = ProviderTrustRecord::new(
        provider_key,
        ProviderTrustAction::Vouch,
        None,
        1,
        None,
        "good",
        &alice_key,
    )?;
    let signal = ProviderTrustSignal::from_trust_record(&record, &alice_key)?;

    let topic_id = trust_stream_topic();
    let alice_topic = alice.gossip_service().subscribe(topic_id, vec![]).await?;
    let (alice_sender, _) = GossipService::split(alice_topic);

    let channel = TopicChannel::<ProviderTrustSignal>::new(
        std::sync::Arc::new(alice.gossip_service().inner().clone()),
        gossip_topic_id(TRUST_STREAM_TOPIC),
        alice_sender,
    );

    let mut bob_topic = bob
        .gossip_service()
        .subscribe(topic_id, vec![alice.endpoint().id()])
        .await?;

    tokio::time::timeout(Duration::from_secs(30), bob_topic.joined())
        .await?
        .context("bob join failed")?;

    let (_bob_sender, mut bob_receiver) = GossipService::split(bob_topic);

    let mut config = syncweb_core::indexing::ReputationConfig::default();
    config.min_samples = 1;
    let mut reputation = ProviderReputationStore::new(config);

    channel.publish(&signal).await?;

    let received = tokio::time::timeout(Duration::from_secs(15), async {
        while let Some(event) = bob_receiver.next().await {
            if let Ok(iroh_gossip::api::Event::Received(msg)) = event
                && let Ok(decoded) = ProviderTrustSignal::from_bytes(&msg.content)
            {
                return Some(decoded);
            }
        }
        None
    })
    .await?
    .context("bob did not receive signal")?;

    let reporter_key =
        iroh::PublicKey::from_bytes(&alice_key.verifying_key().to_bytes()).context("invalid reporter key")?;
    reputation.trust_reporter(&reporter_key)?;
    reputation.ingest_trust_signal(received)?;
    let score = reputation.score(
        provider_key,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    );
    ensure!(
        score > 0.5,
        "ingested vouch signal should improve reputation score, got {score}"
    );

    alice.stop().await?;
    bob.stop().await?;
    Ok(())
}
