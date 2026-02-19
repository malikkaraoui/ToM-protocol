/// Backup module for ToM protocol — the "virus backup" system.
///
/// Messages for offline recipients self-replicate across backup nodes
/// and self-delete when delivered or after 24h TTL.
///
/// Three layers:
/// - **Store**: holds messages, tracks replicas, manages TTL
/// - **Coordinator**: orchestrates queries, replication, delivery confirmation
/// - **Types**: data structures, constants, events
pub mod coordinator;
pub mod store;
pub mod types;

pub use coordinator::BackupCoordinator;
pub use store::BackupStore;
pub use types::{
    BackupAction, BackupEntry, BackupEvent, HostFactors, ReplicationPayload, CLEANUP_INTERVAL_MS,
    DEFAULT_TTL_MS, DELETION_THRESHOLD, MAX_REPLICAS, MAX_TTL_MS, QUERY_DEBOUNCE_MS,
    QUERY_TIMEOUT_MS, REPLICATION_THRESHOLD, VIABILITY_CHECK_INTERVAL_MS,
};
