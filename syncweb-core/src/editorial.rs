//! Opt-in editorial workflow primitives for collections.
//!
//! Every type in this module is optional — collections that carry no
//! editorial metadata behave identically to the pre-editorial system.
//! Consumers opt into editorial flows by subscribing to named
//! [`Channel`]s over gossip.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use syncweb_core::editorial::{
//!     Channel, ContentType, EditorialRole, EditorialState,
//!     workflow::assert_transition,
//! };
//!
//! let curated = Channel::new("curated", Some("Editor-approved content"));
//! assert_eq!(curated.topic_seed(), "syncweb/catalog/curated/v1");
//!
//! assert!(assert_transition(
//!     Some(EditorialState::Draft),
//!     EditorialState::Proposed,
//!     EditorialRole::Maintainer,
//! ).is_ok());
//! ```
pub mod channel;
pub mod content_type;
pub mod role;
pub mod state;
pub mod workflow;

pub use channel::{Channel, ChannelBackend};
pub use content_type::ContentType;
pub use role::EditorialRole;
pub use state::EditorialState;
