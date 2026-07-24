# TopicTracker Integration

Integrate `distributed-topic-tracker`'s `TopicTracker` more deeply into syncweb so folder namespaces are announced to the gossip layer and peers are proactively discovered during sync operations.

## Problem

`TopicTracker` is created in `IrohNode` and exposed to consumers, but it is only called in the repair flow (`daemon/ipc.rs` and `cli/main.rs` `try_repair`). Normal folder operations (create, join) never announce the namespace, and `SyncEngine` never calls `find_peers` during sync. This means:

- Peers that join a folder are invisible to the topic tracker
- Sync/download rely entirely on iroh's built-in endpoint/gossip/docs discovery, with no fallback to the topic tracker
- On daemon restart, existing folders are not re-announced

## Changes

### 1. Thread `TopicTracker` into `FolderManager`

File: `syncweb-core/src/folder/manager.rs`

- Add `topic_tracker: TopicTracker` field
- Update constructor to take `&IrohNode` and store `node.topic_tracker().clone()` (or take `TopicTracker` directly)
- Call `self.topic_tracker.announce(folder.namespace_id()).await` on:
  - `FolderManager::create`  — after namespace is created and mode is set
  - `FolderManager::join`    — after the doc is accepted
  - `FolderManager::accept`  — when an already-available namespace is claimed locally

### 2. Thread `TopicTracker` into `SyncEngine`

File: `syncweb-core/src/sync/engine.rs`

- Add `topic_tracker: Option<TopicTracker>` field
- Update `from_node` or constructor to optionally store it
- In `sync` / `sync_with_params` / `fetch`, before starting reconciliation:
- Call `topic_tracker.find_peers(namespace_id).await` 
- Pass discovered peers into the sync loop or log them for debugging
- (Optional) Use peers to seed the docs engine's peer set via `docs_engine.set_peers(namespace, peer_ids)` (requires resolving `PublicKey` → `PeerId` via `Endpoint:lookup_peer_id`)

### 3. Re-announce on Daemon Startup

File: `syncweb-core/src/daemon/daemon.rs`

- After `Daemon` initializes and loads existing folders, iterate all managed folders and call `topic_tracker.announce(ns).await` for each

### 4. Verify CLI `handle_repair` paths still work

Files: `syncweb-cli/src/main.rs`, `syncweb-core/src/daemon/ipc.rs`

- No changes needed — these already call `topic_tracker.find_peers` for the repair flow
- Verify they continue to work after `TopicTracker` is shared/cloned

## Dependencies

- `TopicTracker` already derives `Clone` — BUT cloning shares the underlying `Arc<Mutex<HashMap<...>>>`, so `announce()` and `find_peers()` calls from different owners race on the same internal state. Ensure callers serialize access to `TopicTracker` where correctness depends on non-interleaving.
- `TopicTracker::announce` is not idempotent — calling it multiple times may re-publish a namespace to the gossip layer. The caller MUST gate on whether the namespace was already announced. Solution: maintain a `HashSet<NamespaceId>` of already-announced namespaces in the caller (e.g., `FolderManager` or `DaemonInner`), and only call `announce()` if the namespace is not yet in the set. On daemon startup, the set is populated from the folder registry before iterating.
- `TopicTracker::find_peers` may return stale results from the last discovery round — callers should treat results as hints, not authoritative peer lists.
- `TopicTracker` has no garbage collection — topics accumulate in the internal `HashMap` forever. For long-running daemons, this is a memory leak. Fix: add a `gc()` method that removes topics not re-announced within a configurable TTL (e.g., 24h). The daemon calls `gc()` on a periodic timer (e.g., every 6h). Topics whose folder is still in the registry are re-announced by the startup loop, so they survive GC. Topics whose folder was removed are naturally GC'd.
- `FolderManager` and `SyncEngine` both take `&IrohNode` already, so the wiring is straightforward - `IrohNode` should expose `topic_tracker()` as `Arc<TopicTracker>` to safely share across consumers.

## Notes

- `TopicTracker::announce` and `find_peers` both lazily subscribe to the discovery layer — so they are safe to call even if the topic was already joined.
- iroh's gossip layer already provides neighbor discovery within a topic once connected. `TopicTracker` adds an explicit global lookup (e.g., via DHT or explicit tracker server) to find initial peers to connect to when gossip propagation is too slow or partition occurs.

---

## Insights from iroh-content-discovery (iroh-experiments)

The [iroh-experiments/content-discovery](https://github.com/n0-computer/iroh-experiments/tree/main/content-discovery) repo implements a complementary architecture using a dedicated tracker server model. Below are observations and gaps relevant to syncweb's TopicTracker approach.

### Architecture Comparison

| Aspect | syncweb (TopicTracker) | iroh-content-discovery |
|---|---|---|
| Model | P2P gossip + DHT auto-discovery | Client-server (dedicated tracker) |
| Discovery | Implicit via gossip topic join | Explicit announce/query over QUIC |
| Protocol | Gossip messages (iroh-gossip) | Binary request/response (postcard over QUIC, ALPN `n0/tracker/1`) |
| Identity | Endpoint's signing key (via `RecordPublisher`) | Any key; announces signed by host |
| Storage | In-memory topic HashMap | Persistent `redb` database on tracker |
| Verification | None (trust gossip neighbors) | Proactive probing (size check, random chunk fetch) |
| Content scope | Namespace IDs (docs) | Blob hashes & hash sequences (`HashAndFormat`) |
| Freshness | No TTL | Timestamped with configurable expiry + GC |
| Multi-tracker | N/A | Parallel announce/query to multiple trackers |

### Key Insights for syncweb

1. Content verification gap
`find_peers` returns gossip neighbors without verifying they actually hold the namespace. iroh-content-discovery's tracker proactively probes hosts — for partial content it checks the unverified blob size; for complete content it randomly fetches a blake3 chunk. syncweb has no equivalent of this verification loop.

2. Signed announcements
Every `SignedAnnounce` includes the host's signature over the serialized announcement. This prevents spoofing and allows the tracker to store/relay claims authoritatively. syncweb's TopicTracker implicitly trusts any peer that joins the gossip topic.

3. Announce kinds (Partial / Complete)
The tracker distinguishes between peers that have partial data vs complete data. syncweb could benefit from a similar distinction: a peer that just joined a folder may only have some entries, while a peer that created the folder has everything.

4. Timestamped announces with GC
The tracker stores announces with `AbsoluteTime` (μs since epoch), and runs a GC loop that:
- Removes announces older than `announce_expiry` (default 7 days)
- Keeps expired announces if they were recently probed (within `probe_expiry`)
- Runs every `gc_interval` (default 5 min)

syncweb's TopicTracker has no GC — topics accumulate in the `HashMap` forever.

5. Probing loop
The tracker maintains continuous probe tasks per endpoint. Each probe:
- Connects to the peer
- For `Partial` claims: checks the unverified blob size
- For `Complete` claims: randomly fetches a chunk (raw blobs) or a child's chunk (hash sequences)
- Records results with timestamps

This gives the tracker confidence in its query responses. syncweb has no equivalent background verification.

6. Protocol designed for blobs, not docs
iroh-content-discovery's protocol is built around `HashAndFormat` (blob hash + raw/hash-seq format). The verification mechanisms (chunk probing, size checks) are blob-specific. This protocol cannot be directly reused for namespace/docs verification — a different verification strategy would be needed (e.g., requesting a random entry's content from the doc).

7. Parallel multi-tracker operations
`announce_all` and `query_all` fan out to multiple trackers concurrently using `buffered_unordered`. This pattern is useful if syncweb ever needs to support multiple discovery backends (e.g., DHT gossip + one or more tracker servers).

8. 0-RTT connection optimization
Both announce and query attempt 0-RTT connections first, falling back to 1-RTT if rejected. syncweb's TopicTracker uses gossip which has its own latency characteristics, but this pattern is worth noting if we add a direct QUIC-based tracker in the future.

### Gaps Identified

1. No content verification in `find_peers` — neighbors are returned without any proof they hold the namespace data
2. No announce metadata — the current announce just joins a gossip topic; it doesn't encode completeness, recency, or host identity for verification
3. No GC or expiry — topic subscriptions accumulate indefinitely in memory
4. Single discovery mechanism — no fallback or parallel to dedicated tracker servers
5. No probing/re-verification — once a peer is discovered as a gossip neighbor, its content availability is never re-checked
6. No signed attestations — no cryptographic proof that a peer actually owns a namespace

### Potential Future Directions (not blocked on current plan)

- Async verification: After `find_peers` returns neighbors, optionally probe them. Caveat: iroh-docs has no per-peer single-entry fetch API — sync is whole-doc. Verification would require establishing a direct connection and querying the peer's blob store for a specific content hash (`iroh_blobs::get`), then checking if the peer holds any blob referenced by the folder's doc.
- Multi-backend discovery: Abstract peer discovery behind a trait so syncweb can use both gossip-based (current TopicTracker) and tracker-server-based (iroh-content-discovery client) backends simultaneously
- Announce metadata: Extend `TopicTracker::announce` to accept optional metadata (e.g., entry count, sync cursor) that could be gossiped alongside presence
