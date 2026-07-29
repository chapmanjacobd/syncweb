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
}

impl SyncMode {
    #[must_use]
    pub const fn can_write_locally(self) -> bool {
        matches!(self, Self::SendReceive | Self::SendOnly)
    }

    #[must_use]
    pub const fn can_grant_write(self) -> bool {
        matches!(self, Self::SendReceive)
    }

    #[must_use]
    pub const fn can_receive(self) -> bool {
        !matches!(self, Self::SendOnly)
    }
}

impl fmt::Display for SyncMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::SendReceive => "sendreceive",
            Self::SendOnly => "sendonly",
            Self::ReceiveOnly => "receiveonly",
            Self::ReceiveEncrypted => "receiveencrypted",
        })
    }
}

impl FromStr for SyncMode {
    type Err = SyncwebError;

    fn from_str(value: &str) -> Result<Self> {
        // Normalize: lowercase, replace hyphens and underscores with
        // a single hyphen, strip leading/trailing separators.
        let mut normalized = String::with_capacity(value.len());
        let mut last_was_sep = true;
        for ch in value.chars().flat_map(char::to_lowercase) {
            if ch == '-' || ch == '_' {
                if !last_was_sep {
                    normalized.push('-');
                    last_was_sep = true;
                }
            } else {
                normalized.push(ch);
                last_was_sep = false;
            }
        }
        let trimmed = normalized.trim_end_matches('-');
        match trimmed {
            "send-receive" | "sendreceive" => Ok(Self::SendReceive),
            "send-only" | "sendonly" => Ok(Self::SendOnly),
            "receive-only" | "receiveonly" => Ok(Self::ReceiveOnly),
            "receive-encrypted" | "receiveencrypted" => Ok(Self::ReceiveEncrypted),
            _ => Err(SyncwebError::InvalidSyncMode(value.to_owned())),
        }
    }
}
