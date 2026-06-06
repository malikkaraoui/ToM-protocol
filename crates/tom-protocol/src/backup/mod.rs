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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NodeId;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    // ── BackupEntry (types) ────────────────────────────────────────────

    #[test]
    fn ttl_max_24h_enforced() {
        // TTL exceeding 24h must be clamped
        let entry = BackupEntry::new(
            "msg-1".into(),
            vec![],
            node_id(1),
            node_id(2),
            0,
            Some(MAX_TTL_MS + 1_000_000), // way over limit
        );
        assert_eq!(
            entry.expires_at, MAX_TTL_MS,
            "expires_at must not exceed stored_at + MAX_TTL_MS"
        );
    }

    #[test]
    fn default_ttl_is_24h() {
        let entry = BackupEntry::new("msg-1".into(), vec![], node_id(1), node_id(2), 0, None);
        assert_eq!(entry.expires_at, MAX_TTL_MS);
        assert_eq!(MAX_TTL_MS, 24 * 60 * 60 * 1000, "MAX_TTL_MS should be exactly 24 hours");
    }

    #[test]
    fn is_expired_boundary() {
        let entry = BackupEntry::new("msg".into(), vec![], node_id(1), node_id(2), 0, Some(5000));
        assert!(!entry.is_expired(4999));
        assert!(entry.is_expired(5000));  // exactly at boundary: expired
        assert!(entry.is_expired(5001));
    }

    #[test]
    fn remaining_ttl_saturates_at_zero() {
        let entry = BackupEntry::new("msg".into(), vec![], node_id(1), node_id(2), 0, Some(1000));
        assert_eq!(entry.remaining_ttl(2000), 0, "past expiry → remaining_ttl == 0");
        assert_eq!(entry.remaining_ttl(u64::MAX), 0, "far future → 0, no underflow");
    }

    #[test]
    fn replica_count_tracks_correctly() {
        let mut entry = BackupEntry::new("msg".into(), vec![], node_id(1), node_id(2), 0, None);
        assert_eq!(entry.replica_count(), 0);
        entry.replicated_to.insert(node_id(3));
        entry.replicated_to.insert(node_id(4));
        assert_eq!(entry.replica_count(), 2);
        // Dedup: inserting same id twice doesn't change count
        entry.replicated_to.insert(node_id(3));
        assert_eq!(entry.replica_count(), 2);
    }

    // ── BackupStore ────────────────────────────────────────────────────

    #[test]
    fn store_emits_stored_event() {
        let mut store = BackupStore::new();
        let events = store.store("m1".into(), vec![1, 2], node_id(1), node_id(2), 1000, None);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], BackupEvent::MessageStored { message_id, .. } if message_id == "m1"));
    }

    #[test]
    fn delivered_message_purged_immediately() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);
        store.store("m1".into(), vec![], r, s, 1000, None);
        store.store("m2".into(), vec![], r, s, 1000, None);

        let events = store.mark_delivered("m1");
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], BackupEvent::MessageDelivered { message_id, .. } if message_id == "m1"));
        assert!(!store.has("m1"), "delivered message must be purged");
        assert!(store.has("m2"), "other messages unaffected");
    }

    #[test]
    fn expired_message_removed_by_cleanup() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);

        // 500ms TTL
        store.store("short".into(), vec![], r, s, 0, Some(500));
        // 5000ms TTL
        store.store("long".into(), vec![], r, s, 0, Some(5000));

        let events = store.cleanup_expired(600); // only "short" expired
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], BackupEvent::MessageExpired { message_id, .. } if message_id == "short"));
        assert!(!store.has("short"));
        assert!(store.has("long"));
    }

    #[test]
    fn cleanup_24h_ttl_scenario() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);

        // Store with default 24h TTL at t=0
        store.store("msg".into(), vec![], r, s, 0, None);

        // Just before 24h — not expired
        let events_before = store.cleanup_expired(MAX_TTL_MS - 1);
        assert!(events_before.is_empty(), "should not expire before 24h");

        // At exactly 24h — expired
        let events_at = store.cleanup_expired(MAX_TTL_MS);
        assert_eq!(events_at.len(), 1);
        assert!(matches!(&events_at[0], BackupEvent::MessageExpired { .. }));
    }

    #[test]
    fn replication_virus_spread_tracked() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);
        store.store("msg".into(), vec![], r, s, 0, None);

        let n3 = node_id(3);
        let n4 = node_id(4);
        store.record_replication("msg", n3);
        store.record_replication("msg", n4);

        let entry = store.get("msg").unwrap();
        assert!(entry.replicated_to.contains(&n3));
        assert!(entry.replicated_to.contains(&n4));
        assert_eq!(entry.replica_count(), 2, "should track virus spread to 2 nodes");
    }

    #[test]
    fn max_replicas_constant_value() {
        // Design invariant: MAX_REPLICAS = 5
        assert_eq!(MAX_REPLICAS, 5, "MAX_REPLICAS should be 5 per design");
    }

    #[test]
    fn replication_payload_preserves_ttl() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);
        store.store("msg".into(), vec![77], r, s, 0, Some(3600_000));

        let payload = store.create_replication_payload("msg").unwrap();
        assert_eq!(payload.expires_at, 3600_000, "payload should carry absolute expiry");
        assert_eq!(payload.payload, vec![77]);
        assert_eq!(payload.recipient_id, r);
    }

    #[test]
    fn store_replica_with_known_nodes() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);
        let relay1 = node_id(3);
        let relay2 = node_id(4);

        let payload = ReplicationPayload {
            message_id: "msg".into(),
            payload: vec![1],
            recipient_id: r,
            sender_id: s,
            expires_at: 100_000,
            viability_score: 90,
            replicated_to: vec![relay1, relay2],
        };

        let events = store.store_replica(&payload, 1000);
        assert_eq!(events.len(), 1);
        let entry = store.get("msg").unwrap();
        assert!(entry.replicated_to.contains(&relay1));
        assert!(entry.replicated_to.contains(&relay2));
        assert_eq!(entry.viability_score, 90);
    }

    // ── BackupCoordinator ─────────────────────────────────────────────

    #[test]
    fn coordinator_tick_expires_old_messages() {
        let local = node_id(0);
        let r = node_id(1);
        let s = node_id(2);
        let mut coord = BackupCoordinator::new(local);

        coord.store_message("msg".into(), vec![], r, s, 0, Some(1000));

        let actions = coord.tick(1500);
        let expired = actions.iter().filter(|a| {
            matches!(a, BackupAction::Event(BackupEvent::MessageExpired { .. }))
        }).count();
        assert_eq!(expired, 1, "coordinator tick should expire stale message");
        assert_eq!(coord.store().message_count(), 0);
    }

    #[test]
    fn coordinator_replicate_max_replicas_cap() {
        let local = node_id(0);
        let r = node_id(1);
        let s = node_id(2);
        let mut coord = BackupCoordinator::new(local);
        coord.store_message("msg".into(), vec![], r, s, 0, None);

        // Simulate MAX_REPLICAS replications
        for i in 3..=(MAX_REPLICAS as u8 + 2) {
            let target = node_id(i);
            coord.store_mut().record_replication("msg", target);
        }

        // Now at MAX_REPLICAS — further replicate_to should be no-op
        let extra_target = node_id(20);
        let actions = coord.replicate_to("msg", extra_target, 1000);
        assert!(actions.is_empty(), "should not replicate beyond MAX_REPLICAS={MAX_REPLICAS}");
    }

    #[test]
    fn delivery_confirmation_purges_store() {
        let local = node_id(0);
        let r = node_id(1);
        let s = node_id(2);
        let mut coord = BackupCoordinator::new(local);

        coord.store_message("m1".into(), vec![], r, s, 0, None);
        coord.store_message("m2".into(), vec![], r, s, 0, None);

        coord.confirm_delivery(&["m1".into(), "m2".into()], r);
        assert_eq!(coord.store().message_count(), 0, "all messages purged after delivery");
    }

    #[test]
    fn incoming_replication_rejected_if_expired() {
        let local = node_id(0);
        let mut coord = BackupCoordinator::new(local);

        let payload = ReplicationPayload {
            message_id: "msg".into(),
            payload: vec![],
            recipient_id: node_id(1),
            sender_id: node_id(2),
            expires_at: 500,
            viability_score: 50,
            replicated_to: vec![],
        };

        let from = node_id(3);
        coord.handle_replication(&payload, from, 1000); // already expired
        assert!(!coord.store().has("msg"), "expired replica must not be stored");
    }
}
