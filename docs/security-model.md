# Security Model

## Access Enforcement Layers

Syncweb enforces content access through three layers: filesystem isolation between networks, connection-level peer verification, and application-level capability checks for private content sharing.

Networks are always private — they exist to create access-controlled spaces. A daemon running without `--network` (the `default/` daemon) applies no gating and is fully open.

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

Each per-network daemon uses iroh's `EndpointHooks` trait (`iroh-1.0.3/src/endpoint/hooks.rs`) to gate connections based on both the peer's identity and the negotiated ALPN protocol. The hook fires after TLS handshake completion, before any protocol handler runs — the remote peer's `EndpointId` and `ALPN` are both known.

```rust
use iroh::endpoint::{Connection, EndpointHooks, AfterHandshakeOutcome, VarInt};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

struct MembershipHook {
    member_keys: Arc<RwLock<HashSet<iroh::PublicKey>>>,
}

impl EndpointHooks for MembershipHook {
    async fn after_handshake(
        &self,
        conn: &Connection,
    ) -> AfterHandshakeOutcome {
        let guard = self.member_keys.read().await;
        // No networks configured — no gating (fully open).
        if guard.is_empty() {
            return AfterHandshakeOutcome::Accept;
        }
        if guard.contains(&conn.remote_id()) {
            AfterHandshakeOutcome::Accept
        } else {
            AfterHandshakeOutcome::Reject {
                error_code: VarInt::from_u32(0),
                reason: b"not a network member".to_vec(),
            }
        }
    }
}
```

Installed at daemon startup via:

```rust
let endpoint = builder
    .secret_key(identity.secret_key().clone())
    .hooks(MembershipHook { member_keys })
    .bind()
    .await?;
```

### Layer 3: Blob Authorization (Connection-Level)

All three ALPNs (blobs, docs, gossip) are gated uniformly at Layer 2. There is no separate per-request protocol handler — connections that reach the iroh protocol handlers are authorized:

```
                    ┌──────────────────────────────────────────┐
                    │         IrohNode (per-network daemon)     │
                    │                                          │
Peer ──QUIC/TLS────┤  EndpointHooks::after_handshake          │
(EndpointId        │  ├── ALPN "iroh-docs/1"   → member only  │
 known)            │  ├── ALPN "iroh-gossip/1" → member only  │
                    │  └── ALPN "/iroh-bytes/4"  → member only  │
                    │      ├── member → serve                  │
                    │      └── otherwise → reject              │
                    │                                          │
                    └── FsStore (plaintext, same on-disk store) ┘
```

### Layer 4: PrivateLink Capability Tokens

Private links use cryptographic bearer tokens for sharing individual content manifests:

```
syncweb://private/<manifest-hash>/<capability>?expires=<unix-timestamp>
```

- `capability`: 64 hex chars from two concatenated UUIDv4 values (128 bits of entropy)
- `expires_at`: Unix timestamp after which the capability is rejected
- Revocation lists are stored in the node database and checked at serve time

Capability links are enforced at blob serve time — the requesting peer must be a network member AND the capability must be valid, unexpired, and unrevoked.

### Access Enforcement by Network Type

| Mode | Enforcement |
|---|---|
| Network daemon (`--network <name>`) | Network membership required at Layer 2 for all ALPNs (docs, gossip, blobs) |
| Default daemon (no `--network`) | No gating — any authenticated peer can connect |

### Default Daemon (no --network)

The `default/` daemon has no network membership concept. It applies no peer whitelist — any authenticated peer can connect and download any blob. This is the fully public mode. Users who want access control should adopt named networks.

### Identity per Network

Each per-network daemon uses a separate Ed25519 identity key (`data_dir/<network>/identity.key`), generated on first run. This means:

- A peer has a different `EndpointId` (node ID) in each network it belongs to
- Key compromise in one network does not affect other networks
- Keys can be rotated per-network without affecting other memberships
- The `default/` daemon keeps its own identity (or shares one if the user copies the key file)

## Threat Model

### What a Modified Client CANNOT Do

- Download blobs from networks it is not a member of. The `after_handshake()` hook (Layer 2) rejects the connection before any protocol handler runs.
- Use an expired or revoked PrivateLink. Capability expiry and revocation are checked server-side before serving content.
- Forge network membership. Requires the Ed25519 secret key of a legitimate member (the TLS handshake authenticates peers cryptographically).
- Access blobs by guessing content hashes. Without network membership, the connection is rejected at Layer 2.

### What a Modified Client CAN Still Do

- Exfiltrate any blob it is authorized to access. If a peer is a legitimate network member, it can download and save any blob. This is inherent to any system where authorized readers receive plaintext.
- Use a stolen Ed25519 key from a member. Identity theft at the connection level. Mitigated by key rotation and out-of-band trust management.
- Keep plaintext data after network removal. A peer removed from a network retains any blobs already downloaded. Retroactive revocation requires encryption-at-rest, which syncweb does not implement by design (plaintext storage enables efficient deduplication and serving).
- Observe metadata. Hash-based requests reveal what content a peer is interested in. Size and timing patterns may leak information about the content.

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
