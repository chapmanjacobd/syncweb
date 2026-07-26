pub mod config;
pub mod node_db;
pub mod stats_db;

pub use config::{AdvancedConfig, BandwidthConfig, BepConfig, CacheConfig, Config, ParallelConfig};

/// Common trait for database maintenance operations.
pub trait Vacuumable {
    /// Run VACUUM to reclaim space.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    fn vacuum(&self) -> crate::Result<()>;

    /// Return the freelist page count (indicates whether VACUUM would help).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    fn freelist_count(&self) -> crate::Result<i64>;
}
