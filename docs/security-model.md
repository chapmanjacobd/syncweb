# Security Model

## Access Enforcement Layers

Syncweb enforces content access through three layers: filesystem isolation between networks, connection-level peer verification, and application-level capability checks for private content sharing.

### Layer 1: Process Isolation per Network

Each network runs as a separate daemon process with its own data directory, identity key, and blob store:

```
~/.local/share/syncweb/
├── default/
│   ├── blobs/               # iroh FsStore
│   ├── node.db              # NodeDatabase
│   ├── identity.key         # Ed25519 secret key
│   └── syncweb.sock         # IPC socket
├── work/
│   ├── blobs/
│   ├── node.db
│   ├── identity.key         # different key from default/
│   └── syncweb.sock
└── personal/
    ├── blobs/
    ...
```

A daemon in network `work` physically cannot serve blobs from network `personal` — the blob stores are separate directories managed by separate processes. Blobs shared across networks are deduplicated via filesystem hardlinks (e.g. `work/blobs/<hash>` ↔ `personal/blobs/<hash>`).

The CLI routes all commands to the correct daemon via the `--network` flag:

```bash
syncweb --network work start           # daemon for work network
syncweb --network work download <url>  # routes to work daemon IPC socket
syncweb start                          # uses default/ daemon
```

### Layer 2: Peer Whitelist (Connection-Level)

Each per-network daemon uses iroh's `EndpointHooks` trait (`iroh-1.0.3/src/endpoint/hooks.rs`) to gate connections based on both the peer's identity and the negotiated ALPN protocol. The hook fires **after TLS handshake completion, before any protocol handler runs** — the remote peer's `EndpointId` and `ALPN` are both known.

```rust
use iroh::endpoint::{Connection, EndpointHooks, AfterHandshakeOutcome};
use iroh_base::EndpointId;
use std::collections::HashSet;

struct NetworkMembershipHook {
    member_keys: HashSet<EndpointId>,
}

impl EndpointHooks for NetworkMembershipHook {
    async fn after_handshake(
        &self,
        conn: &Connection,
    ) -> AfterHandshakeOutcome {
        match conn.alpn() {
            // Docs and gossip protocols: only network members can connect.
            // These protocols are inherently network-scoped (doc namespaces derive
            // from network shared secrets; gossip topics are network-specific).
            b"iroh-docs/1" | b"iroh-gossip/1" => {
                if self.member_keys.contains(&conn.remote_id()) {
                    AfterHandshakeOutcome::Accept
                } else {
                    AfterHandshakeOutcome::Reject {
                        error_code: VarInt::from_u32(0),
                        reason: b"not a network member".to_vec(),
                    }
                }
            }
            // Blob protocol: allow all peers through. Per-blob authorization
            // (membership check or PublicReadOnly bypass) is enforced at
            // the protocol handler level.
            _ => AfterHandshakeOutcome::Accept,
        }
    }
}
```

Installed at daemon startup via:

```rust
let endpoint = builder
    .secret_key(identity.secret_key().clone())
    .hooks(NetworkMembershipHook { member_keys })
    .bind()
    .await?;
```

### Layer 3: Per-Blob Authorization (Protocol-Level)

Blob serving uses a custom auth-gated ALPN protocol that replaces the default `iroh-blobs/1` ALPN. The protocol handler reads from the same on-disk `FsStore` (plaintext) and enforces per-blob authorization:

```
                    ┌──────────────────────────────────────────┐
                    │         IrohNode (per-network daemon)     │
                    │                                          │
Peer ──QUIC/TLS────┤  Router                                  │
(EndpointId        │  ├── ALPN "iroh-docs/1"   → Docs        │
 known)            │  ├── ALPN "iroh-gossip/1" → Gossip      │
                    │  └── ALPN "syncweb-blobs-auth/1"         │
                    │      → AuthBlobProtocol                  │
                    │        ├── blob is PublicReadOnly? → serve │
                    │        ├── peer in network? → serve      │
                    │        └── otherwise → reject            │
                    │                                          │
                    └── FsStore (plaintext, same on-disk store) ┘
```

Per-request logic in `AuthBlobProtocol`:

1. Extract `blob_hash` from the request
2. Query `node_db`: is this blob in a `PublicReadOnly` folder?
   - Yes → serve immediately (world-readable)
3. Query `node_db`: is the requesting peer a member of this daemon's network?
   - Yes → serve
   - No → reject (`AccessDenied`)

### Layer 4: PrivateLink Capability Tokens

Private/private links use cryptographic bearer tokens for sharing individual content manifests:

```
syncweb://private/<manifest-hash>/<capability>?expires=<unix-timestamp>
```

- `capability`: 64 hex chars from two concatenated UUIDv4 values (128 bits of entropy)
- `expires_at`: Unix timestamp after which the capability is rejected
- Revocation lists are stored in the node database and checked at serve time

Capability links flow through Layer 3 — the requesting peer must be a network member (or the blob must be PublicReadOnly) AND the capability must be valid, unexpired, and unrevoked.

### SyncMode Enforcement Summary

| SyncMode | Enforcement |
|---|---|
| `PublicReadOnly` | Any authenticated peer can download (no membership check at Layer 2 or Layer 3) |
| `SendReceive` | Network membership required at Layer 2 (docs/gossip) and Layer 3 (blobs) |
| `SendOnly` | Network membership required at Layer 2 (docs/gossip) and Layer 3 (blobs) |
| `ReceiveOnly` | Network membership required at Layer 2 (docs/gossip) and Layer 3 (blobs) |
| `ReceiveEncrypted` | Same as `ReceiveOnly` (encryption is not yet implemented; placeholder enum) |
| `PrivateLink` | Network membership at Layer 2 and Layer 3, plus valid capability token at Layer 4 |

### Default Daemon (no --network)

The `default/` daemon has no network membership concept. It applies **no** peer whitelist and no per-blob authorization — any authenticated peer can connect and download any blob. This preserves backward compatibility with the current unstructured usage model. Users who want access control should adopt named networks.

### Identity per Network

Each per-network daemon uses a separate Ed25519 identity key (`data_dir/<network>/identity.key`), generated on first run. This means:

- A peer has a different `EndpointId` (node ID) in each network it belongs to
- Key compromise in one network does not affect other networks
- Keys can be rotated per-network without affecting other memberships
- The `default/` daemon keeps its own identity (or shares one if the user copies the key file)

PublicReadOnly blobs served from a network daemon use that daemon's identity. There is no need for a separate identity just for public serving — the auth protocol handles the PublicReadOnly exception internally. However, users who want to isolate public serving (e.g., for independent key rotation or relay configuration) can create a dedicated network with only PublicReadOnly folders.

## Threat Model

### What a Modified Client CANNOT Do

- **Download non-public blobs from networks it is not a member of.** The protocol-level auth check (Layer 3) rejects the blob request. For docs/gossip protocols, the `after_handshake()` hook (Layer 2) rejects the connection entirely before protocol handlers run.
- **Use an expired or revoked PrivateLink.** Capability expiry and revocation are checked server-side before serving content (Layer 4).
- **Forge network membership.** Requires the Ed25519 secret key of a legitimate member (the TLS handshake authenticates peers cryptographically).
- **Access non-PublicReadOnly blobs by guessing content hashes.** Without network membership OR a valid PrivateLink token, the blob request is rejected at Layer 3.

### What a Modified Client CAN Still Do

- **Exfiltrate any blob it is authorized to access.** If a peer is a legitimate network member, it can download and save any blob. This is inherent to any system where authorized readers receive plaintext.
- **Use a stolen Ed25519 key from a member.** Identity theft at the connection level. Mitigated by key rotation and out-of-band trust management.
- **Keep plaintext data after network removal.** A peer removed from a network retains any blobs already downloaded. Retroactive revocation requires encryption-at-rest, which syncweb does not implement by design (plaintext storage enables efficient deduplication and serving).
- **Serve public blobs to anyone.** PublicReadOnly content is world-readable — this is the intended behavior.
- **Observe metadata.** Hash-based requests reveal what content a peer is interested in. Size and timing patterns may leak information about the content.

## Design Rationale

### Why Not Encryption-at-Rest?

Syncweb stores blobs as plaintext on disk for several reasons:
- Content-addressed storage already provides integrity verification (BLAKE3-Bao)
- Plaintext enables direct serving via iroh-blobs without a decrypt-then-serve step
- Filesystem hardlinks allow cross-network deduplication without re-encryption
- Users who need encrypted storage can place syncweb's data directory on an encrypted filesystem (LUKS, fscrypt, etc.)

### Why Not Per-Blob Cryptographic Authorization?

Per-blob authorization (e.g. encrypting each blob with a network-specific key) would require either:
1. Modifying iroh-blobs serve path, which has no authorization hook, or
2. Changing content addressing (the iroh hash would be of ciphertext, not plaintext)

The per-network daemon approach achieves equivalent security with simpler implementation: each daemon only has the blobs for its network, and non-members cannot connect.

### Cost of Per-Network Daemons

Each additional network daemon adds:
- 1 OS process
- ~50-100 MB RAM (iroh endpoint, gossip mesh, blob store indices)
- 1 QUIC socket (port 0 by default — OS-assigned)
- Optional relay connection (unless `--no-relay`)

Disk usage is sub-linear — blob hardlinks deduplicate content shared across networks.
