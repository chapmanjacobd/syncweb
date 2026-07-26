# Plan 20: Network Access Control via EndpointHooks

## Problem

Non-network-members can discover blob hashes through `iroh-docs/1` and `iroh-gossip/1`
ALPNs, then fetch those blobs from the `iroh-blobs/1` ALPN with no gating. The
SyncMode (`PublicReadOnly` vs private) has zero effect at the protocol level.
Private blob hashes that leak out-of-band are fetchable by any authenticated
iroh peer.

## Design

EndpointHooks gating on `iroh-docs/1` + `iroh-gossip/1` by network membership,
plus wiring the `is_public` flag so `PublicReadOnly` folders record it in
`blob_folders`.

`iroh-blobs/1` stays open — no custom ALPN. The protection is **discovery**:
non-members can't sync docs or join gossip, so they can't learn private blob
hashes. A leaked hash is still fetchable, but that's consistent with iroh's
design (the hash is the capability).

### SyncMode × Access Interaction Table

| SyncMode | `can_write_local()` | `can_receive()` | `can_grant_write()` | Blobs recorded `is_public` | Ticket ShareMode |
|---|---|---|---|---|---|
| `SendReceive` | yes | yes | **yes** | no | Write |
| `SendOnly` | yes | no | **no** | no | Read |
| `ReceiveOnly` | no | yes | **no** | no | Read |
| `ReceiveEncrypted` | no | yes | **no** | no | Read |
| `PublicReadOnly` | no | yes | **no** | **yes** | Read |

Key: `can_write_local()` = local node may write to the doc. `can_grant_write()` = tickets from this folder may be ShareMode::Write. Currently these are conflated in a single `can_write()`.

---

## Part A: Split `can_write` into local-write and grant-write

### Motivation

`SendOnly` needs `can_write() == true` so the local node can `set_blob()` and
publish entries. But tickets should be `ShareMode::Read` — a joiner should not
be able to modify a SendOnly folder's doc. The single `can_write()` can't
express both.

### Changes

**1. `syncweb-core/src/folder/sync_mode.rs`** — rename/refactor

```rust
impl SyncMode {
    // Replaces existing can_write(). Controls whether the *local* node
    // may create, modify, or delete entries.
    pub const fn can_write_locally(self) -> bool {
        matches!(self, Self::SendReceive | Self::SendOnly)
    }

    // NEW. Controls whether tickets issued from this folder may grant
    // ShareMode::Write to the joining peer.
    pub const fn can_grant_write(self) -> bool {
        matches!(self, Self::SendReceive)
    }

    // Unchanged
    pub const fn can_receive(self) -> bool {
        matches!(self, Self::SendReceive | Self::ReceiveOnly | Self::ReceiveEncrypted | Self::PublicReadOnly)
    }
}
```

**2. `syncweb-core/src/folder/syncweb_folder.rs`** — update call sites

| Line | Current | Replacement |
|---|---|---|
| 29 | `fn can_write(self)` delegation | Rename to `can_write_locally(self)` and delegate |
| 147 | `self.sync_mode.can_write()` in `can_write_as()` | `self.sync_mode.can_write_locally()` |
| 218 | `!self.sync_mode.can_write()` in `set_blob()` | `!self.sync_mode.can_write_locally()` |
| 240 | `!self.sync_mode.can_write()` in `set_blob_ref()` | `!self.sync_mode.can_write_locally()` |
| 257 | `!self.sync_mode.can_write()` in `delete_entry()` | `!self.sync_mode.can_write_locally()` |
| 269 | `writable && self.sync_mode.can_write()` in ticket gen | `writable && self.sync_mode.can_grant_write()` |

---

## Part B: EndpointHooks gating `iroh-docs/1` + `iroh-gossip/1`

### Overview

Implement `iroh::endpoint::EndpointHooks` with an `after_handshake` hook that
checks the connecting peer against the network member list. Install the hook
on `Endpoint::builder()` before `.bind()`.

### 1. New file: `syncweb-core/src/node/membership_hook.rs`

```rust
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use iroh::endpoint::{Connection, EndpointHooks, AfterHandshakeOutcome};
use iroh_base::VarInt;

#[derive(Debug)]
pub struct MembershipHook {
    pub member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
}

impl EndpointHooks for MembershipHook {
    fn after_handshake<'a>(
        &'a self,
        conn: &'a Connection,
    ) -> impl Future<Output = AfterHandshakeOutcome> + Send + 'a {
        async move {
            let alpn = conn.alpn();
            match alpn {
                b"iroh-docs/1" | b"iroh-gossip/1" => {
                    let guard = self.member_keys.read().await;
                    if guard.contains(&conn.remote_id()) {
                        AfterHandshakeOutcome::Accept
                    } else {
                        AfterHandshakeOutcome::Reject {
                            error_code: VarInt::from_u32(0),
                            reason: b"not a network member".to_vec(),
                        }
                    }
                }
                _ => AfterHandshakeOutcome::Accept,
            }
        }
    }
}
```

Note: the `after_handshake` method returns `impl Future`. In iroh 1.0.3,
`EndpointHooks::after_handshake` takes `&self` and `&Connection` and returns
`impl Future<Output = AfterHandshakeOutcome>`. The exact signature may require
a return of `Box::pin(async move { ... })` in practice — verify against
`target/doc/src/iroh/endpoint/hooks.rs.html`.

### 2. Update `syncweb-core/src/node/iroh_node.rs`

Add the hook to `Endpoint::builder()`:

```rust
// In new_with_address_lookup(), before .bind():
let hook = MembershipHook { member_keys: member_keys.clone() };
let endpoint = builder
    .address_lookup(address_lookup.clone())
    .secret_key(identity.secret_key().clone())
    .hooks(hook)                                          // NEW
    .bind()
    .await?;
```

Add field to `IrohNode` struct:

```rust
pub struct IrohNode {
    // ... existing fields ...
    member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,   // NEW
}
```

Update signature:

```rust
pub async fn new_with_address_lookup(
    identity: IdentityManager,
    data_dir: PathBuf,
    relay_mode: RelayMode,
    address_lookup: MemoryLookup,
    member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,    // NEW
) -> Result<Self>
```

Update convenience `new()`:

```rust
pub async fn new(..., member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>) -> Result<Self> {
    Self::new_with_address_lookup(..., MemoryLookup::new(), member_keys).await
}
```

### 3. Update `syncweb-core/src/daemon/daemon.rs`

At daemon startup, load network members into the shared set:

```rust
// In Daemon::new(), before open_identity_and_node():
let member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>> = {
    let networks = node_db.list_networks()?;
    let keys: HashSet<iroh::PublicKey> = networks
        .iter()
        .flat_map(|n| n.members.iter().copied())
        .collect();
    Arc::new(RwLock::new(keys))
};
```

Pass to `open_identity_and_node()` → `IrohNode::new_with_address_lookup()`.

### 4. Update `syncweb-core/src/net/network_manager.rs`

When members are added or removed, update the shared set:

```rust
// Add to NetworkManager struct:
pub member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,

// In NetworkManager::add_member(), after DB write:
self.member_keys.write().await.insert(member);

// In NetworkManager::kick() / remove_member(), after DB write:
self.member_keys.write().await.remove(&member);
```

Give `Daemon::new()` access to this shared set by passing it to
`NetworkManager` at construction.

### 5. Update `syncweb-core/src/init.rs`

Update `open_node()` to pass empty member set (standalone/CLI usage):

```rust
pub async fn open_node(data_dir: &Path) -> Result<IrohNode> {
    let identity = IdentityManager::new(data_dir.join("identity.key"))?;
    let empty_keys = Arc::new(RwLock::new(HashSet::new()));
    IrohNode::new(identity, data_dir.join("data"), RelayMode::Default, empty_keys).await
}
```

### 6. Update all test helpers

Update `test_node()` in `tests/test_utils/mod.rs` and every per-file test helper
that constructs an `IrohNode` to pass an empty member set.

---

## Part C: Wire `is_public` into blob recording

### Motivation

The `is_public` column and `is_blob_public()` query exist (Phase 3) but no
production code sets `is_public: true`. PublicReadOnly folders should record
their blobs with this flag.

### Changes

Find every call site of `record_blob_folder()` and `record_blob_folder_in_network()`
(if it exists). At each site, determine whether the folder's SyncMode is
`PublicReadOnly` and pass `is_public: true` accordingly.

The blob→folder associations are recorded in the sync engine when entries are
processed. The sync engine has access to the folder's `SyncwebFolder` (via
`SyncEngine` or `FolderManager`), which knows its `SyncMode`.

At each `record_blob_folder` call site, add:

```rust
let is_public = folder_sync_mode.is_public();
node_db.record_blob_folder(&hash, &namespace_id, entry_key, is_public)?;
```

---

## Part D: Delete unreachable `downloadable` filter code

### Motivation

`syncweb-core/src/search.rs` lines 312–318 check `FindQuery.sync_mode` against
`"publicreadonly"` and `"sendonly"` when `downloadable` is true. But
`FindQuery.sync_mode` is never set by any code path — it is always `None`.
This is dead code.

### Change

Remove the `sync_mode` field from `FindQuery`, remove the `downloadable` filter
logic that checks it, and update any dead references. The `--downloadable` CLI
flag still works for excluding folders whose SyncMode forbids receiving (the
downstream filter does not need per-entry sync_mode).

Actually verify whether `FindQuery.sync_mode` is truly unused everywhere before
deleting — search `sync_mode` in the codebase to confirm no write paths exist.

---

## Files Changed (Summary)

| File | Change |
|---|---|
| `syncweb-core/src/folder/sync_mode.rs` | Split `can_write()` → `can_write_locally()` + `can_grant_write()` |
| `syncweb-core/src/folder/syncweb_folder.rs` | Update 4 gate checks + 1 ticket call site |
| `syncweb-core/src/node/membership_hook.rs` | **NEW** — `MembershipHook` implementing `EndpointHooks` |
| `syncweb-core/src/node/iroh_node.rs` | Add `member_keys` param, `hooks()` call, `IrohNode` field |
| `syncweb-core/src/daemon/daemon.rs` | Load member set, pass to `IrohNode` and `NetworkManager` |
| `syncweb-core/src/net/network_manager.rs` | Shared `member_keys: Arc<RwLock<>>`, update on add/remove |
| `syncweb-core/src/init.rs` | Pass empty member set to `IrohNode::new()` |
| `syncweb-core/src/sync/engine.rs` or indexing | Wire `is_public` into `record_blob_folder()` calls |
| `tests/test_utils/mod.rs` + per-file helpers | Pass empty member set to all `IrohNode` constructors |
| `syncweb-core/src/search.rs` | Remove dead `sync_mode` / downloadable filter code (verify first) |

---

## Test Plan

### Unit tests (lib, `#[cfg(test)]`)

1. **`can_write` refactor**: Test `can_write_locally()` and `can_grant_write()` for each SyncMode variant against the expected values in the table above.
2. **`MembershipHook`**: Unit test with a mock `Connection` (or check if iroh test-utils provides one). Test accept for member, reject for non-member for both ALPNs, accept for blob ALPN regardless.

### Integration tests (`tests/integration/`)

3. **Two-node network gating test**: Alice creates a network, Bob is a member, Charlie is not. Connect Charlie to Alice's endpoint — assert `iroh-docs/1` rejected, `iroh-blobs/1` accepted.
4. **PublicReadOnly blob recording**: Create a folder with `SyncMode::PublicReadOnly`, add a blob, assert `is_blob_public()` returns true. Create a folder with `SyncMode::SendReceive`, add a blob, assert `is_blob_public()` returns false.

---

## Dependencies

| Part | Depends on |
|---|---|
| A (can_write refactor) | Nothing |
| B (EndpointHooks) | Nothing (iroh 1.0.3 has the trait) |
| C (wire is_public) | A (needs SyncMode for folder context) |
| D (dead code removal) | Nothing |
