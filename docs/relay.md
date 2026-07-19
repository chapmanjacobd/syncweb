# Syncthing Relay Piggyback (Phase 2)

### Problem

iroh uses QUIC with UDP hole punching (QNT). This works in ~90% of network configurations. But when both peers are behind different strict CGNATs that block UDP, iroh's hole punching fails. Syncthing's relay infrastructure is TCP-based and works in these scenarios.

### Design

The goal is **not** to translate between BEP and iroh protocols. Instead, we **piggyback on Syncthing's relay network** as a transport layer when iroh's direct/relay connectivity fails. Both endpoints remain syncweb nodes.

```text
syncweb Node A (behind CGNAT-A, UDP blocked)
    ↓ TCP + TLS (BEP session mode, as a transport tunnel)
Syncthing Relay (tcp://relay.syncthing.net)
    ↓ TCP (session mode, plain relay between devices)
syncweb Node B (behind CGNAT-B, UDP blocked)
```

The key insight: Syncthing relays are **protocol-agnostic**. They relay raw bytes between two devices that share a session key. We can tunnel iroh QUIC traffic through a Syncthing relay session by wrapping QUIC datagrams in the relay's byte-stream protocol.

### Implementation

```rust
/// Syncthing relay transport — fallback when iroh's QUIC hole punching fails
struct SyncthingRelayTransport {
    /// TCP connection to Syncthing relay
    tcp_stream: TcpStream,
    /// TLS wrapper (BEP-compatible handshake)
    tls: TlsStream<TcpStream>,
    /// Session key from relay protocol
    session_key: [u8; 32],
}

impl SyncthingRelayTransport {
    /// Connect to Syncthing relay and establish a session with a peer
    /// Uses the relay protocol v1: JoinRelayRequest → SessionInvitation → JoinSessionRequest
    async fn connect(relay_url: &str, peer_device_id: &DeviceId) -> Result<Self>;

    /// Tunnel iroh QUIC datagrams through the relay session
    /// The relay just forwards bytes — we encapsulate QUIC packets inside
    fn tunnel_quic(&self, quic_socket: &QuicSocket) -> Result<()>;
}

/// Transport fallback manager — tries iroh first, falls back to Syncthing relay
struct TransportFallback {
    /// Primary: iroh QUIC (direct + iroh relay)
    iroh_endpoint: Endpoint,
    /// Fallback: Syncthing relay tunnel
    syncthing_relay: Option<SyncthingRelayTransport>,
    /// Config
    config: RelayConfig,
}

impl TransportFallback {
    /// Connect to a peer, trying iroh first, then Syncthing relay
    async fn connect(&self, peer: &NodeId) -> Result<Box<dyn Transport>> {
        // 1. Try iroh direct connection (QUIC hole punch)
        if let Ok(conn) = self.iroh_endpoint.connect(peer, ALPN).await {
            return Ok(Box::new(conn));
        }

        // 2. Try iroh relay (if configured)
        if let Ok(conn) = self.iroh_endpoint.connect_via_relay(peer, ALPN).await {
            return Ok(Box::new(conn));
        }

        // 3. Fall back to Syncthing relay tunnel
        if let Some(relay) = &self.syncthing_relay {
            let tunnel = relay.connect(&self.config.relay_url, &peer.to_device_id()).await?;
            return Ok(Box::new(tunnel));
        }

        Err(Error::NoTransportAvailable)
    }
}
```

### Syncthing Relay Protocol (v1)

The relay protocol has two modes:

1. **Protocol mode** (TLS): Join relay, wait for session invitations
2. **Session mode** (plain): Relay bytes between two devices

We use protocol mode to register with the relay and receive session invitations, then session mode to tunnel QUIC traffic:

```rust
/// Syncthing relay protocol messages (XDR-encoded)
enum RelayMessage {
    JoinRelayRequest { device_id: [u8; 32] },
    ConnectRequest { device_id: [u8; 32] },
    SessionInvitation { session_key: [u8; 32], server_socket: bool },
    ResponseSuccess,
    ResponseNotFound,
    RelayFull,
}
```

### Configuration

```toml
[bep]
# Enable Syncthing relay fallback (for CGNAT traversal)
enabled = true
# Syncthing relay URLs (from Syncthing's config, or public relays)
relay_urls = ["tcp://relay.syncthing.net:22270"]
# Timeout for relay connection attempt (seconds)
relay_timeout = 10
# Auto-detect CGNAT and use relay when needed
auto_fallback = true
```

### CLI Usage

```bash
# Enable relay fallback globally
syncweb config set bep.enabled true

# Or per-connection
syncweb join --relay-fallback syncweb://folder-id#NODE-ID

# Test relay connectivity
syncweb network test-relay
```

### What This Enables

- Two syncweb nodes behind different CGNATs can communicate
- Automatic fallback: iroh tries direct first, falls back to relay only when needed
- No dependency on iroh's relay infrastructure for the data path
- Leverages Syncthing's mature, well-tested relay network
- Both nodes remain fully syncweb — no protocol translation needed

### Device Identity Compatibility

Syncthing and Iroh both use Ed25519 keypairs. The 56-char Syncthing Device ID (base32, grouped) and the 52-char Iroh NodeId (base32) are derived from the same key. Conversion is zero-cost re-encoding:

```rust
impl DeviceId {
    /// Convert from Syncthing Device ID (56-char base32, grouped)
    fn from_syncthing(id: &str) -> Result<Self>;

    /// Convert to Syncthing Device ID format
    fn to_syncthing(&self) -> String;

    /// Get the underlying Iroh NodeId
    fn to_node_id(&self) -> NodeId;
}
```

This allows `syncweb devices` to display both formats and config files to reference devices by either format.

---
