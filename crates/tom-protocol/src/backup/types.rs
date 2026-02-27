/// Backup types for the ToM "virus backup" system.
///
/// Messages for offline recipients self-replicate across backup nodes,
/// and self-delete when delivered or after 24h TTL.
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::types::NodeId;

// ── Constants ────────────────────────────────────────────────────────────

/// Maximum TTL: 24 hours (design decision #2 — non-negotiable).
pub const MAX_TTL_MS: u64 = 24 * 60 * 60 * 1000;

/// Default TTL (same as max).
pub const DEFAULT_TTL_MS: u64 = MAX_TTL_MS;

/// Cleanup interval (60 seconds). Caller uses this as timer interval.
pub const CLEANUP_INTERVAL_MS: u64 = 60_000;

/// Viability check interval (30 seconds).
pub const VIABILITY_CHECK_INTERVAL_MS: u64 = 30_000;

/// Viability score at or below which replication is triggered.
pub const REPLICATION_THRESHOLD: u8 = 30;

/// Viability score at or below which self-deletion is triggered.
pub const DELETION_THRESHOLD: u8 = 10;

/// Maximum replicas per message.
pub const MAX_REPLICAS: usize = 5;

/// Query timeout (30 seconds).
pub const QUERY_TIMEOUT_MS: u64 = 30_000;

/// Query debounce window (5 seconds).
pub const QUERY_DEBOUNCE_MS: u64 = 5_000;

// ── Types ────────────────────────────────────────────────────────────────

/// A backed-up message held for an offline recipient.
#[derive(Debug, Clone)]
pub struct BackupEntry {
    /// Original message ID (from Envelope).
    pub message_id: String,
    /// Encrypted payload bytes (opaque — we never decrypt).
    pub payload: Vec<u8>,
    /// Who this message is for.
    pub recipient_id: NodeId,
    /// Who sent the original message.
    pub sender_id: NodeId,
    /// When we stored this backup (Unix ms).
    pub stored_at: u64,
    /// Absolute expiry time (Unix ms). Hard limit: stored_at + MAX_TTL_MS.
    pub expires_at: u64,
    /// Viability score (0–100). Drives replication/deletion decisions.
    pub viability_score: u8,
    /// Nodes that have confirmed replicas.
    pub replicated_to: HashSet<NodeId>,
}

impl BackupEntry {
    /// Create a new backup entry.
    pub fn new(
        message_id: String,
        payload: Vec<u8>,
        recipient_id: NodeId,
        sender_id: NodeId,
        now: u64,
        ttl_ms: Option<u64>,
    ) -> Self {
        let ttl = ttl_ms.unwrap_or(DEFAULT_TTL_MS).min(MAX_TTL_MS);
        Self {
            message_id,
            payload,
            recipient_id,
            sender_id,
            stored_at: now,
            expires_at: now + ttl,
            viability_score: 100,
            replicated_to: HashSet::new(),
        }
    }

    /// Whether this entry has expired.
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }

    /// Remaining TTL in ms (0 if expired).
    pub fn remaining_ttl(&self, now: u64) -> u64 {
        self.expires_at.saturating_sub(now)
    }

    /// Number of confirmed replicas.
    pub fn replica_count(&self) -> usize {
        self.replicated_to.len()
    }
}

/// Events emitted by the backup system.
#[derive(Debug, Clone)]
pub enum BackupEvent {
    /// A new message was stored for backup.
    MessageStored {
        message_id: String,
        recipient_id: NodeId,
    },

    /// A message expired (TTL exceeded).
    MessageExpired {
        message_id: String,
        recipient_id: NodeId,
    },

    /// A message was delivered (recipient confirmed).
    MessageDelivered {
        message_id: String,
        recipient_id: NodeId,
    },

    /// Viability dropped — replication needed (score <= 30%).
    ReplicationNeeded {
        message_id: String,
        recipient_id: NodeId,
        score: u8,
    },

    /// Viability critical — self-deletion recommended (score <= 10%).
    SelfDeleteRecommended {
        message_id: String,
        recipient_id: NodeId,
        score: u8,
    },

    /// Message was successfully replicated to another node.
    MessageReplicated {
        message_id: String,
        target_node: NodeId,
    },
}

/// Payload for replication requests between backup nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationPayload {
    pub message_id: String,
    pub payload: Vec<u8>,
    pub recipient_id: NodeId,
    pub sender_id: NodeId,
    pub expires_at: u64,
    pub viability_score: u8,
    pub replicated_to: Vec<NodeId>,
}

/// Actions returned by the backup system for the caller to execute.
#[derive(Debug, Clone)]
pub enum BackupAction {
    /// Send a replication request to a specific node.
    Replicate {
        target: NodeId,
        payload: ReplicationPayload,
    },

    /// Broadcast delivery confirmation to all backup nodes.
    ConfirmDelivery {
        message_ids: Vec<String>,
        recipient_id: NodeId,
    },

    /// Query network for pending messages for a recipient.
    QueryPending {
        recipient_id: NodeId,
    },

    /// Emit an event (for the application layer).
    Event(BackupEvent),
}

/// Host factors used to compute viability score.
#[derive(Debug, Clone, Copy)]
pub struct HostFactors {
    /// Connection stability (0–100).
    pub stability: u8,
    /// Bandwidth capacity (0–100).
    pub bandwidth: u8,
    /// Contribution score (0–100).
    pub contribution: u8,
}

impl Default for HostFactors {
    fn default() -> Self {
        Self {
            stability: 50,
            bandwidth: 50,
            contribution: 50,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn backup_entry_new() {
        let entry = BackupEntry::new(
            "msg-1".into(),
            vec![1, 2, 3],
            node_id(1),
            node_id(2),
            10_000,
            None,
        );
        assert_eq!(entry.expires_at, 10_000 + MAX_TTL_MS);
        assert_eq!(entry.viability_score, 100);
        assert_eq!(entry.replica_count(), 0);
    }

    #[test]
    fn ttl_clamped_to_max() {
        let entry = BackupEntry::new(
            "msg-1".into(),
            vec![],
            node_id(1),
            node_id(2),
            10_000,
            Some(MAX_TTL_MS * 2), // Try to exceed max
        );
        // Clamped to MAX_TTL_MS
        assert_eq!(entry.expires_at, 10_000 + MAX_TTL_MS);
    }

    #[test]
    fn expiry_check() {
        let entry = BackupEntry::new(
            "msg-1".into(),
            vec![],
            node_id(1),
            node_id(2),
            10_000,
            Some(1000), // 1 second TTL
        );
        assert!(!entry.is_expired(10_500));
        assert!(!entry.is_expired(10_999));
        assert!(entry.is_expired(11_000));
        assert!(entry.is_expired(12_000));
    }

    #[test]
    fn remaining_ttl() {
        let entry = BackupEntry::new(
            "msg-1".into(),
            vec![],
            node_id(1),
            node_id(2),
            10_000,
            Some(5000),
        );
        assert_eq!(entry.remaining_ttl(10_000), 5000);
        assert_eq!(entry.remaining_ttl(12_000), 3000);
        assert_eq!(entry.remaining_ttl(15_000), 0);
        assert_eq!(entry.remaining_ttl(20_000), 0);
    }

    #[test]
    fn replication_payload_roundtrip() {
        let payload = ReplicationPayload {
            message_id: "msg-1".into(),
            payload: vec![1, 2, 3],
            recipient_id: node_id(1),
            sender_id: node_id(2),
            expires_at: 100_000,
            viability_score: 75,
            replicated_to: vec![node_id(3)],
        };
        let bytes = rmp_serde::to_vec(&payload).unwrap();
        let decoded: ReplicationPayload = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(decoded.message_id, "msg-1");
        assert_eq!(decoded.viability_score, 75);
        assert_eq!(decoded.replicated_to.len(), 1);
    }

    #[test]
    fn host_factors_default() {
        let factors = HostFactors::default();
        assert_eq!(factors.stability, 50);
        assert_eq!(factors.bandwidth, 50);
        assert_eq!(factors.contribution, 50);
    }
}
