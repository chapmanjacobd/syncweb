# syncweb-core

Core library for [syncweb](https://github.com/chapmanjacobd/syncweb) -- delay-tolerant peer-to-peer file synchronization.

## Modules

| Module | Description |
|--------|-------------|
| `folder` | Folder management, sync modes, namespace lifecycle |
| `node` | Iroh node wrapper, identity management (HKDF-derived per-folder keys) |
| `net` | Networking and transport fallback (QUIC + relay) |
| `storage` | Configuration and persistent storage (redb) |

## Key Types

```rust
use syncweb_core::folder::{FolderManager, SyncMode};
use syncweb_core::node::iroh_node::{IrohNode, RelayMode};
use syncweb_core::node::identity::{DeviceId, IdentityManager};
use syncweb_core::net::TransportFallback;
use syncweb_core::storage::Config;
```

## Design

- Content-addressed storage -- BLAKE3 + Bao trees for verified streaming and deduplication
- CRDT conflict resolution -- last-writer-wins with best-effort text diffs
- Engine pattern -- dedicated storage thread with message-passing keeps the async runtime responsive
- One namespace per folder -- independent sync, permissions, and sharing
- Per-folder author keys -- derived via HKDF from a master identity; revocable per-folder

## Dependencies

Built on the [Iroh](https://iroh.computer/) networking stack:

- `iroh` -- peer connections and relay
- `iroh-blobs` -- content-addressed blob transfer
- `iroh-docs` -- CRDT document replication
- `iroh-gossip` -- pub/sub messaging
- `distributed-topic-tracker` -- DHT-based peer discovery

## License

[MIT](https://opensource.org/licenses/MIT)
