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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct JoinRelayRequest {
    pub device_id: DeviceId,
}

impl JoinRelayRequest {
    #[must_use]
    pub const fn new(device_id: DeviceId) -> Self {
        Self { device_id }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SessionInvitation {
    pub session_key: [u8; 32],
    pub server_socket: bool,
}

impl SessionInvitation {
    #[must_use]
    pub const fn new(session_key: [u8; 32], server_socket: bool) -> Self {
        Self {
            session_key,
            server_socket,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct JoinSessionRequest {
    pub session_key: [u8; 32],
    pub device_id: DeviceId,
}

impl JoinSessionRequest {
    #[must_use]
    pub const fn new(session_key: [u8; 32], device_id: DeviceId) -> Self {
        Self { session_key, device_id }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum RelayMessage {
    JoinRelayRequest(JoinRelayRequest),
    SessionInvitation(SessionInvitation),
    JoinSessionRequest(JoinSessionRequest),
    ResponseSuccess,
    ResponseNotFound,
    RelayFull,
}

impl RelayMessage {
    #[must_use]
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

    /// # Errors
    ///
    /// Returns an error if the bytes cannot be decoded into a relay message.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let (tag, body) = bytes.split_first().context("relay message is missing a type tag")?;
        match (*tag, body) {
            (1, device_id) if device_id.len() == 32 => Ok(Self::JoinRelayRequest(JoinRelayRequest {
                device_id: device_id_from_bytes(device_id)?,
            })),
            (2, [session_key @ .., server_socket]) if session_key.len() == 32 => {
                if *server_socket > 1 {
                    bail!("invalid session invitation socket flag");
                }
                Ok(Self::SessionInvitation(SessionInvitation {
                    session_key: session_key.try_into().unwrap_or([0; 32]),
                    server_socket: *server_socket == 1,
                }))
            }
            (3, body_bytes) if body_bytes.len() == 64 => {
                let session_key = body_bytes.get(..32).context("invalid body length")?;
                let device_id = body_bytes.get(32..).context("invalid body length")?;
                Ok(Self::JoinSessionRequest(JoinSessionRequest {
                    session_key: session_key.try_into().unwrap_or([0; 32]),
                    device_id: device_id_from_bytes(device_id)?,
                }))
            }
            (4, []) => Ok(Self::ResponseSuccess),
            (5, []) => Ok(Self::ResponseNotFound),
            (6, []) => Ok(Self::RelayFull),
            _ => bail!("invalid relay message encoding"),
        }
    }
}

fn device_id_from_bytes(bytes: &[u8]) -> Result<DeviceId> {
    let arr: [u8; 32] = bytes.try_into().context("relay device ID must be 32 bytes")?;
    Ok(DeviceId::from_node_id(PublicKey::from_bytes(&arr)?))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[non_exhaustive]
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

const fn default_auto_fallback() -> bool {
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
    /// # Errors
    ///
    /// Returns an error if the connection to the relay fails.
    pub async fn connect(
        relay_url_in: impl Into<String>,
        device_id: DeviceId,
        timeout_duration: Duration,
    ) -> Result<Self> {
        let relay_url = relay_url_in.into();
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
            .send_message(&RelayMessage::JoinRelayRequest(JoinRelayRequest { device_id }))
            .await?;
        Ok(transport)
    }

    /// # Errors
    ///
    /// Returns an error if the relay message fails to send.
    pub async fn send_message(&self, message: &RelayMessage) -> Result<()> {
        self.write_frame(&message.encode()).await
    }

    /// # Errors
    ///
    /// Returns an error if receiving a relay message fails.
    pub async fn recv_message(&self) -> Result<RelayMessage> {
        RelayMessage::decode(&self.read_frame().await?)
    }

    /// # Errors
    ///
    /// Returns an error if the packet fails to send.
    pub async fn send_packet(&self, packet: &[u8]) -> Result<()> {
        self.write_frame(packet).await
    }

    /// # Errors
    ///
    /// Returns an error if receiving the packet fails.
    pub async fn recv_packet(&self) -> Result<Vec<u8>> {
        self.read_frame().await
    }

    async fn write_frame(&self, payload: &[u8]) -> Result<()> {
        if payload.len() > MAX_FRAME_SIZE {
            bail!("relay frame exceeds {MAX_FRAME_SIZE} byte limit");
        }
        let len = u32::try_from(payload.len()).context("payload length exceeds u32::MAX")?;
        let mut stream = self.stream.lock().await;
        stream.write_u32(len).await?;
        stream.write_all(payload).await?;
        stream.flush().await?;
        drop(stream);
        Ok(())
    }

    async fn read_frame(&self) -> Result<Vec<u8>> {
        let mut stream = self.stream.lock().await;
        let length = usize::try_from(stream.read_u32().await?).context("length exceeds usize::MAX")?;
        if length > MAX_FRAME_SIZE {
            bail!("relay frame exceeds {MAX_FRAME_SIZE} byte limit");
        }
        let mut payload = vec![0; length];
        stream.read_exact(&mut payload).await?;
        drop(stream);
        Ok(payload)
    }
}

#[derive(Clone, Debug, Default)]
pub struct TransportFallback {
    config: RelayConfig,
}

impl TransportFallback {
    #[must_use]
    pub const fn new(config: RelayConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub const fn config(&self) -> &RelayConfig {
        &self.config
    }

    /// # Errors
    ///
    /// Returns an error if connecting to the device via relay fails.
    pub async fn connect_relay(&self, device_id: DeviceId) -> Result<SyncthingRelayTransport> {
        if !self.config.auto_fallback {
            bail!("Syncthing relay fallback is disabled");
        }
        let mut failures = Vec::new();
        for relay_url in &self.config.relay_urls {
            match SyncthingRelayTransport::connect(relay_url, device_id, self.config.timeout).await {
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
