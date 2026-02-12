use crate::NodeId;
use std::time::{Duration, Instant};

/// The kind of network path to a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    /// Traffic goes through a relay server.
    Relay,
    /// Direct UDP connection (hole-punched).
    Direct,
    /// Path type not yet determined.
    Unknown,
}

impl std::fmt::Display for PathKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathKind::Relay => write!(f, "RELAY"),
            PathKind::Direct => write!(f, "DIRECT"),
            PathKind::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// A path change event for a connected peer.
#[derive(Debug, Clone)]
pub struct PathEvent {
    /// Current path kind (relay or direct).
    pub kind: PathKind,
    /// Round-trip time on this path.
    pub rtt: Duration,
    /// The remote peer.
    pub remote: NodeId,
    /// When this event occurred.
    pub timestamp: Instant,
}
