//! A UDP beacon address lookup, inspired by Zyre's passive discovery beacon.
//!
//! The beacon broadcasts a node's current [`EndpointData`] (direct addresses and
//! an optional relay URL) as a small JSON datagram on a fixed UDP port. Unlike
//! mDNS there is no service-name registry; peers on the same LAN simply listen
//! on the same port and accept datagrams that carry the expected scope token.
//!
//! Each configured network derives a 16-byte scope from its name. The scope is
//! used twice:
//! * to derive the beacon port (the scope offsets the base port), and
//! * as a token inside every datagram so that different networks never accept
//!   each other's beacons even when the derived ports collide.
//!
//! This is an [`AddressLookup`] and can be registered next to the mDNS lookup.
//! Like mDNS it is a fallback for local discovery and is *not* an access
//! boundary: any peer that knows the derived port and scope can still be
//! reached, and membership is still enforced by the layer 2/3 peer allowlist.

use std::{
    collections::{BTreeSet, HashMap},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    pin::Pin,
    ptr,
    str::FromStr,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use iroh::{
    PublicKey,
    address_lookup::{
        AddressLookup, EndpointData, EndpointInfo, Error as AddressLookupError, Item as AddressLookupItem,
    },
};
use n0_future::{Stream, boxed::BoxStream, task::AbortOnDropHandle};
use serde::{Deserialize, Serialize};
use tokio::{net::UdpSocket, sync::mpsc};
use tracing::{debug, warn};

use crate::error::SyncwebError;

/// Magic value that prefixes every beacon datagram. It both identifies the
/// protocol and provides cheap sanity checking before JSON parsing.
pub const BEACON_MAGIC: &str = "syncweb-beacon-v1";

/// Default base UDP port for the beacon. The effective port is
/// `base + (scope[0..2] % BEACON_PORT_SPREAD)`.
pub const DEFAULT_BEACON_PORT: u16 = 15_200;

/// Number of ports a scope may spread over when deriving the effective port.
pub const BEACON_PORT_SPREAD: u16 = 2_048;

/// How long a discovered peer's addresses stay usable without a refresh.
pub const BEACON_PEER_TTL: Duration = Duration::from_secs(30);

/// How long a resolve request waits for a fresh beacon before giving up.
const LOOKUP_DURATION: Duration = Duration::from_secs(10);

/// Maximum accepted datagram size.
const MAX_DATAGRAM_SIZE: usize = 2_048;

/// Number of beacons sent back-to-back when the published data changes.
const BURST_COUNT: usize = 3;

/// A single interface with its IPv4 address and directed broadcast address.
struct InterfaceInfo {
    name: String,
    addr: Ipv4Addr,
    broadcast: Ipv4Addr,
}

/// Datagram payload broadcast by a beacon.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BeaconMessage {
    magic: String,
    scope: [u8; 16],
    id: String,
    addrs: Vec<SocketAddr>,
    relay: Option<String>,
}

/// Pending response channels for a resolve request.
struct PendingResolve {
    deadline: Instant,
    sender: mpsc::Sender<Result<AddressLookupItem, AddressLookupError>>,
}

/// A peer's published data and when it was last heard from.
struct PeerEntry {
    data: Arc<EndpointData>,
    last_seen: Instant,
}

/// Messages sent to the beacon actor.
enum Message {
    Publish(Arc<EndpointData>),
    Resolve(PublicKey, mpsc::Sender<Result<AddressLookupItem, AddressLookupError>>),
}

/// An [`AddressLookup`] that publishes and resolves addresses over a UDP beacon.
///
/// The `_handle` field is never read; it exists only so that dropping the lookup
/// aborts the background actor.
#[derive(Debug, Clone)]
pub struct BeaconAddressLookup {
    sender: mpsc::Sender<Message>,
    _handle: Arc<AbortOnDropHandle<()>>,
}

/// A small adapter exposing a [`tokio::sync::mpsc::Receiver`] as a stream.
struct ReceiverStream<T>(mpsc::Receiver<T>);

impl<T> ReceiverStream<T> {
    const fn new(receiver: mpsc::Receiver<T>) -> Self {
        Self(receiver)
    }
}

impl<T> Stream for ReceiverStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.0.poll_recv(cx)
    }
}

impl BeaconAddressLookup {
    /// Creates a beacon address lookup and starts its background task.
    ///
    /// The receiver socket is bound to the effective port on all interfaces
    /// (or on the configured interface when one is given) and the sender
    /// broadcasts to `255.255.255.255` plus the directed broadcast of every
    /// up, non-loopback IPv4 interface (or only the configured one).
    ///
    /// # Errors
    ///
    /// Returns an error if the receiver or sender socket cannot be created.
    pub fn new(
        endpoint_id: PublicKey,
        scope: Option<[u8; 16]>,
        base_port: u16,
        interval: Duration,
        interface: Option<&str>,
    ) -> crate::error::Result<Self> {
        let port = beacon_port(base_port, scope.as_ref());
        let bind_ip = interface.and_then(interface_ipv4).or_else(|| {
            if interface.is_some() {
                warn!(interface, "beacon interface not found; using all interfaces");
            }
            None
        });
        let bind_addr = SocketAddr::new(bind_ip.map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), IpAddr::V4), port);
        let targets = broadcast_targets(interface, port);
        Self::with_bind_targets(endpoint_id, scope, bind_addr, targets, interval)
    }

    /// Creates a beacon with an explicit bind address and set of targets.
    fn with_bind_targets(
        endpoint_id: PublicKey,
        scope: Option<[u8; 16]>,
        bind_addr: SocketAddr,
        targets: Vec<SocketAddr>,
        beacon_interval: Duration,
    ) -> crate::error::Result<Self> {
        let recv_socket = tokio::net::UdpSocket::from_std(beacon_socket(bind_addr, true)?)?;
        let send_socket = tokio::net::UdpSocket::from_std(beacon_socket(SocketAddr::new(bind_addr.ip(), 0), false)?)?;

        let (sender, mut recv) = mpsc::channel(64);
        let actor = async move {
            let mut advertised: Option<Arc<EndpointData>> = None;
            let mut peers: HashMap<PublicKey, PeerEntry> = HashMap::new();
            let mut pending: HashMap<PublicKey, Vec<PendingResolve>> = HashMap::new();
            let mut buf = vec![0_u8; MAX_DATAGRAM_SIZE];
            let mut timer = tokio::time::interval(beacon_interval.max(Duration::from_millis(1)));
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        broadcast_beacon(&send_socket, &targets, advertised.as_deref(), scope, endpoint_id).await;
                        let now = Instant::now();
                        sweep(&mut peers, &mut pending, now);
                    }
                    message = recv.recv() => {
                        match message {
                            None => break,
                            Some(Message::Publish(data)) => {
                                debug!("beacon data published");
                                advertised = Some(data.clone());
                                for _ in 0..BURST_COUNT {
                                    broadcast_beacon(&send_socket, &targets, Some(data.as_ref()), scope, endpoint_id).await;
                                }
                            }
                            Some(Message::Resolve(id, responder)) => {
                                if let Some(entry) = peers.get(&id) {
                                    let _ = responder.send(Ok(make_item(id, entry.data.as_ref()))).await;
                                } else {
                                    pending.entry(id).or_default().push(PendingResolve {
                                        deadline: Instant::now().checked_add(LOOKUP_DURATION).unwrap_or_else(Instant::now),
                                        sender: responder,
                                    });
                                }
                            }
                        }
                    }
                    received = recv_socket.recv_from(&mut buf) => {
                        match received {
                            Ok((length, _source)) => {
                                if let Some(message) = parse_beacon(buf.get(..length)) {
                                    handle_discovered(&message, endpoint_id, scope, &mut peers, &mut pending).await;
                                }
                            }
                            Err(error) => warn!(%error, "beacon receive failed"),
                        }
                    }
                }
            }
        };
        let handle = tokio::spawn(actor);
        Ok(Self {
            sender,
            _handle: Arc::new(AbortOnDropHandle::new(handle)),
        })
    }
}

/// Derives the effective beacon port for a scope.
#[must_use]
pub const fn beacon_port(base: u16, scope: Option<&[u8; 16]>) -> u16 {
    let offset = match scope {
        Some(bytes) => u16::from_be_bytes([bytes[0], bytes[1]]) % BEACON_PORT_SPREAD,
        None => 0,
    };
    base.saturating_add(offset)
}

/// Builds a bound UDP socket with broadcast enabled and non-blocking I/O.
fn beacon_socket(bind_addr: SocketAddr, reuse: bool) -> crate::error::Result<std::net::UdpSocket> {
    let socket = std::net::UdpSocket::bind(bind_addr)
        .map_err(|error| SyncwebError::operation("failed to bind beacon socket", error))?;
    socket
        .set_broadcast(true)
        .map_err(|error| SyncwebError::operation("failed to enable beacon broadcast", error))?;
    socket
        .set_nonblocking(true)
        .map_err(|error| SyncwebError::operation("failed to set beacon socket non-blocking", error))?;
    if reuse {
        set_reuseaddr(&socket)?;
    }
    Ok(socket)
}

/// Sets `SO_REUSEADDR` on a UDP socket so multiple daemons on one host can
/// share the same beacon port.
fn set_reuseaddr(socket: &std::net::UdpSocket) -> crate::error::Result<()> {
    use std::os::fd::AsRawFd;

    let one: libc::c_int = 1;
    let option_length = libc::socklen_t::try_from(std::mem::size_of::<libc::c_int>())
        .map_err(|error| SyncwebError::operation("invalid beacon socket option length", error))?;
    // SAFETY: `socket.as_raw_fd()` is a valid socket descriptor for the lifetime
    // of `socket`, and setting SO_REUSEADDR on a UDP socket only affects the
    // kernel's bind behavior; it cannot cause memory unsafety.
    let result = unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            ptr::from_ref(&one).cast(),
            option_length,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(SyncwebError::operation(
            "failed to set SO_REUSEADDR on beacon socket",
            std::io::Error::last_os_error(),
        ))
    }
}

/// The addresses every beacon is broadcast to: the limited broadcast plus the
/// directed broadcast of every relevant IPv4 interface.
fn broadcast_targets(interface: Option<&str>, port: u16) -> Vec<SocketAddr> {
    let mut targets = Vec::with_capacity(8);
    targets.push(SocketAddr::new(Ipv4Addr::BROADCAST.into(), port));
    for iface in ipv4_interfaces(interface) {
        let target = SocketAddr::new(iface.broadcast.into(), port);
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    targets
}

/// IPv4 interfaces that can carry beacon traffic.
fn ipv4_interfaces(interface: Option<&str>) -> Vec<InterfaceInfo> {
    let mut interfaces = Vec::new();
    for iface in ipv4_interfaces_raw() {
        if interface.is_none_or(|name| iface.name == name) {
            interfaces.push(iface);
        }
    }
    interfaces
}

/// Resolves an interface name to its IPv4 address, if any.
fn interface_ipv4(interface: &str) -> Option<Ipv4Addr> {
    ipv4_interfaces(Some(interface))
        .into_iter()
        .next()
        .map(|iface| iface.addr)
}

/// Enumerates IPv4 interfaces from the operating system.
#[cfg(unix)]
fn ipv4_interfaces_raw() -> Vec<InterfaceInfo> {
    use std::ffi::CStr;

    let mut ifap: *mut libc::ifaddrs = ptr::null_mut();
    // SAFETY: `getifaddrs` allocates a linked list of interface records that
    // must be released with `freeifaddrs`; we guarantee that on every exit path.
    if unsafe { libc::getifaddrs(ptr::from_mut(&mut ifap)) } != 0 {
        warn!("failed to enumerate network interfaces for the beacon");
        return Vec::new();
    }
    let mut interfaces = Vec::new();
    let mut current = ifap;
    while !current.is_null() {
        // SAFETY: `current` points to a node in the list allocated by
        // `getifaddrs` that remains valid until `freeifaddrs` is called below.
        let ifa = unsafe { &*current };
        let flags = ifa.ifa_flags;
        if flag_is_set(flags, libc::IFF_UP)
            && !flag_is_set(flags, libc::IFF_LOOPBACK)
            && let Some(addr) = sockaddr_ipv4(ifa.ifa_addr)
            && let Some(netmask) = sockaddr_ipv4(ifa.ifa_netmask)
        {
            let addr_bits = u32::from_be_bytes(addr.octets());
            let mask_bits = u32::from_be_bytes(netmask.octets());
            if mask_bits != 0 {
                // SAFETY: `ifa_name` points to a NUL-terminated interface
                // name that is valid for the lifetime of `ifa`.
                let name = unsafe { CStr::from_ptr(ifa.ifa_name) }.to_string_lossy().into_owned();
                interfaces.push(InterfaceInfo {
                    name,
                    addr,
                    broadcast: Ipv4Addr::from(addr_bits | !mask_bits),
                });
            }
        }
        current = ifa.ifa_next;
    }
    // SAFETY: `ifap` still points to the list returned by `getifaddrs` above.
    unsafe { libc::freeifaddrs(ifap) };
    interfaces
}

/// Enumerates IPv4 interfaces from the operating system.
#[cfg(not(unix))]
fn ipv4_interfaces_raw() -> Vec<InterfaceInfo> {
    Vec::new()
}

/// Tests whether an interface flag constant is set.
fn flag_is_set(flags: u32, flag: i32) -> bool {
    flags & u32::try_from(flag).unwrap_or(0) != 0
}

/// Reads the IPv4 address from a `sockaddr`, if it is an IPv4 address.
fn sockaddr_ipv4(raw: *mut libc::sockaddr) -> Option<Ipv4Addr> {
    if raw.is_null() {
        return None;
    }
    // SAFETY: dereferencing `sa_family` is always safe for a valid `sockaddr`.
    let family = unsafe { (*raw).sa_family };
    if family != libc::AF_INET.try_into().unwrap_or(0) {
        return None;
    }
    // SAFETY: the family is AF_INET, so `raw` points to a valid `sockaddr_in`.
    let address = unsafe { ptr::read_unaligned(raw.cast::<libc::sockaddr_in>()) };
    Some(Ipv4Addr::from(address.sin_addr.s_addr))
}

/// Serializes the current endpoint data into a beacon datagram.
fn encode_message(data: &EndpointData, scope: Option<[u8; 16]>, endpoint_id: PublicKey) -> BeaconMessage {
    BeaconMessage {
        magic: BEACON_MAGIC.to_owned(),
        scope: scope.unwrap_or([0; 16]),
        id: endpoint_id.to_string(),
        addrs: data.ip_addrs().copied().collect(),
        relay: data.relay_urls().next().map(ToString::to_string),
    }
}

/// Parses and sanity-checks a received datagram.
fn parse_beacon(data: Option<&[u8]>) -> Option<BeaconMessage> {
    let message: BeaconMessage = serde_json::from_slice(data?).ok()?;
    (message.magic == BEACON_MAGIC).then_some(message)
}

/// Records a discovered peer and answers any pending resolve requests.
async fn handle_discovered(
    message: &BeaconMessage,
    endpoint_id: PublicKey,
    scope: Option<[u8; 16]>,
    peers: &mut HashMap<PublicKey, PeerEntry>,
    pending: &mut HashMap<PublicKey, Vec<PendingResolve>>,
) {
    if let Some(expected) = scope
        && message.scope != expected
    {
        return;
    }
    let Ok(peer_id) = PublicKey::from_str(&message.id) else {
        return;
    };
    if peer_id == endpoint_id {
        return;
    }
    let data = message_to_endpoint_data(message);
    peers.insert(
        peer_id,
        PeerEntry {
            data: Arc::new(data.clone()),
            last_seen: Instant::now(),
        },
    );
    if let Some(senders) = pending.get_mut(&peer_id) {
        for resolve in senders.drain(..) {
            let _ = resolve.sender.send(Ok(make_item(peer_id, &data))).await;
        }
    }
}

/// Converts a parsed beacon message into endpoint data.
fn message_to_endpoint_data(message: &BeaconMessage) -> EndpointData {
    let addrs: BTreeSet<SocketAddr> = message.addrs.iter().copied().collect();
    let mut data = EndpointData::from(addrs);
    if let Some(relay) = &message.relay
        && let Ok(relay_url) = relay.parse()
    {
        data.add_relay_url(relay_url);
    }
    data
}

/// Builds the item returned to resolve callers.
fn make_item(endpoint_id: PublicKey, data: &EndpointData) -> AddressLookupItem {
    AddressLookupItem::new(EndpointInfo::from_parts(endpoint_id, data.clone()), "beacon", None)
}

/// Sends the current endpoint data to every broadcast target.
async fn broadcast_beacon(
    socket: &UdpSocket,
    targets: &[SocketAddr],
    data: Option<&EndpointData>,
    scope: Option<[u8; 16]>,
    endpoint_id: PublicKey,
) {
    let Some(current) = data else {
        return;
    };
    let Ok(payload) = serde_json::to_vec(&encode_message(current, scope, endpoint_id)) else {
        warn!("failed to serialize beacon payload");
        return;
    };
    for target in targets {
        if let Err(error) = socket.send_to(&payload, *target).await {
            warn!(%target, %error, "beacon broadcast failed");
        }
    }
}

/// Drops peers and pending resolves that have exceeded their lifetime.
fn sweep(
    peers: &mut HashMap<PublicKey, PeerEntry>,
    pending: &mut HashMap<PublicKey, Vec<PendingResolve>>,
    now: Instant,
) {
    peers.retain(|_, entry| {
        entry
            .last_seen
            .checked_add(BEACON_PEER_TTL)
            .is_some_and(|deadline| deadline > now)
    });
    pending.retain(|_, resolves| {
        resolves.retain(|resolve| resolve.deadline > now);
        !resolves.is_empty()
    });
}

impl AddressLookup for BeaconAddressLookup {
    fn resolve(&self, endpoint_id: PublicKey) -> Option<BoxStream<Result<AddressLookupItem, AddressLookupError>>> {
        let (send, recv) = mpsc::channel(20);
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let _ = sender.send(Message::Resolve(endpoint_id, send)).await;
        });
        Some(Box::pin(ReceiverStream::new(recv)))
    }

    fn publish(&self, data: &EndpointData) {
        let _ = self.sender.try_send(Message::Publish(Arc::new(data.clone())));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use n0_future::StreamExt;
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::time::timeout;

    fn endpoint_id(seed: u8) -> PublicKey {
        let mut bytes = [0_u8; 32];
        bytes[0] = seed;
        let secret = iroh::SecretKey::from_bytes(&bytes);
        secret.public()
    }

    fn endpoint_data(port: u16) -> EndpointData {
        EndpointData::from(BTreeSet::from([SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)]))
    }

    fn make_beacon(id: PublicKey, scope: [u8; 16], bind_port: u16, target_port: u16) -> BeaconAddressLookup {
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), bind_port);
        let targets = vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), target_port)];
        BeaconAddressLookup::with_bind_targets(id, Some(scope), bind_addr, targets, Duration::from_secs(1))
            .expect("beacon should bind")
    }

    #[tokio::test]
    async fn beacon_port_is_scoped() {
        let scope = [7_u8; 16];
        let port = beacon_port(DEFAULT_BEACON_PORT, Some(&scope));
        let offset = u16::from_be_bytes([scope[0], scope[1]]) % BEACON_PORT_SPREAD;
        assert_eq!(port, DEFAULT_BEACON_PORT.saturating_add(offset));
        assert_eq!(beacon_port(DEFAULT_BEACON_PORT, None), DEFAULT_BEACON_PORT);
    }

    #[tokio::test]
    async fn publish_and_resolve_over_loopback() {
        let scope = [9_u8; 16];
        let id_a = endpoint_id(1);
        let id_b = endpoint_id(2);
        let beacon_a = make_beacon(id_a, scope, 21_000, 21_001);
        let beacon_b = make_beacon(id_b, scope, 21_001, 21_000);

        beacon_a.publish(&endpoint_data(42_000));
        beacon_b.publish(&endpoint_data(42_001));

        let mut stream = beacon_a.resolve(id_b).expect("resolve stream");
        let item = timeout(Duration::from_secs(5), stream.next())
            .await
            .expect("resolve should complete in time")
            .expect("stream should yield an item")
            .expect("resolve should succeed");
        assert_eq!(item.endpoint_info().endpoint_id, id_b);
        let published: BTreeSet<SocketAddr> = item.ip_addrs().copied().collect();
        assert_eq!(
            published,
            BTreeSet::from([SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 42_001)])
        );
    }

    #[tokio::test]
    async fn different_scope_is_ignored() {
        let scope_a = [11_u8; 16];
        let scope_b = [13_u8; 16];
        let id_a = endpoint_id(3);
        let id_b = endpoint_id(4);

        let message = BeaconMessage {
            magic: BEACON_MAGIC.to_owned(),
            scope: scope_b,
            id: id_b.to_string(),
            addrs: vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 42_011)],
            relay: None,
        };
        let mut peers = HashMap::new();
        let mut pending = HashMap::new();
        handle_discovered(&message, id_a, Some(scope_a), &mut peers, &mut pending).await;
        assert!(peers.is_empty());
        assert!(pending.is_empty());
    }
}
