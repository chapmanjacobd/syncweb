//! Relay manager: persistent registration with Syncthing relays, session
//! establishment, and the CLI `test-relay` check.
//!
//! Two distinct connections exist per peer relationship:
//!
//! * **Control connections** are TLS with ALPN `bep-relay` and carry our
//!   X.509 client certificate, which is our device identity. We register
//!   with every configured relay and then simply read, answering pings and
//!   reacting to incoming [`Message::SessionInvitation`]s.
//! * **Sessions** are plain TCP `[u32 length][data]` tunnels that a relay
//!   brokers between two devices that joined with the same session key.

use std::collections::HashSet;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use iroh::PublicKey;
use iroh_base::CustomAddr;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpStream, lookup_host};
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};
use tokio_rustls::client::TlsStream;
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::error::{Result, SyncwebError};

use super::RelayConfig;
use super::certs::{self, RelayCerts};
use super::pool;
use super::protocol::{
    MAX_BYTE_FIELD, Message, PROTOCOL_NAME, read_data_frame, read_message, write_data_frame, write_message,
};
use super::transport::{Backend, IncomingPacket, Session, SyncthingRelayTransport, own_addr, peer_addr};

/// Default relay listen port, also used for brokered sessions.
const DEFAULT_RELAY_PORT: u16 = 22_067;
/// Session queue depth before datagrams are dropped on overflow.
const SESSION_QUEUE: usize = 1024;
/// How often the relay list is refreshed.
const RELAY_REFRESH: Duration = Duration::from_mins(30);
/// How often to retry fetching the pool when no relays are configured.
const POOL_RETRY: Duration = Duration::from_secs(30);
/// How long a registration connection may go without any traffic.
const CONTROL_IDLE: Duration = Duration::from_mins(1);
/// Reconnect backoff when a registration connection drops.
const JOIN_BACKOFF: Duration = Duration::from_secs(2);
/// How many relays a dial attempts before giving up.
const MAX_DIAL_ATTEMPTS: usize = 4;
/// All-zero node id used when a peer's node id is not yet known.
const UNKNOWN_NODE: [u8; 32] = [0_u8; 32];

/// Options for starting a [`RelayManager`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RelayManagerOptions {
    /// Relay configuration.
    pub config: RelayConfig,
    /// Directory that persists the relay identity certificate.
    pub data_dir: PathBuf,
}

impl RelayManagerOptions {
    /// Create options from a config and data directory.
    #[must_use]
    pub const fn new(config: RelayConfig, data_dir: PathBuf) -> Self {
        Self { config, data_dir }
    }
}

/// A connected relay client: the persistent identity, registration loops and
/// live sessions, plus the transport that exposes it to iroh.
#[derive(Clone)]
pub struct RelayManager {
    backend: Arc<Backend>,
    config: RelayConfig,
    cancel: CancellationToken,
}

impl std::fmt::Debug for RelayManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelayManager").finish_non_exhaustive()
    }
}

impl RelayManager {
    /// Start registration with the configured relays and build the transport
    /// for iroh. The transport advertises a custom address carrying `own_node`.
    ///
    /// # Errors
    ///
    /// Returns an error if the relay identity cannot be loaded or generated,
    /// or if the TLS configuration cannot be built.
    pub fn start(options: RelayManagerOptions, own_node: PublicKey) -> Result<Self> {
        let certs = Arc::new(RelayCerts::load_or_create(&options.data_dir)?);
        let connector = certs::tls_connector(&certs)?;
        let backend = Arc::new(Backend::new(certs, connector, options.config.timeout));
        // Publish our relay address immediately: sharing a ticket must not race
        // the supervisor's first relay-list resolution.
        backend.update_local_addr(own_addr(&own_node, backend.certs.relay_id()));
        let cancel = CancellationToken::new();
        let _spawned = tokio::spawn(supervisor(
            backend.clone(),
            options.config.clone(),
            own_node,
            cancel.clone(),
        ));
        Ok(Self {
            backend,
            config: options.config,
            cancel,
        })
    }

    /// The transport to install on the iroh endpoint.
    #[must_use]
    pub fn transport(&self) -> SyncthingRelayTransport {
        SyncthingRelayTransport::new(self.backend.clone())
    }

    /// The custom address this node advertises.
    #[must_use]
    pub fn local_addr(&self) -> CustomAddr {
        self.backend.local_addr()
    }

    /// The relay device id of this node (SHA-256 of its certificate).
    #[must_use]
    pub fn relay_id(&self) -> [u8; 32] {
        *self.backend.certs.relay_id()
    }

    /// Record the iroh node id owning a peer relay id so inbound datagrams
    /// from that peer can be attributed.
    pub fn remember_node(&self, relay_id: &[u8; 32], node: PublicKey) {
        self.backend.remember_node(*relay_id, *node.as_bytes());
    }

    /// Test whether `url` accepts our registration. A fresh TLS join request
    /// is made; receiving the "joined" or "already connected" responses both
    /// prove the relay knows our identity.
    ///
    /// # Errors
    ///
    /// Returns an error if the relay is unreachable, misbehaving, or rejects
    /// our identity.
    pub async fn test_relay(&self, url: &str) -> Result<TestRelayReport> {
        let started = Instant::now();
        let (host, port, token) = parse_relay_url(url)?;
        let mut stream = control_connect(&self.backend, url, &host, port).await?;
        limited(
            self.config.timeout,
            write_message(
                &mut stream,
                &Message::JoinRelayRequest {
                    token: token.unwrap_or_default(),
                },
            ),
        )
        .await?;
        let reply = limited(self.config.timeout, read_message(&mut stream)).await?;
        match &reply {
            Message::Response {
                code: Message::RESPONSE_SUCCESS | Message::RESPONSE_ALREADY_CONNECTED,
                ..
            } => {}
            Message::Response { code, message } => {
                return Err(SyncwebError::RelayUnreachable {
                    reasons: format!("{url} rejected our registration (code {code}): {message}"),
                });
            }
            Message::RelayFull => {
                return Err(SyncwebError::RelayUnreachable {
                    reasons: format!("{url} is full"),
                });
            }
            Message::Ping
            | Message::Pong
            | Message::JoinRelayRequest { .. }
            | Message::JoinSessionRequest { .. }
            | Message::ConnectRequest { .. }
            | Message::SessionInvitation { .. } => {
                return Err(unexpected_reply(&reply, "unexpected reply to join request"));
            }
        }
        Ok(TestRelayReport {
            url: url.to_owned(),
            own_relay_id: *self.backend.certs.relay_id(),
            elapsed: started.elapsed(),
        })
    }

    /// Send a `ConnectRequest` to a registered relay for `peer`.
    ///
    /// Exposed for tests: it exercises the same dial path the transport's send
    /// side uses. The session is established asynchronously by the
    /// registration reader when the relay answers with a `SessionInvitation`.
    ///
    /// # Errors
    ///
    /// Returns an error if no relay registration is available.
    #[doc(hidden)]
    pub fn dial_peer_for_test(&self, peer: [u8; 32], node: PublicKey) -> Result<()> {
        dial_peer(&self.backend, peer, *node.as_bytes())
    }

    /// Stop all registration loops and sessions.
    pub async fn shutdown(&self) {
        self.cancel.cancel();
        self.backend.cancel.cancel();
        sleep(Duration::from_millis(50)).await;
    }
}

/// Result of a successful `test-relay` handshake.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct TestRelayReport {
    /// The relay url that accepted our identity.
    pub url: String,
    /// Our relay device id.
    pub own_relay_id: [u8; 32],
    /// Round-trip time of the handshake.
    pub elapsed: Duration,
}

impl std::fmt::Display for TestRelayReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "relay {} reachable in {:?} (device {})",
            self.url,
            self.elapsed,
            device_id(self.own_relay_id)
        )
    }
}

/// The error raised when a control reply does not match the expected state.
fn unexpected_reply(reply: &Message, context: &str) -> SyncwebError {
    SyncwebError::RelayDecode(format!("{context}: {reply:?}"))
}

/// Start a background dial to a peer from the transport's send path. Always
/// returns immediately; if a dial is already running for this peer it is a
/// no-op.
pub(crate) fn start_peer_dial(backend: Arc<Backend>, peer: [u8; 32], node: [u8; 32]) {
    if !backend.dials.lock().insert(peer) {
        return;
    }
    tokio::spawn(async move {
        let result = dial_peer(&backend, peer, node);
        if result.is_ok() {
            // Hold the dial slot until the session comes up so the transport's
            // repeated sends do not spam the relay with duplicate
            // `ConnectRequest`s (each of which brokers a fresh session key).
            for _ in 0..150 {
                if backend.sessions.read().contains_key(&peer) {
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        }
        backend.dials.lock().remove(&peer);
        if let Err(error) = result {
            warn!(%error, "relay dial to peer failed");
        }
    });
}

/// Resolve a registered relay and send a `ConnectRequest` for `peer` over its
/// existing registration connection.
///
/// The relay attributes the request to the device that owns that connection
/// and delivers a `SessionInvitation` back on it, which the registration
/// reader turns into a session. Opening a fresh connection instead would make
/// the relay treat us as a duplicate registration and drop it.
fn dial_peer(backend: &Arc<Backend>, peer: [u8; 32], node: [u8; 32]) -> Result<()> {
    if peer == [0_u8; 32] {
        return Err(SyncwebError::RelayDecode("peer device id is empty".to_owned()));
    }
    // Record the peer's node id so the invitation delivered on our
    // registration connection can be attributed to it.
    backend.remember_node(peer, node);
    let mut failures = Vec::new();
    for _ in 0..MAX_DIAL_ATTEMPTS {
        let Some(url) = backend.next_relay() else {
            failures.push("no relays configured".to_owned());
            break;
        };
        let sender = backend.control.read().get(&url).cloned();
        match sender {
            Some(control) => match control.send(Message::ConnectRequest { id: peer.to_vec() }) {
                Ok(()) => return Ok(()),
                Err(_) => failures.push(format!("{url}: registration connection closed")),
            },
            None => failures.push(format!("{url}: not registered")),
        }
    }
    Err(SyncwebError::RelayUnreachable {
        reasons: failures.join("; "),
    })
}

/// A pending session negotiated with the relay.
struct SessionSpec {
    key: Vec<u8>,
    address: Vec<u8>,
    port: u16,
}

/// The join connection handler: keep one registration alive for a relay and
/// act on incoming invitations and keepalives.
async fn join_loop(backend: Arc<Backend>, url: String, cancel: CancellationToken) {
    loop {
        if cancel.is_cancelled() {
            return;
        }
        let (host, port, token) = match parse_relay_url(&url) {
            Ok(parts) => parts,
            Err(error) => {
                warn!(%url, %error, "invalid relay url");
                return;
            }
        };
        if let Err(error) = serve_registration(&backend, &url, &host, port, token).await {
            warn!(%url, %error, "relay registration dropped");
        }
        sleep_with_cancel(cancel.clone(), JOIN_BACKOFF).await;
    }
}

/// Perform the join handshake and then read registrations until the
/// connection breaks.
async fn serve_registration(
    backend: &Arc<Backend>,
    url: &str,
    host: &str,
    port: u16,
    token: Option<String>,
) -> Result<()> {
    let mut stream = control_connect(backend, url, host, port).await?;
    limited(
        backend.timeout,
        write_message(
            &mut stream,
            &Message::JoinRelayRequest {
                token: token.unwrap_or_default(),
            },
        ),
    )
    .await?;
    let reply = limited(backend.timeout, read_message(&mut stream)).await?;
    match &reply {
        Message::Response {
            code: Message::RESPONSE_SUCCESS | Message::RESPONSE_ALREADY_CONNECTED,
            ..
        } => {}
        Message::Response { code, message } => {
            return Err(SyncwebError::RelayUnreachable {
                reasons: format!("{url} rejected registration (code {code}): {message}"),
            });
        }
        Message::RelayFull => {
            return Err(SyncwebError::RelayUnreachable {
                reasons: format!("{url} is full"),
            });
        }
        Message::Ping
        | Message::Pong
        | Message::JoinRelayRequest { .. }
        | Message::JoinSessionRequest { .. }
        | Message::ConnectRequest { .. }
        | Message::SessionInvitation { .. } => {
            return Err(unexpected_reply(&reply, "unexpected reply to join"));
        }
    }

    // Split the connection so outbound control messages (keepalive pongs and
    // `ConnectRequest`s from the transport's send path) can be written while
    // this task owns the read side. The writer channel is published under the
    // relay url so `dial_peer` can address this registration.
    let (mut reader, mut writer) = tokio::io::split(stream);
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    backend.control.write().insert(url.to_owned(), tx.clone());
    let control_tx = tx.clone();
    let writer_tx = tx;
    let writer_backend = backend.clone();
    let writer_url = url.to_owned();
    let writer_cancel = backend.cancel.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                () = writer_cancel.cancelled() => break,
                incoming = rx.recv() => {
                    let Some(message) = incoming else { break };
                    if write_message(&mut writer, &message).await.is_err() {
                        break;
                    }
                }
            }
        }
        let mut control = writer_backend.control.write();
        if control
            .get(&writer_url)
            .is_some_and(|existing| existing.same_channel(&writer_tx))
        {
            control.remove(&writer_url);
        }
    });
    loop {
        let message = match limited(backend.timeout.max(CONTROL_IDLE), read_message(&mut reader)).await {
            Err(error) => return Err(error),
            Ok(message) => message,
        };
        match &message {
            Message::Ping => {
                let _ = control_tx.send(Message::Pong);
            }
            Message::Pong | Message::RelayFull => {}
            Message::SessionInvitation {
                from: inviter,
                key,
                address,
                port: invite_port,
                ..
            } => {
                let from = try_into_32(inviter)?;
                let node = backend.node_ids.read().get(&from).copied().unwrap_or(UNKNOWN_NODE);
                let spec = SessionSpec {
                    key: key.clone(),
                    address: address.clone(),
                    port: *invite_port,
                };
                tokio::spawn(inbound_session(backend.clone(), url.to_owned(), from, node, spec));
            }
            Message::JoinRelayRequest { .. }
            | Message::JoinSessionRequest { .. }
            | Message::Response { .. }
            | Message::ConnectRequest { .. } => {
                return Err(unexpected_reply(
                    &message,
                    "unexpected message on registration connection",
                ));
            }
        }
    }
}

/// Establish an inbound session from an invitation on our registration
/// connection, in the background.
async fn inbound_session(backend: Arc<Backend>, url: String, peer: [u8; 32], node: [u8; 32], spec: SessionSpec) {
    if let Err(error) = establish_session(&backend, &url, peer, node, spec).await {
        warn!(%url, %error, "inbound session failed");
    }
}

/// Broker a session: connect to the relay's session listener and join.
async fn establish_session(
    backend: &Arc<Backend>,
    url: &str,
    peer: [u8; 32],
    node: [u8; 32],
    spec: SessionSpec,
) -> Result<()> {
    let addrs = session_addrs(url, spec.address.as_slice(), spec.port).await?;
    let mut last_error = None;
    for addr in addrs {
        match join_session(backend, url, peer, node, addr, &spec.key).await {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| SyncwebError::RelayUnreachable {
        reasons: format!("{url}: no session address was usable"),
    }))
}

/// Join a single session listener.
async fn join_session(
    backend: &Arc<Backend>,
    url: &str,
    peer: [u8; 32],
    node: [u8; 32],
    addr: SocketAddr,
    key: &[u8],
) -> Result<()> {
    if key.len() > MAX_BYTE_FIELD {
        return Err(SyncwebError::RelayDecode(
            "session key exceeds the protocol limit".to_owned(),
        ));
    }
    let mut stream = limited_io(url, backend.timeout, TcpStream::connect(addr)).await?;
    limited(
        backend.timeout,
        write_message(&mut stream, &Message::JoinSessionRequest { key: key.to_vec() }),
    )
    .await?;
    let reply = limited(backend.timeout, read_message(&mut stream)).await?;
    match &reply {
        Message::Response {
            code: Message::RESPONSE_SUCCESS,
            ..
        } => {}
        Message::Response { code, message } => {
            return Err(SyncwebError::RelayUnreachable {
                reasons: format!("{url}: session join rejected (code {code}): {message}"),
            });
        }
        Message::Ping
        | Message::Pong
        | Message::JoinRelayRequest { .. }
        | Message::JoinSessionRequest { .. }
        | Message::ConnectRequest { .. }
        | Message::SessionInvitation { .. }
        | Message::RelayFull => {
            return Err(unexpected_reply(&reply, "unexpected reply to session join"));
        }
    }
    register_session(backend, peer, node, stream);
    Ok(())
}

/// Wrap a stream as a session, register it and start its reader/writer tasks.
fn register_session(backend: &Arc<Backend>, peer: [u8; 32], node: [u8; 32], stream: TcpStream) {
    let (read_half, write_half) = stream.into_split();
    let (tx, rx) = mpsc::channel::<bytes::Bytes>(SESSION_QUEUE);
    let cancel = backend.cancel.clone();
    let session = Arc::new(Session {
        peer,
        peer_node: node,
        queue: tx,
        cancel: cancel.clone(),
    });
    {
        let mut sessions = backend.sessions.write();
        if let Some(previous) = sessions.insert(peer, session.clone()) {
            previous.cancel.cancel();
        }
    }
    tokio::spawn(run_session_reader(backend.clone(), session.clone(), read_half));
    tokio::spawn(run_session_writer(backend.clone(), session, write_half, rx, cancel));
}

/// The reader task of a session: deliver frames to iroh.
async fn run_session_reader(backend: Arc<Backend>, session: Arc<Session>, mut read_half: OwnedReadHalf) {
    let remote = peer_addr(session.peer, session.peer_node);
    let local = backend.local_addr();
    while let Ok(frame) = read_data_frame(&mut read_half).await {
        let Some(tx) = backend.ingress.lock().clone() else {
            continue;
        };
        if tx
            .send(IncomingPacket {
                remote: remote.clone(),
                local: local.clone(),
                data: bytes::Bytes::from(frame),
            })
            .is_err()
        {
            break;
        }
    }
    remove_session(&backend, session.peer, &session);
}

/// The writer task of a session: drain the queue into framed datagrams.
async fn run_session_writer(
    backend: Arc<Backend>,
    session: Arc<Session>,
    mut write_half: OwnedWriteHalf,
    mut rx: mpsc::Receiver<bytes::Bytes>,
    cancel: CancellationToken,
) {
    loop {
        let data = tokio::select! {
            () = cancel.cancelled() => break,
            data = rx.recv() => match data {
                Some(packet) => packet,
                None => break,
            },
        };
        if write_data_frame(&mut write_half, &data).await.is_err() {
            break;
        }
    }
    remove_session(&backend, session.peer, &session);
}

/// Remove `session` from the map only if it is still the registered one.
fn remove_session(backend: &Backend, peer: [u8; 32], session: &Arc<Session>) {
    let mut sessions = backend.sessions.write();
    if sessions
        .get(&peer)
        .is_some_and(|existing| Arc::ptr_eq(existing, session))
    {
        sessions.remove(&peer);
    }
}

/// The supervisor task resolves the relay list, publishes our address and
/// keeps a registration loop running for every relay.
async fn supervisor(backend: Arc<Backend>, config: RelayConfig, own_node: PublicKey, cancel: CancellationToken) {
    let mut running: HashSet<String> = HashSet::new();
    let mut pool_seen = false;
    loop {
        if cancel.is_cancelled() {
            return;
        }
        let urls = match relay_list(&config, pool_seen).await {
            Ok(urls) => urls,
            Err(error) => {
                warn!(%error, "relay list unavailable");
                sleep_with_cancel(cancel.clone(), POOL_RETRY).await;
                continue;
            }
        };
        pool_seen = true;
        let addr = own_addr(&own_node, backend.certs.relay_id());
        let stored = urls.clone();
        *backend.relays.write() = stored;
        backend.update_local_addr(addr);
        for url in urls {
            if running.insert(url.clone()) {
                tokio::spawn(join_loop(backend.clone(), url, cancel.clone()));
            }
        }
        sleep_with_cancel(cancel.clone(), RELAY_REFRESH).await;
    }
}

/// Resolve the list of relay urls: the configured ones, or the live pool
/// when auto-fallback is enabled.
async fn relay_list(config: &RelayConfig, pool_seen: bool) -> Result<Vec<String>> {
    if !config.relay_urls.is_empty() {
        return Ok(config.relay_urls.clone());
    }
    if !config.auto_fallback {
        return Ok(Vec::new());
    }
    match pool::fetch_relay_pool(config.timeout.max(Duration::from_secs(5))).await {
        Ok(urls) if urls.is_empty() => Err(SyncwebError::RelayUnreachable {
            reasons: "relay pool returned no relays".to_owned(),
        }),
        Ok(urls) => Ok(urls),
        Err(error) => {
            if !pool_seen {
                warn!(%error, "initial relay pool fetch failed");
            }
            Err(error)
        }
    }
}

/// Sleep for `duration` unless cancelled.
async fn sleep_with_cancel(cancel: CancellationToken, duration: Duration) {
    tokio::select! {
        () = cancel.cancelled() => {}
        () = sleep(duration) => {}
    }
}

/// Open a TLS control connection to a relay and validate the ALPN handshake.
async fn control_connect(backend: &Backend, url: &str, host: &str, port: u16) -> Result<TlsStream<TcpStream>> {
    let socket = resolve(host, port)
        .await
        .ok_or_else(|| SyncwebError::RelayUnreachable {
            reasons: format!("{url}: host {host} did not resolve"),
        })?;
    let name = certs::server_name_for(host)?;
    let tcp = limited_io(url, backend.timeout, TcpStream::connect(socket)).await?;
    let tls = limited_io(url, backend.timeout, backend.connector.connect(name, tcp)).await?;
    if let Some(named) = tls.get_ref().1.alpn_protocol()
        && named != PROTOCOL_NAME
    {
        return Err(SyncwebError::RelayUnreachable {
            reasons: format!(
                "{url}: relay negotiated unexpected ALPN protocol {:?}",
                String::from_utf8_lossy(named)
            ),
        });
    }
    Ok(tls)
}

/// Resolve `host:port`; returns the first address.
async fn resolve(host: &str, port: u16) -> Option<SocketAddr> {
    lookup_host((host, port)).await.ok()?.next()
}

/// The session listener addresses to try for an invitation, falling back to
/// the relay's own host when the invitation address is empty or unspecified.
async fn session_addrs(url: &str, address: &[u8], invite_port: u16) -> Result<Vec<SocketAddr>> {
    let port = if invite_port == 0 {
        DEFAULT_RELAY_PORT
    } else {
        invite_port
    };
    if address.is_empty() || address.iter().all(|byte| *byte == 0) {
        let (host, _token, _port) = parse_relay_url(url)?;
        let addresses = lookup_host((host.as_str(), port)).await?.collect::<Vec<SocketAddr>>();
        if addresses.is_empty() {
            return Err(SyncwebError::RelayUnreachable {
                reasons: format!("{url}: session host {host} did not resolve"),
            });
        }
        Ok(addresses)
    } else {
        Ok(vec![SocketAddr::new(ip_from_bytes(address)?, port)])
    }
}

/// Convert invitation address bytes into an [`IpAddr`].
fn ip_from_bytes(bytes: &[u8]) -> Result<IpAddr> {
    match bytes.len() {
        4 => Ok(IpAddr::from(try_into_array::<4>(bytes)?)),
        16 => Ok(IpAddr::from(try_into_array::<16>(bytes)?)),
        length => Err(SyncwebError::RelayDecode(format!(
            "invitation address has invalid length {length}"
        ))),
    }
}

/// Convert a byte slice into a fixed-size array.
fn try_into_array<const N: usize>(bytes: &[u8]) -> Result<[u8; N]> {
    bytes.try_into().map_err(|error| {
        SyncwebError::RelayDecode(format!("byte field has unexpected length {}: {error}", bytes.len()))
    })
}

/// Convert an invitation/connect byte field into a 32-byte device id.
fn try_into_32(bytes: &[u8]) -> Result<[u8; 32]> {
    try_into_array(bytes)
}

/// Parse a relay url into its host, port, and join token.
fn parse_relay_url(url: &str) -> Result<(String, u16, Option<String>)> {
    let parsed = url::Url::parse(url).map_err(|error| SyncwebError::RelayBadAddress(format!("{url}: {error}")))?;
    match parsed.scheme() {
        "relay" | "tcp" => {}
        _ => return Err(SyncwebError::RelayBadScheme),
    }
    let host = parsed
        .host_str()
        .filter(|host| !host.is_empty())
        .ok_or_else(|| SyncwebError::RelayBadAddress(url.to_owned()))?;
    let port = parsed.port().unwrap_or(DEFAULT_RELAY_PORT);
    let token = parsed
        .query_pairs()
        .find(|(key, _)| key == "token")
        .map(|(_, value)| value.into_owned());
    Ok((host.to_owned(), port, token))
}

/// Run a future with a timeout, mapping the ticking timeout into
/// [`SyncwebError::RelayUnreachable`].
async fn limited<T>(duration: Duration, future: impl Future<Output = Result<T>>) -> Result<T> {
    timeout(duration, future).await.map_or_else(
        |_| {
            Err(SyncwebError::RelayUnreachable {
                reasons: "relay operation timed out".to_owned(),
            })
        },
        std::convert::identity,
    )
}

/// Run an IO future with a timeout, mapping errors and timeouts into
/// [`SyncwebError::RelayUnreachable`].
async fn limited_io<T>(
    context: &str,
    duration: Duration,
    future: impl Future<Output = std::io::Result<T>>,
) -> Result<T> {
    match timeout(duration, future).await {
        Err(_) => Err(SyncwebError::RelayUnreachable {
            reasons: format!("{context}: operation timed out"),
        }),
        Ok(Err(error)) => Err(SyncwebError::RelayUnreachable {
            reasons: format!("{context}: {error}"),
        }),
        Ok(Ok(value)) => Ok(value),
    }
}

/// Format a device id the way Syncthing relays display them.
#[must_use]
fn device_id(bytes: [u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for (index, byte) in bytes.iter().enumerate() {
        if index == 8 || index == 16 || index == 24 {
            out.push('-');
        }
        let _ = write!(out, "{byte:02X}");
    }
    out
}
