use std::time::Duration;

use syncweb_core::{
    net::{
        JoinRelayRequest, JoinSessionRequest, RelayConfig, RelayMessage, SessionInvitation,
        SyncthingRelayTransport, TransportFallback,
    },
    node::identity::{DeviceId, IdentityManager},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn device_id() -> DeviceId {
    let path = std::env::temp_dir().join(format!("syncweb-relay-{}", uuid::Uuid::new_v4()));
    let identity = IdentityManager::new(&path).expect("create identity");
    let device_id = DeviceId::from_node_id(identity.node_id());
    std::fs::remove_file(path).expect("remove identity");
    device_id
}

#[test]
fn relay_messages_round_trip() {
    let device_id = device_id();
    let messages = [
        RelayMessage::JoinRelayRequest(JoinRelayRequest { device_id }),
        RelayMessage::SessionInvitation(SessionInvitation {
            session_key: [9; 32],
            server_socket: true,
        }),
        RelayMessage::JoinSessionRequest(JoinSessionRequest {
            session_key: [3; 32],
            device_id,
        }),
        RelayMessage::ResponseSuccess,
        RelayMessage::ResponseNotFound,
        RelayMessage::RelayFull,
    ];
    for message in messages {
        assert_eq!(
            RelayMessage::decode(&message.encode()).expect("decode"),
            message
        );
    }
    assert!(RelayMessage::decode(&[1; 4]).is_err());
}

#[tokio::test]
async fn relay_transport_frames_packets() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept connection");
        let length = stream.read_u32().await.expect("read join length") as usize;
        let mut join = vec![0; length];
        stream.read_exact(&mut join).await.expect("read join");
        assert!(matches!(
            RelayMessage::decode(&join).expect("decode join"),
            RelayMessage::JoinRelayRequest(_)
        ));
        let length = stream.read_u32().await.expect("read packet length") as usize;
        let mut packet = vec![0; length];
        stream.read_exact(&mut packet).await.expect("read packet");
        stream
            .write_u32(packet.len() as u32)
            .await
            .expect("write length");
        stream.write_all(&packet).await.expect("write packet");
    });

    let transport = SyncthingRelayTransport::connect(
        format!("tcp://{address}"),
        device_id(),
        Duration::from_secs(1),
    )
    .await
    .expect("connect transport");
    transport.send_packet(b"packet").await.expect("send packet");
    assert_eq!(
        transport.recv_packet().await.expect("receive packet"),
        b"packet"
    );
    server.await.expect("server completes");
}

#[tokio::test]
async fn relay_fallback_uses_the_next_configured_relay() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let address = listener.local_addr().expect("listener address");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept connection");
        let length = stream.read_u32().await.expect("read join length") as usize;
        let mut join = vec![0; length];
        stream.read_exact(&mut join).await.expect("read join");
        assert!(matches!(
            RelayMessage::decode(&join).expect("decode join"),
            RelayMessage::JoinRelayRequest(_)
        ));
    });

    let fallback = TransportFallback::new(RelayConfig {
        relay_urls: vec![
            "tcp://relay.invalid:22270".to_owned(),
            format!("tcp://{address}"),
        ],
        timeout: Duration::from_secs(1),
        auto_fallback: true,
    });
    let transport = fallback
        .connect_relay(device_id())
        .await
        .expect("fallback relay connects");
    assert_eq!(transport.relay_url, format!("tcp://{address}"));
    server.await.expect("server completes");
}

#[tokio::test]
async fn relay_fallback_can_be_disabled() {
    let fallback = TransportFallback::new(RelayConfig {
        relay_urls: vec!["tcp://127.0.0.1:1".to_owned()],
        timeout: Duration::from_millis(10),
        auto_fallback: false,
    });
    let result = fallback.connect_relay(device_id()).await;
    match result {
        Ok(_) => panic!("disabled fallback must fail"),
        Err(error) => assert!(error.to_string().contains("disabled")),
    }
}
