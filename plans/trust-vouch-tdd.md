# TDD Plan: `trust provider vouch` / `trust provider distrust`

## Note
This is NOT a "not hooked up" bug. `trust provider vouch` is local bookkeeping — storing your opinion about a provider in your local trust database. The `trust stream publish` command already exists for broadcasting trust signals over gossip. This plan is a **convenience enhancement**: adding `--broadcast` so users can vouch/distrust and optionally share that opinion via the existing gossip topic, without needing to learn the `trust stream publish` syntax.

## Current state vs. `trust stream publish`

| Command | Type used | Network? |
|---|---|---|
| `trust provider vouch` | `ProviderTrustRecord` | Local only — correct by design |
| `trust provider distrust` | `ProviderTrustRecord` | Local only — correct by design |
| `trust stream publish` | `ProviderTrustSignal` | Gossip broadcast |

## Decision
- Add optional `--broadcast` flag to `trust provider vouch` and `trust provider distrust`
- When set, convert the `ProviderTrustRecord` to a `ProviderTrustSignal` and broadcast on the existing `syncweb/provider-trust-stream/v1` gossip topic
- Without `--broadcast`, behavior is unchanged (local only — this is the normal use case)
- The existing `trust stream subscribe` handler already receives and applies incoming `ProviderTrustSignal`s, so broadcasted vouch/distrust signals are processed by peers automatically

---

## Tests

### Phase 1 — Verify current local-only behavior works correctly

```rust
// syncweb-core/tests/provider_trust_test.rs  (new file)

#[test]
fn test_vouch_local_bookkeeping() {
    let signing = SigningKey::from_bytes(&secret_key.to_bytes());
    let provider = PublicKey::from_bytes(&[2u8; 32]);
    let record = ProviderTrustRecord::new(
        provider, ProviderTrustAction::Vouch, None, 1, None,
        "good provider".into(), &signing,
    )?;
    assert!(record.verify().is_ok());
    // Local bookkeeping is correct by design
    // "trust provider show" will display this record
}
```

### Phase 2 — Convert ProviderTrustRecord to ProviderTrustSignal and gossip

```rust
// syncweb-core/tests/provider_trust_gossip_test.rs  (new file)

#[tokio::test]
async fn test_vouch_gossip_reaches_peers() -> anyhow::Result<()> {
    let dir = TestDirectory::new("vouch-gossip")?;
    let (relay, relay_url, _server) = iroh::test_utils::run_relay_server().await?;

    let alice = make_node(&dir, "alice", &relay, &relay_url).await?;
    let bob = make_node(&dir, "bob", &relay, &relay_url).await?;
    let provider = PublicKey::from_bytes(&[3u8; 32]);

    // Alice vouches for provider
    let record = ProviderTrustRecord::new(
        provider, ProviderTrustAction::Vouch, None, 1, None,
        "good provider".into(), alice.endpoint().secret_key(),
    )?;

    // Convert to ProviderTrustSignal and gossip
    let signal = ProviderTrustSignal::from_trust_record(&record)?;
    let topic = trust_stream_topic();
    let alice_gossip = alice.gossip_service();
    let gossip_topic = alice_gossip.subscribe(topic, vec![]).await?;
    let (sender, _) = GossipService::split(gossip_topic);
    alice_gossip.publish(&sender, signal.to_bytes()?).await?;

    // Bob receives the signal
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

    let received_signal: ProviderTrustSignal = serde_json::from_slice(&received.content)?;
    assert!(received_signal.verify().is_ok());
    assert_eq!(received_signal.provider, provider);

    alice.stop().await?;
    bob.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_distrust_gossip_reaches_peers() -> anyhow::Result<()> {
    // Same as vouch but with ProviderTrustAction::Distrust
    Ok(())
}

#[tokio::test]
async fn test_incoming_vouch_is_applied_to_wot() -> anyhow::Result<()> {
    // Bob receives ProviderTrustSignal with Vouch action
    // Bob's WotService applies it as a provider trust record
    // `trust provider show <provider>` reflects the vouch
    Ok(())
}
```

### Phase 3 — Round-trip: vouch + subscribe

```rust
#[tokio::test]
async fn test_vouch_distrust_round_trip() -> anyhow::Result<()> {
    // 1. Alice runs `trust provider vouch --broadcast <provider>`
    // 2. Bob's daemon receives via subscribed gossip topic
    // 3. Bob runs `trust provider show <provider>` → sees Alice's vouch
    // 4. Alice runs `trust provider distrust --broadcast <provider>`
    // 5. Bob's daemon receives the distrust signal
    // 6. Bob's WoT evaluation shows provider as distrusted
    Ok(())
}
```

### Phase 4 — CLI integration

```rust
// syncweb-cli/tests/cli_test.rs

#[test]
fn test_provider_vouch_with_broadcast() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("vouch-broadcast");
    // Start embedded node
    // Run `syncweb trust provider vouch <pubkey> --broadcast`
    // Verify signal appears on gossip topic
    Ok(())
}

#[test]
fn test_provider_distrust_with_broadcast() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("distrust-broadcast");
    // Start embedded node
    // Run `syncweb trust provider distrust <pubkey> --broadcast`
    // Verify signal appears on gossip topic
    Ok(())
}

#[test]
fn test_vouch_without_broadcast_still_local() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("vouch-local");
    // Run without --broadcast → no node opened, no gossip
    // Verify it's saved to indexing-state.json
    Ok(())
}
```

---

## Implementation

### Phase A — `ProviderTrustRecord` → `ProviderTrustSignal` conversion

#### `syncweb-core/src/indexing/wot.rs` or `reputation.rs`

```rust
impl ProviderTrustSignal {
    /// Create a trust signal from a signed provider trust record.
    pub fn from_trust_record(record: &ProviderTrustRecord) -> Result<Self> {
        let reporter = parse_pubkey(&record.issuer)?;
        Self::new_with_time(
            record.provider,
            match record.action {
                ProviderTrustAction::Vouch => SignalKind::Positive,
                ProviderTrustAction::Distrust => SignalKind::Negative,
                _ => return Err(SyncwebError::InvalidConfig(
                    "only vouch/distrust records can be converted to signals".into()
                )),
            },
            record.scope,
            record.sequence,
            &SigningKey::from_bytes(&record.issuer.as_bytes()), // needs actual key
        )
    }
}
```

Note: Conversion requires the signing key, which is only available during the CLI handler. The conversion should happen at the CLI layer, not in core.

### Phase B — CLI changes

#### `syncweb-cli/src/cli/commands.rs`

```rust
pub struct ProviderVouchArgs {
    provider: String,
    #[arg(long)]
    scope: Option<String>,
    #[arg(long, default_value = "locally vouched provider")]
    reason: String,
    #[arg(long, help = "Broadcast vouch via gossip trust stream")]
    broadcast: bool,
}

pub struct ProviderDistrustArgs {
    provider: String,
    #[arg(long)]
    scope: Option<String>,
    #[arg(long, default_value = "locally distrusted provider")]
    reason: String,
    #[arg(long, help = "Broadcast distrust via gossip trust stream")]
    broadcast: bool,
}
```

Keep existing `Vouch` and `Distrust` variants but add `broadcast` field.

#### `syncweb-cli/src/cli/indexing.rs` — `handle_provider_trust_record`

```rust
fn handle_provider_trust_record(
    ctx: &CliContext<'_>,
    provider: &str, scope: Option<&str>, reason: String,
    action: ProviderTrustAction, broadcast: bool,
) -> Result<()> {
    // ... existing local creation and persistence ...
    let record = ProviderTrustRecord::new(...)?;
    // ... local save ...

    if broadcast {
        // Open node, broadcast as ProviderTrustSignal on existing gossip topic
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            let node = open_node(data_dir).await?;
            let signal = ProviderTrustSignal::new_with_time(
                record.provider,
                match action {
                    ProviderTrustAction::Vouch => SignalKind::Positive,
                    ProviderTrustAction::Distrust => SignalKind::Negative,
                    _ => return Err(...),
                },
                record.scope,
                record.sequence,
                &signing_key(&IdentityManager::new(data_dir.join("identity.key"))?),
            )?;
            let gossip = ProviderReputationStore::default();
            let topic = gossip.subscribe_trust_stream(
                node.gossip_service(), Vec::new()
            ).await?;
            let (sender, _receiver) = GossipService::split(topic);
            gossip.publish_signal(node.gossip_service(), &sender, &signal).await?;
            node.stop().await?;
            Ok(())
        })?;
    }
    print_status(...)
}
```

---

## Gossip/network integration note

- Reuses the **existing** gossip topic `syncweb/provider-trust-stream/v1` (already defined in `reputation.rs`)
- Reuses the **existing** `ProviderTrustSignal` type
- Reuses the **existing** `ProviderReputationStore::publish_signal()` and `subscribe_trust_stream()` methods
- The `trust stream subscribe` handler already receives and applies incoming signals via `reputation.ingest_trust_signal()`
- No new gossip infrastructure needed — just wiring the existing vouch/distrust commands to the existing gossip machinery
- The `--broadcast` flag is optional; without it, behavior is unchanged (local only)

## Files to modify/plan

| File | Changes |
|------|---------|
| `syncweb-cli/src/cli/commands.rs` | Add `--broadcast` flag to `Vouch` and `Distrust` subcommands |
| `syncweb-cli/src/cli/indexing.rs` | Wire gossip broadcast in `handle_provider_trust_record` |
| `syncweb-core/tests/provider_trust_gossip_test.rs` | New — two-node vouch/distrust gossip tests |
| `syncweb-cli/tests/cli_test.rs` | CLI integration tests with/without `--broadcast` |
