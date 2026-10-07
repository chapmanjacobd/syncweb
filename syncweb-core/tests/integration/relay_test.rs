//! End-to-end tests for the Syncthing relay (BEP) support.
//!
//! A small mock relay speaks the real bep-relay protocol against
//! [`RelayManager`]: a TLS control listener with ALPN `bep-relay` and a
//! client certificate requirement, XDR-framed messages, keepalive ping/pong,
//! and a plain TCP session listener that brokers `JoinSessionRequest`
//! handshakes. Building the mock on the crate's own `protocol` module keeps
//! both directions of every frame honest.
//!
//! Sessions are driven over the manager's inbound path: the mock delivers a
//! `SessionInvitation` on the registration connection and the manager must
//! dial the session listener and join with the invited key. The outbound
//! dial path (`ConnectRequest`) is exercised only through iroh endpoint
//! wiring, which lands at the daemon stage.

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::Context;
use rcgen::{CertificateParams, DnType, KeyPair, KeyUsagePurpose, PKCS_ECDSA_P256_SHA256, SanType};
use rustls::SignatureScheme;
use rustls::pki_types::CertificateDer;
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use syncweb_core::net::relay::protocol::{self, Message};
use syncweb_core::net::{RelayConfig, RelayManager, RelayManagerOptions};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;

/// How the mock relay answers a `JoinRelayRequest`.
#[derive(Debug, Clone, Copy)]
enum JoinPolicy {
    /// Accept every registration, then verify the client answers a keepalive
    /// ping with a pong.
    Accept,
    /// Reject every registration with a wrong-token response.
    Reject,
    /// Answer every registration with `RelayFull`.
    RelayFull,
    /// Accept the registration, verify the keepalive, then send one
    /// `SessionInvitation` that points at our session listener.
    AcceptThenInvite { key: [u8; 32], from: [u8; 32] },
    /// Accept the registration, verify the keepalive, then answer any
    /// `ConnectRequest` with a `SessionInvitation` for the requested device.
    AcceptThenConnect { key: [u8; 32] },
}

/// A control-plane event recorded by the mock relay.
#[derive(Debug)]
enum ControlEvent {
    #[expect(dead_code)]
    Joined {
        token: String,
    },
    PongReceived,
}

/// A running mock relay with a TLS control listener and a raw session
/// listener.
struct MockRelay {
    control_addr: SocketAddr,
    control_receiver: mpsc::UnboundedReceiver<ControlEvent>,
    session_receiver: mpsc::UnboundedReceiver<Vec<u8>>,
    _control_task: tokio::task::JoinHandle<anyhow::Result<()>>,
    _session_task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl MockRelay {
    async fn spawn(policy: JoinPolicy) -> anyhow::Result<Self> {
        let acceptor = TlsAcceptor::from(Arc::new(mock_server_config()?));
        let control_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("bind control listener")?;
        let session_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("bind session listener")?;
        let control_addr = control_listener.local_addr().context("control address")?;
        let session_port = session_listener.local_addr().context("session address")?.port();
        let (control_sender, control_receiver) = mpsc::unbounded_channel();
        let (session_sender, session_receiver) = mpsc::unbounded_channel();
        let invite_gate = Arc::new(AtomicBool::new(false));
        let control_task = tokio::spawn(serve_control(
            acceptor,
            control_listener,
            policy,
            session_port,
            control_sender,
            invite_gate,
        ));
        let session_task = tokio::spawn(serve_session_listener(session_listener, session_sender));
        Ok(Self {
            control_addr,
            control_receiver,
            session_receiver,
            _control_task: control_task,
            _session_task: session_task,
        })
    }

    async fn next_control_event(&mut self) -> anyhow::Result<ControlEvent> {
        timeout(Duration::from_secs(5), self.control_receiver.recv())
            .await
            .context("timed out waiting for a control event")?
            .context("control event channel closed")
    }

    async fn next_session_key(&mut self) -> anyhow::Result<Vec<u8>> {
        timeout(Duration::from_secs(5), self.session_receiver.recv())
            .await
            .context("timed out waiting for a session join")?
            .context("session event channel closed")
    }
}

/// Accept every TLS control connection: the client certificate is the relay
/// identity, but these tests do not need to inspect it.
#[derive(Debug)]
struct AcceptAnyClientCert;

impl ClientCertVerifier for AcceptAnyClientCert {
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<ClientCertVerified, rustls::Error> {
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }

    fn client_auth_mandatory(&self) -> bool {
        true
    }
}

/// Build a self-signed TLS server config for the mock relay.
fn mock_server_config() -> anyhow::Result<rustls::ServerConfig> {
    let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).context("generate test key pair")?;
    let mut params = CertificateParams::new(Vec::new()).context("certificate parameters")?;
    params.distinguished_name.push(DnType::CommonName, "syncweb test relay");
    params.is_ca = rcgen::IsCa::NoCa;
    params.key_usages = vec![KeyUsagePurpose::KeyEncipherment, KeyUsagePurpose::DigitalSignature];
    params.subject_alt_names = vec![SanType::IpAddress(IpAddr::from([127, 0, 0, 1]))];
    let certificate = params.self_signed(&key_pair).context("sign the relay certificate")?;
    let server_chain = vec![CertificateDer::from(certificate.der().to_vec())];
    let private_key =
        rustls::pki_types::PrivateKeyDer::from(rustls::pki_types::PrivatePkcs8KeyDer::from(key_pair.serialize_der()));
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
        .context("server protocol versions")?
        .with_client_cert_verifier(Arc::new(AcceptAnyClientCert))
        .with_single_cert(server_chain, private_key)
        .context("server certificate")?;
    config.alpn_protocols = vec![protocol::PROTOCOL_NAME.to_vec()];
    Ok(config)
}

/// Accept control connections and handle each on its own task.
async fn serve_control(
    acceptor: TlsAcceptor,
    listener: TcpListener,
    policy: JoinPolicy,
    session_port: u16,
    events: mpsc::UnboundedSender<ControlEvent>,
    invite_gate: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    loop {
        let (socket, _peer) = listener.accept().await.context("control accept")?;
        tokio::spawn(control_connection(
            acceptor.clone(),
            socket,
            policy,
            session_port,
            events.clone(),
            invite_gate.clone(),
        ));
    }
}

/// Serve one control (TLS) connection: registration, keepalives and, when
/// configured, a session invitation.
async fn control_connection(
    acceptor: TlsAcceptor,
    socket: TcpStream,
    policy: JoinPolicy,
    session_port: u16,
    events: mpsc::UnboundedSender<ControlEvent>,
    invite_gate: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    let mut stream = acceptor.accept(socket).await.context("control TLS handshake")?;
    loop {
        match protocol::read_message(&mut stream).await.context("control frame")? {
            Message::Pong => {}
            Message::Ping => {
                protocol::write_message(&mut stream, &Message::Pong)
                    .await
                    .context("write pong")?;
            }
            Message::JoinRelayRequest { token: _ } => match policy {
                JoinPolicy::Accept => {
                    write_response(&mut stream, Message::RESPONSE_SUCCESS, "joined").await?;
                    events
                        .send(ControlEvent::Joined { token: String::new() })
                        .map_err(|e| send_error(&e))?;
                    serve_keepalive(&mut stream, &events).await?;
                }
                JoinPolicy::AcceptThenInvite { key, from } => {
                    write_response(&mut stream, Message::RESPONSE_SUCCESS, "joined").await?;
                    events
                        .send(ControlEvent::Joined { token: String::new() })
                        .map_err(|e| send_error(&e))?;
                    serve_keepalive(&mut stream, &events).await?;
                    if !invite_gate.swap(true, Ordering::Relaxed) {
                        protocol::write_message(
                            &mut stream,
                            &Message::SessionInvitation {
                                from: from.to_vec(),
                                key: key.to_vec(),
                                address: vec![127, 0, 0, 1],
                                port: session_port,
                                server_socket: false,
                            },
                        )
                        .await
                        .context("write session invitation")?;
                    }
                }
                JoinPolicy::AcceptThenConnect { key } => {
                    write_response(&mut stream, Message::RESPONSE_SUCCESS, "joined").await?;
                    events
                        .send(ControlEvent::Joined { token: String::new() })
                        .map_err(|e| send_error(&e))?;
                    serve_keepalive(&mut stream, &events).await?;
                    loop {
                        match protocol::read_message(&mut stream).await.context("connect frame")? {
                            Message::ConnectRequest { id } => {
                                protocol::write_message(
                                    &mut stream,
                                    &Message::SessionInvitation {
                                        from: id,
                                        key: key.to_vec(),
                                        address: vec![127, 0, 0, 1],
                                        port: session_port,
                                        server_socket: false,
                                    },
                                )
                                .await
                                .context("write connect invitation")?;
                            }
                            Message::Ping => {
                                protocol::write_message(&mut stream, &Message::Pong)
                                    .await
                                    .context("write pong")?;
                            }
                            Message::Pong | Message::Response { .. } => {}
                            // Package the remaining known control messages
                            // (and any future ones: `Message` is
                            // `#[non_exhaustive]` in this crate) into a bail.
                            Message::RelayFull
                            | Message::JoinRelayRequest { .. }
                            | Message::JoinSessionRequest { .. }
                            | Message::SessionInvitation { .. }
                            | _ => {
                                anyhow::bail!("unexpected message after registration");
                            }
                        }
                    }
                }
                JoinPolicy::Reject => {
                    write_response(&mut stream, Message::RESPONSE_WRONG_TOKEN, "bad token").await?;
                }
                JoinPolicy::RelayFull => {
                    protocol::write_message(&mut stream, &Message::RelayFull)
                        .await
                        .context("write relay full")?;
                }
            },
            Message::ConnectRequest { .. }
            | Message::JoinSessionRequest { .. }
            | Message::Response { .. }
            | Message::SessionInvitation { .. }
            | Message::RelayFull
            | _ => {
                write_response(&mut stream, Message::RESPONSE_UNEXPECTED, "unexpected message").await?;
            }
        }
    }
}

/// Send a keepalive ping and require the client to answer with a pong.
async fn serve_keepalive<W>(stream: &mut W, events: &mpsc::UnboundedSender<ControlEvent>) -> anyhow::Result<()>
where
    W: AsyncRead + AsyncWrite + Unpin,
{
    protocol::write_message(stream, &Message::Ping)
        .await
        .context("write keepalive ping")?;
    let reply = protocol::read_message(stream).await.context("keepalive pong")?;
    if reply != Message::Pong {
        anyhow::bail!("expected Pong to keepalive ping, got {reply:?}");
    }
    events.send(ControlEvent::PongReceived).map_err(|e| send_error(&e))?;
    Ok(())
}

/// Accept raw session connections and join them with the invited key.
async fn serve_session_listener(listener: TcpListener, sessions: mpsc::UnboundedSender<Vec<u8>>) -> anyhow::Result<()> {
    loop {
        let (socket, _peer) = listener.accept().await.context("session accept")?;
        tokio::spawn(session_connection(socket, sessions.clone()));
    }
}

/// Handle one session connection: verify the join, then relay a beacon frame.
async fn session_connection(mut stream: TcpStream, sessions: mpsc::UnboundedSender<Vec<u8>>) -> anyhow::Result<()> {
    let message = protocol::read_message(&mut stream)
        .await
        .context("session join frame")?;
    let Message::JoinSessionRequest { key } = message else {
        anyhow::bail!("expected JoinSessionRequest, got {message:?}");
    };
    sessions.send(key).map_err(|e| send_error(&e))?;
    write_response(&mut stream, Message::RESPONSE_SUCCESS, "joined").await?;
    protocol::write_data_frame(&mut stream, b"relay beacon")
        .await
        .context("session beacon")?;
    Ok(())
}

/// Write a `Response` message.
async fn write_response<W>(stream: &mut W, code: i32, message: &str) -> anyhow::Result<()>
where
    W: AsyncWrite + Unpin,
{
    protocol::write_message(
        stream,
        &Message::Response {
            code,
            message: message.to_owned(),
        },
    )
    .await
    .context("write response")
}

/// Turn a closed test channel into a readable error.
fn send_error<T>(error: &mpsc::error::SendError<T>) -> anyhow::Error {
    anyhow::anyhow!("relay event channel closed: {error}")
}

/// Wait until the mock has seen a join request and the resulting pong.
///
/// Both the supervisor registration loop and a `test_relay` probe open
/// control connections, so the events arrive in an arbitrary order.
async fn receive_join_and_pong(mock: &mut MockRelay) -> anyhow::Result<()> {
    let mut joined = false;
    let mut ponged = false;
    let mut seen = 0_u8;
    while seen < 8 && !(joined && ponged) {
        match mock.next_control_event().await? {
            ControlEvent::Joined { .. } => joined = true,
            ControlEvent::PongReceived => ponged = true,
        }
        seen = seen.saturating_add(1);
    }
    anyhow::ensure!(joined, "the relay never saw a join request");
    anyhow::ensure!(ponged, "the relay never saw a pong reply");
    Ok(())
}

/// Build a fresh data directory for a [`RelayManager`].
fn temp_relay_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("syncweb-relay-test-{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Start a [`RelayManager`] pointed at the relay under test.
fn manager(relay_url: String) -> anyhow::Result<(RelayManager, PathBuf)> {
    let dir = temp_relay_dir();
    let mut config = RelayConfig::default();
    config.relay_urls = vec![relay_url];
    config.timeout = Duration::from_secs(5);
    let options = RelayManagerOptions::new(config, dir.clone());
    let node = iroh::SecretKey::generate().public();
    let relay_manager = RelayManager::start(options, node)?;
    Ok((relay_manager, dir))
}

#[test]
fn relay_messages_round_trip() -> anyhow::Result<()> {
    let messages = vec![
        Message::Ping,
        Message::Pong,
        Message::RelayFull,
        Message::JoinRelayRequest {
            token: "s3cret".to_owned(),
        },
        Message::JoinSessionRequest { key: vec![3_u8; 32] },
        Message::Response {
            code: 0,
            message: "ok".to_owned(),
        },
        Message::ConnectRequest { id: vec![5_u8; 32] },
        Message::SessionInvitation {
            from: vec![1_u8; 32],
            key: vec![2_u8; 32],
            address: vec![127, 0, 0, 1],
            port: 22_067,
            server_socket: true,
        },
    ];
    for message in messages {
        let frame = message.encode_frame()?;
        anyhow::ensure!(Message::decode_frame(&frame)? == message);
    }
    anyhow::ensure!(Message::decode_frame(&[1_u8; 4]).is_err());
    Ok(())
}

#[tokio::test]
async fn control_and_session_frames_round_trip() -> anyhow::Result<()> {
    let (mut writer, mut reader) = tokio::io::duplex(128);
    let message = Message::SessionInvitation {
        from: vec![1_u8; 32],
        key: vec![2_u8; 32],
        address: vec![127, 0, 0, 1],
        port: 1,
        server_socket: false,
    };
    protocol::write_message(&mut writer, &message).await?;
    anyhow::ensure!(protocol::read_message(&mut reader).await? == message);
    protocol::write_data_frame(&mut writer, b"payload").await?;
    anyhow::ensure!(protocol::read_data_frame(&mut reader).await? == b"payload");
    Ok(())
}

#[tokio::test]
async fn test_relay_accepts_registration() -> anyhow::Result<()> {
    let mut mock = MockRelay::spawn(JoinPolicy::Accept).await?;
    let relay_url = format!("tcp://{}", mock.control_addr);
    let (relay_manager, _dir) = manager(relay_url.clone())?;

    let report = relay_manager.test_relay(&relay_url).await?;
    anyhow::ensure!(report.url == relay_url);
    anyhow::ensure!(!report.elapsed.is_zero());
    anyhow::ensure!(!report.own_relay_id.is_empty());

    receive_join_and_pong(&mut mock).await?;
    relay_manager.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_relay_rejects_wrong_token() -> anyhow::Result<()> {
    let mock = MockRelay::spawn(JoinPolicy::Reject).await?;
    let relay_url = format!("tcp://{}", mock.control_addr);
    let (relay_manager, _dir) = manager(relay_url.clone())?;

    let error = relay_manager.test_relay(&relay_url).await.unwrap_err();
    anyhow::ensure!(error.to_string().contains("rejected our registration"));
    relay_manager.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_relay_reports_full_relay() -> anyhow::Result<()> {
    let mock = MockRelay::spawn(JoinPolicy::RelayFull).await?;
    let relay_url = format!("tcp://{}", mock.control_addr);
    let (relay_manager, _dir) = manager(relay_url.clone())?;

    let error = relay_manager.test_relay(&relay_url).await.unwrap_err();
    anyhow::ensure!(error.to_string().contains("is full"));
    relay_manager.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn inbound_invitation_brokers_a_session() -> anyhow::Result<()> {
    let session_key = [9_u8; 32];
    let initiator = [1_u8; 32];
    let mut mock = MockRelay::spawn(JoinPolicy::AcceptThenInvite {
        key: session_key,
        from: initiator,
    })
    .await?;
    let relay_url = format!("tcp://{}", mock.control_addr);
    let (relay_manager, _dir) = manager(relay_url.clone())?;

    receive_join_and_pong(&mut mock).await?;
    let joined_key = mock.next_session_key().await?;
    anyhow::ensure!(joined_key == session_key);

    relay_manager.shutdown().await;
    Ok(())
}

/// Regression: the address advertised immediately must encode the node's
/// relay id and iroh node id, so a shared ticket carries it before the
/// supervisor has connected anywhere.
#[tokio::test]
async fn local_addr_is_published_immediately() -> anyhow::Result<()> {
    let mut config = RelayConfig::default();
    config.relay_urls = vec!["tcp://127.0.0.1:1".to_owned()];
    let node = iroh::SecretKey::generate().public();
    let relay_manager = RelayManager::start(RelayManagerOptions::new(config, temp_relay_dir()), node)?;

    let addr = relay_manager.local_addr();
    anyhow::ensure!(addr.id() == syncweb_core::net::relay::BEP_TRANSPORT_ID);
    let data = addr.data();
    let (version, rest) = data.split_first().context("address payload must not be empty")?;
    let (node_part, relay_part) = rest.split_at(node.as_bytes().len());
    anyhow::ensure!(*version == 1, "unexpected address version {version}");
    anyhow::ensure!(node_part == node.as_bytes(), "node id mismatch");
    anyhow::ensure!(
        relay_part == relay_manager.relay_id(),
        "relay id mismatch ({} bytes)",
        relay_part.len()
    );
    Ok(())
}

/// Regression: a folder ticket shared by a relay-enabled node must carry the
/// node's Syncthing relay custom address, so a joining peer can reach it when
/// direct paths are blocked.
#[tokio::test]
async fn shared_ticket_carries_relay_custom_addr() -> anyhow::Result<()> {
    use syncweb_core::folder::{FolderManager, SyncMode};
    use syncweb_core::node::iroh_node::{DiscoveryConfig, IrohNode, RelayMode};

    let directory = crate::test_utils::TestDirectory::new("syncweb-relay-ticket")?;
    let root = directory.path().join("node");
    let identity = syncweb_core::node::identity::IdentityManager::new(root.join("identity.key"))?;
    let mut config = RelayConfig::default();
    config.relay_urls = vec!["tcp://127.0.0.1:1".to_owned()];
    let relay = RelayManager::start(RelayManagerOptions::new(config, root.join("relay")), identity.node_id())?;
    let expected = relay.local_addr();
    let node = IrohNode::new_with_relay(
        identity,
        root.join("data"),
        RelayMode::None,
        crate::test_utils::empty_member_keys(),
        DiscoveryConfig::disabled(),
        Some(relay),
    )
    .await?;

    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::SendReceive).await?;
    let ticket = folder.ticket(true).await?;
    let carried = ticket
        .nodes
        .iter()
        .flat_map(|n| n.addrs.iter())
        .any(|addr| matches!(addr, iroh::TransportAddr::Custom(custom) if custom == &expected));
    anyhow::ensure!(carried, "ticket should carry the relay custom address: {ticket:?}");

    node.stop().await?;
    Ok(())
}

/// Regression: a node-id-only ticket must carry no addresses at all, even when
/// the sharing node has a Syncthing relay address, so a peer is forced to
/// resolve the node over the DHT topic tracker rather than the ticket.
#[tokio::test]
async fn id_only_ticket_carries_no_addresses() -> anyhow::Result<()> {
    use syncweb_core::folder::{FolderManager, SyncMode};
    use syncweb_core::node::iroh_node::{DiscoveryConfig, IrohNode, RelayMode};

    let directory = crate::test_utils::TestDirectory::new("syncweb-relay-id-ticket")?;
    let root = directory.path().join("node");
    let identity = syncweb_core::node::identity::IdentityManager::new(root.join("identity.key"))?;
    let mut config = RelayConfig::default();
    config.relay_urls = vec!["tcp://127.0.0.1:1".to_owned()];
    let relay = RelayManager::start(RelayManagerOptions::new(config, root.join("relay")), identity.node_id())?;
    let node = IrohNode::new_with_relay(
        identity,
        root.join("data"),
        RelayMode::None,
        crate::test_utils::empty_member_keys(),
        DiscoveryConfig::disabled(),
        Some(relay),
    )
    .await?;

    let manager = FolderManager::new(&node);
    let folder = manager.create(SyncMode::SendReceive).await?;
    let ticket = folder
        .ticket_with_options(true, iroh_docs::api::protocol::AddrInfoOptions::Id)
        .await?;
    anyhow::ensure!(
        ticket.nodes.iter().all(|n| n.addrs.is_empty()),
        "id-only ticket should carry no addresses: {ticket:?}"
    );

    node.stop().await?;
    Ok(())
}

/// Regression: a `ConnectRequest` sent over the existing registration
/// connection must be answered by a session invitation that the registration
/// reader turns into a session. (The old code opened a fresh, duplicate
/// registration connection, which real relays drop with `early eof`.)
#[tokio::test]
async fn outbound_connect_request_brokers_a_session() -> anyhow::Result<()> {
    let session_key = [7_u8; 32];
    let peer = [4_u8; 32];
    let mut mock = MockRelay::spawn(JoinPolicy::AcceptThenConnect { key: session_key }).await?;
    let relay_url = format!("tcp://{}", mock.control_addr);
    let (relay_manager, _dir) = manager(relay_url)?;

    receive_join_and_pong(&mut mock).await?;
    let node = iroh::SecretKey::generate().public();
    relay_manager.dial_peer_for_test(peer, node)?;
    let joined_key = mock.next_session_key().await?;
    anyhow::ensure!(joined_key == session_key);

    relay_manager.shutdown().await;
    Ok(())
}
