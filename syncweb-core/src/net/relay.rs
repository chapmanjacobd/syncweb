//! Syncthing relay (BEP) client and the iroh custom transport that carries
//! QUIC traffic through it.
//!
//! The wire protocol is the "bep-relay" protocol spoken by `strelaysrv` and
//! Syncthing (upstream `lib/relay/protocol`). Its identities are X.509 client
//! certificates: a device ID is the SHA-256 of the certificate in DER form.
//! A device that wants to reach another asks the relay for a
//! [`Message::SessionInvitation`], then both peers tunnel raw bytes through a
//! plain TCP session the relay brokers between them.
//!
//! syncweb speaks this protocol as an [`iroh`] custom transport: the iroh
//! endpoint registers a local [`CustomAddr`] derived from our relay identity
//! (`BEP_TRANSPORT_ID` + 32-byte certificate hash), and QUIC datagrams are
//! framed over relay sessions. Because iroh treats custom transports as a
//! primary path (ahead of its own relay fallback), the Syncthing relay works
//! as a drop-in TCP carrier between syncweb peers, even where UDP and the n0
//! relay network are unreachable.

use std::time::Duration;

use serde::{Deserialize, Serialize};

pub(crate) mod certs;
pub mod manager;
pub mod pool;
pub mod protocol;
mod transport;
mod xdr;

pub use manager::{RelayManager, RelayManagerOptions, TestRelayReport};
pub use transport::{BEP_TRANSPORT_ID, SyncthingRelayTransport};

/// Which relays to reach out to and how to time out their handshakes.
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
