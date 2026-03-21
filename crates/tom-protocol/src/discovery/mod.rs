/// Discovery module for ToM protocol.
///
/// Application-level peer discovery on top of iroh's low-level
/// address resolution. Handles: announcements, heartbeats,
/// liveness tracking, and ephemeral subnet clustering.
pub mod heartbeat;
pub mod relay_announce;
pub mod relay_registry;
pub mod role_sync;
pub mod subnet;
pub mod types;

pub use heartbeat::HeartbeatTracker;
pub use relay_announce::RelayReadyAnnounce;
pub use relay_registry::{
    RelayRegistry, RelayRegistryEntry, UpsertResult, DEFAULT_RELAY_REGISTRY_TTL_MS,
};
pub use role_sync::RoleChangeAnnounce;
pub use subnet::{
    CommunicationEdge, DissolveReason, EphemeralSubnetManager, SubnetEvent, SubnetInfo,
};
pub use types::{
    DiscoveryEvent, DiscoverySource, LivenessState, PeerAnnounce, GOSSIP_INTERVAL_MS,
    HEARTBEAT_INTERVAL_MS, MAX_FUTURE_DRIFT_MS, MAX_PEERS_PER_GOSSIP, OFFLINE_THRESHOLD_MS,
    STALE_THRESHOLD_MS,
};
