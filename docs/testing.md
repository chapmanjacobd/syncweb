# Testing Strategy

## CI gates

The default CI suite runs `make check` and `make test` on Linux, Windows, and
macOS. The Linux `network-scenarios` job also runs:

```sh
make test-network
```

This focused gate selects the in-process network scenarios for folder sync,
network membership, blob transfer, snapshots, daemon synchronization, and the
Syncthing relay mock. It is intentionally separate from the full suite so a
network regression is visible as a named CI failure rather than being buried
among unit and CLI tests.

Tests that require an external Syncthing relay are opt-in with
`SYNCWEB_INTEROP=1`; they are not part of the default CI gate because they
depend on third-party network availability.

## Reproducible performance fixtures

The performance numbers below remain targets, not release gates. The
repository provides a repeatable fixture entry point so changes can be
compared against the same workloads:

```sh
make perf-fixtures
make perf-fixtures ARGS=bench
```

The default target runs the fixed-size 10,000-file scanner fixture, 10,000
filter evaluations across 100 rules, 1,000 peer-cache lookups, and the
deterministic 10/50/100/1000-node swarm fixture. `ARGS=bench` runs the
Criterion archive, daemon, filter, search, sort, schedule, and stats
workloads. Set `CARGO_TARGET_DIR` to an isolated directory when comparing
clean builds or running multiple worktrees concurrently.

The minimum reference fixture for future LAN gates is a Linux x86_64 host
with 8 physical cores, 16 GiB RAM, NVMe storage, and a dedicated 1 GbE
network. Live multi-process gates should record the same host profile and
remain separate from the deterministic simulator until equivalent hardware is
available. Record CPU, memory, storage, and network details with results;
timings from different hardware are not release decisions.

### LAN scale acceptance gates

The deterministic control-plane gate runs one verified block through 10, 50,
100, and 1,000 simulated nodes with a four-round budget. Each size must
converge, report non-zero inventory traffic and repair attempts, and emit
inventory bytes, duplicate advertisements, completed and failed repairs, and
the process's observable file-descriptor count. `make perf-fixtures` also
records user CPU, system CPU, and maximum resident memory through GNU
`/usr/bin/time` when available.

These simulator results are regression gates for control-plane policy, not a
claim of live-LAN capacity. A future 10/50/100-node multi-process gate must
use the reference host profile above and additionally record transfer
completion, network traffic, open files, CPU, and memory before its results
can become a release decision.

### Local discovery control-plane budget

The UDP beacon carries endpoint addresses and an optional relay URL only. It
does not carry blob inventories, manifests, or provider leases. Named networks
scope both the beacon port and payload, while the configured beacon interval
and three-message change burst bound local traffic. For a node with `T`
broadcast targets and interval `I`, the conservative per-minute estimate is:

```text
ceil(60 seconds / I) + 3 change broadcasts, multiplied by T
```

At the default one-second interval and two broadcast targets this is 126
datagrams per node per minute: 12,600 for 100 nodes, 63,000 for 500 nodes,
and 126,000 for 1,000 nodes. Operators running larger LANs should increase
`discovery.beacon_interval_ms`, restrict `discovery.interface`, or disable
the beacon and use relay/DHT discovery. Inventory reconciliation remains on
the scoped gossip/DHT control plane, where `dht_max_records_per_minute`
limits writes per topic and `dht_read_interval_ms` limits discovery polling.
mDNS is an independent system service and should be included in host packet
captures when measuring the complete LAN budget.

### Unit Tests
- Identity management
- DeviceId bidirectional conversion (Syncthing ↔ Iroh)
- Ticket parsing/generation
- Capability serialization
- Filter engine evaluation
- Version tracking
- PeerTracker age-based cache eviction (LRU/FIFO)
- EfficientPeerCache bitmask operations
- ParallelScanner directory traversal
- Partial fetch: filter by peer count (min_peers/max_peers)
- Health check: seeder count per blob
- Find engine: regex, glob, exact matching with constraints
- Sort engine: niche, frecency, peers, folder-aggregate
- Stat output: format, terse, custom template
- Network: create, join, leave, invite, kick

### Integration Tests
- Two nodes: create folder, join, sync files
- Three nodes: sendonly -> sendreceive -> receiveonly
- Public folder: publish -> subscribe -> read
- Selective sync: ls without download, then download
- Network partition: offline edits, reconnect, merge
- Data versioning: bump, check, update
- Data package lifecycle: init, add, bump, publish, search, install, upgrade, remove
- Multi-version coexistence: install v1, install v2, switch between them
- Atomic upgrade: verify rollback works if upgrade fails
- Package integrity: verify catches corrupted files
- Package discovery: publish, search, info across two nodes
- Parallel operations: ls/import use parallelism by default; `--threads 1` disables it
- Partial fetch: download --max-peers improves seeder counts
- Cache eviction: test LRU and FIFO under memory pressure
- Large peer network: test EfficientPeerCache with 1000+ peers
- Networks: two-node network create, invite, join, folder sync
- Networks: three-node network with mixed roles
- Swarm scale fixture: deterministic replication at 10, 50, 100, and 1000 simulated nodes
- Find: regex, glob, exact search across folder boundaries
- Sort: niche, frecency, peers with various filter combinations
- Stat: detailed output, local/global diffs, availability display
- Init: folder creation with URL output and network membership

### Interop Tests
- Syncthing node to syncweb folder join
- syncweb to Syncthing folder join
- Bidirectional sync
- Relay-only connection
- `syncweb devices` output is a valid Syncthing DeviceId (`config bep` shows relay settings)

### Local Tests
- Schedule parsing, cross-midnight windows, and per-folder overrides
- Persisted bandwidth statistics and reset behavior
- CLI JSON output and command discovery
- Configured schedule persistence

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
| Package export (1000 files, default) | < 2.5s (6x speedup) |
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
| File conflict resolution UX | Low | Medium | Clear naming convention; LWW for text, keep-both for binary |
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

1. Functional parity: The documented core flows work (`folders create`, `folders join`, `ls`, `find`, `sort`, `stat`, `download`, `devices`, `folders`, `watch`, `start`, `stop`, `version`)
2. Performance: Faster sync, lower resource usage than Syncthing
3. Public folders: `share`/`folders join`/`download` work end-to-end
4. Data versioning: Data package lifecycle works (init, add, bump, publish, search, install, upgrade, remove, verify)
5. Networks: `network create/join/invite/kick` work across devices
6. Syncthing relay: Two syncweb nodes can communicate via Syncthing relay when direct QUIC fails
7. UX: A single binary supports embedded operation and optional daemon supervision; configuration remains optional
8. Reliability: No data loss, verified transfers, BLAKE3 integrity on all transfers
9. Conflict resolution: Automatic LWW for text, keep-both for binary, older version renamed
10. Parallel operations: 4-6x speedup for ls and import (default on)
11. Memory efficiency: PeerTracker handles 1000+ peers without OOM
12. Network robustness: Filter-based partial fetch improves seeder counts for rare content
13. Cache efficiency: Age-based eviction prevents unbounded memory growth
14. Find parity: regex/glob/exact search with all syncweb-py filter options
15. Sort parity: niche/frecency/peers/random sorting with folder aggregates
16. Stat parity: detailed file info with availability, version vectors, local/global diffs
17. Logging: Structured tracing with configurable levels and log rotation
18. Schedules: Global + per-folder active-hours scheduling works


```console
$ syncweb folders create --network home ~/Documents
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
