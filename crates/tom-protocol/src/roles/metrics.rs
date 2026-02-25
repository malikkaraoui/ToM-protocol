//! Role metrics for observability and debugging.

use crate::relay::PeerRole;
use crate::types::NodeId;

/// Complete role metrics snapshot for a peer (debug/observability).
#[derive(Debug, Clone)]
pub struct RoleMetrics {
    pub node_id: NodeId,
    pub role: PeerRole,
    pub score: f64,
    pub relay_count: u64,
    pub relay_failures: u64,
    pub success_rate: f64,
    pub bytes_relayed: u64,
    pub bytes_received: u64,
    pub bandwidth_ratio: f64,
    pub uptime_hours: f64,
    pub first_seen: u64,
    pub last_activity: u64,
}
