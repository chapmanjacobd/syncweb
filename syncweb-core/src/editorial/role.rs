use serde::{Deserialize, Serialize};

/// Editorial role assigned to a participant.
///
/// These are orthogonal to the existing [`Capability`] sync roles
/// (Admin / Write / Read) — they gate content lifecycle transitions
/// rather than data-plane access.
///
/// [`Capability`]: crate::folder::syncweb_folder::Capability
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EditorialRole {
    /// Full control over the collection's editorial lifecycle.
    Owner,
    /// Can create versions, edit entries, and request reviews.
    Maintainer,
    /// Can propose edits and respond to review feedback but cannot
    /// approve or publish.
    Editor,
    /// Can approve, request changes, or reject proposed content.
    Reviewer,
    /// Can transition approved content to Published.
    Publisher,
    /// Read-only consumer of curated content — no editorial actions.
    #[default]
    Viewer,
}

impl EditorialRole {
    /// Whether a holder of this role may initiate new content versions.
    #[must_use]
    pub const fn can_initiate(self) -> bool {
        matches!(self, Self::Owner | Self::Maintainer | Self::Editor)
    }

    /// Whether a holder of this role may approve or reject review items.
    #[must_use]
    pub const fn can_review(self) -> bool {
        matches!(self, Self::Owner | Self::Reviewer)
    }

    /// Whether a holder of this role may publish approved content.
    #[must_use]
    pub const fn can_publish(self) -> bool {
        matches!(self, Self::Owner | Self::Publisher)
    }

    /// Whether a holder of this role may retract or archive published content.
    #[must_use]
    pub const fn can_withdraw(self) -> bool {
        matches!(self, Self::Owner | Self::Maintainer)
    }
}
