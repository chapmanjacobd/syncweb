//! bep-relay protocol messages and their framing.
//!
//! Every message is a fixed 12-byte header (big-endian `magic`, `messageType`,
//! `messageLength`) followed by an XDR-encoded payload of at most
//! [`MAX_MESSAGE_LENGTH`] bytes. Long-lived relay *sessions* carry raw bytes
//! using the same framing (`u32` length prefix, no padding).
//!
//! Upstream reference: <https://github.com/syncthing/syncthing/tree/main/lib/relay/protocol>.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::{Result, SyncwebError};

use super::xdr::{Reader, push_bool, push_bytes, push_u16, push_u32};

/// The bep-relay ALPN identifier used for the TLS join connection.
pub const PROTOCOL_NAME: &[u8] = b"bep-relay";
/// XDR magic identifying a bep-relay header.
pub const MAGIC: u32 = 0x9E79_BC40;
/// Maximum payload size accepted by relay servers.
pub const MAX_MESSAGE_LENGTH: usize = 1024;
/// Safety cap for session data frames (QUIC datagrams are well below this).
pub const MAX_DATA_FRAME: usize = 8192;
/// Maximum length of the byte fields in connect/invitation messages.
pub const MAX_BYTE_FIELD: usize = 32;

const TYPE_PING: u8 = 0;
const TYPE_PONG: u8 = 1;
const TYPE_JOIN_RELAY_REQUEST: u8 = 2;
const TYPE_JOIN_SESSION_REQUEST: u8 = 3;
const TYPE_RESPONSE: u8 = 4;
const TYPE_CONNECT_REQUEST: u8 = 5;
const TYPE_SESSION_INVITATION: u8 = 6;
const TYPE_RELAY_FULL: u8 = 7;

/// One bep-relay control message.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Message {
    /// Periodic keepalive sent by either side.
    Ping,
    /// Keepalive acknowledgement.
    Pong,
    /// Join the relay registration with an optional access token.
    JoinRelayRequest { token: String },
    /// Propose to join an existing session with its shared key.
    JoinSessionRequest { key: Vec<u8> },
    /// Status reply to a previous request.
    Response { code: i32, message: String },
    /// Ask the relay to broker a session towards a device id.
    ConnectRequest { id: Vec<u8> },
    /// An invitation to a session, delivered to the dialed device on its
    /// registered connection and to the dialing device on the fresh
    /// connection that carried the connection request.
    SessionInvitation {
        from: Vec<u8>,
        key: Vec<u8>,
        address: Vec<u8>,
        port: u16,
        server_socket: bool,
    },
    /// The relay has no capacity left.
    RelayFull,
}

impl Message {
    /// Response code for a successful request.
    pub const RESPONSE_SUCCESS: i32 = 0;
    /// Response code: the dialed device is unknown to the relay.
    pub const RESPONSE_NOT_FOUND: i32 = 1;
    /// Response code: the device is already connected to the relay.
    pub const RESPONSE_ALREADY_CONNECTED: i32 = 2;
    /// Response code: the join token was rejected.
    pub const RESPONSE_WRONG_TOKEN: i32 = 3;
    /// Response code: an unexpected message arrived.
    pub const RESPONSE_UNEXPECTED: i32 = 100;

    const fn type_id(&self) -> u8 {
        match self {
            Self::Ping => TYPE_PING,
            Self::Pong => TYPE_PONG,
            Self::JoinRelayRequest { .. } => TYPE_JOIN_RELAY_REQUEST,
            Self::JoinSessionRequest { .. } => TYPE_JOIN_SESSION_REQUEST,
            Self::Response { .. } => TYPE_RESPONSE,
            Self::ConnectRequest { .. } => TYPE_CONNECT_REQUEST,
            Self::SessionInvitation { .. } => TYPE_SESSION_INVITATION,
            Self::RelayFull => TYPE_RELAY_FULL,
        }
    }

    /// Encode `self` into a complete wire frame (header + payload).
    ///
    /// # Errors
    ///
    /// Returns an error if the payload exceeds [`MAX_MESSAGE_LENGTH`].
    pub fn encode_frame(&self) -> Result<Vec<u8>> {
        let mut payload = Vec::new();
        match self {
            Self::Ping | Self::Pong | Self::RelayFull => {}
            Self::JoinRelayRequest { token } => {
                push_bytes(&mut payload, token.as_bytes()).map_err(decode_error)?;
            }
            Self::JoinSessionRequest { key } => {
                push_bytes(&mut payload, key).map_err(decode_error)?;
            }
            Self::Response { code, message } => {
                push_u32(&mut payload, u32::from_le_bytes(code.to_le_bytes()));
                push_bytes(&mut payload, message.as_bytes()).map_err(decode_error)?;
            }
            Self::ConnectRequest { id } => {
                push_bytes(&mut payload, id).map_err(decode_error)?;
            }
            Self::SessionInvitation {
                from,
                key,
                address,
                port,
                server_socket,
            } => {
                push_bytes(&mut payload, from).map_err(decode_error)?;
                push_bytes(&mut payload, key).map_err(decode_error)?;
                push_bytes(&mut payload, address).map_err(decode_error)?;
                push_u16(&mut payload, *port);
                push_bool(&mut payload, *server_socket);
            }
        }
        if payload.len() > MAX_MESSAGE_LENGTH {
            return Err(SyncwebError::RelayFrameTooLarge {
                max: MAX_MESSAGE_LENGTH,
            });
        }
        let mut frame = Vec::with_capacity(payload.len().saturating_add(12));
        push_u32(&mut frame, MAGIC);
        push_u32(&mut frame, u32::from(self.type_id()));
        push_u32(
            &mut frame,
            u32::try_from(payload.len())
                .map_err(|error| SyncwebError::RelayDecode(format!("payload length exceeds u32::MAX: {error}")))?,
        );
        frame.extend_from_slice(&payload);
        Ok(frame)
    }

    /// Decode a complete wire frame (header + payload) into a message.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame is malformed, truncated, or carries an
    /// unknown message type.
    pub fn decode_frame(frame: &[u8]) -> Result<Self> {
        let (rest, payload) = frame
            .split_at_checked(12)
            .ok_or_else(|| SyncwebError::RelayDecode("frame is shorter than the header".to_owned()))?;
        let header: [u8; 12] = rest
            .try_into()
            .map_err(|error| SyncwebError::RelayDecode(format!("frame header is not twelve bytes: {error}")))?;
        let (magic, message_type, length_words) = read_header_words(header);
        if magic != MAGIC {
            return Err(SyncwebError::RelayDecode(format!("bad frame magic {magic:#x}")));
        }
        let length = usize::try_from(length_words)
            .map_err(|error| SyncwebError::RelayDecode(format!("frame length exceeds usize: {error}")))?;
        if length > MAX_MESSAGE_LENGTH {
            return Err(SyncwebError::RelayFrameTooLarge {
                max: MAX_MESSAGE_LENGTH,
            });
        }
        if payload.len() != length {
            return Err(SyncwebError::RelayDecode(format!(
                "frame payload is {} bytes but the header announces {length}",
                payload.len()
            )));
        }
        Self::decode_payload(message_type, payload)
    }

    fn decode_payload(message_type: u8, payload: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(payload);
        let message = match message_type {
            TYPE_PING => Message::Ping,
            TYPE_PONG => Message::Pong,
            TYPE_RELAY_FULL => Message::RelayFull,
            TYPE_JOIN_RELAY_REQUEST => {
                let token = reader.take_bytes(MAX_MESSAGE_LENGTH).map_err(decode_error)?;
                Message::JoinRelayRequest {
                    token: utf8_field(token, "token")?,
                }
            }
            TYPE_JOIN_SESSION_REQUEST => Message::JoinSessionRequest {
                key: reader.take_bytes(MAX_BYTE_FIELD).map_err(decode_error)?,
            },
            TYPE_RESPONSE => {
                let code = i32::from_ne_bytes(reader.take_u32().map_err(decode_error)?.to_ne_bytes());
                let message = reader.take_bytes(MAX_MESSAGE_LENGTH).map_err(decode_error)?;
                Message::Response {
                    code,
                    message: utf8_field(message, "response message")?,
                }
            }
            TYPE_CONNECT_REQUEST => Message::ConnectRequest {
                id: reader.take_bytes(MAX_BYTE_FIELD).map_err(decode_error)?,
            },
            TYPE_SESSION_INVITATION => {
                let from = reader.take_bytes(MAX_BYTE_FIELD).map_err(decode_error)?;
                let key = reader.take_bytes(MAX_BYTE_FIELD).map_err(decode_error)?;
                let address = reader.take_bytes(MAX_BYTE_FIELD).map_err(decode_error)?;
                let port_words = reader.take_u32().map_err(decode_error)?;
                let port_bytes = port_words.to_be_bytes();
                let port = u16::from_be_bytes([port_bytes[2], port_bytes[3]]);
                let server_socket = reader.take_bool().map_err(decode_error)?;
                Message::SessionInvitation {
                    from,
                    key,
                    address,
                    port,
                    server_socket,
                }
            }
            other => {
                return Err(SyncwebError::RelayDecode(format!("unknown message type {other}")));
            }
        };
        reader.finish().map_err(decode_error)?;
        Ok(message)
    }
}

/// Wrap an encoding or decoding detail message in [`SyncwebError::RelayDecode`].
fn decode_error(detail: impl Into<String>) -> SyncwebError {
    SyncwebError::RelayDecode(detail.into())
}

fn utf8_field(bytes: Vec<u8>, field: &str) -> Result<String> {
    String::from_utf8(bytes).map_err(|error| SyncwebError::RelayDecode(format!("{field} is not UTF-8: {error}")))
}

/// Write `message` as a single length-prefixed frame to `writer`.
///
/// # Errors
///
/// Returns an error if encoding or the underlying write fails.
pub async fn write_message<W>(writer: &mut W, message: &Message) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let frame = message.encode_frame()?;
    writer.write_all(&frame).await?;
    writer.flush().await?;
    Ok(())
}

/// Read one length-prefixed message from `reader`.
///
/// # Errors
///
/// Returns an error on EOF or if the frame is malformed.
pub async fn read_message<R>(reader: &mut R) -> Result<Message>
where
    R: AsyncRead + Unpin,
{
    let mut header = [0_u8; 12];
    reader.read_exact(&mut header).await?;
    let (magic, message_type, length_words) = read_header_words(header);
    if magic != MAGIC {
        return Err(SyncwebError::RelayDecode(format!("bad frame magic {magic:#x}")));
    }
    let length = usize::try_from(length_words)
        .map_err(|error| SyncwebError::RelayDecode(format!("frame length exceeds usize: {error}")))?;
    if length > MAX_MESSAGE_LENGTH {
        return Err(SyncwebError::RelayFrameTooLarge {
            max: MAX_MESSAGE_LENGTH,
        });
    }
    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload).await?;
    Message::decode_payload(message_type, &payload)
}

/// Decode the three big-endian words of a 12-byte header.
fn read_header_words(header: [u8; 12]) -> (u32, u8, u32) {
    let magic = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    let type_words = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
    let length = u32::from_be_bytes([header[8], header[9], header[10], header[11]]);
    // The type word must fit in a single byte; default to `u8::MAX` for
    // anything larger so decoding reports an unknown type cleanly.
    let message_type = u8::try_from(type_words).unwrap_or(u8::MAX);
    (magic, message_type, length)
}

/// Write a session data frame (`u32` length prefix, unpadded).
///
/// # Errors
///
/// Returns an error if `data` is too large or the underlying write fails.
pub async fn write_data_frame<W>(writer: &mut W, data: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    if data.len() > MAX_DATA_FRAME {
        return Err(SyncwebError::RelayFrameTooLarge { max: MAX_DATA_FRAME });
    }
    let length = u32::try_from(data.len())
        .map_err(|error| SyncwebError::operation("data frame length exceeds u32::MAX", error))?;
    writer.write_all(&length.to_be_bytes()).await?;
    writer.write_all(data).await?;
    Ok(())
}

/// Read one session data frame.
///
/// Returns an [`std::io::Error`] on EOF, propagating the transport closure so
/// sessions can be re-established.
///
/// # Errors
///
/// Returns an error on EOF or if the frame is malformed or oversized.
pub async fn read_data_frame<R>(reader: &mut R) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut length_bytes = [0_u8; 4];
    reader.read_exact(&mut length_bytes).await?;
    let length = usize::try_from(u32::from_be_bytes(length_bytes))
        .map_err(|error| SyncwebError::operation("data frame length exceeds usize", error))?;
    if length > MAX_DATA_FRAME {
        return Err(SyncwebError::RelayFrameTooLarge { max: MAX_DATA_FRAME });
    }
    let mut data = vec![0_u8; length];
    reader.read_exact(&mut data).await?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_message_round_trips() {
        let messages = [
            Message::Ping,
            Message::Pong,
            Message::RelayFull,
            Message::JoinRelayRequest {
                token: "thunder".to_owned(),
            },
            Message::JoinSessionRequest { key: vec![1_u8; 32] },
            Message::Response {
                code: 0,
                message: "success".to_owned(),
            },
            Message::ConnectRequest { id: vec![7_u8; 32] },
            Message::SessionInvitation {
                from: vec![9_u8; 32],
                key: vec![3_u8; 32],
                address: vec![10, 0, 0, 1],
                port: 22_067,
                server_socket: true,
            },
        ];
        for message in messages {
            let frame = message.encode_frame().expect("encode");
            assert_eq!(Message::decode_frame(&frame).expect("decode"), message);
        }
    }

    #[test]
    fn session_invitation_padding_round_trips_odd_lengths() {
        let message = Message::SessionInvitation {
            from: vec![9_u8; 32],
            key: vec![1_u8; 32],
            address: vec![10, 0, 0],
            port: 1,
            server_socket: false,
        };
        let frame = message.encode_frame().expect("encode");
        assert_eq!(Message::decode_frame(&frame).expect("decode"), message);
    }

    #[test]
    fn bad_magic_is_rejected() {
        let mut frame = Message::Ping.encode_frame().expect("encode");
        if let Some(first) = frame.first_mut() {
            *first = 0;
        }
        assert!(Message::decode_frame(&frame).is_err());
    }

    #[test]
    fn truncated_frame_is_rejected() {
        let frame = Message::Ping.encode_frame().expect("encode");
        let truncated = frame.get(..10).unwrap_or(&frame);
        assert!(Message::decode_frame(truncated).is_err());
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let mut frame = Message::Ping.encode_frame().expect("encode");
        frame.push(0);
        assert!(Message::decode_frame(&frame).is_err());
    }

    #[tokio::test]
    async fn tokio_framing_round_trips() {
        let mut buffer = Vec::new();
        for message in [
            Message::JoinRelayRequest { token: "x".to_owned() },
            Message::ConnectRequest { id: vec![5_u8; 32] },
        ] {
            write_message(&mut buffer, &message).await.expect("write");
            assert_eq!(read_message(&mut &buffer[..]).await.expect("read"), message);
            buffer.clear();
        }
    }
}
