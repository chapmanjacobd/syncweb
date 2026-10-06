//! The iroh [`CustomTransport`] that carries QUIC datagrams over bep-relay
//! sessions.
//!
//! Each syncweb node advertises a [`CustomAddr`] whose payload carries both
//! its iroh node id (so the receiving side can attribute traffic to a node)
//! and its relay device id (so the receiving side can ask a Syncthing relay
//! to broker a session). Datagrams handed over by iroh are framed into an
//! existing session towards the peer, or (on first use) a session is dialed
//! in the background while QUIC retransmits.

use std::io::{self, IoSliceMut};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use iroh::PublicKey;
use iroh::endpoint::transports::{CustomEndpoint, CustomSender, CustomTransport, RecvInfo, Transmit};
use iroh_base::CustomAddr;
use noq_udp::RecvMeta;
use parking_lot::RwLock;
use tokio::sync::mpsc as tokio_mpsc;
use tokio_rustls::TlsConnector;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use super::certs::RelayCerts;
use super::manager::start_peer_dial;
use super::protocol::Message;

/// Transport id of the syncweb bep-relay transport ("SWSB").
pub const BEP_TRANSPORT_ID: u64 = 0x5357_5342;
/// Length of a relay device id (a SHA-256 digest).
pub const RELAY_ID_LEN: usize = 32;
/// Version marker for [`ADDR_LEN`] payloads carried in our custom addresses.
const ADDR_VERSION: u8 = 1;
/// Byte length of a custom address payload: version + node + relay id.
pub const ADDR_LEN: usize = 1 + 32 + RELAY_ID_LEN;
/// Length of the node-id prefix inside the payload.
const NODE_LEN: usize = 32;
/// Offset of the relay device id inside the payload.
const RELAY_START: usize = 1 + NODE_LEN;
/// Byte length of a relay device id inside the payload after the prefix.
const RELAY_LEN: usize = 32;

/// An incoming datagram delivered from a relay session to iroh.
#[derive(Debug)]
pub struct IncomingPacket {
    /// Custom address of the peer that sent the datagram.
    pub remote: CustomAddr,
    /// Our own custom address that received it.
    pub local: CustomAddr,
    /// Complete datagram bytes.
    pub data: Bytes,
}

/// A live relay session towards one peer.
///
/// The bounded queue lets the synchronous send path enqueue datagrams
/// without blocking; overflow simply drops the frame and QUIC retransmits.
#[derive(Debug)]
pub struct Session {
    /// Relay device id of the peer.
    pub peer: [u8; 32],
    /// Node id used to label incoming datagrams from this peer.
    pub peer_node: [u8; 32],
    /// Queue consumed by the session writer task.
    pub queue: tokio_mpsc::Sender<Bytes>,
    /// Cooperative cancellation for the session tasks.
    pub cancel: CancellationToken,
}

/// State shared between the manager (session establishment) and the
/// transport (socket I/O).
pub struct Backend {
    /// Sessions towards peers, keyed by the peer relay device id.
    pub sessions: RwLock<std::collections::HashMap<[u8; 32], Arc<Session>>>,
    /// Relay device id to iroh node id, for labeling inbound traffic.
    pub node_ids: RwLock<std::collections::HashMap<[u8; 32], [u8; 32]>>,
    /// The ingress sink into iroh, created when the transport is bound.
    pub ingress: parking_lot::Mutex<Option<tokio_mpsc::UnboundedSender<IncomingPacket>>>,
    /// Our published custom address.
    pub local_addrs: n0_watcher::Watchable<Vec<CustomAddr>>,
    /// Dials currently in flight, keyed by the peer relay device id.
    pub dials: parking_lot::Mutex<std::collections::HashSet<[u8; 32]>>,
    /// Round-robin index over [`Backend::relays`].
    pub relay_index: parking_lot::Mutex<usize>,
    /// The relays we are registered with, in dial order.
    pub relays: RwLock<Vec<String>>,
    /// Outbound control-message senders for the live registration connection
    /// of each relay, keyed by relay url. `ConnectRequest`s are sent here so
    /// the relay attributes them to our registered device id.
    pub control: RwLock<std::collections::HashMap<String, tokio_mpsc::UnboundedSender<Message>>>,
    /// Persistent identity used for control connections.
    pub certs: Arc<RelayCerts>,
    /// TLS connector for control connections.
    pub connector: TlsConnector,
    /// Timeout budget for connection and frame operations.
    pub timeout: Duration,
    /// Cooperative cancellation for all relay tasks.
    pub cancel: CancellationToken,
}

impl std::fmt::Debug for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backend").finish_non_exhaustive()
    }
}

impl Backend {
    /// Create an empty backend with the given identity and connector.
    #[must_use]
    pub(crate) fn new(certs: Arc<RelayCerts>, connector: TlsConnector, timeout: Duration) -> Self {
        Self {
            sessions: RwLock::new(std::collections::HashMap::new()),
            node_ids: RwLock::new(std::collections::HashMap::new()),
            ingress: parking_lot::Mutex::new(None),
            local_addrs: n0_watcher::Watchable::new(Vec::new()),
            dials: parking_lot::Mutex::new(std::collections::HashSet::new()),
            relay_index: parking_lot::Mutex::new(0),
            relays: RwLock::new(Vec::new()),
            control: RwLock::new(std::collections::HashMap::new()),
            certs,
            connector,
            timeout,
            cancel: CancellationToken::new(),
        }
    }

    /// Build the custom address this node publishes.
    #[must_use]
    pub(crate) fn local_addr(&self) -> CustomAddr {
        self.local_addrs
            .get()
            .first()
            .cloned()
            .unwrap_or_else(|| CustomAddr::from_parts(BEP_TRANSPORT_ID, &[0_u8; ADDR_LEN]))
    }

    /// Update the address we publish, notifying the bind watcher.
    pub(crate) fn update_local_addr(&self, addr: CustomAddr) {
        let _ = self.local_addrs.set(vec![addr]);
    }

    /// Record a relay id to node id mapping for labeling inbound datagrams.
    pub(crate) fn remember_node(&self, relay_id: [u8; 32], node: [u8; 32]) {
        self.node_ids.write().insert(relay_id, node);
    }

    /// The next relay url in round-robin dial order.
    #[must_use]
    pub(crate) fn next_relay(&self) -> Option<String> {
        if self.relays.read().is_empty() {
            return None;
        }
        let count = self.relays.read().len();
        let slot = {
            let mut index = self.relay_index.lock();
            let slot = *index;
            *index = if slot.saturating_add(1) >= count {
                0
            } else {
                slot.saturating_add(1)
            };
            slot
        };
        self.relays.read().get(slot).cloned()
    }
}

/// A custom transport that talks bep-relay to Syncthing relays.
#[derive(Clone, Debug)]
pub struct SyncthingRelayTransport(Arc<Backend>);

impl SyncthingRelayTransport {
    /// Wrap a manager backend.
    #[must_use]
    pub(crate) const fn new(backend: Arc<Backend>) -> Self {
        Self(backend)
    }

    /// The custom address this transport advertises locally.
    #[must_use]
    pub fn local_addr(&self) -> CustomAddr {
        self.0.local_addr()
    }

    /// Record the iroh node id that owns the given relay device id, so
    /// inbound datagrams from that peer can be attributed correctly.
    pub fn remember_node(&self, relay_id: &[u8; 32], node: PublicKey) {
        self.0.remember_node(*relay_id, *node.as_bytes());
    }
}

impl CustomTransport for SyncthingRelayTransport {
    fn bind(&self) -> io::Result<Box<dyn CustomEndpoint>> {
        let (tx, rx) = tokio_mpsc::unbounded_channel();
        *self.0.ingress.lock() = Some(tx);
        Ok(Box::new(RelayEndpoint {
            backend: self.0.clone(),
            recv: rx,
        }))
    }
}

/// The bound endpoint for the bep-relay transport.
#[derive(Debug)]
struct RelayEndpoint {
    backend: Arc<Backend>,
    recv: tokio_mpsc::UnboundedReceiver<IncomingPacket>,
}

impl CustomEndpoint for RelayEndpoint {
    fn watch_local_addrs(&self) -> n0_watcher::Direct<Vec<CustomAddr>> {
        self.backend.local_addrs.watch()
    }

    fn create_sender(&self) -> Arc<dyn CustomSender> {
        Arc::new(RelaySender(self.backend.clone()))
    }

    fn poll_recv(
        &mut self,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
        metas: &mut [RecvMeta],
        recv_infos: &mut [RecvInfo],
    ) -> Poll<io::Result<usize>> {
        let mut count = 0;
        while count < bufs.len() {
            match self.recv.poll_recv(cx) {
                Poll::Pending | Poll::Ready(None) => break,
                Poll::Ready(Some(packet)) => {
                    let len = packet.data.len();
                    let Some(buffer) = bufs.get_mut(count) else {
                        break;
                    };
                    let Some(dest) = buffer.get_mut(..len) else {
                        warn!(
                            bytes = len,
                            capacity = buffer.len(),
                            "datagram larger than consumer buffer, dropping"
                        );
                        continue;
                    };
                    dest.copy_from_slice(&packet.data);
                    let Some(meta) = metas.get_mut(count) else {
                        break;
                    };
                    *meta = RecvMeta::default();
                    meta.len = len;
                    meta.stride = len;
                    let Some(recv_info) = recv_infos.get_mut(count) else {
                        break;
                    };
                    *recv_info = RecvInfo::new(packet.remote, Some(packet.local));
                    count = count.saturating_add(1);
                }
            }
        }
        if count == 0 {
            return Poll::Pending;
        }
        Poll::Ready(Ok(count))
    }
}

/// The sender half of the bep-relay transport.
#[derive(Debug)]
struct RelaySender(Arc<Backend>);

impl CustomSender for RelaySender {
    fn is_valid_send_addr(&self, addr: &CustomAddr) -> bool {
        decode_peer_addr(addr).is_some()
    }

    fn poll_send(
        &self,
        _cx: &mut Context<'_>,
        dst: &CustomAddr,
        _src: Option<&CustomAddr>,
        transmit: &Transmit<'_>,
    ) -> Poll<io::Result<()>> {
        let Some((peer, node)) = decode_peer_addr(dst) else {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "custom address is not a syncweb relay address",
            )));
        };
        let candidate = self.0.sessions.read().get(&peer).cloned();
        if let Some(session) = candidate {
            if session
                .queue
                .try_send(Bytes::copy_from_slice(transmit.contents))
                .is_err()
            {
                start_peer_dial(self.0.clone(), peer, node);
            }
            return Poll::Ready(Ok(()));
        }
        start_peer_dial(self.0.clone(), peer, node);
        Poll::Ready(Ok(()))
    }
}

/// Encode a node id and relay device id into our custom address payload.
#[must_use]
pub fn encode_peer_addr(node: &PublicKey, relay_id: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(ADDR_LEN);
    payload.push(ADDR_VERSION);
    payload.extend_from_slice(node.as_bytes());
    payload.extend_from_slice(relay_id);
    payload
}

/// Build the custom address this node advertises.
#[must_use]
pub fn own_addr(node: &PublicKey, relay_id: &[u8; 32]) -> CustomAddr {
    CustomAddr::from_parts(BEP_TRANSPORT_ID, &encode_peer_addr(node, relay_id))
}

/// Parse one of our custom addresses into its peer (relay id, node id).
#[must_use]
pub fn decode_peer_addr(addr: &CustomAddr) -> Option<([u8; 32], [u8; 32])> {
    if addr.id() != BEP_TRANSPORT_ID {
        return None;
    }
    let data = addr.data();
    if data.len() != ADDR_LEN || data.first() != Some(&ADDR_VERSION) {
        return None;
    }
    let mut node = [0_u8; NODE_LEN];
    node.copy_from_slice(data.get(1..=NODE_LEN)?);
    let mut relay = [0_u8; RELAY_LEN];
    relay.copy_from_slice(data.get(RELAY_START..)?);
    Some((relay, node))
}

/// Build the remote custom address to label datagrams on a session to a peer
/// whose relay id is `relay` and node id `node`.
#[must_use]
pub fn peer_addr(relay: [u8; 32], node: [u8; 32]) -> CustomAddr {
    let mut payload = Vec::with_capacity(ADDR_LEN);
    payload.push(ADDR_VERSION);
    payload.extend_from_slice(&node);
    payload.extend_from_slice(&relay);
    CustomAddr::from_parts(BEP_TRANSPORT_ID, &payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_addr_round_trips() {
        let node_id = iroh::SecretKey::generate().public();
        let node = *node_id.as_bytes();
        let relay = [9_u8; 32];
        let payload = encode_peer_addr(&node_id, &relay);
        let addr = CustomAddr::from((BEP_TRANSPORT_ID, payload.as_ref()));
        let (decoded_relay, decoded_node) = decode_peer_addr(&addr).expect("decode");
        assert_eq!(decoded_relay, relay);
        assert_eq!(decoded_node, node);
    }

    #[test]
    fn foreign_addrs_are_rejected() {
        let addr = CustomAddr::from((42, b"junk".as_slice()));
        assert!(decode_peer_addr(&addr).is_none());
    }

    #[test]
    fn short_payloads_are_rejected() {
        let addr = CustomAddr::from((BEP_TRANSPORT_ID, b"1".as_slice()));
        assert!(decode_peer_addr(&addr).is_none());
    }
}
