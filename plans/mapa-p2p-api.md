# API Plan: mapa P2P bridge (remove Rust FFI from mapa)

## Current state

mapa (`../mapa`) is a Kotlin Multiplatform collaborative mapping app. Its P2P
sync layer lives in `core/src/sync.rs` as a Rust library loaded into the JVM
via **UniFFI** (JNI bindings). This forces mapa to bundle `libmapa_core.so`,
depend on `cargo-ndk`, and carry an entire Rust build toolchain.

The P2P API surface exposed through UniFFI is:

```
IrohNode
├── new(event_handler)              → constructor, in-memory
├── new_persistent(data_dir, handler) → constructor, persistent storage
├── node_id()                       → String
├── dial_peer(node_id)              → Result<bool>
├── append_event(collection, blob)  → Result<SyncEvent>
├── get_events(collection)          → Result<Vec<SyncEvent>>
├── get_events_paged(coll, p, sz)   → Result<Vec<SyncEvent>>
├── share_collection_read_only(c)   → Result<String>    (ticket)
├── import_collection(c, ticket)    → Result<()>
├── join_gossip_topic(topic)        → Result<()>
├── join_gossip_topic_with_discovery(topic) → Result<()>
├── leave_gossip_topic(topic)       → Result<()>
├── send_gossip_message(t, bytes)   → Result<()>
├── get_connected_peers()           → Result<Vec<ConnectedPeer>>
├── block_peer(node_id)             → Result<()>
├── unblock_peer(node_id)           → Result<()>
├── get_blocked_peers()             → Result<Vec<String>>
└── shutdown()                      → Result<()>

SyncEventHandler  (implemented by Kotlin, called by Rust)
├── on_sync_event(collection_id, SyncEvent)
├── on_node_status(status: String)
└── on_gossip_event(topic, sender, message)

SyncEvent { author: String, payload: Vec<u8>, timestamp: u64 }
ConnectedPeer { node_id: String, first_seen_secs: u64, last_seen_secs: u64 }
```

syncweb is a pure Rust P2P file-sync tool using the **same Iroh stack**
(iroh 1.0, iroh-blobs, iroh-docs, iroh-gossip). It already has
`DocsEngine`, `GossipService`, `BlobStore`, `IdentityManager`, and a daemon
with Unix-socket IPC. Adding a WebSocket frontend that exposes the same
Iroh primitives mapa needs is a thin layer.

> **Scope note:** This plan covers only the Iroh-based P2P primitives.
> GIS/OSM/analysis (MapaEngine, PBF parsing, MVT tiles, GeoParquet,
> geometry utilities) stays in mapa and is out of scope for syncweb.

---

## Target architecture

```
┌──────────────────────────────────────────────┐
│  mapa (Kotlin — Compose Multiplatform)        │
│  ┌─────────────────────────────────────────┐ │
│  │ WsIrohNode (new `actual` implementation)│ │
│  │   → WebSocket client (Ktor / OkHttp)    │ │
│  │   → binary frames, arraybuffer          │ │
│  └────────────────┬────────────────────────┘ │
└───────────────────┼───────────────────────────┘
                    │ ws://localhost:PORT/v1
         ┌──────────┴──────────┐
         │  syncweb            │
         │  ┌────────────────┐ │
         │  │ WsBridgeServer │ │  ← new module in syncweb-core
         │  │  ┌───────────┐ │ │
         │  │  │ IrohNode   │ │ │  ← existing, wraps endpoint/docs/gossip/blobs
         │  │  │ DocsEngine │ │ │
         │  │  │ GossipSvc  │ │ │
         │  │  │ BlobStore  │ │ │
         │  │  └───────────┘ │ │
         │  └────────────────┘ │
         └─────────────────────┘
```

syncweb runs a WebSocket server on a local port. mapa connects as a
client. All the Iroh node lifecycle, doc operations, and gossip run
inside syncweb. mapa speaks the binary protocol defined below.

syncweb already has:
- `IrohNode` wrapping endpoint, docs, gossip, blobs, topic-tracker
- `DocsEngine` wrapping `iroh_docs::Docs`
- `GossipService` wrapping `iroh_gossip::net::Gossip`
- `BlobStore` wrapping `iroh_blobs`

The new layer is a `WsBridgeServer` that accepts WebSocket connections
and translates binary frames into calls on these existing types.

---

## Binary WebSocket protocol — `v1`

All frames are **binary** (`WebSocket.binaryType = "arraybuffer"` in
browser terms; `ByteArray` in Kotlin).

### Frame envelope

```
┌────────────────────┬──────────┬──────────────────────────┐
│ tag (1 byte)       │ seq (4)  │ payload (remaining bytes)│
│ message type ID    │ big-end  │ type-specific            │
│                    │ u32      │                          │
└────────────────────┴──────────┴──────────────────────────┘
```

- **tag**: Identifies the message kind (command request, command response,
  or server-push event). See the tag table below.
- **seq**: Request/response correlation ID. Commands set a non-zero
  `seq`; the response echoes it. Server-push events always use `seq=0`.
- **payload**: Tag-specific binary encoding described per-message below.

### Tag assignments

| tag | kind          | direction      | name                             |
|-----|---------------|----------------|----------------------------------|
| 0x01 | request       | client→server  | `dial_peer`                      |
| 0x02 | request       | client→server  | `append_event`                   |
| 0x03 | request       | client→server  | `get_events`                     |
| 0x04 | request       | client→server  | `get_events_paged`               |
| 0x05 | request       | client→server  | `share_collection`               |
| 0x06 | request       | client→server  | `import_collection`              |
| 0x07 | request       | client→server  | `join_gossip_topic`              |
| 0x08 | request       | client→server  | `join_gossip_topic_with_discovery` |
| 0x09 | request       | client→server  | `leave_gossip_topic`             |
| 0x0A | request       | client→server  | `send_gossip_message`            |
| 0x0B | request       | client→server  | `get_connected_peers`            |
| 0x0C | request       | client→server  | `block_peer`                     |
| 0x0D | request       | client→server  | `unblock_peer`                   |
| 0x0E | request       | client→server  | `get_blocked_peers`              |
| 0x0F | request       | client→server  | `get_node_id`                    |
| 0x10 | —             | —              | *(reserved)*                     |
| 0x80 | response      | server→client  | `ok`                             |
| 0x81 | response      | server→client  | `error`                          |
| 0x82 | push event    | server→client  | `sync_event`                     |
| 0x83 | push event    | server→client  | `node_status`                    |
| 0x84 | push event    | server→client  | `gossip_event`                   |

Tags 0x01–0x0F are commands (client→server). Tags 0x80–0x81 are
responses (server→client) that echo the request `seq`. Tags 0x82–0x84
are unsolicited push events (server→client, `seq=0`).

Tags 0x10–0x7F are reserved for additional commands. Tags 0x85–0xFF are
reserved for additional events/responses.

---

### Payload encoding rules

Unless noted otherwise:
- **u32 / u64**: big-endian fixed-width integer.
- **string**: 2-byte big-endian u16 length prefix, then UTF-8 bytes.
- **bytes**: 4-byte big-endian u32 length prefix, then raw bytes.
- **peer_list**: 2-byte u16 count, then count × (string `node_id` + u64 `first_seen` + u64 `last_seen`).
- **string_list**: 2-byte u16 count, then count × string.

### Session handshake

On connect the server sends one `node_status` push event (tag 0x83)
with `status="connected"` and the node's identity. No separate auth
step—the `data_dir` is configured via CLI on server startup. If the
client needs a different data dir, it starts a separate server process.

---

### Command payloads (client → server)

#### 0x01 — `dial_peer`

Ask the Iroh node to dial a peer's QUIC endpoint.

```
u16  node_id_len
     node_id (UTF-8)
```

Response: `ok` with empty payload, or `error`.

#### 0x02 — `append_event`

Append a binary payload to a collection's append-only log. Creates the
collection (iroh-docs namespace) on first use.

```
u16  collection_id_len
     collection_id (UTF-8)
u32  payload_len
     payload (raw bytes)
```

Response: `ok` with:
```
u16  author_len
     author (UTF-8, node_id)
u64  timestamp (unused, always 0)
```

Or `error`.

#### 0x03 — `get_events`

Fetch all events in a collection.

```
u16  collection_id_len
     collection_id (UTF-8)
```

Response: `ok` with:
```
u32  event_count
for each:
  u16  author_len  → author (UTF-8)
  u32  payload_len → payload (raw bytes)
  u64  timestamp
```

Or `error`.

#### 0x04 — `get_events_paged`

Fetch a page of events. 1-based page number.

```
u16  collection_id_len
     collection_id (UTF-8)
u64  page
u64  page_size
```

Response: same payload as `get_events`. Or `error`.

#### 0x05 — `share_collection`

Create a read-only iroh-docs ticket for a collection.

```
u16  collection_id_len
     collection_id (UTF-8)
```

Response: `ok` with `u16 ticket_len` + `ticket` (UTF-8). Or `error`.

#### 0x06 — `import_collection`

Import a collection from an iroh-docs ticket.

```
u16  collection_id_len
     collection_id (UTF-8)
u16  ticket_len
     ticket (UTF-8, DocTicket string)
```

Response: `ok` with empty payload. Or `error`.

#### 0x07 — `join_gossip_topic`

Subscribe to a gossip topic (prefix-truncated to 32 bytes as TopicId).

```
u16  topic_len
     topic (UTF-8)
```

Response: `ok` with empty payload. Or `error`.

#### 0x08 — `join_gossip_topic_with_discovery`

Subscribe to a gossip topic with DHT-based peer discovery for the topic.

```
u16  topic_len
     topic (UTF-8)
```

Response: `ok` with empty payload. Or `error`.

#### 0x09 — `leave_gossip_topic`

Unsubscribe from a gossip topic.

```
u16  topic_len
     topic (UTF-8)
```

Response: `ok` with empty payload. Or `error`.

#### 0x0A — `send_gossip_message`

Broadcast a message to all peers on a gossip topic.

```
u16  topic_len
     topic (UTF-8)
u32  message_len
     message (raw bytes)
```

Response: `ok` with empty payload. Or `error`.

#### 0x0B — `get_connected_peers`

List peers discovered over LAN since the node started.

```
(empty payload)
```

Response: `ok` with:
```
u16  peer_count
for each:
  u16  node_id_len   → node_id (UTF-8)
  u64  first_seen_secs
  u64  last_seen_secs
```

Or `error`.

#### 0x0C — `block_peer`

Block a peer. Future gossip/doc messages from this peer are dropped.

```
u16  node_id_len
     node_id (UTF-8)
```

Response: `ok` with empty payload. Or `error`.

#### 0x0D — `unblock_peer`

Remove a peer from the blocklist.

```
u16  node_id_len
     node_id (UTF-8)
```

Response: `ok` with empty payload. Or `error`.

#### 0x0E — `get_blocked_peers`

List blocked peer IDs.

```
(empty payload)
```

Response: `ok` with `string_list` of node IDs. Or `error`.

#### 0x0F — `get_node_id`

Get the local node's Iroh identity.

```
(empty payload)
```

Response: `ok` with:
```
u16  node_id_len
     node_id (UTF-8)
```

---

### Response payloads (server → client)

#### 0x80 — `ok`

Response to a successful command. The `seq` field matches the request.
Payload: the per-command output described above.

#### 0x81 — `error`

Response to a failed command. The `seq` field matches the request.

```
u16  message_len
     message (UTF-8)
```

---

### Push event payloads (server → client, `seq=0`)

#### 0x82 — `sync_event`

An event written to a collection by a remote peer was received.

```
u16  collection_id_len
     collection_id (UTF-8)
u16  author_len
     author (UTF-8)
u32  payload_len
     payload (raw bytes)
u64  timestamp
```

#### 0x83 — `node_status`

Node lifecycle status change. Sent on connect and on transitions.

```
u16  status_len
     status (UTF-8, e.g. "connected", "disconnected", "error")
u16  node_id_len        ← sent on the initial connect frame only
     node_id (UTF-8)    ← empty string ("") on subsequent transitions
```

#### 0x84 — `gossip_event`

A gossip message was received on a subscribed topic.

```
u16  topic_len
     topic (UTF-8)
u16  sender_len
     sender (UTF-8, peer node_id)
u32  message_len
     message (raw bytes)
```

---

### Error handling

- If a frame's tag is unknown, the server responds with `error` (tag
  0x81, `message="unknown tag: 0xNN"`) and the connection stays open.
- If a frame's payload is malformed (e.g., truncated length prefix),
  the server responds with `error` (`message="frame parse error"`).
- If the Iroh node is shutting down, all pending commands get `error`
  (`message="node shutting down"`).
- The server sends `node_status` with `status="disconnected"` before
  closing the WebSocket on orderly shutdown.

### Threading note

A single WebSocket connection handles all commands sequentially (the
server processes frames in order on a task-per-connection model).
Pipelining is allowed: the client may send multiple requests before
receiving responses, and the server processes them in order, matching
by `seq`. The client should use a monotonic sequence counter.

A client MUST NOT reuse a `seq` value until the response for that
`seq` has been received.

---

## Server startup

```bash
# Start syncweb with the WebSocket bridge on port 9192
syncweb bridge --data-dir /home/user/.syncweb --bind 127.0.0.1:9192

# Or as a daemon subcommand:
syncweb daemon --bridge-listen 127.0.0.1:9192
```

For mapa's use case the server runs locally (`127.0.0.1`) so no TLS
is needed. For remote access, a reverse proxy should add TLS.

---

## Migration plan — Kotlin side

### Step 1: Define the WebSocket client contract

Create a new `commonMain` interface `IrohNodeClient` that mirrors the
existing `IrohNode` interface but uses suspend functions with
request/response semantics. The push events become Kotlin `SharedFlow`s
exposed by a `WsBridgeConnection` class:

```kotlin
// shared/src/commonMain/kotlin/com/xk/mapa/shared/sync/WsBridgeConnection.kt

interface WsBridgeConnection {
    // Request-response
    suspend fun dialPeer(nodeId: String): Result<Boolean>
    suspend fun appendEvent(collectionId: String, payload: ByteArray): Result<Unit>
    suspend fun getEvents(collectionId: String): Result<List<SyncEvent>>
    suspend fun getEventsPaged(collectionId: String, page: Long, pageSize: Long): Result<List<SyncEvent>>
    suspend fun shareCollectionReadOnly(collectionId: String): Result<String>
    suspend fun importCollection(collectionId: String, ticket: String): Result<Unit>
    suspend fun joinGossipTopic(topic: String): Result<Unit>
    suspend fun joinGossipTopicWithDiscovery(topic: String): Result<Unit>
    suspend fun leaveGossipTopic(topic: String): Result<Unit>
    suspend fun sendGossipMessage(topic: String, message: ByteArray): Result<Unit>
    suspend fun getConnectedPeers(): Result<List<ConnectedPeerInfo>>
    suspend fun blockPeer(nodeId: String): Result<Unit>
    suspend fun unblockPeer(nodeId: String): Result<Unit>
    suspend fun getBlockedPeers(): Result<List<String>>
    suspend fun getNodeId(): Result<String>

    // Server-push flows
    val nodeStatus: StateFlow<String>
    val syncEvents: SharedFlow<MapEvent>   // deserialized from protobuf
    val gossipMessages: SharedFlow<GossipMessage>
    val connectedPeers: StateFlow<List<ConnectedPeerInfo>>

    suspend fun connect(url: String)
    suspend fun disconnect()
    val isConnected: Boolean
}
```

### Step 2: Implement the binary WebSocket client (actual, JVM)

In `jvmMain`, implement `WsBridgeConnection` using Ktor's WebSocket
client. Key details:

```kotlin
// shared/src/jvmMain/.../sync/WsBridgeConnectionImpl.kt

class WsBridgeConnectionImpl(private val scope: CoroutineScope) : WsBridgeConnection {
    private var session: WebSocketSession? = null
    private val pendingRequests = ConcurrentHashMap<Int, CompletableDeferred<ByteArray>>()
    private var nextSeq = AtomicInteger(1)

    override suspend fun connect(url: String) {
        val client = HttpClient { install(WebSockets) }
        session = client.webSocketSession(url) {
            binaryType = io.ktor.websocket.BinaryType.ARRAY_BUFFER
        }
        // launch reader coroutine
        scope.launch { readLoop() }
    }

    private suspend fun readLoop() {
        val session = session ?: return
        session.incoming.receiveAsFlow().collect { frame ->
            val bytes = (frame as Frame.Binary).data
            val tag = bytes[0].toInt() and 0xFF
            val seq = ((bytes[1].toInt() and 0xFF) shl 24) or
                       ((bytes[2].toInt() and 0xFF) shl 16) or
                       ((bytes[3].toInt() and 0xFF) shl 8) or
                       (bytes[4].toInt() and 0xFF)
            val payload = bytes.drop(5).toByteArray()
            when (tag) {
                0x80.toByte() -> dispatchResult(seq, payload)    // ok
                0x81.toByte() -> dispatchError(seq, payload)    // error
                0x82.toByte() -> handleSyncEvent(payload)
                0x83.toByte() -> handleNodeStatus(payload)
                0x84.toByte() -> handleGossipEvent(payload)
            }
        }
    }

    private suspend fun sendCommand(tag: Int, payload: ByteArray): Result<ByteArray> {
        val seq = nextSeq.getAndIncrement()
        val frame = buildPacket {
            writeByte(tag.toByte())
            writeInt(seq)  // big-endian
            writeFully(payload)
        }
        val deferred = CompletableDeferred<ByteArray>()
        pendingRequests[seq] = deferred
        session?.send(Frame.Binary(frame)) ?: return Result.failure(...)
        return withTimeout(30_000) { Result.success(deferred.await()) }
    }
}
```

The binary encoding/decoding follows the payload schema above. Each
`sendCommand` builds the tag-specific payload, awaits the response via
`pendingRequests`, and returns `Result`.

### Step 3: Update `MapaSyncManager` and `IrohNodeImpl`

Replace the two `commonSyncMain` `actual` implementations:

| Old | New |
|-----|-----|
| `MapaSyncManager` implements `uniffi.mapa_core.SyncEventHandler` | `MapaSyncManager` creates `WsBridgeConnectionImpl`, subscribes to its flows |
| `IrohNodeImpl` wraps `uniffi.mapa_core.IrohNode` | `IrohNodeImpl` delegates to `WsBridgeConnectionImpl` |

The `MapaSyncManager.reconnectionLoop` changes from calling
`IrohNode.newPersistent()` to calling `wsBridge.connect(url)`. The
StateFlow wiring stays the same—data still flows into
`_incomingEvents`, `_nodeStatus`, `_gossipMessages`, etc.

### Step 4: Remove Rust FFI from mapa

After the WebSocket path is stable:
1. Remove `core/build.gradle.kts` (UniFFI build step).
2. Remove `core/src/` Rust sources (or split them into analysis-only
   and P2P; the P2P parts move entirely to syncweb).
3. Remove `RustLibraryLoader` from `AnalyzerEngine.jvm.kt` (the
   analysis engine stays in mapa via a separate decision).
4. Remove `uniffi.mapa_core.*` imports.
5. Remove prebuilt `.so` files from `app/shared/src/androidMain/jniLibs/`.

---

## Implementation plan — Rust side (syncweb)

### New crate/module

Add `syncweb-core/src/bridge/` containing:

```
bridge/
├── mod.rs           # re-exports
├── server.rs        # WsBridgeServer — binds TCP, accepts connections
├── session.rs       # per-connection state, frame encode/decode, command dispatch
├── encoding.rs      # binary payload encode/decode helpers
└── service.rs       # BridgeService — wraps IrohNode, holds active sessions, blocklist
```

### Key types

```rust
// bridge/service.rs

pub struct BridgeService {
    node: Arc<IrohNode>,
    // Maps collection_id → NamespaceId for opened/created docs
    doc_map: RwLock<HashMap<String, NamespaceId>>,
    // Maps topic_str → GossipSender for active gossip subscriptions
    gossip_topics: RwLock<HashMap<String, GossipSender>>,
    // Peers seen over LAN
    connected_peers: RwLock<HashMap<String, ConnectedPeer>>,
    // Blocklist (loaded from blocklist.txt in data_dir)
    blocked_peers: RwLock<HashSet<String>>,
    // List of running sessions so we can shut down cleanly
    shutdown: Arc<broadcast::Sender<()>>,
}

pub struct ConnectedPeer {
    pub node_id: String,
    pub first_seen_secs: u64,
    pub last_seen_secs: u64,
}
```

```rust
// bridge/session.rs

pub struct WsSession {
    service: Arc<BridgeService>,
    sender: SplitSink<WebSocketStream<...>, Message>,
    shutdown: broadcast::Receiver<()>,
}

impl WsSession {
    async fn handle_frame(&mut self, bytes: Vec<u8>) -> Result<()> {
        let tag = bytes[0];
        let seq = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        let payload = &bytes[5..];
        let result = match tag {
            0x01 => self.handle_dial_peer(payload).await,
            0x02 => self.handle_append_event(payload).await,
            // ...
            0x0F => self.handle_get_node_id(payload).await,
            _   => Err(format!("unknown tag: 0x{tag:02X}")),
        };
        match result {
            Ok(response_bytes) => self.send_response(0x80, seq, &response_bytes).await,
            Err(msg)          => self.send_error(seq, &msg).await,
        }
    }
}
```

### Integration into syncweb daemon

The `syncweb daemon` command is extended with:

```
--bridge-listen <addr>    Bind WebSocket bridge to this address (e.g. 127.0.0.1:9192)
```

When set, the daemon starts a `WsBridgeServer` alongside the existing
Unix-socket IPC server. Both share the same `Arc<IrohNode>`. The
`--bridge-listen` flag can also be used with `syncweb start` for a
lightweight bridge-only mode.

### Canonical port

Default bridge port: **9192**. Derived from: 9 (map) + 192 (iroh/docs).
Convention: the variable name `SYNCWEB_BRIDGE_PORT` in configs.

---

## Compatibility with existing mapa sync formats

The bridge is a transport change only. Ownership semantics do not change:

- **Events** are still protobuf-encoded `MapEvent` messages (defined in
  mapa's `proto/map_event.proto`). The bridge transports raw bytes—it
  does not inspect or parse them. Serialization/deserialization stays
  in Kotlin.
- **Collection IDs** are free-form strings that mapa uses as keys into
  the `doc_map`. The bridge creates an iroh-docs namespace per
  collection_id on first `append_event`.
- **Gossip topics** are arbitrary byte strings truncated to 32 bytes.
- **Tickets** are iroh-docs `DocTicket` strings (standard format).

---

## Migration timeline

```
Phase 1  (syncweb)    Build WsBridgeServer, manual test with a WebSocket client.
Phase 2  (mapa)       Implement WsBridgeConnectionImpl, route through feature flag.
Phase 3  (mapa)       Delete Rust FFI: remove UniFFI, .so files, RustLibraryLoader.
Phase 4  (both)       Cleanup: remove orphaned Rust code in mapa's core/src/sync.rs.
```

Feature flag in Kotlin for gated rollout:

```kotlin
// shared/src/commonMain/.../sync/IrohNode.kt
expect fun createIrohNode(dataDir: String?, syncManager: MapaSyncManager): IrohNode

// shared/src/jvmMain/.../sync/IrohNodeFactory.kt
actual fun createIrohNode(dataDir: String?, syncManager: MapaSyncManager): IrohNode =
    if (System.getProperty("mapa.useWsBridge") == "true") {
        IrohNodeWsImpl(syncManager, dataDir)
    } else {
        IrohNodeNativeImpl(syncManager, dataDir)   // existing UniFFI path
    }
```

---

## Design decisions

### Why binary frames instead of JSON?

The payloads contain protobuf blobs (`MapEvent`) that are already
binary. Encoding binary → base64 within JSON → decode is wasteful.
Binary frames carry raw bytes directly. The frame envelope (tag + seq
+ payload) adds only 5 bytes overhead per message.

### Why WebSocket instead of HTTP REST?

The `SyncEventHandler` callback interface requires the server to push
events to the client. HTTP REST can only do request→response. WebSocket
provides full-duplex: commands go client→server, events go
server→client, on a single persistent connection.

### Why not gRPC?

gRPC requires protobuf service definitions and code generation on both
sides. The surface is small enough (14 commands, 3 push types) that a
custom binary protocol is simpler and has no codegen dependency.

### Why not reuse syncweb's Unix-socket IPC?

Unix sockets are not available on Android (which mapa targets). Also,
the existing syncweb IPC protocol is JSON-over-newline, designed for
CLI→daemon admin commands. Adding streaming push events would
complicate it. A separate WebSocket port keeps the contract clean.

### How does the server handle multiple clients?

One client per process is the intended use. Each client connection gets
its own session that shares the same `Arc<IrohNode>`. Gossip
subscriptions, doc subscriptions, and blocklists are per-node (shared
across sessions). If multiple mapa instances connect, they all see
the same gossip topics and doc events.

### What happens when the client disconnects?

The WebSocket closes. Doc subscriptions (LiveEvent listeners spawned
for this session) are cleaned up. Gossip subscriptions are NOT cleaned
up—the node continues listening for gossip so data isn't lost.

The client's reconnection loop (already exists in `MapaSyncManager`)
handles reconnecting with exponential backoff.

### Blocklist persistence

Blocklist is stored in `<data_dir>/blocklist.txt` (one node_id per
line), matching the existing format in mapa's Rust code. The bridge
reads it at startup and writes on each `block_peer`/`unblock_peer`.

### Node identity persistence

When the server starts with `--data-dir`, it loads the identity from
`<data_dir>/identity` (syncweb's existing `IdentityManager`). If no
identity exists, one is generated and persisted. The node_id is sent
to the client in the initial `node_status` push.
