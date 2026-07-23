# TDD Plan: `mirror` command

## Divergence
Name "mirror" implies data replication (copying content). Actual: only registers a blob ticket as a fallback provider — no data is copied.

## Decision
- **Rename** current `mirror add` → `provider add` (accurately describes: register a provider ticket)
- **Add** a real `mirror` subcommand that actually replicates/fetches data from one or more providers
- The new `mirror` command invokes `ResilienceService::ensure_replication()` (alias `replicate()`) to fetch and pin blobs
- Keep `mirror add` as a deprecated alias for `provider add` for backward compatibility

---

## Tests

### Phase 1 — Existing behavior (still works after rename)

```rust
// syncweb-cli/tests/cli_test.rs — test_provider_add_alias in full_suite_test.rs
// (add "provider" to the full_help_lists_all_commands test)
```

### Phase 2 — `provider add` tests (renamed from `mirror add`)

```rust
// syncweb-core/tests/links_test.rs  (new file, or inline in links.rs)

use syncweb_core::indexing::links::{LinkResolver, Mirror};

#[test]
fn test_provider_add_registers_ticket() {
    let resolver = LinkResolver::new();
    let blob_hash = Hash::from_bytes([0u8; 32]);
    // Create a ticket (simplified — real test uses actual iroh BlobTicket)
    let ticket = BlobTicket::new(blob_hash, ...);
    resolver.register_mirror(ticket.clone())?;
    // Verify it's stored
    assert!(resolver.has_mirror(&blob_hash, &ticket));
}

#[test]
fn test_provider_add_rejects_hash_mismatch() {
    let resolver = LinkResolver::new();
    let hash_a = Hash::from_bytes([1u8; 32]);
    let hash_b = Hash::from_bytes([2u8; 32]);
    // Create ticket for hash_b but claim it's for hash_a
    let ticket = make_ticket(hash_b);
    assert!(resolver.register_mirror_for(hash_a, ticket).is_err());
}

#[test]
fn test_provider_add_deduplicates() {
    let resolver = LinkResolver::new();
    let ticket = make_ticket(some_hash);
    resolver.register_mirror(ticket.clone())?;
    resolver.register_mirror(ticket.clone())?;
    assert_eq!(resolver.mirror_count(&some_hash), 1);
}
```

### Phase 3 — CLI alias test

```rust
// syncweb-cli/tests/cli_test.rs

#[test]
fn test_provider_add_alias_works() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("provider-add");
    // Run `syncweb provider add <hash> <ticket>`
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir.to_str().unwrap(),
               "provider", "add", "somehash", "someticket"])
        .output()?;
    assert!(output.status.success());

    // Verify it's persisted in indexing-state.json
    let state_path = data_dir.join("indexing-state.json");
    assert!(state_path.exists());
    let state: serde_json::Value = serde_json::from_slice(&fs::read(&state_path)?)?;
    assert!(state["links"]["mirrors"].as_array().unwrap().len() >= 1);

    fs::remove_dir_all(&data_dir)?;
    Ok(())
}

#[test]
fn test_deprecated_mirror_add_still_works() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("mirror-add-deprecated");
    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", data_dir.to_str().unwrap(),
               "mirror", "add", "somehash", "someticket"])
        .output()?;
    assert!(output.status.success());
    fs::remove_dir_all(&data_dir)?;
    Ok(())
}
```

### Phase 4 — New `mirror` (actual replication) integration test

```rust
// syncweb-core/tests/mirror_test.rs  (new file)

#[tokio::test]
async fn test_mirror_fetches_and_pins_blob() -> anyhow::Result<()> {
    let dir = TestDirectory::new("syncweb-mirror")?;
    let (relay, relay_url, _server) = iroh::test_utils::run_relay_server().await?;

    // Provider node has the blob
    let provider = make_node(&dir, "provider", &relay, &relay_url).await?;
    let hash = provider.blob_store().add_bytes(b"mirror me").await?;
    let ticket = provider.blob_store().ticket(provider.endpoint(), hash);

    // Consumer node has no blob, runs `mirror`
    let consumer = make_node(&dir, "consumer", &relay, &relay_url).await?;
    let indexing = IndexingService::in_memory()?;
    let resilience = indexing.resilience_service(
        ResilienceConfig::new(ReplicationBudget::new(1))
    );
    // Register provider
    let lease = make_lease(hash, ticket, provider.endpoint())?;
    resilience.record_lease(lease)?;

    // Act: run ensure_replication
    let result = resilience
        .ensure_replication(consumer.endpoint(), consumer.blob_store(), hash)
        .await?;

    // Assert: blob was fetched and pinned
    assert!(result.pinned);
    assert!(consumer.blob_store().has(hash).await?);
    assert_eq!(
        consumer.blob_store().get(hash).await?.to_vec(),
        b"mirror me"
    );

    provider.stop().await?;
    consumer.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_mirror_skips_already_pinned() -> anyhow::Result<()> {
    // Blob already exists locally → ensure_replication should return early
    // ... setup similar to above, but pre-fetch the blob ...
    let result = resilience
        .ensure_replication(consumer.endpoint(), consumer.blob_store(), hash)
        .await?;
    assert!(result.skipped); // or result.already_satisfied
    Ok(())
}
```

### Phase 5 — CLI `mirror` command integration test

```rust
// syncweb-cli/tests/cli_test.rs

#[tokio::test]
async fn test_mirror_command_replicates() -> anyhow::Result<()> {
    // Two-node test:
    // 1. Start provider node, create blob
    // 2. Start consumer node with --embedded
    // 3. Run `syncweb mirror --from <ticket>` on consumer
    // 4. Verify blob is now available on consumer
    let dir = cli_test_dir("mirror-cli");
    // ... setup nodes ...

    let output = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", dir.to_str().unwrap(),
               "--embedded",
               "mirror", "--hash", &hash.to_string(),
               "--from", &ticket.to_string()])
        .output()?;
    assert!(output.status.success());

    // Verify blob exists
    let verify = Command::new(env!("CARGO_BIN_EXE_syncweb"))
        .args(["--data-dir", dir.to_str().unwrap(),
               "stat", "--hash", &hash.to_string()])
        .output()?;
    assert!(verify.status.success());

    fs::remove_dir_all(&dir)?;
    Ok(())
}
```

---

## Implementation

### Phase A — Rename `mirror add` → `provider add`

#### `syncweb-cli/src/cli/commands.rs`

```rust
// New top-level command
#[command(about = "Manage blob provider registrations")]
Provider {
    #[command(subcommand)]
    command: ProviderCommand,
},

// Mirrors the current MirrorCommand
#[derive(Debug, Subcommand)]
pub enum ProviderCommand {
    #[command(about = "Register a blob ticket as an alternative provider")]
    Add { collection: String, provider: String },
}

// Keep Mirror but add deprecated notice + alias
#[command(about = "Register alternate content providers (deprecated: use 'provider add')")]
Mirror {
    #[command(subcommand)]
    command: MirrorCommand,
},
```

#### `syncweb-cli/src/main.rs`

```rust
Command::Provider { command } => cli::indexing::handle_provider(&ctx, command)?,
Command::Mirror { command } => {
    eprintln!("warning: 'mirror add' is deprecated, use 'provider add'");
    cli::indexing::handle_provider(&ctx, convert(command))?;
}
```

#### `syncweb-cli/src/cli/indexing.rs`

```rust
// Rename handle_mirror → handle_provider
pub fn handle_provider(ctx: &CliContext<'_>, command: ProviderCommand) -> Result<()> { ... }
// Keep handle_mirror as thin wrapper for backward compat
```

### Phase B — New `mirror` command (actual replication)

#### `syncweb-core/src/indexing/links.rs` or `resilience.rs`

```rust
// CLI-facing MirrorConfig for the new command
pub struct MirrorConfig {
    pub hash: Hash,
    pub tickets: Vec<BlobTicket>,
    pub min_providers: usize,
}

// Already exists: ResilienceService::ensure_replication()
// Just needs CLI wiring
```

#### `syncweb-cli/src/cli/commands.rs`

```rust
#[command(about = "Fetch and pin a blob from provider(s) to replicate content locally")]
Mirror(MirrorArgs),

pub struct MirrorArgs {
    #[arg(long, help = "Content hash to mirror")]
    pub hash: String,
    #[arg(long, help = "Blob ticket(s) for providers (can repeat)")]
    pub from: Vec<String>,
    #[arg(long, default_value_t = 2, help = "Minimum providers for healthy replication")]
    pub min_providers: usize,
}
```

#### `syncweb-cli/src/main.rs`

```rust
// handle_mirror_replication — uses --embedded node or daemon IPC
async fn handle_mirror_replication(ctx: &CliContext<'_>, args: &MirrorArgs) -> Result<()> {
    let hash = parse_hash(&args.hash)?;
    let tickets: Vec<BlobTicket> = args.from.iter().map(|t| t.parse()).collect::<Result<_>>()?;
    let node = open_node(ctx.data_dir).await?;
    let budget = ReplicationBudget::new(args.min_providers);
    let resilience = IndexingService::new(ctx.data_dir)?
        .resilience_service(ResilienceConfig::new(budget));
    for ticket in &tickets {
        let lease = ProviderLease::from_ticket(ticket, node.endpoint())?;
        resilience.record_lease(lease)?;
    }
    let result = resilience.ensure_replication(
        node.endpoint(), node.blob_store(), hash
    ).await?;
    println!("mirror result: pinned={}, fetched_from={:?}",
        result.pinned, result.fetched_from);
    node.stop().await?;
    Ok(())
}
```

---

## Gossip/network integration note

- The new `mirror` command uses `ensure_replication()` which already handles:
  - Provider ranking by reputation/WoT/XOR distance
  - Jitter-based retry
  - Failure tracking and provider invalidation
- No new gossip channels needed — provider leases are learned through existing gossip or manually registered
- The `provider add` subcommand is a manual override (same as current `mirror add`)

## Files to modify/plan

| File | Changes |
|------|---------|
| `syncweb-cli/src/cli/commands.rs` | Add `Provider` + `ProviderCommand` enum, add new `Mirror(MirrorArgs)`, deprecate old `Mirror` variant |
| `syncweb-cli/src/cli/indexing.rs` | Rename `handle_mirror` → `handle_provider`, add `handle_mirror_replication` |
| `syncweb-cli/src/main.rs` | Dispatch for `Provider` and new `Mirror`, add `handle_mirror_replication` |
| `syncweb-core/tests/mirror_test.rs` | New file — integration tests for `ensure_replication` via CLI |
| `syncweb-cli/tests/cli_test.rs` | Test `provider add`, deprecated `mirror add`, new `mirror` |
| `syncweb-cli/tests/full_suite_test.rs` | Add "provider" to help listing test |
