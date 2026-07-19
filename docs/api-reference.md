# API Reference and Implementation Patterns

Errors should name the layer that failed. For example, `manifest verified; no
provider reachable`, `capability rejected by peer`, and `materialization blocked:
path escapes target` are more actionable than `sync failed`.

## Grounded implementation patterns and libraries

The code examples in this plan describe intended boundaries; exact Iroh APIs
must be confirmed against the pinned crate versions during Phase 1.

### Workspace and service boundaries

Prefer a library-first workspace so the CLI does not become the application
boundary:

```text
core/       typed IDs, manifests, policy, errors
store/      blobs, docs, SQLite/config persistence
net/        Iroh endpoint, gossip, discovery, provider resolution
service/    folder, queue, catalog, package use cases
cli/        clap parsing and human/JSON rendering
```

Commands call typed services and render returned values. Core code must not print
progress or parse CLI strings. This also allows integration tests to use services
directly and keeps a future daemon or GUI from duplicating behavior.

```rust
struct AppServices<C, S, N> {
    catalog: C,
    store: S,
    network: N,
}

impl<C: Catalog, S: ContentStore, N: Network> AppServices<C, S, N> {
    async fn download(&self, request: DownloadRequest)
        -> Result<DownloadHandle, AppError>;
}
```

### Persistence and crash consistency

Use SQLite for small mutable control-plane state and Iroh blobs/docs for
content-addressed and replicated state. A user-visible operation that spans both
cannot rely on one atomic transaction, so model it as a resumable workflow:

```rust
enum PublishStep {
    Drafted,
    BlobsPinned,
    ManifestStored,
    HeadUpdated,
    CatalogAnnounced,
}
```

Persist completion of each idempotent step. On restart, resume forward or perform
an explicit compensating action, such as unpinning blobs that never became
reachable from a published manifest. Never mark a transfer or publication
complete before verification and durable state updates finish.

### Security boundaries

- Parse tickets, URLs, manifests, and gossip records into untrusted types; only
  verification produces `Verified<T>`.
- Domain-separate signed bytes by protocol, record type, and schema version.
- Validate normalized logical paths before joining them to an output directory,
  and materialize through temporary files plus atomic rename.
- Bound record sizes, collection entry counts, extraction output, queue length,
  concurrent peers, and HTTP request bodies.
- Resolve policy at publication, indexing, replication, fetch, and
  materialization time. Cached decisions may explain prior actions but cannot
  authorize new ones.
- Redact capability tokens and shared secrets from URLs in logs and error
  reports.

### Validation gates by phase

| Phase | Smallest meaningful gate |
|---|---|
| Foundation | restart preserves identity; corrupted config fails explicitly |
| Folder core | two local nodes reconcile one file and reject an unauthorized writer |
| File operations | interrupted import resumes without exposing a partial file |
| Networks | invitation grants only the intended network/folder capabilities |
| Public/package | manifest signature and every materialized blob are verified |
| Backup/partial fetch | restore is byte-identical; provider loss triggers fallback |
| Polish/interop | CLI JSON remains compatible; optional BEP failure cannot corrupt Iroh state |

Performance targets should be treated as benchmark hypotheses until fixtures,
hardware, file-size distribution, concurrency, and warm/cold cache conditions are
specified. Correctness gates must not be relaxed to reach a throughput target.


---

## Appendix: Iroh 1.0.2 API Reference

### iroh-blobs (0.103.0)

```rust
use iroh::{Endpoint, endpoint::presets, protocol::Router};
use iroh_blobs::{BlobsProtocol, store::fs::Store as BlobStore, ticket::BlobTicket, ALPN as BLOBS_ALPN};

// Setup
let endpoint = Endpoint::bind(presets::N0).await?;
let blob_store = BlobStore::persistent(data_dir.join("blobs"))?;
let blobs = BlobsProtocol::new(&blob_store, None);
let router = Router::builder(endpoint.clone())
    .accept(BLOBS_ALPN, blobs)
    .spawn();

// Add data
let tag = blob_store.add_bytes(data).await?;
let hash = tag.hash;

// Create public ticket
let addr = endpoint.addr();
let ticket = BlobTicket::new(addr, hash, tag.format);

// Fetch (lazy)
let reader = blob_store.get(hash).await?;
// Range request
let reader = blob_store.get_range(hash, 0..1024).await?;
```

### iroh-docs (0.101.0)

```rust
use iroh::{Endpoint, endpoint::presets, protocol::Router};
use iroh_blobs::{BlobsProtocol, store::fs::Store as BlobStore, ALPN as BLOBS_ALPN};
use iroh_docs::{protocol::Docs, ALPN as DOCS_ALPN};
use iroh_gossip::{net::Gossip, ALPN as GOSSIP_ALPN};

// Setup (requires blobs + gossip)
let endpoint = Endpoint::bind(presets::N0).await?;
let blob_store = BlobStore::persistent(data_dir.join("blobs"))?;
let gossip = Gossip::builder().spawn(endpoint.clone());
let docs = Docs::persistent(data_dir.join("docs"))
    .spawn(endpoint.clone(), blob_store.clone(), gossip.clone())
    .await?;
let blobs = BlobsProtocol::new(&blob_store, None);
let router = Router::builder(endpoint.clone())
    .accept(BLOBS_ALPN, blobs)
    .accept(GOSSIP_ALPN, gossip)
    .accept(DOCS_ALPN, docs)
    .spawn();

// Create author + namespace
let author = Author::generate();
let namespace = NamespaceSecret::generate();
let namespace_id = namespace.public();

// Create doc (replica)
let mut replica = docs.create_replica(namespace.clone()).await?;
replica.insert_entry(Entry::new(
    &author,
    b"path/to/file",
    hash,  // blob hash
    size,
)).await?;

// Subscribe to changes
let mut events = replica.subscribe().await?;
while let Some(event) = events.next().await {
    match event {
        DocEvent::Insert(entry) => { /* new entry */ }
        DocEvent::Remove(entry) => { /* entry removed */ }
    }
}
```

### iroh-gossip (0.101.0)

```rust
use iroh_gossip::{net::Gossip, proto::TopicId};

// Setup (done as part of docs setup above)

// Subscribe to topic
let topic = TopicId::from_bytes(*b"iroh-syncthing/public-folders");
let mut events = gossip.subscribe(topic).await?;

// Publish
gossip.publish(topic, payload).await?;

// Get peers on topic
let peers = gossip.peers(topic).await?;
```

### distributed-topic-tracker (0.3.5)

```rust
use distributed_topic_tracker::Node;

// Create a topic tracker node backed by the BitTorrent DHT
// Automatically bootstraps gossip topics via DHT lookup
let tracker = Node::new(gossip.clone(), endpoint.clone()).await?;

// The tracker provides an AutoDiscoveryGossip extension trait on iroh::Gossip.
// When subscribed to a gossip topic through the tracker, it will:
// 1. Query the DHT for other nodes on the same topic
// 2. Decrypt & verify DHT records using time-rotated keys
// 3. Join discovered peers with pacing
// 4. Spawn background actors for bubble detection and merge

// Subscribe to a gossip topic with DHT-based auto-discovery
let topic = TopicId::from_bytes(*b"iroh-syncthing/folder-abc123");
let mut events = tracker.subscribe(topic).await?;

// Announce presence on a topic via DHT
tracker.announce(topic).await?;

// The tracker also handles:
// - Time-rotated signing/encryption keys (per-minute from topic hash)
// - Rate limiting (default: 5 DHT writes per minute)
// - Bubble detection (merge isolated clusters < 4 neighbors)
// - Message overlap merge (detect network partitions)
```

### iroh (Endpoint)

```rust
use iroh::{Endpoint, endpoint::presets};

// Bind endpoint with default presets (relay + discovery)
let endpoint = Endpoint::bind(presets::N0).await?;

// Get identity
let node_id = endpoint.node_id();  // EndpointId / NodeId
let addr = endpoint.addr();        // EndpointAddr with relay + direct addrs

// Connect to peer
let conn = endpoint.connect(addr, b"my-alpn").await?;

// Accept connections
let conn = endpoint.accept().await?.await?;

// Graceful shutdown
endpoint.close().await;
```

---

*Document version: 3.2*
*Amended: 2026-07-17*
*Target: iroh 1.0.2, iroh-blobs 0.103.0, iroh-docs 0.101.0, iroh-gossip 0.101.0, distributed-topic-tracker 0.3.5*
*Added: Networks concept (multi-folder + multi-device groups under gossip topics)*
*Added: find command design (regex/glob/exact search with depth/size/time filters)*
*Added: stat command design (detailed file metadata, availability, version vectors, local/global diffs)*
*Added: sort command design (niche, frecency, peers, folder-aggregate sorting)*
*Added: init/config command design (folder creation + URL output, config management)*
*Added: BEP Phase 2 minimal identity (DeviceId conversion, --bep flag annotation)*
*Added: BEP Phase 7 full protocol translation (moved from Phase 7+ deprioritized — still complex, but identity is cheap)*
*Added: Standard CS patterns (cache eviction, parallel traversal, bitmask presence, consistent hashing)*
*Added: Data Package Management (non-apt alternative to dapt) — full lifecycle with iroh-docs manifests, iroh-blobs content addressing, gossip-based discovery, atomic upgrades, multi-version coexistence*
*Added: Opt-In Indexing Service (syncweb indexing) for Catalogs, Resilience, and WoT Metadata (merged Proposals 1, 5, 7, 11)*
