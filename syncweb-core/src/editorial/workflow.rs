use super::{EditorialRole, EditorialState};

/// Transition a collection from `current` to `target`, checking that `role` is
/// sufficient.
///
/// # Errors
///
/// Returns `Err` when the transition is not allowed for the given role.
pub fn assert_transition(
    current: Option<EditorialState>,
    target: EditorialState,
    role: EditorialRole,
) -> Result<EditorialState, TransitionError> {
    let source = current.unwrap_or_default();
    if !source.can_transition_to(target) {
        return Err(TransitionError::InvalidTransition { source, target });
    }
    if role_can_transition(role, source, target) {
        Ok(target)
    } else {
        Err(TransitionError::InsufficientRole { role, source, target })
    }
}

const fn role_can_transition(role: EditorialRole, source: EditorialState, target: EditorialState) -> bool {
    use EditorialState as S;
    match (source, target) {
        (S::Proposed | S::ChangesRequested | S::Retracted, S::Draft)
        | (S::Draft | S::ChangesRequested | S::Approved, S::Proposed) => role.can_initiate(),
        (S::Proposed, S::InReview) | (S::InReview, S::ChangesRequested | S::Approved) => role.can_review(),
        (S::Approved | S::Archived, S::Published) => role.can_publish(),
        (S::Published, S::Archived | S::Retracted) => role.can_withdraw(),
        _ => false,
    }
}

/// Error returned when a state transition is rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TransitionError {
    /// The state machine does not permit a direct jump.
    InvalidTransition {
        source: EditorialState,
        target: EditorialState,
    },
    /// The actor's role is insufficient for the requested transition.
    InsufficientRole {
        role: EditorialRole,
        source: EditorialState,
        target: EditorialState,
    },
}

impl core::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidTransition { source, target } => {
                write!(f, "invalid editorial transition from {source:?} to {target:?}")
            }
            Self::InsufficientRole { role, source, target } => {
                write!(f, "role {role:?} cannot transition {source:?} → {target:?}")
            }
        }
    }
}
