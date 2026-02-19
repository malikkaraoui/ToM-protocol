/// Discovery types for ToM protocol.
///
/// Application-level peer metadata broadcast over the protocol layer.
/// iroh handles low-level address resolution; this module handles
/// what a node announces about itself (username, roles, capabilities).
use serde::{Deserialize, Serialize};

use crate::relay::PeerRole;
use crate::types::NodeId;

// ── Constants ────────────────────────────────────────────────────────────

/// Heartbeat interval (5 seconds).
pub const HEARTBEAT_INTERVAL_MS: u64 = 5_000;

/// Stale threshold — peer becomes stale after missing 2 heartbeats (10s).
pub const STALE_THRESHOLD_MS: u64 = 10_000;

/// Offline threshold — peer becomes offline after missing 4 heartbeats (20s).
pub const OFFLINE_THRESHOLD_MS: u64 = 20_000;

/// Maximum allowed clock drift for timestamps (5 minutes).
pub const MAX_FUTURE_DRIFT_MS: u64 = 5 * 60 * 1000;

/// Gossip round interval (30 seconds).
pub const GOSSIP_INTERVAL_MS: u64 = 30_000;

/// Max peers returned in a single gossip response.
pub const MAX_PEERS_PER_GOSSIP: usize = 20;

// ── PeerAnnounce ─────────────────────────────────────────────────────────

/// Payload for PeerAnnounce messages — what a node broadcasts about itself.
///
/// Serialized into `Envelope.payload` with `MessageType::PeerAnnounce`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerAnnounce {
    /// The announcing node's identity.
    pub node_id: NodeId,
    /// Human-readable display name.
    pub username: String,
    /// Roles this node can serve.
    pub roles: Vec<PeerRole>,
    /// Ed25519 public key for E2E encryption (32 bytes).
    pub encryption_key: Option<[u8; 32]>,
    /// Announcement timestamp (Unix ms).
    pub timestamp: u64,
}

impl PeerAnnounce {
    /// Create a new peer announcement.
    pub fn new(node_id: NodeId, username: String, roles: Vec<PeerRole>) -> Self {
        Self {
            node_id,
            username,
            roles,
            encryption_key: Some(node_id.as_bytes()),
            timestamp: now_ms(),
        }
    }

    /// Whether this announcement is within acceptable clock drift.
    pub fn is_timestamp_valid(&self, now: u64) -> bool {
        // Not too far in the future
        if self.timestamp > now + MAX_FUTURE_DRIFT_MS {
            return false;
        }
        // Not absurdly old (1 hour)
        if now > self.timestamp && now - self.timestamp > 60 * 60 * 1000 {
            return false;
        }
        true
    }
}

// ── DiscoveryEvent ───────────────────────────────────────────────────────

/// Events emitted by the discovery system.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// A new peer was discovered.
    PeerDiscovered {
        node_id: NodeId,
        username: String,
        source: DiscoverySource,
    },

    /// A peer went stale (missed heartbeats but might recover).
    PeerStale {
        node_id: NodeId,
    },

    /// A peer went offline (confirmed departed).
    PeerOffline {
        node_id: NodeId,
    },

    /// A peer came back online.
    PeerOnline {
        node_id: NodeId,
    },
}

/// How we learned about a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoverySource {
    /// Direct connection / bootstrap.
    Direct,
    /// Learned via gossip from another peer.
    Gossip,
    /// Peer announced itself.
    Announce,
}

// ── LivenessState ────────────────────────────────────────────────────────

/// Current liveness state of a tracked peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessState {
    /// Actively sending heartbeats.
    Alive,
    /// Missed heartbeats, might recover.
    Stale,
    /// Confirmed departed.
    Departed,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn peer_announce_new() {
        let id = node_id(1);
        let announce = PeerAnnounce::new(id, "alice".into(), vec![PeerRole::Peer]);

        assert_eq!(announce.node_id, id);
        assert_eq!(announce.username, "alice");
        assert_eq!(announce.roles, vec![PeerRole::Peer]);
        assert!(announce.encryption_key.is_some());
        assert!(announce.timestamp > 0);
    }

    #[test]
    fn peer_announce_roundtrip() {
        let id = node_id(1);
        let announce = PeerAnnounce::new(id, "alice".into(), vec![PeerRole::Relay]);

        let bytes = rmp_serde::to_vec(&announce).expect("serialize");
        let decoded: PeerAnnounce = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(announce, decoded);
    }

    #[test]
    fn timestamp_validation() {
        let id = node_id(1);
        let now = now_ms();

        let mut announce = PeerAnnounce::new(id, "alice".into(), vec![]);
        announce.timestamp = now;
        assert!(announce.is_timestamp_valid(now));

        // Slightly in future — OK
        announce.timestamp = now + 1000;
        assert!(announce.is_timestamp_valid(now));

        // Too far in future — reject
        announce.timestamp = now + MAX_FUTURE_DRIFT_MS + 1;
        assert!(!announce.is_timestamp_valid(now));

        // Old but within 1 hour — OK
        announce.timestamp = now - 30 * 60 * 1000;
        assert!(announce.is_timestamp_valid(now));

        // Too old — reject
        announce.timestamp = now - 2 * 60 * 60 * 1000;
        assert!(!announce.is_timestamp_valid(now));
    }

    #[test]
    fn discovery_source_roundtrip() {
        for source in [DiscoverySource::Direct, DiscoverySource::Gossip, DiscoverySource::Announce] {
            let bytes = rmp_serde::to_vec(&source).expect("serialize");
            let decoded: DiscoverySource = rmp_serde::from_slice(&bytes).expect("deserialize");
            assert_eq!(source, decoded);
        }
    }
}
