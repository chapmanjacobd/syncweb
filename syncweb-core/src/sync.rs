//! Synchronization primitives shared by the CLI and higher-level engines.

pub mod actor;
pub mod checkpoint;
pub mod conflict;
pub mod deleted;
pub mod engine;
pub mod intents;
pub mod lazy_fetch;
pub mod partial_fetch;
pub mod peer_tracker;
pub mod session;
pub mod sessions;
pub mod subscribe;

pub use actor::{Actor, ActorHandle, ActorPanic};
pub use conflict::{
    ConflictReport, ConflictVariant, MaterializeOutcome, MaterializeSummary, conflict_path, diff_path, list_conflicts,
    materialize_folder_content, select_winner,
};
pub use deleted::{DeletedInfo, DeletedTracker, PruneEvent};
pub use engine::{SyncEngine, TransferStats};
pub use intents::{IntentControl, IntentHandle, SyncCommand, SyncEvent};
pub use lazy_fetch::LazyFetch;
pub use partial_fetch::{
    BlobHealth, FetchCandidate, FetchFilter, FetchStrategy, HealthReport, TransferReport, TransferRequest,
    TransferScheduler, TransferSchedulerConfig, TransferSegment, fetch_blob_requests, fetch_selected_content,
    fetch_selected_content_with_cancel, materialize_selected_content, strategy_selects_only_local,
};
pub use peer_tracker::{
    DEFAULT_MAX_PEERS_PER_BLOB, EfficientPeerCache, EvictionStrategy, ObservationSource, PeerTracker,
    PeerTransferStats, ProviderObservation,
};
pub use session::SessionMode;
pub use sessions::{ActiveSession, cancel_session, is_active};
pub use subscribe::{AreaFilter, AreaOfInterest, SubscribeParams};
