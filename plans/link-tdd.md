# TDD Plan: `link` command family

## Divergence
Name "Create and resolve stable syncweb links" implies network-addressable links usable by other peers. Actual: all operations are purely local — stored only in `indexing-state.json`, never published to iroh-docs or iroh-blobs.

## Subcommand breakdown

| Subcommand | About text | Actual behavior | Problem |
|---|---|---|---|
| `link create` | "Create an immutable, private, or mutable link" | `--name` (mutable): signs a `MutablePointer`, saves to `state.links.pointers`, never publishes to iroh-docs. `--private`: generates a random capability, saves to `state.links.revoked`. No network I/O. | Created links are only accessible on the creating machine. Other peers have no way to discover them. |
| `link resolve` | "Resolve a stable link" | Called **synchronously** (no `.await`). Reads from in-memory `ResolverState` loaded from `indexing-state.json`. Never connects to network. `LinkResolver::fetch_with_mirrors()` exists but is never called by CLI. | Cannot resolve links created by other peers. Shows stale/cached data only. |
| `link revoke` | "Revoke a private capability link" | Adds revocation key to in-memory `HashSet` + `state.links.revoked`. Never propagates to peers. | If Alice shares a private link with Bob, then revokes it, Bob never learns. |

---

## Tests

### Phase 1 — Capture current local-only behavior

```rust
// syncweb-core/tests/links_test.rs  (new file)

#[test]
fn test_link_create_is_local_only() {
    // Create a mutable link
    let mut state = IndexingState::default();
    let pointer = MutablePointer::signed_with_secret_key(
        node_id, "my-alias", hash, 1, &secret_key
    )?;
    state.links.pointers.push(pointer.clone());
    // No iroh-docs document was created, no blob was stored
    // The only persistence is in `state` (JSON on disk)
    assert_eq!(state.links.pointers.len(), 1);
}

#[test]
fn test_link_resolve_never_fetches_network() {
    // Build a LinkResolver with empty state
    let resolver = LinkResolver::new();
    // Create a link to a hash that doesn't exist anywhere
    let link = ContentLink::new(Hash::from_bytes([0u8; 32]));
    // Resolve succeeds because it only reads in-memory state
    let resolution = resolver.resolve(&Link::Content(link))?;
    assert_eq!(resolution.manifest, hash);
    // No connection was attempted — would fail if it tried
}

#[test]
fn test_link_revoke_does_not_propagate() {
    // Alice creates a private link, Bob has a copy
    let link = PrivateLink::generate(hash, far_future)?;
    // Alice revokes it locally
    resolver.revoke(&link)?;
    // Bob's resolver has no knowledge of the revocation
    let bob_resolver = LinkResolver::new();
    let bob_resolution = bob_resolver.resolve(&Link::Private(link.clone()))?;
    // Bob still resolves successfully — revocation not propagated
    assert_eq!(bob_resolution.manifest, hash);
}
```

### Phase 2 — Network publication for link create

```rust
// syncweb-core/tests/links_publish_test.rs  (new file)

#[tokio::test]
async fn test_link_create_mutable_publishes_to_docs() -> anyhow::Result<()> {
    let dir = TestDirectory::new("link-publish")?;
    let (relay, relay_url, _server) = iroh::test_utils::run_relay_server().await?;

    // Alice creates a node
    let alice = make_node(&dir, "alice", &relay, &relay_url).await?;
    let hash = alice.blob_store().add_bytes(b"hello").await?;

    // Alice creates a mutable pointer and publishes to a folder document
    let manager = FolderManager::new(&alice);
    let folder = manager.create(SyncMode::SendReceive).await?;
    let pointer = MutablePointer::signed_with_secret_key(
        alice.endpoint().id(), "docs", hash, 1, alice.endpoint().secret_key(),
    )?;

    // Publish pointer to folder document
    let key = format!("sys/links/mutable/{}", pointer.alias);
    folder.doc().set_bytes(
        folder.author(), key.as_bytes(), pointer.to_bytes()?,
    ).await?;

    // Bob joins the folder and can read the pointer
    let bob = make_node(&dir, "bob", &relay, &relay_url).await?;
    let ticket = folder.ticket(alice.endpoint().addr(), true).await?;
    let bob_folder = FolderManager::new(&bob)
        .join(ticket.to_string(), SyncMode::ReadOnly).await?;

    // Bob reads the pointer from the document
    let entry = bob_folder.doc().get(bob_folder.author(), key.as_bytes()).await?;
    assert!(entry.is_some());

    alice.stop().await?;
    bob.stop().await?;
    Ok(())
}

#[tokio::test]
async fn test_link_create_private_capability_publishes() -> anyhow::Result<()> {
    // Similar to above but for PrivateLink — store the capability key
    // in a folder document with appropriate access controls
    Ok(())
}
```

### Phase 3 — Network resolution for link resolve

```rust
#[tokio::test]
async fn test_link_resolve_fetches_remote_pointer() -> anyhow::Result<()> {
    // Alice publishes a mutable pointer to a folder document
    // Bob resolves it and fetches the pointer from Alice's node
    // ... setup two nodes, folder sync ...

    let link = Link::Name(NameLink { publisher: alice_id, alias: "docs".into() });

    // Bob's resolver should attempt to fetch from the network
    let mut resolver = LinkResolver::new();
    resolver.add_provider_fetch(Box::new(|name_link: &NameLink| {
        // Connect to Alice's node, query the folder document
        Box::pin(async move { /* ... */ })
    }));
    let resolution = resolver.resolve(&link)?;
    assert_eq!(resolution.manifest, expected_hash);
    Ok(())
}
```

### Phase 4 — Revocation propagation

```rust
#[tokio::test]
async fn test_link_revoke_propagates_to_peers() -> anyhow::Result<()> {
    // Alice creates a private link, shares with Bob
    // Alice revokes → publishes revocation to gossip topic
    // Bob receives revocation → his resolver rejects the link

    let gossip_topic = TopicId::from_bytes(*blake3::hash(b"syncweb/link-revocations/v1").as_bytes());

    // Alice publishes revocation
    let alice_gossip = alice_node.gossip_service();
    alice_gossip.subscribe(gossip_topic, vec![]).await?;
    let (sender, _) = GossipService::split(topic);
    sender.broadcast(revocation_bytes.into()).await?;

    // Bob receives revocation
    let bob_gossip = bob_node.gossip_service();
    bob_gossip.subscribe(gossip_topic, vec![alice_node.endpoint().id()]).await?;
    let received = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(Some(msg)) = bob_gossip.try_recv(gossip_topic) {
                return msg;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }).await?;

    let revocation: PrivateLink = serde_json::from_slice(&received)?;
    bob_resolver.revoke(&revocation)?;
    assert!(bob_resolver.resolve(&Link::Private(revocation)).is_err());
    Ok(())
}
```

### Phase 5 — CLI integration tests

```rust
// syncweb-cli/tests/cli_test.rs

#[test]
fn test_link_create_with_publish_flag() -> anyhow::Result<()> {
    let data_dir = cli_test_dir("link-publish-cli");
    // Create folder, add file
    // Run `syncweb link create file.txt --name mylink --publish <namespace>`
    // Verify pointer appears in the folder document
    Ok(())
}

#[test]
fn test_link_resolve_from_network() -> anyhow::Result<()> {
    // Start two nodes
    // Node A creates + publishes a link
    // Node B resolves it with `syncweb link resolve <link>`
    // Verify manifest hash matches
    Ok(())
}

#[test]
fn test_link_revoke_propagates() -> anyhow::Result<()> {
    // Node A creates private link, shares ticket
    // Node B imports the link, can use it
    // Node A revokes with `syncweb link revoke --broadcast`
    // Node B tries to resolve → fails
    Ok(())
}
```

---

## Implementation

### Phase A — `link create --publish`

#### `syncweb-cli/src/cli/commands.rs`

```rust
pub struct LinkCreateArgs {
    source: PathBuf,
    // ... existing args ...
    #[arg(long, help = "Namespace (folder) to publish the link into")]
    publish: Option<String>,
}
```

#### `syncweb-cli/src/main.rs` / `syncweb-cli/src/cli/indexing.rs`

In `handle_link`, when `--publish <namespace>` is given:
1. Open node, get folder
2. After creating the pointer/link, write it to the folder's iroh-docs document at key `sys/links/mutable/{alias}` or `sys/links/private/{capability}`
3. The link is now synced to all folder peers via iroh-docs

### Phase B — `link resolve` with network fetch

#### `syncweb-core/src/indexing/links.rs`

Add an optional fetch hook to `LinkResolver`:

```rust
pub struct LinkResolver {
    state: Arc<Mutex<ResolverState>>,
    provider_fetch: Option<Box<dyn Fn(&NameLink) -> Pin<Box<dyn Future<Output=Result<LinkResolution>>> + Send> + Send + Sync>>,
}

impl LinkResolver {
    pub fn with_provider_fetch(fetch: impl ...) -> Self { ... }
    pub async fn resolve_remote(&self, link: &Link) -> Result<LinkResolution> {
        // Try local first, then fall back to provider_fetch
    }
}
```

#### `syncweb-cli/src/cli/indexing.rs`

Make `handle_link` async. For `Resolve`:
1. Try local resolution first
2. If not found, open node, try `resolve_remote()` which connects to peers via gossip to find the folder hosting the link's mutable pointer
3. Cache fetched pointers locally

### Phase C — `link revoke` gossip broadcast

#### `syncweb-core/src/indexing/links.rs`

Add revocation gossip topic and service:

```rust
pub const REVOCATION_GOSSIP_TOPIC: &[u8] = b"syncweb/link-revocations/v1";

pub async fn publish_revocation(gossip: &GossipService, link: &PrivateLink) -> Result<()> {
    let topic = TopicId::from_bytes(*blake3::hash(REVOCATION_GOSSIP_TOPIC).as_bytes());
    gossip.subscribe(topic, vec![]).await?;
    let (sender, _) = GossipService::split(topic);
    gossip.publish(&sender, serde_json::to_vec(link)?).await
}
```

#### `syncweb-core/src/daemon/daemon.rs`

On startup, subscribe to revocation gossip topic and apply incoming revocations to local `LinkResolver`.

#### `syncweb-cli/src/main.rs`

In `handle_link` for `Revoke`:
1. Add `--broadcast` flag
2. When set, open node, publish revocation via gossip

---

## Gossip/network integration note

All three subcommands need network wiring:
- `link create --publish` — uses existing iroh-docs folder sync (no new gossip)
- `link resolve` — needs optional provider fetch (connects to peers on demand)
- `link revoke --broadcast` — adds a new gossip topic `syncweb/link-revocations/v1` for propagating revocations

The iroh-docs approach for `--publish` is preferred because folder participants already sync docs automatically. For discovery of links outside of folders, a gossip topic could be added, but initial implementation should focus on the folder-docs approach which is simpler.

## Files to modify/plan

| File | Changes |
|------|---------|
| `syncweb-core/src/indexing/links.rs` | Add `LinkResolver::with_provider_fetch()`, `resolve_remote()`, `publish_revocation()` |
| `syncweb-core/src/indexing.rs` | Export new link functions |
| `syncweb-core/src/daemon/daemon.rs` | Subscribe to revocation gossip on startup |
| `syncweb-cli/src/cli/commands.rs` | Add `--publish` to `Create`, make `handle_link` async-capable |
| `syncweb-cli/src/cli/indexing.rs` | Rewrite `handle_link`: network publish, async resolve, gossip revoke |
| `syncweb-cli/src/main.rs` | Update dispatch for async `handle_link` |
| `syncweb-core/tests/links_publish_test.rs` | New — two-node publish/resolve/revoke tests |
| `syncweb-cli/tests/cli_test.rs` | CLI integration tests |
