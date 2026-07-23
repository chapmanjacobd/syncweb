# TDD Plan: `attest`

## Divergence
Name "Sign content provenance attestations" accurately describes the signing part. But the signature is useless if nobody can discover it. Attestations are signed then buried in local `indexing-state.json` — never broadcast, never indexed, never shareable.

## Current flow
```
handle_attest → Attestation::new(content, kind, value, sequence, signing_key)
  → wot.append_attestation(attestation)     // in-memory WotState HashMap
  → state.attestations.push(attestation)    // local JSON
  → save_state(data_dir, &state)            // disk write
  → print_status(...)                        // done — nobody else ever sees it
```

`append_attestation` also checks `policy.is_trusted_for_at(issuer, content)` — meaning only the local trust root (self) can attest, since no delegations from others are known (they're also local-only, see `trust-delegate-tdd.md`).

## Read-side gap
Even if attestations were broadcast, there's no `syncweb attest verify <hash>` command to check them. `trust show <hash>` shows attestations, but only from local state.

## Decision
- Add `--broadcast` flag to `attest` — publishes signed attestation on a gossip topic
- Add `syncweb attest verify <hash>` command that checks attestations from the network
- Add daemon listener that auto-ingests incoming attestations from gossip
- Use deterministic gossip topic `syncweb/attestations/v1`

---

## Tests

### Phase 1 — Capture current local-only behavior

```rust
// syncweb-core/tests/attest_test.rs  (new file)

#[test]
fn test_attestation_is_signed_but_trapped() {
    let signing = SigningKey::from_bytes(&secret_key.to_bytes());
    let attestation = Attestation::new(
        content_hash, AttestationKind::License, "MIT".into(), 1, &signing,
    )?;
    assert!(attestation.verify_signature().is_ok());
    // Stored only in local state.attestations + indexing-state.json
    // No other peer can discover this attestation
}
```

### Phase 2 — Gossip broadcast of attestations

```rust
// syncweb-core/tests/attest_gossip_test.rs  (new file)

#[tokio::test]
async fn test_attestation_gossip_reaches_peers() -> anyhow::Result<()> {
    let dir = TestDirectory::new("attest-gossip")?;
    let (relay, relay_url, _server) = iroh::test_utils::run_relay_server().await?;

    let alice = make_node(&dir, "alice", &relay, &relay_url).await?;
    let bob = make_node(&dir, "bob", &relay, &relay_url).await?;

    // Alice creates an attestation
    let hash = alice.blob_store().add_bytes(b"hello").await?;
    let attestation = Attestation::new(
        hash, AttestationKind::License, "MIT".into(), 1,
        alice.endpoint().secret_key(),
    )?;

    // Alice broadcasts on attestation gossip topic
    let topic = attestation_topic();
    let alice_gossip = alice.gossip_service();
    alice_gossip.subscribe(topic, vec![]).await?;
    let gossip_topic = alice_gossip.subscribe(topic, vec![]).await?;
    let (sender, _) = GossipService::split(gossip_topic);
    alice_gossip.publish(&sender, attestation.to_bytes()?).await?;

    // Bob receives the attestation
    let bob_gossip = bob.gossip_service();
    bob_gossip.subscribe(topic, vec![alice.endpoint().id()]).await?;
    let received = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Ok(Some(event)) = bob_gossip.try_recv(topic) {
                return event;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await?;

    let received_attestation: Attestation = serde_json::from_slice(&received.content)?;
    assert!(received_attestation.verify_signature().is_ok());
    assert_eq!(received_attestation.value, "MIT");

    alice.stop().await?;
    bob.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_incoming_attestation_auto_applied() -> anyhow::Result<()> {
    // Bob receives attestation via gossip
    // Bob's daemon applies it to local WotService
    // `syncweb trust show <hash>` shows the attestation from Alice
    Ok(())
}

#[tokio::test]
async fn test_attestation_verify_checks_gossip_and_local() -> anyhow::Result<()> {
    // Alice broadcasts attestation
    // Bob runs `syncweb attest verify <hash>` (new command)
    // Result shows Alice's attestation: License = MIT
    Ok(())
}
```

### Phase 3 — CLI integration

```rust
// syncweb-cli/tests/cli_test.rs

#[test]
fn test_attest_with_broadcast() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("attest-broadcast");
    // Start embedded node
    // Run `syncweb attest <hash> --license MIT --broadcast`
    // Verify attestation appears on gossip topic
    Ok(())
}

#[test]
fn test_attest_verify_remote() -> anyhow::Result<()> {
    // Node A attests content with broadcast
    // Node B runs `syncweb attest verify <hash>`
    // B sees A's attestation
    Ok(())
}

#[test]
fn test_attest_without_broadcast_still_local() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("attest-local");
    // Run without --broadcast → no node, no gossip
    // `syncweb trust show <hash>` shows the attestation locally
    Ok(())
}
```

---

## Implementation

### `syncweb-core/src/indexing/attestation_gossip.rs` (new file)

```rust
pub const ATTESTATION_GOSSIP_TOPIC: &[u8] = b"syncweb/attestations/v1";

pub fn attestation_topic() -> TopicId {
    TopicId::from_bytes(*blake3::hash(ATTESTATION_GOSSIP_TOPIC).as_bytes())
}

pub async fn publish_attestation(
    gossip: &GossipService,
    attestation: &Attestation,
) -> Result<()> {
    attestation.verify_signature()?;
    let topic = attestation_topic();
    let gossip_topic = gossip.subscribe(topic, vec![]).await?;
    let (sender, _) = GossipService::split(gossip_topic);
    gossip.publish(&sender, serde_json::to_vec(attestation)?).await
}

pub async fn subscribe_attestations(
    gossip: &GossipService,
    bootstrap: Vec<PublicKey>,
) -> Result<GossipReceiver> {
    let topic = attestation_topic();
    let gossip_topic = gossip.subscribe(topic, bootstrap).await?;
    let (_, receiver) = GossipService::split(gossip_topic);
    Ok(receiver)
}

/// Collect all attestations for a content hash from gossip within a timeout.
pub async fn collect_attestations(
    gossip: &GossipService,
    bootstrap: Vec<PublicKey>,
    content: &Hash,
    timeout_duration: Duration,
) -> Result<Vec<Attestation>> {
    let receiver = subscribe_attestations(gossip, bootstrap).await?;
    let mut results = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout_duration;
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if let Ok(Some(event)) = gossip.try_recv(attestation_topic()) {
            if let Ok(attestation) = serde_json::from_slice::<Attestation>(&event.content) {
                if attestation.content == *content && attestation.verify_signature().is_ok() {
                    results.push(attestation);
                }
            }
        }
    }
    Ok(results)
}
```

### `syncweb-core/src/indexing/wot.rs` — Add `Attestation::to_bytes()`/`from_bytes()`

```rust
impl Attestation {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }
}
```

### `syncweb-core/src/daemon/daemon.rs`

On startup, spawn a task that:
1. Subscribes to `syncweb/attestations/v1` gossip topic
2. Receives `Attestation` messages
3. Verifies signatures
4. Applies to local `WotService` via `wot.append_attestation()`
5. Persists to `indexing-state.json`

### `syncweb-cli/src/cli/commands.rs`

```rust
#[command(about = "Sign content provenance attestations")]
Attest(AttestArgs),

// New sub-subcommand for attest
#[derive(Debug, Subcommand)]
pub enum AttestCommand {
    #[command(about = "Sign and optionally broadcast a content attestation")]
    Create {
        content: String,
        // ... existing flags ...
        #[arg(long, help = "Broadcast attestation via gossip")]
        broadcast: bool,
    },
    #[command(about = "Verify attestations for content from the network")]
    Verify {
        hash: String,
        #[arg(long, help = "Timeout in seconds for gossip collection")]
        timeout: Option<u64>,
    },
}
```

Or restructure `Attest` from a simple `Args` to a `Subcommand`:

```rust
#[command(about = "Sign content provenance attestations")]
Attest {
    #[command(subcommand)]
    command: AttestCommand,
},
```

For backward compatibility, keep the flat `Attest(AttestArgs)` but with a deprecation warning, and add the subcommand-based version.

### `syncweb-cli/src/cli/indexing.rs`

```rust
pub fn handle_attest(ctx: &CliContext<'_>, command: AttestCommand) -> Result<()> {
    match command {
        AttestCommand::Create { content, license, provenance, derivative, sequence, broadcast } => {
            // ... existing creation code ...
            let attestation = Attestation::new(...)?;
            // ... existing local persistence ...

            if broadcast {
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async {
                    let node = open_node(data_dir).await?;
                    attestation_gossip::publish_attestation(
                        node.gossip_service(), &attestation
                    ).await?;
                    node.stop().await?;
                    Ok(())
                })?;
            }
        }
        AttestCommand::Verify { hash, timeout } => {
            let content_hash = parse_hash(&hash)?;
            let timeout_duration = Duration::from_secs(timeout.unwrap_or(5));
            let rt = tokio::runtime::Handle::current();
            let attestations = rt.block_on(async {
                let node = open_node(data_dir).await?;
                let result = attestation_gossip::collect_attestations(
                    node.gossip_service(), vec![], &content_hash, timeout_duration
                ).await?;
                node.stop().await?;
                Ok(result)
            })?;

            if output_json {
                println!("{}", serde_json::to_string_pretty(&attestations)?);
            } else {
                for att in &attestations {
                    println!("{}: {} (by {})", att.kind, att.value, att.issuer);
                }
            }
        }
    }
    Ok(())
}
```

---

## Gossip/network integration note

- New gossip topic: `syncweb/attestations/v1` — deterministic topic ID from BLAKE3 hash
- Same pattern as `syncweb/provider-trust-stream/v1` and the proposed `syncweb/trust-delegations/v1`
- Attestations are self-authenticating (signed by the issuer's key)
- The `--broadcast` flag is optional; without it, attestation is local-only
- `attest verify` connects to gossip and collects attestations for a hash with a configurable timeout
- Daemon auto-subscribes on startup to build a local attestation database

## Files to modify/plan

| File | Changes |
|------|---------|
| `syncweb-core/src/indexing/attestation_gossip.rs` | New — topic, publish, subscribe, collect functions |
| `syncweb-core/src/indexing.rs` | Re-export attestation gossip module |
| `syncweb-core/src/indexing/wot.rs` | Add `Attestation::to_bytes()`/`from_bytes()` if missing |
| `syncweb-core/src/daemon/daemon.rs` | Spawn attestation listener on startup |
| `syncweb-cli/src/cli/commands.rs` | Restructure `Attest` as subcommand with `Create` + `Verify` |
| `syncweb-cli/src/cli/indexing.rs` | Wire gossip broadcast + verify handler |
| `syncweb-core/tests/attest_gossip_test.rs` | New — two-node gossip test |
| `syncweb-cli/tests/cli_test.rs` | CLI integration tests |
