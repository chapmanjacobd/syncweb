use serde::{Deserialize, Serialize};

/// Content lifecycle stage.
///
/// The default is [`Published`](Self::Published) so that collections that
/// do not opt into editorial flows behave identically to the pre-editorial
/// behaviour.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EditorialState {
    /// Local-only, not yet shared.
    Draft,
    /// Shared for review, not yet approved.
    Proposed,
    /// Actively being reviewed.
    InReview,
    /// Reviewers have requested changes.
    ChangesRequested,
    /// Reviewers have approved.
    Approved,
    /// Publicly available — pinned and announced.
    /// This is the default state (backward-compatible).
    #[default]
    Published,
    /// Deprecated but kept for reference.
    Archived,
    /// Published then pulled (tombstone entry remains).
    Retracted,
}

impl EditorialState {
    /// Whether the content is considered finalised and visible to general
    /// consumers.
    #[must_use]
    pub const fn is_published(self) -> bool {
        matches!(self, Self::Published)
    }

    /// Whether the content is available for public consumption (published or
    /// archived).
    #[must_use]
    pub const fn is_public(self) -> bool {
        matches!(self, Self::Published | Self::Archived)
    }

    /// Whether the state is part of the pre-publication review pipeline.
    #[must_use]
    pub const fn is_pre_publication(self) -> bool {
        matches!(
            self,
            Self::Draft | Self::Proposed | Self::InReview | Self::ChangesRequested | Self::Approved
        )
    }

    /// Whether the content has been withdrawn (retracted).
    #[must_use]
    pub const fn is_withdrawn(self) -> bool {
        matches!(self, Self::Retracted)
    }

    /// Check whether a transition from `self` to `target` is a valid editorial
    /// workflow step.
    ///
    /// Content that enters the editorial pipeline at `Draft` must go through
    /// review (`Proposed → InReview → Approved`) before being published.
    /// Retracted content must complete the full review cycle again before
    /// re-publishing — `Draft → Published` is not permitted.
    #[must_use]
    pub const fn can_transition_to(self, target: Self) -> bool {
        use EditorialState as S;
        matches!(
            (self, target),
            (S::Draft | S::Approved, S::Proposed)
                | (S::Proposed, S::InReview | S::Draft)
                | (S::InReview, S::ChangesRequested | S::Approved)
                | (S::ChangesRequested, S::Proposed | S::Draft)
                | (S::Approved | S::Archived, S::Published)
                | (S::Published, S::Archived | S::Retracted)
                | (S::Retracted, S::Draft)
        )
    }
}
