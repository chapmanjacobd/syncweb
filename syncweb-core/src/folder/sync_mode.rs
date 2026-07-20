use std::{fmt, str::FromStr};

use crate::error::{Result, SyncwebError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SyncMode {
    #[default]
    SendReceive,
    SendOnly,
    ReceiveOnly,
    ReceiveEncrypted,
    PublicReadOnly,
}

impl SyncMode {
    #[must_use]
    pub const fn can_write(self) -> bool {
        matches!(self, Self::SendReceive | Self::SendOnly)
    }

    #[must_use]
    pub const fn can_receive(self) -> bool {
        !matches!(self, Self::SendOnly)
    }

    #[must_use]
    pub const fn is_public(self) -> bool {
        matches!(self, Self::PublicReadOnly)
    }
}

impl fmt::Display for SyncMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SendReceive => "sendreceive",
            Self::SendOnly => "sendonly",
            Self::ReceiveOnly => "receiveonly",
            Self::ReceiveEncrypted => "receiveencrypted",
            Self::PublicReadOnly => "publicreadonly",
        })
    }
}

impl FromStr for SyncMode {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
            "sendreceive" => Ok(Self::SendReceive),
            "sendonly" => Ok(Self::SendOnly),
            "receiveonly" => Ok(Self::ReceiveOnly),
            "receiveencrypted" => Ok(Self::ReceiveEncrypted),
            "publicreadonly" => Ok(Self::PublicReadOnly),
            _ => Err(SyncwebError::InvalidSyncMode(value.to_owned())),
        }
    }
}
