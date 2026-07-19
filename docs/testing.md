# Testing Strategy

### Unit Tests
- [ ] Identity management
- [ ] DeviceId bidirectional conversion (Syncthing ↔ Iroh)
- [ ] Ticket parsing/generation
- [ ] Capability serialization
- [ ] Filter engine evaluation
- [ ] Version tracking
- [ ] PeerTracker age-based cache eviction (LRU/FIFO)
- [ ] EfficientPeerCache bitmask operations
- [ ] ParallelScanner directory traversal
- [ ] Partial fetch: filter by peer count (min_peers/max_peers)
- [ ] Health check: seeder count per blob
- [ ] Find engine: regex, glob, exact matching with constraints
- [ ] Sort engine: niche, frecency, peers, folder-aggregate
- [ ] Stat output: format, terse, custom template
- [ ] Network: create, join, leave, invite, kick

### Integration Tests
- [ ] Two nodes: create folder, join, sync files
- [ ] Three nodes: sendonly -> sendreceive -> receiveonly
- [ ] Public folder: publish -> subscribe -> read
- [ ] Selective sync: ls without download, then download
- [ ] Network partition: offline edits, reconnect, merge
- [ ] Data versioning: bump, check, update
- [ ] Data package lifecycle: init → add → bump → publish → search → install → upgrade → remove
- [ ] Multi-version coexistence: install v1, install v2, switch between them
- [ ] Atomic upgrade: verify rollback works if upgrade fails
- [ ] Package integrity: verify catches corrupted files
- [ ] Package discovery: publish → search → info across two nodes
- [ ] Parallel operations: ls --parallel, import --parallel, export --parallel
- [ ] Partial fetch: download --max-peers improves seeder counts
- [ ] Cache eviction: test LRU and FIFO under memory pressure
- [ ] Large peer network: test EfficientPeerCache with 1000+ peers
- [ ] Networks: two-node network create, invite, join, folder sync
- [ ] Networks: three-node network with mixed roles
- [ ] Find: regex, glob, exact search across folder boundaries
- [ ] Sort: niche, frecency, peers with various filter combinations
- [ ] Stat: detailed output, local/global diffs, availability display
- [ ] Init: folder creation with URL output + network membership

### Interop Tests (Phase 7: with `--bep` flag)
- [ ] Syncthing node -> iroh-syncthing folder join
- [ ] iroh-syncthing -> Syncthing folder join
- [ ] Bidirectional sync
- [ ] Relay-only connection

---

## Performance Targets

| Metric | Target |
|--------|--------|
| Startup time | < 500ms |
| Memory (idle) | < 50MB |
| Memory (syncing 10GB) | < 200MB |
| Blob throughput (LAN) | > 500 MB/s |
| Blob throughput (WAN) | > 50 MB/s |
| Doc sync latency (LAN) | < 50ms |
| Discovery time (local) | < 1s |
| Discovery time (global/DHT) | < 10s (distributed-topic-tracker via BitTorrent DHT) |
| Peer cache lookup | < 1ms |
| Filter evaluation | < 10ms per entry |
| Scan (10k files, default) | < 500ms (6x speedup) |
| Import (1000 files, default) | < 3s (6x speedup) |
| Export (1000 files, default) | < 2.5s (6x speedup) |
| Cache eviction (10k entries) | < 10ms |
| Efficient cache memory (1000 peers) | < 1MB |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Iroh API changes | Low (1.0 released) | High | Pin versions, test against main branch |
| iroh-docs performance at scale | Medium | Medium | Benchmark early, optimize queries |
| Public folder spam/abuse | Medium | Low | Rate limit gossip, allowlist |
| Syncthing relay protocol v1 changes | Low | High | Protocol is simple (3 message types); monitor releases; fallback to iroh relay |
| Windows file locking | Medium | Medium | Test early, use iroh-blobs async API |
| File conflict resolution UX | Low | Medium | Clear naming convention; `syncweb conflicts` command; LWW for text, keep-both for binary |
| BitTorrent DHT availability | Low | Medium | DHT has ~10M+ nodes; fallback to iroh relays for connectivity |
| DHT write rate limits | Medium | Low | Tune `dht_write_limit` per-folder; accept slower re-discovery after long offline periods |
| distributed-topic-tracker version drift | Medium | Medium | Pin version; monitor upstream releases when upgrading iroh |
| Cache eviction thrashing | Low | Medium | Tune `max_cache_size` and `eviction_strategy` based on workload |
| Efficient cache overhead | Low | Low | Fallback to HashMap for small peer networks (< 100 peers) |
| Networks gossip overhead | Low | Low | Per-network topics are lightweight; merge under common topic via bubble detection |
| Relay key compatibility | Low | Low | Both use Ed25519; add validation tests for edge cases (padding, encoding variants) |
| Network invite spam | Low | Low | Shared-secret networks mitigate; rate-limit invites |
| Parallelism deadlocks | Low | High | Use rayon's work-stealing; limit thread count; add timeouts |
| Syncthing relay protocol breakage | Low | High | Protocol is simple (3 message types); monitor Syncthing releases; implement fallback |
| Unbounded blob store growth | Medium | Medium | Content pinning; GC for unpinned content; configurable max cache size |
| DHT blocked by corporate firewalls | Low | Medium | iroh-relay remains primary; DHT is supplementary; graceful degradation |
| Config file corruption on crash | Low | Low | Atomic writes (write to temp, rename); backup old config |
| Millions of entries in namespace | Low | Medium | iroh-docs lazy enumeration; pagination in `ls`/`find`; avoid loading all entries |

---

## Success Criteria

1. **Functional parity**: All syncweb-py commands work (create, join, accept, drop, ls, find, sort, stat, download, devices, folders, automatic, start, shutdown, version, repl)
2. **Performance**: Faster sync, lower resource usage than Syncthing
3. **Public folders**: `publish`/`subscribe` work end-to-end
4. **Data versioning**: Data package lifecycle works (init, add, bump, publish, search, install, upgrade, remove, verify)
5. **Networks**: `network create/join/invite/kick` work across devices
6. **Syncthing relay**: Two iroh-syncthing nodes can communicate via Syncthing relay when direct QUIC fails
7. **UX**: Single binary, no daemon, config file optional
8. **Reliability**: No data loss, verified transfers, BLAKE3 integrity on all transfers
9. **Conflict resolution**: Automatic LWW for text, keep-both for binary, older version renamed
10. **Parallel operations**: 4-6x speedup for ls, import, export (default on)
11. **Memory efficiency**: PeerTracker handles 1000+ peers without OOM
12. **Network robustness**: Filter-based partial fetch improves seeder counts for rare content
13. **Cache efficiency**: Age-based eviction prevents unbounded memory growth
14. **Find parity**: regex/glob/exact search with all syncweb-py filter options
15. **Sort parity**: niche/frecency/peers/random sorting with folder aggregates
16. **Stat parity**: detailed file info with availability, version vectors, local/global diffs
17. **Logging**: Structured tracing with configurable levels and log rotation
18. **Schedules**: Global + per-folder bandwidth scheduling works


```console
$ syncweb init --network home ~/Documents
Created folder documents
Local files: 1,284; imported: 1,284; verified: 1,284
Private by default.

$ syncweb network invite home laptop
Invitation: syncweb://network/...

$ syncweb folders
NAME       MODE         LOCAL     REMOTE  STATE
documents  SendReceive  1,284     1,284   up to date

$ syncweb stat documents/report.pdf
Content: b3:8e7a...
Local: yes (verified)
Known providers: 2 (last checked 14s ago)
Policy: private from network "home"
```
