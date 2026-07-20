use std::time::Duration;

use anyhow::{Context, Result, bail};
use iroh::PublicKey;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
    time::timeout,
};

use crate::node::identity::DeviceId;

const MAX_FRAME_SIZE: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JoinRelayRequest {
    pub device_id: DeviceId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionInvitation {
    pub session_key: [u8; 32],
    pub server_socket: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JoinSessionRequest {
    pub session_key: [u8; 32],
    pub device_id: DeviceId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelayMessage {
    JoinRelayRequest(JoinRelayRequest),
    SessionInvitation(SessionInvitation),
    JoinSessionRequest(JoinSessionRequest),
    ResponseSuccess,
    ResponseNotFound,
    RelayFull,
}

impl RelayMessage {
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(66);
        match self {
            Self::JoinRelayRequest(request) => {
                bytes.push(1);
                bytes.extend_from_slice(request.device_id.node_id().as_bytes());
            }
            Self::SessionInvitation(invitation) => {
                bytes.push(2);
                bytes.extend_from_slice(&invitation.session_key);
                bytes.push(u8::from(invitation.server_socket));
            }
            Self::JoinSessionRequest(request) => {
                bytes.push(3);
                bytes.extend_from_slice(&request.session_key);
                bytes.extend_from_slice(request.device_id.node_id().as_bytes());
            }
            Self::ResponseSuccess => bytes.push(4),
            Self::ResponseNotFound => bytes.push(5),
            Self::RelayFull => bytes.push(6),
        }
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let (tag, body) = bytes
            .split_first()
            .context("relay message is missing a type tag")?;
        match (*tag, body) {
            (1, device_id) if device_id.len() == 32 => {
                Ok(Self::JoinRelayRequest(JoinRelayRequest {
                    device_id: device_id_from_bytes(device_id)?,
                }))
            }
            (2, [session_key @ .., server_socket]) if session_key.len() == 32 => {
                if *server_socket > 1 {
                    bail!("invalid session invitation socket flag");
                }
                Ok(Self::SessionInvitation(SessionInvitation {
                    session_key: session_key
                        .try_into()
                        .expect("length checked before session key conversion"),
                    server_socket: *server_socket == 1,
                }))
            }
            (3, body) if body.len() == 64 => Ok(Self::JoinSessionRequest(JoinSessionRequest {
                session_key: body[..32]
                    .try_into()
                    .expect("length checked before session key conversion"),
                device_id: device_id_from_bytes(&body[32..])?,
            })),
            (4, []) => Ok(Self::ResponseSuccess),
            (5, []) => Ok(Self::ResponseNotFound),
            (6, []) => Ok(Self::RelayFull),
            _ => bail!("invalid relay message encoding"),
        }
    }
}

fn device_id_from_bytes(bytes: &[u8]) -> Result<DeviceId> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("relay device ID must be 32 bytes"))?;
    Ok(DeviceId::from_node_id(PublicKey::from_bytes(&bytes)?))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelayConfig {
    #[serde(default)]
    pub relay_urls: Vec<String>,
    #[serde(with = "duration_seconds")]
    pub timeout: Duration,
    #[serde(default = "default_auto_fallback")]
    pub auto_fallback: bool,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            relay_urls: Vec::new(),
            timeout: Duration::from_secs(10),
            auto_fallback: true,
        }
    }
}

fn default_auto_fallback() -> bool {
    true
}

mod duration_seconds {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let seconds = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(seconds))
    }
}

/// A length-delimited byte tunnel negotiated using Syncthing relay v1 messages.
///
/// The TLS/BEP authentication layer is intentionally left to the caller that owns
/// Syncthing-compatible credentials; this type only handles the relay's typed
/// control messages and the subsequent byte tunnel.
pub struct SyncthingRelayTransport {
    stream: Mutex<TcpStream>,
    pub relay_url: String,
    pub session_key: [u8; 32],
}

impl SyncthingRelayTransport {
    pub async fn connect(
        relay_url: impl Into<String>,
        device_id: DeviceId,
        timeout_duration: Duration,
    ) -> Result<Self> {
        let relay_url = relay_url.into();
        let address = relay_address(&relay_url)?;
        let stream = timeout(timeout_duration, TcpStream::connect(&address))
            .await
            .context("relay connection timed out")?
            .with_context(|| format!("failed to connect to relay {address}"))?;
        let transport = Self {
            stream: Mutex::new(stream),
            relay_url,
            session_key: [0; 32],
        };
        transport
            .send_message(&RelayMessage::JoinRelayRequest(JoinRelayRequest {
                device_id,
            }))
            .await?;
        Ok(transport)
    }

    pub async fn send_message(&self, message: &RelayMessage) -> Result<()> {
        self.write_frame(&message.encode()).await
    }

    pub async fn recv_message(&self) -> Result<RelayMessage> {
        RelayMessage::decode(&self.read_frame().await?)
    }

    pub async fn send_packet(&self, packet: &[u8]) -> Result<()> {
        self.write_frame(packet).await
    }

    pub async fn recv_packet(&self) -> Result<Vec<u8>> {
        self.read_frame().await
    }

    async fn write_frame(&self, payload: &[u8]) -> Result<()> {
        if payload.len() > MAX_FRAME_SIZE {
            bail!("relay frame exceeds {MAX_FRAME_SIZE} byte limit");
        }
        let mut stream = self.stream.lock().await;
        stream.write_u32(payload.len() as u32).await?;
        stream.write_all(payload).await?;
        stream.flush().await?;
        Ok(())
    }

    async fn read_frame(&self) -> Result<Vec<u8>> {
        let mut stream = self.stream.lock().await;
        let length = stream.read_u32().await? as usize;
        if length > MAX_FRAME_SIZE {
            bail!("relay frame exceeds {MAX_FRAME_SIZE} byte limit");
        }
        let mut payload = vec![0; length];
        stream.read_exact(&mut payload).await?;
        Ok(payload)
    }
}

#[derive(Clone, Debug, Default)]
pub struct TransportFallback {
    config: RelayConfig,
}

impl TransportFallback {
    pub fn new(config: RelayConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &RelayConfig {
        &self.config
    }

    pub async fn connect_relay(&self, device_id: DeviceId) -> Result<SyncthingRelayTransport> {
        if !self.config.auto_fallback {
            bail!("Syncthing relay fallback is disabled");
        }
        let mut failures = Vec::new();
        for relay_url in &self.config.relay_urls {
            match SyncthingRelayTransport::connect(relay_url, device_id, self.config.timeout).await
            {
                Ok(transport) => return Ok(transport),
                Err(error) => failures.push(format!("{relay_url}: {error:#}")),
            }
        }
        bail!(
            "no Syncthing relay is reachable{}",
            if failures.is_empty() {
                ": no relay URLs configured".to_owned()
            } else {
                format!(": {}", failures.join("; "))
            }
        )
    }
}

fn relay_address(relay_url: &str) -> Result<String> {
    let address = relay_url
        .strip_prefix("tcp://")
        .ok_or_else(|| anyhow::anyhow!("relay URL must use tcp://"))?;
    if address.is_empty() || address.contains('/') {
        bail!("relay URL must contain a host and port");
    }
    if address.parse::<std::net::SocketAddr>().is_err() && !address.contains(':') {
        bail!("relay URL must contain a port");
    }
    Ok(address.to_owned())
}
