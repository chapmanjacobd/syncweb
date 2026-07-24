# Shared Gossip Abstraction

## Problem

Four plans propose gossip-based broadcast of signed messages:
- `08-report-tdd.md` — `syncweb/reports` topic for `ReportRecord`
- `07-attest-tdd.md` — `syncweb/attestations/v1` topic for `Attestation`
- `06-trust-vouch-tdd.md` — `syncweb/provider-trust-stream/v1` topic for `ProviderTrustSignal`
- `09-link-tdd.md` (revoke) — `syncweb/link-revocations/v1` topic for revocation

Each plan independently defines `publish`, `subscribe`, and `collect` functions with near-identical logic (serialize → broadcast, receive → deserialize → verify → apply). This creates code duplication and inconsistent error handling.

## Solution

Define a single generic gossip abstraction that all four topics reuse.

---

## Generic types

### `syncweb-core/src/gossip/signed_topic.rs`

```rust
use iroh_gossip::net::Gossip;
use iroh_gossip::proto::TopicId;
use serde::{Serialize, Deserialize};
use std::marker::PhantomData;

/// Anything that can be signed, broadcast, and verified over gossip.
pub trait SignedGossipMessage: Serialize + for<'de> Deserialize<'de> {
    /// Verify this message's signature. Returns Ok(()) if valid.
    fn verify_signature(&self) -> Result<()>;

    /// Serialize for wire transport (JSON or postcard).
    fn to_wire_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Deserialize from wire transport.
    fn from_wire_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }
}
```

### `syncweb-core/src/gossip/topic_channel.rs`

```rust
/// A typed gossip topic that can publish and subscribe to one message type.
///
/// Uses `GossipService` (syncweb's wrapper around `iroh_gossip::net::Gossip`)
/// for lifecycle management. `TopicChannel` only handles publish and
/// per-subscription stream filtering — the underlying gossip connection,
/// subscription, and peer bootstrap are managed by `GossipService`.
pub struct TopicChannel<T: SignedGossipMessage> {
    gossip: Arc<Gossip>,
    topic_id: TopicId,
    _phantom: PhantomData<T>,
}

impl<T: SignedGossipMessage> TopicChannel<T> {
    /// Create a deterministic topic ID from a byte-string topic name.
    pub fn new(gossip: Arc<Gossip>, topic_name: &[u8]) -> Self {
        Self {
            gossip,
            topic_id: TopicId::from_bytes(*blake3::hash(topic_name).as_bytes()),
            _phantom: PhantomData,
        }
    }

    /// Publish a signed message to the topic. All subscribers receive it.
    pub async fn publish(&self, message: &T) -> Result<()> {
        message.verify_signature()?;
        let bytes = message.to_wire_bytes()?;
        self.gossip.publish(self.topic_id, bytes.into()).await?;
        Ok(())
    }

    /// Receive messages from an already-subscribed gossip stream.
    ///
    /// The caller must already have subscribed to `self.topic_id` via
    /// `GossipService` (which handles lifecycle, peer bootstrap, and connection
    /// management). This method wraps the raw event stream with deserialization
    /// and signature verification.
    ///
    /// Returns a stream of verified, deserialized `T` messages.
    /// Messages that fail deserialization or signature verification are silently
    /// dropped (logged at debug level).
    pub fn receive_from(
        &self,
        stream: impl tokio_stream::Stream<Item = iroh_gossip::net::Event>,
    ) -> impl tokio_stream::Stream<Item = T> {
        use futures::StreamExt;
        stream.filter_map(|event| {
            let topic = self.topic_id;
            async move {
                if event.topic != topic {
                    return None;
                }
                let msg = T::from_wire_bytes(&event.content).ok()?;
                msg.verify_signature().ok().map(|_| msg)
            }
        })
    }

    /// Collect messages for a specific content key within a timeout.
    ///
    /// The caller must already be subscribed to the topic via `GossipService`.
    /// `stream` is the raw gossip event stream from the subscription.
    pub async fn collect_for(
        &self,
        stream: impl tokio_stream::Stream<Item = iroh_gossip::net::Event>,
        filter: impl Fn(&T) -> bool + Send + 'static,
        timeout: std::time::Duration,
    ) -> Result<Vec<T>> {
        let mut filtered = self.receive_from(stream);
        let mut results = Vec::new();
        let _ = tokio::time::timeout(timeout, async {
            while let Some(msg) = filtered.next().await {
                if filter(&msg) {
                    results.push(msg);
                }
            }
        }).await;
        Ok(results)
    }

    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }
}
```

### Daemon auto-subscribe

```rust
// In Daemon::run_inner()

// GossipService handles lifecycle: connect to topic, bootstrap peers,
// manage subscriptions. TopicChannel only handles typed pub/filter.
let gossip_svc = self.node.gossip_service();
let trust_topic = TopicChannel::<ProviderTrustSignal>::new(
    gossip_svc.gossip(), b"syncweb/provider-trust-stream/v1"
);
let attest_topic = TopicChannel::<Attestation>::new(
    gossip_svc.gossip(), b"syncweb/attestations/v1"
);
let report_topic = TopicChannel::<ReportRecord>::new(
    gossip_svc.gossip(), b"syncweb/reports"
);
let revoke_topic = TopicChannel::<PrivateLink>::new(
    gossip_svc.gossip(), b"syncweb/link-revocations/v1"
);

// GossipService subscribes to all topics once, returns per-topic event streams.
let (trust_stream, attest_stream, report_stream, revoke_stream) = gossip_svc
    .subscribe_all(&[
        trust_topic.topic_id(),
        attest_topic.topic_id(),
        report_topic.topic_id(),
        revoke_topic.topic_id(),
    ])
    .await?;

// TopicChannel wraps each stream with typed deserialization + verification.
tokio::spawn(trust_listener(trust_topic.receive_from(trust_stream), wot.clone()));
tokio::spawn(attestation_listener(attest_topic.receive_from(attest_stream), wot.clone()));
tokio::spawn(report_listener(report_topic.receive_from(report_stream), trust_policy.clone()));
tokio::spawn(revocation_listener(revoke_topic.receive_from(revoke_stream), link_resolver.clone()));
```

---

## Topic Table

| Topic key | `SignedGossipMessage` impl | Authentication scheme | Plan |
|-----------|---------------------------|----------------------|------|
| `syncweb/provider-trust-stream/v1` | `ProviderTrustSignal` | Ed25519, `signature` field | 06-trust-vouch-tdd |
| `syncweb/attestations/v1` | `Attestation` | Ed25519, `signature` field | 07-attest-tdd |
| `syncweb/reports` | `ReportRecord` | Ed25519, `signature` field | 08-report-tdd |
| `syncweb/link-revocations/v1` | `PrivateLink` | Bearer capability (no signature — see security note below) | 09-link-tdd |

Each type must implement `SignedGossipMessage`. The `verify_signature` implementation is type-specific:

```rust
impl SignedGossipMessage for ProviderTrustSignal {
    fn verify_signature(&self) -> Result<()> {
        // ProviderTrustSignal already has a verify() method
        self.verify().map_err(|_| SyncwebError::InvalidSignature)
    }
}

impl SignedGossipMessage for Attestation {
    fn verify_signature(&self) -> Result<()> {
        self.verify_signature().map_err(|_| SyncwebError::InvalidSignature)
    }
}

impl SignedGossipMessage for ReportRecord {
    fn verify_signature(&self) -> Result<()> {
        self.verify().map_err(|_| SyncwebError::InvalidSignature)
    }
}

// PrivateLink uses bearer-capability authentication — no cryptographic signature.
//
// SECURITY DESIGN NOTE (not just a comment — this is a required implementation constraint):
// Since verify_signature() always returns Ok(()), ANY peer on the gossip topic can
// broadcast a revocation for any capability hash. This is an inherent limitation of
// bearer-capability auth. The daemon revocation listener (09-link-tdd.md) MUST enforce:
//   1. Only apply revocations for capabilities the local node actually possesses.
//   2. Maintain a local capability index (HashMap<capability_hash, PrivateLink>)
//      that the listener checks BEFORE applying any incoming revocation.
//   3. Log but ignore revocations for unknown capability hashes.
// Without this enforcement, a malicious peer can DoS link access for any hash.
// The SignedGossipMessage trait is reused here for transport uniformity only —
// the real auth happens at the application layer, not in verify_signature().
impl SignedGossipMessage for PrivateLink {
    fn verify_signature(&self) -> Result<()> {
        Ok(()) // Bearer-capability auth: validated at the application layer, not here
    }
}
```

---

## Per-plan updates

### `08-report-tdd.md`
Replace `ReportGossip` struct with `TopicChannel<ReportRecord>`:
```rust
let topic = TopicChannel::new(gossip, b"syncweb/reports");
topic.publish(&report).await?;
```

### `07-attest-tdd.md`
Replace `attestation_gossip::publish_attestation` / `collect_attestations` with `TopicChannel<Attestation>`:
```rust
let topic = TopicChannel::new(gossip, b"syncweb/attestations/v1");
topic.publish(&attestation).await?;
// To collect: caller must already be subscribed via GossipService.
// The event stream from the subscription is passed to collect_for():
let results = topic.collect_for(stream, |a| a.content == hash, timeout).await?;
```

### `06-trust-vouch-tdd.md`
Reuse `TopicChannel<ProviderTrustSignal>` instead of calling raw gossip methods:
```rust
let trust_topic = TopicChannel::new(gossip, b"syncweb/provider-trust-stream/v1");
let signal = ProviderTrustSignal::from_trust_record(&record)?;
trust_topic.publish(&signal).await?;
```

### `09-link-tdd.md` (revoke)
Replace `publish_revocation()` with `TopicChannel<PrivateLink>`:
```rust
let revoke_topic = TopicChannel::new(gossip, b"syncweb/link-revocations/v1");
revoke_topic.publish(&revocation).await?;
```

---

## Files

| File | Action |
|------|--------|
| `syncweb-core/src/gossip/mod.rs` | NEW — re-exports; lives as a sibling to `node/`, not replacing `node/gossip_service.rs` |
| `syncweb-core/src/gossip/signed_topic.rs` | NEW — `SignedGossipMessage` trait |
| `syncweb-core/src/gossip/topic_channel.rs` | NEW — `TopicChannel<T>` generic implementation |
| `syncweb-core/src/gossip/daemon_listener.rs` | NEW — daemon startup helper that spawns topic listeners |
| `syncweb-core/src/indexing/attestation_gossip.rs` | REMOVE — replaced by `TopicChannel<Attestation>` |
| `syncweb-core/src/indexing/gossip_report.rs` | REMOVE — replaced by `TopicChannel<ReportRecord>` |

## Relationship to existing gossip code

`syncweb-core/src/node/gossip_service.rs` wraps the low-level `iroh_gossip::net::Gossip`
type and provides daemon-level lifecycle (connect, subscribe, disconnect). The new
`gossip/` module sits at the same level (`syncweb-core/src/gossip/`) and provides the
typed, application-level `TopicChannel<T>` abstraction. `TopicChannel` takes an
`Arc<Gossip>` obtained from `GossipService` — it does NOT replace or modify
`GossipService`. The daemon startup code calls `self.node.gossip_service()` to get the
`Arc<Gossip>`, then creates `TopicChannel` instances from it.

## Migration note

Existing gossip topics (`syncweb/provider-trust-stream/v1`) use the same `TopicId` derivation as before (`blake3::hash(topic_name)[..32]`). No backward-compatibility issue — the protocol is identical, just the code that wraps it changes.
