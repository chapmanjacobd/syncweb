use std::time::Duration;

use syncweb_core::{
    net::{
        JoinRelayRequest, JoinSessionRequest, RelayConfig, RelayMessage, SessionInvitation, SyncthingRelayTransport,
        TransportFallback,
    },
    node::identity::{DeviceId, IdentityManager},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

fn device_id() -> anyhow::Result<DeviceId> {
    let path = std::env::temp_dir().join(format!("syncweb-relay-{}", uuid::Uuid::new_v4()));
    let identity = IdentityManager::new(&path)?;
    let device_id = DeviceId::from_node_id(identity.node_id());
    std::fs::remove_file(path)?;
    Ok(device_id)
}

#[test]
fn relay_messages_round_trip() -> anyhow::Result<()> {
    let id = device_id()?;
    let messages = [
        RelayMessage::JoinRelayRequest(JoinRelayRequest::new(id)),
        RelayMessage::SessionInvitation(SessionInvitation::new([9; 32], true)),
        RelayMessage::JoinSessionRequest(JoinSessionRequest::new([3; 32], id)),
        RelayMessage::ResponseSuccess,
        RelayMessage::ResponseNotFound,
        RelayMessage::RelayFull,
    ];
    for message in messages {
        assert_eq!(RelayMessage::decode(&message.encode())?, message);
    }
    assert!(RelayMessage::decode(&[1; 4]).is_err());
    Ok(())
}

#[tokio::test]
async fn relay_transport_frames_packets() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await?;
        let join_length = usize::from(stream.read_u32().await?);
        let mut join = vec![0; join_length];
        stream.read_exact(&mut join).await?;
        assert!(matches!(
            RelayMessage::decode(&join)?,
            RelayMessage::JoinRelayRequest(_)
        ));
        let packet_length = usize::from(stream.read_u32().await?);
        let mut packet = vec![0; packet_length];
        stream.read_exact(&mut packet).await?;
        stream.write_u32(u32::try_from(packet.len())?).await?;
        stream.write_all(&packet).await?;
        anyhow::Ok(())
    });

    let transport =
        SyncthingRelayTransport::connect(format!("tcp://{address}"), device_id()?, Duration::from_secs(1)).await?;
    transport.send_packet(b"packet").await?;
    assert_eq!(transport.recv_packet().await?, b"packet");
    server.await??;
    Ok(())
}

#[tokio::test]
async fn relay_fallback_uses_the_next_configured_relay() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await?;
        let length = usize::from(stream.read_u32().await?);
        let mut join = vec![0; length];
        stream.read_exact(&mut join).await?;
        assert!(matches!(
            RelayMessage::decode(&join)?,
            RelayMessage::JoinRelayRequest(_)
        ));
        anyhow::Ok(())
    });

    let mut config = RelayConfig::default();
    config.relay_urls = vec!["tcp://relay.invalid:22270".to_owned(), format!("tcp://{address}")];
    config.timeout = Duration::from_secs(1);
    config.auto_fallback = true;
    let fallback = TransportFallback::new(config);
    let transport = fallback.connect_relay(device_id()?).await?;
    assert_eq!(transport.relay_url, format!("tcp://{address}"));
    server.await??;
    Ok(())
}

#[tokio::test]
async fn relay_fallback_can_be_disabled() -> anyhow::Result<()> {
    let mut config = RelayConfig::default();
    config.relay_urls = vec!["tcp://127.0.0.1:1".to_owned()];
    config.timeout = Duration::from_millis(10);
    config.auto_fallback = false;
    let fallback = TransportFallback::new(config);
    let result = fallback.connect_relay(device_id()?).await;
    assert!(result.is_err());
    if let Err(error) = result {
        assert!(error.to_string().contains("disabled"));
    }
    Ok(())
}
