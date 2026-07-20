use std::time::Duration;

use anyhow::Context;
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
        RelayMessage::SessionInvitation(SessionInvitation::new([9_u8; 32], true)),
        RelayMessage::JoinSessionRequest(JoinSessionRequest::new([3_u8; 32], id)),
        RelayMessage::ResponseSuccess,
        RelayMessage::ResponseNotFound,
        RelayMessage::RelayFull,
    ];
    for message in messages {
        anyhow::ensure!(RelayMessage::decode(&message.encode())? == message);
    }
    anyhow::ensure!(RelayMessage::decode(&[1_u8; 4]).is_err());
    Ok(())
}

#[tokio::test]
async fn relay_transport_frames_packets() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await?;
        let join_length = usize::try_from(stream.read_u32().await?)?;
        let mut join = vec![0_u8; join_length];
        stream.read_exact(&mut join).await?;
        anyhow::ensure!(matches!(
            RelayMessage::decode(&join)?,
            RelayMessage::JoinRelayRequest(_)
        ));
        let packet_length = usize::try_from(stream.read_u32().await?)?;
        let mut packet = vec![0_u8; packet_length];
        stream.read_exact(&mut packet).await?;
        stream.write_u32(u32::try_from(packet.len())?).await?;
        stream.write_all(&packet).await?;
        anyhow::Ok(())
    });

    let transport =
        SyncthingRelayTransport::connect(format!("tcp://{address}"), device_id()?, Duration::from_secs(1)).await?;
    transport.send_packet(b"packet").await?;
    anyhow::ensure!(transport.recv_packet().await? == b"packet");
    server.await??;
    Ok(())
}

#[tokio::test]
async fn relay_fallback_uses_the_next_configured_relay() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await?;
        let length = usize::try_from(stream.read_u32().await?)?;
        let mut join = vec![0_u8; length];
        stream.read_exact(&mut join).await?;
        anyhow::ensure!(matches!(
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
    anyhow::ensure!(transport.relay_url == format!("tcp://{address}"));
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
    anyhow::ensure!(result.is_err());
    if let Err(error) = result {
        anyhow::ensure!(error.to_string().contains("disabled"));
    }
    Ok(())
}

async fn write_relay_message(stream: &mut tokio::net::TcpStream, msg: &RelayMessage) -> anyhow::Result<()> {
    let bytes = msg.encode();
    stream.write_u32(u32::try_from(bytes.len())?).await?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_relay_message(stream: &mut tokio::net::TcpStream) -> anyhow::Result<RelayMessage> {
    let len = usize::try_from(stream.read_u32().await?)?;
    let mut buf = vec![0_u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(RelayMessage::decode(&buf)?)
}

#[tokio::test]
async fn test_quic_over_tcp() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;

    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await?;
        let join = read_relay_message(&mut stream).await?;
        anyhow::ensure!(matches!(join, RelayMessage::JoinRelayRequest(_)));

        write_relay_message(
            &mut stream,
            &RelayMessage::SessionInvitation(SessionInvitation::new([42_u8; 32], false)),
        )
        .await?;

        let join_session = read_relay_message(&mut stream).await?;
        anyhow::ensure!(matches!(join_session, RelayMessage::JoinSessionRequest(_)));

        write_relay_message(&mut stream, &RelayMessage::ResponseSuccess).await?;

        for i in 0_u32..20 {
            let pkt = format!("stream-{i}");
            stream.write_u32(u32::try_from(pkt.len())?).await?;
            stream.write_all(pkt.as_bytes()).await?;
        }
        stream.flush().await?;

        for _ in 0..5 {
            let len = usize::try_from(stream.read_u32().await?)?;
            let mut buf = vec![0_u8; len];
            stream.read_exact(&mut buf).await?;
            anyhow::ensure!(buf.starts_with(b"client-"));
        }
        anyhow::Ok(())
    });

    let transport =
        SyncthingRelayTransport::connect(format!("tcp://{address}"), device_id()?, Duration::from_secs(2)).await?;

    let join_invitation = transport.recv_message().await?;
    let session_key = match join_invitation {
        RelayMessage::SessionInvitation(inv) => inv.session_key,
        RelayMessage::JoinSessionRequest(_)
        | RelayMessage::ResponseSuccess
        | RelayMessage::ResponseNotFound
        | RelayMessage::RelayFull
        | _ => {
            anyhow::bail!("expected SessionInvitation, got {join_invitation:?}")
        }
    };

    transport
        .send_message(&RelayMessage::JoinSessionRequest(JoinSessionRequest::new(
            session_key,
            device_id()?,
        )))
        .await?;

    let response = transport.recv_message().await?;
    anyhow::ensure!(matches!(response, RelayMessage::ResponseSuccess));

    for i in 0_u32..20 {
        let pkt = transport.recv_packet().await?;
        anyhow::ensure!(pkt == format!("stream-{i}").as_bytes());
    }

    for i in 0_u32..5 {
        transport.send_packet(format!("client-{i}").as_bytes()).await?;
    }

    server.await??;
    Ok(())
}

#[tokio::test]
async fn test_relay_connection() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let relay_addr = listener.local_addr()?;

    let server = tokio::spawn(async move {
        let (mut stream_a, _) = listener.accept().await?;
        let join_a = read_relay_message(&mut stream_a).await?;
        let _device_a = match join_a {
            RelayMessage::JoinRelayRequest(req) => req.device_id,
            RelayMessage::JoinSessionRequest(_)
            | RelayMessage::ResponseSuccess
            | RelayMessage::ResponseNotFound
            | RelayMessage::RelayFull
            | _ => {
                anyhow::bail!("expected JoinRelayRequest from A, got {join_a:?}")
            }
        };

        let (mut stream_b, _) = listener.accept().await?;
        let join_b = read_relay_message(&mut stream_b).await?;
        let _device_b = match join_b {
            RelayMessage::JoinRelayRequest(req) => req.device_id,
            RelayMessage::JoinSessionRequest(_)
            | RelayMessage::ResponseSuccess
            | RelayMessage::ResponseNotFound
            | RelayMessage::RelayFull
            | _ => {
                anyhow::bail!("expected JoinRelayRequest from B, got {join_b:?}")
            }
        };

        let session_key: [u8; 32] = [7_u8; 32];

        write_relay_message(
            &mut stream_a,
            &RelayMessage::SessionInvitation(SessionInvitation::new(session_key, false)),
        )
        .await?;
        write_relay_message(
            &mut stream_b,
            &RelayMessage::SessionInvitation(SessionInvitation::new(session_key, true)),
        )
        .await?;

        let js_a = read_relay_message(&mut stream_a).await?;
        anyhow::ensure!(matches!(js_a, RelayMessage::JoinSessionRequest(_)));
        let js_b = read_relay_message(&mut stream_b).await?;
        anyhow::ensure!(matches!(js_b, RelayMessage::JoinSessionRequest(_)));

        write_relay_message(&mut stream_a, &RelayMessage::ResponseSuccess).await?;
        write_relay_message(&mut stream_b, &RelayMessage::ResponseSuccess).await?;

        let mut buf_a = [0_u8; 1024];
        let mut buf_b = [0_u8; 1024];
        loop {
            tokio::select! {
                result = stream_a.read(&mut buf_a) => {
                    let n = result?;
                    if n == 0 { break; }
                    let slice = buf_a.get(..n).context("slice read_a")?;
                    stream_b.write_all(slice).await?;
                }
                result = stream_b.read(&mut buf_b) => {
                    let n = result?;
                    if n == 0 { break; }
                    let slice = buf_b.get(..n).context("slice read_b")?;
                    stream_a.write_all(slice).await?;
                }
            }
        }
        anyhow::Ok(())
    });

    let id_a = device_id()?;
    let id_b = device_id()?;
    let addr = format!("tcp://{relay_addr}");

    let client_a = tokio::spawn({
        let addr_for_a = addr.clone();
        async move {
            let transport = SyncthingRelayTransport::connect(addr_for_a, id_a, Duration::from_secs(5)).await?;
            let invitation = transport.recv_message().await?;
            let session_key = match invitation {
                RelayMessage::SessionInvitation(inv) => inv.session_key,
                RelayMessage::JoinSessionRequest(_)
                | RelayMessage::ResponseSuccess
                | RelayMessage::ResponseNotFound
                | RelayMessage::RelayFull
                | _ => {
                    anyhow::bail!("expected invitation for A")
                }
            };
            transport
                .send_message(&RelayMessage::JoinSessionRequest(JoinSessionRequest::new(
                    session_key,
                    id_a,
                )))
                .await?;
            anyhow::ensure!(matches!(transport.recv_message().await?, RelayMessage::ResponseSuccess));
            transport.send_packet(b"hello from A").await?;
            let received = transport.recv_packet().await?;
            anyhow::ensure!(received == b"hello from B");
            anyhow::Ok(())
        }
    });

    let client_b = tokio::spawn(async move {
        let transport = SyncthingRelayTransport::connect(addr, id_b, Duration::from_secs(5)).await?;
        let invitation = transport.recv_message().await?;
        let session_key = match invitation {
            RelayMessage::SessionInvitation(inv) => inv.session_key,
            RelayMessage::JoinSessionRequest(_)
            | RelayMessage::ResponseSuccess
            | RelayMessage::ResponseNotFound
            | RelayMessage::RelayFull
            | _ => {
                anyhow::bail!("expected invitation for B")
            }
        };
        transport
            .send_message(&RelayMessage::JoinSessionRequest(JoinSessionRequest::new(
                session_key,
                id_b,
            )))
            .await?;
        anyhow::ensure!(matches!(transport.recv_message().await?, RelayMessage::ResponseSuccess));
        let received = transport.recv_packet().await?;
        anyhow::ensure!(received == b"hello from A");
        transport.send_packet(b"hello from B").await?;
        anyhow::Ok(())
    });

    client_a.await??;
    client_b.await??;
    server.await??;
    Ok(())
}

#[tokio::test]
async fn test_relay_fallback_chain() -> anyhow::Result<()> {
    let dead_listener_1 = TcpListener::bind("127.0.0.1:0").await?;
    let dead_addr_1 = dead_listener_1.local_addr()?;
    drop(dead_listener_1);

    let dead_listener_2 = TcpListener::bind("127.0.0.1:0").await?;
    let dead_addr_2 = dead_listener_2.local_addr()?;
    drop(dead_listener_2);

    let working_listener = TcpListener::bind("127.0.0.1:0").await?;
    let working_addr = working_listener.local_addr()?;
    let server = tokio::spawn(async move {
        let (mut stream, _) = working_listener.accept().await?;
        let join = read_relay_message(&mut stream).await?;
        anyhow::ensure!(matches!(join, RelayMessage::JoinRelayRequest(_)));
        write_relay_message(
            &mut stream,
            &RelayMessage::SessionInvitation(SessionInvitation::new([55_u8; 32], false)),
        )
        .await?;
        let join_session = read_relay_message(&mut stream).await?;
        anyhow::ensure!(matches!(join_session, RelayMessage::JoinSessionRequest(_)));
        write_relay_message(&mut stream, &RelayMessage::ResponseSuccess).await?;
        anyhow::Ok(())
    });

    let mut config = RelayConfig::default();
    config.relay_urls = vec![
        format!("tcp://{dead_addr_1}"),
        format!("tcp://{dead_addr_2}"),
        format!("tcp://{working_addr}"),
    ];
    config.timeout = Duration::from_millis(200);
    config.auto_fallback = true;

    let fallback = TransportFallback::new(config);
    let transport = fallback.connect_relay(device_id()?).await?;
    anyhow::ensure!(transport.relay_url == format!("tcp://{working_addr}"));

    let invitation = transport.recv_message().await?;
    let session_key = match invitation {
        RelayMessage::SessionInvitation(inv) => inv.session_key,
        RelayMessage::JoinSessionRequest(_)
        | RelayMessage::ResponseSuccess
        | RelayMessage::ResponseNotFound
        | RelayMessage::RelayFull
        | _ => {
            anyhow::bail!("expected invitation, got {invitation:?}")
        }
    };
    anyhow::ensure!(session_key == [55_u8; 32]);

    transport
        .send_message(&RelayMessage::JoinSessionRequest(JoinSessionRequest::new(
            session_key,
            device_id()?,
        )))
        .await?;
    anyhow::ensure!(matches!(transport.recv_message().await?, RelayMessage::ResponseSuccess));

    drop(transport);
    server.await??;
    Ok(())
}
