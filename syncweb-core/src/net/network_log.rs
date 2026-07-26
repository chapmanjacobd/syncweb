use std::sync::Arc;

use crate::{
    Result,
    storage::stats_db::{NetworkEventRecord, StatsDatabase, SyncSessionRecord},
};

/// Events that can be recorded in the network event log.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum NetworkEventType {
    PeerJoined,
    PeerLeft,
    SyncStarted,
    SyncFinished,
    RelayConnected,
    RelayDisconnected,
    RelayFailed,
    TopicSubscribed,
    TopicUnsubscribed,
    MemberAdded,
    MemberRemoved,
    FolderAdded,
    FolderRemoved,
    TicketCreated,
    TicketAccepted,
    Kicked,
}

impl NetworkEventType {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::PeerJoined => "peer_joined",
            Self::PeerLeft => "peer_left",
            Self::SyncStarted => "sync_started",
            Self::SyncFinished => "sync_finished",
            Self::RelayConnected => "relay_connected",
            Self::RelayDisconnected => "relay_disconnected",
            Self::RelayFailed => "relay_failed",
            Self::TopicSubscribed => "topic_subscribed",
            Self::TopicUnsubscribed => "topic_unsubscribed",
            Self::MemberAdded => "member_added",
            Self::MemberRemoved => "member_removed",
            Self::FolderAdded => "folder_added",
            Self::FolderRemoved => "folder_removed",
            Self::TicketCreated => "ticket_created",
            Self::TicketAccepted => "ticket_accepted",
            Self::Kicked => "kicked",
        }
    }
}

/// Logger for network-level events, sync sessions, and relay health.
///
/// All records are written to `stats.db`.
#[derive(Clone, Debug)]
pub struct NetworkLogger {
    database: Arc<StatsDatabase>,
}

impl NetworkLogger {
    /// Create a new `NetworkLogger` backed by the given stats database.
    #[must_use]
    pub fn new(database: StatsDatabase) -> Self {
        Self {
            database: Arc::new(database),
        }
    }

    /// Create a new `NetworkLogger` from an already-`Arc`-wrapped database.
    #[must_use]
    pub const fn from_arc(database: Arc<StatsDatabase>) -> Self {
        Self { database }
    }

    /// Record a network event.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_event(
        &self,
        network_id: &str,
        event: NetworkEventType,
        peer: Option<&str>,
        details: Option<&str>,
    ) -> Result<()> {
        self.database
            .record_network_event(network_id, event.as_str(), peer, details, None)
    }

    /// Record a network event with metadata JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_event_with_metadata(
        &self,
        network_id: &str,
        event: NetworkEventType,
        peer: Option<&str>,
        details: Option<&str>,
        metadata_json: Option<&str>,
    ) -> Result<()> {
        self.database
            .record_network_event(network_id, event.as_str(), peer, details, metadata_json)
    }

    /// Record the start of a sync session. Returns the session ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_sync_start(&self, network_id: &str, folder_namespace: &str) -> Result<i64> {
        self.database.record_sync_session_start(network_id, folder_namespace)
    }

    /// Record the finish of a sync session.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_sync_finish(&self, session_id: i64, files: u64, bytes: u64, errors: u64, status: &str) -> Result<()> {
        self.database
            .record_sync_session_finish(session_id, files, bytes, errors, status)
    }

    /// Record a relay health check result.
    ///
    /// # Errors
    ///
    /// Returns an error if the database write fails.
    pub fn record_relay_check(
        &self,
        relay_url: &str,
        connected: bool,
        latency_ms: Option<i64>,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.database
            .record_relay_check(relay_url, connected, latency_ms, error_message)
    }

    /// Query recent events for a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn recent_events(&self, network_id: &str, limit: usize) -> Result<Vec<NetworkEventRecord>> {
        self.database.recent_network_events(network_id, limit)
    }

    /// Query recent sync sessions for a network.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn recent_sessions(&self, network_id: &str, limit: usize) -> Result<Vec<SyncSessionRecord>> {
        self.database.recent_sync_sessions(network_id, limit)
    }

    /// Calculate relay uptime as a ratio (0.0-1.0) over the given time window.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn relay_uptime(&self, relay_url: &str, window_secs: i64) -> Result<f64> {
        self.database.relay_uptime(relay_url, window_secs)
    }
}
