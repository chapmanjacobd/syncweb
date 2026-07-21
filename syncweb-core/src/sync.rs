//! Synchronization primitives shared by the CLI and higher-level engines.

pub mod actor;
pub mod deleted;
pub mod engine;
pub mod intents;
pub mod lazy_fetch;
pub mod partial_fetch;
pub mod peer_tracker;
pub mod session;
pub mod subscribe;

pub use actor::{Actor, ActorHandle};
pub use deleted::{DeletedInfo, DeletedTracker, PruneEvent};
pub use engine::{SyncEngine, TransferStats};
pub use intents::{IntentHandle, SyncCommand, SyncEvent};
pub use lazy_fetch::LazyFetch;
pub use partial_fetch::{BlobHealth, FetchCandidate, FetchFilter, FetchStrategy, HealthReport};
pub use peer_tracker::{EfficientPeerCache, EvictionStrategy, PeerTracker};
pub use session::SessionMode;
pub use subscribe::{AreaFilter, AreaOfInterest, SubscribeParams};
