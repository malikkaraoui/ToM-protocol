/// BackupCoordinator — orchestrates backup queries, replication, and delivery.
///
/// Pure state machine: processes events from BackupStore, produces actions.
/// No I/O — caller handles transport.
///
/// Three responsibilities:
/// 1. Query: when a peer comes online, query the network for their pending messages
/// 2. Replicate: spread messages to other nodes for redundancy
/// 3. Confirm: when delivery succeeds, notify all replica holders to clean up
use std::collections::{HashMap, HashSet};

use crate::backup::store::BackupStore;
use crate::backup::types::*;
use crate::types::NodeId;

/// Orchestrates backup operations across the network.
pub struct BackupCoordinator {
    /// Our node ID.
    local_id: NodeId,
    /// The backup store we manage.
    store: BackupStore,
    /// Active queries: recipient → (query_time, responding_message_ids).
    active_queries: HashMap<NodeId, QueryState>,
    /// Debounce: last query time per recipient.
    last_query_time: HashMap<NodeId, u64>,
    /// Pending replications: message_id → (target, sent_at).
    pending_replications: HashMap<String, Vec<(NodeId, u64)>>,
}

struct QueryState {
    started_at: u64,
    received_ids: HashSet<String>,
}

impl BackupCoordinator {
    /// Create a new coordinator.
    pub fn new(local_id: NodeId) -> Self {
        Self {
            local_id,
            store: BackupStore::new(),
            active_queries: HashMap::new(),
            last_query_time: HashMap::new(),
            pending_replications: HashMap::new(),
        }
    }

    /// Access the underlying store (read-only).
    pub fn store(&self) -> &BackupStore {
        &self.store
    }

    /// Access the underlying store (mutable).
    pub fn store_mut(&mut self) -> &mut BackupStore {
        &mut self.store
    }

    // ── Store operations (delegated) ─────────────────────────────────────

    /// Store a message for an offline recipient.
    pub fn store_message(
        &mut self,
        message_id: String,
        payload: Vec<u8>,
        recipient_id: NodeId,
        sender_id: NodeId,
        now: u64,
        ttl_ms: Option<u64>,
    ) -> Vec<BackupAction> {
        self.store
            .store(message_id, payload, recipient_id, sender_id, now, ttl_ms)
            .into_iter()
            .map(BackupAction::Event)
            .collect()
    }

    /// Handle an incoming replication payload from another node.
    pub fn handle_replication(&mut self, payload: &ReplicationPayload, from: NodeId, now: u64) -> Vec<BackupAction> {
        let mut actions: Vec<BackupAction> = self.store
            .store_replica(payload, now)
            .into_iter()
            .map(BackupAction::Event)
            .collect();

        // ACK: tell the sender we now have it
        if self.store.has(&payload.message_id) {
            self.store.record_replication(&payload.message_id, self.local_id);
            actions.push(BackupAction::Event(BackupEvent::MessageReplicated {
                message_id: payload.message_id.clone(),
                target_node: self.local_id,
            }));

            // Also record that `from` has it
            self.store.record_replication(&payload.message_id, from);
        }

        actions
    }

    // ── Query: peer comes online ─────────────────────────────────────────

    /// A peer came online — query network for their pending messages.
    /// Returns QueryPending action if not debounced.
    pub fn query_pending(&mut self, recipient_id: NodeId, now: u64) -> Vec<BackupAction> {
        // Debounce
        if let Some(&last) = self.last_query_time.get(&recipient_id) {
            if now.saturating_sub(last) < QUERY_DEBOUNCE_MS {
                return vec![];
            }
        }

        self.last_query_time.insert(recipient_id, now);
        self.active_queries.insert(
            recipient_id,
            QueryState {
                started_at: now,
                received_ids: HashSet::new(),
            },
        );

        // Also check our own store
        let mut actions = vec![];
        let local_msgs = self.store.get_for_recipient(&recipient_id);
        if !local_msgs.is_empty() {
            // We have messages locally — no need to query network for these
            for msg in &local_msgs {
                if let Some(query) = self.active_queries.get_mut(&recipient_id) {
                    query.received_ids.insert(msg.message_id.clone());
                }
            }
        }

        actions.push(BackupAction::QueryPending { recipient_id });
        actions
    }

    /// Handle response to a pending query: another node has messages for recipient.
    /// Returns the message IDs we haven't seen yet.
    pub fn handle_query_response(
        &mut self,
        recipient_id: &NodeId,
        message_ids: &[String],
        _now: u64,
    ) -> Vec<String> {
        let Some(query) = self.active_queries.get_mut(recipient_id) else {
            return vec![]; // No active query
        };

        let mut new_ids = vec![];
        for id in message_ids {
            if query.received_ids.insert(id.clone()) {
                new_ids.push(id.clone());
            }
        }
        new_ids
    }

    /// Clean up expired queries.
    pub fn cleanup_queries(&mut self, now: u64) {
        self.active_queries
            .retain(|_, state| now.saturating_sub(state.started_at) < QUERY_TIMEOUT_MS);
    }

    // ── Delivery confirmation ────────────────────────────────────────────

    /// Recipient confirmed delivery — clear backups and notify replicas.
    pub fn confirm_delivery(
        &mut self,
        message_ids: &[String],
        recipient_id: NodeId,
    ) -> Vec<BackupAction> {
        let mut actions = vec![];

        // Remove from local store
        let events = self.store.mark_delivered_batch(message_ids);
        actions.extend(events.into_iter().map(BackupAction::Event));

        // Cancel pending replications
        for id in message_ids {
            self.pending_replications.remove(id);
        }

        // Notify other replica holders
        actions.push(BackupAction::ConfirmDelivery {
            message_ids: message_ids.to_vec(),
            recipient_id,
        });

        // Clean up active query for this recipient
        self.active_queries.remove(&recipient_id);

        actions
    }

    /// Handle delivery confirmation from another node — clear our copies.
    pub fn handle_delivery_confirmation(
        &mut self,
        message_ids: &[String],
    ) -> Vec<BackupAction> {
        self.store
            .mark_delivered_batch(message_ids)
            .into_iter()
            .map(BackupAction::Event)
            .collect()
    }

    // ── Replication ──────────────────────────────────────────────────────

    /// Initiate replication of a message to a target node.
    pub fn replicate_to(
        &mut self,
        message_id: &str,
        target: NodeId,
        now: u64,
    ) -> Vec<BackupAction> {
        let Some(entry) = self.store.get(message_id) else {
            return vec![];
        };

        // Already replicated to this target
        if entry.replicated_to.contains(&target) {
            return vec![];
        }

        // Max replicas reached
        if entry.replica_count() >= MAX_REPLICAS {
            return vec![];
        }

        let Some(payload) = self.store.create_replication_payload(message_id) else {
            return vec![];
        };

        // Track pending
        self.pending_replications
            .entry(message_id.to_string())
            .or_default()
            .push((target, now));

        vec![BackupAction::Replicate { target, payload }]
    }

    /// Handle replication ACK — mark as replicated.
    pub fn handle_replication_ack(
        &mut self,
        message_id: &str,
        from: NodeId,
    ) -> Vec<BackupAction> {
        self.store.record_replication(message_id, from);

        // Remove from pending
        if let Some(pending) = self.pending_replications.get_mut(message_id) {
            pending.retain(|(target, _)| *target != from);
            if pending.is_empty() {
                self.pending_replications.remove(message_id);
            }
        }

        vec![BackupAction::Event(BackupEvent::MessageReplicated {
            message_id: message_id.to_string(),
            target_node: from,
        })]
    }

    // ── Periodic maintenance ─────────────────────────────────────────────

    /// Run periodic maintenance: cleanup expired, check viability.
    pub fn tick(&mut self, now: u64) -> Vec<BackupAction> {
        let mut actions = vec![];

        // Cleanup expired messages
        let expired = self.store.cleanup_expired(now);
        actions.extend(expired.into_iter().map(BackupAction::Event));

        // Cleanup expired queries
        self.cleanup_queries(now);

        // Check viability
        let viability = self.store.check_viability();
        actions.extend(viability.into_iter().map(BackupAction::Event));

        // Cleanup stale pending replications (30s timeout)
        for pending in self.pending_replications.values_mut() {
            pending.retain(|(_, sent_at)| now.saturating_sub(*sent_at) < QUERY_TIMEOUT_MS);
        }
        self.pending_replications.retain(|_, v| !v.is_empty());

        actions
    }

    /// Number of active queries.
    pub fn active_query_count(&self) -> usize {
        self.active_queries.len()
    }

    /// Number of messages with pending replications.
    pub fn pending_replication_count(&self) -> usize {
        self.pending_replications.len()
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

    fn setup() -> (BackupCoordinator, NodeId, NodeId, NodeId) {
        let local = node_id(0);
        let alice = node_id(1);
        let bob = node_id(2);
        let coord = BackupCoordinator::new(local);
        (coord, local, alice, bob)
    }

    #[test]
    fn store_and_query() {
        let (mut coord, _local, alice, bob) = setup();
        let now = 10_000u64;

        // Store message for offline alice
        let actions = coord.store_message("msg-1".into(), vec![42], alice, bob, now, None);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], BackupAction::Event(BackupEvent::MessageStored { .. })));

        assert_eq!(coord.store().message_count(), 1);
        assert!(coord.store().has("msg-1"));
    }

    #[test]
    fn query_pending_debounce() {
        let (mut coord, _local, alice, _bob) = setup();
        let now = 10_000u64;

        // First query — OK
        let actions = coord.query_pending(alice, now);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], BackupAction::QueryPending { .. }));

        // Immediate retry — debounced
        let actions = coord.query_pending(alice, now + 1000);
        assert!(actions.is_empty());

        // After debounce window — OK
        let actions = coord.query_pending(alice, now + QUERY_DEBOUNCE_MS + 1);
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn query_response_dedup() {
        let (mut coord, _local, alice, _bob) = setup();
        let now = 10_000u64;

        coord.query_pending(alice, now);

        // First response
        let new = coord.handle_query_response(&alice, &["msg-1".into(), "msg-2".into()], now);
        assert_eq!(new.len(), 2);

        // Duplicate response
        let new = coord.handle_query_response(&alice, &["msg-1".into(), "msg-3".into()], now);
        assert_eq!(new.len(), 1); // Only msg-3 is new
        assert_eq!(new[0], "msg-3");
    }

    #[test]
    fn confirm_delivery_clears_store() {
        let (mut coord, _local, alice, bob) = setup();
        let now = 10_000u64;

        coord.store_message("msg-1".into(), vec![], alice, bob, now, None);
        coord.store_message("msg-2".into(), vec![], alice, bob, now, None);

        let actions = coord.confirm_delivery(&["msg-1".into()], alice);

        // Should have: delivered event + confirm broadcast
        let event_count = actions.iter().filter(|a| matches!(a, BackupAction::Event(_))).count();
        let confirm_count = actions.iter().filter(|a| matches!(a, BackupAction::ConfirmDelivery { .. })).count();
        assert_eq!(event_count, 1);
        assert_eq!(confirm_count, 1);

        assert!(!coord.store().has("msg-1"));
        assert!(coord.store().has("msg-2"));
    }

    #[test]
    fn handle_delivery_confirmation_from_network() {
        let (mut coord, _local, alice, bob) = setup();
        let now = 10_000u64;

        coord.store_message("msg-1".into(), vec![], alice, bob, now, None);

        // Another node confirmed delivery
        let actions = coord.handle_delivery_confirmation(&["msg-1".into()]);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], BackupAction::Event(BackupEvent::MessageDelivered { .. })));
        assert!(!coord.store().has("msg-1"));
    }

    #[test]
    fn replicate_to_node() {
        let (mut coord, _local, alice, bob) = setup();
        let target = node_id(5);
        let now = 10_000u64;

        coord.store_message("msg-1".into(), vec![42], alice, bob, now, None);

        let actions = coord.replicate_to("msg-1", target, now);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], BackupAction::Replicate { .. }));
        assert_eq!(coord.pending_replication_count(), 1);

        // Duplicate — already pending
        // (But not yet ACK'd, so replicated_to doesn't contain target)
        // The action is still generated since it's not yet confirmed
    }

    #[test]
    fn replicate_ack() {
        let (mut coord, _local, alice, bob) = setup();
        let target = node_id(5);
        let now = 10_000u64;

        coord.store_message("msg-1".into(), vec![], alice, bob, now, None);
        coord.replicate_to("msg-1", target, now);

        let actions = coord.handle_replication_ack("msg-1", target);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], BackupAction::Event(BackupEvent::MessageReplicated { .. })));

        // Replication confirmed in store
        let entry = coord.store().get("msg-1").unwrap();
        assert!(entry.replicated_to.contains(&target));

        // Pending cleared
        assert_eq!(coord.pending_replication_count(), 0);
    }

    #[test]
    fn handle_incoming_replication() {
        let (mut coord, local, alice, bob) = setup();
        let now = 10_000u64;

        let payload = ReplicationPayload {
            message_id: "msg-1".into(),
            payload: vec![42],
            recipient_id: alice,
            sender_id: bob,
            expires_at: now + 60_000,
            viability_score: 75,
            replicated_to: vec![],
        };

        let from = node_id(5);
        let actions = coord.handle_replication(&payload, from, now);

        // Should store + ACK
        assert!(coord.store().has("msg-1"));
        let entry = coord.store().get("msg-1").unwrap();
        assert!(entry.replicated_to.contains(&local));
        assert!(entry.replicated_to.contains(&from));

        let replicated = actions.iter().filter(|a| {
            matches!(a, BackupAction::Event(BackupEvent::MessageReplicated { .. }))
        }).count();
        assert!(replicated >= 1);
    }

    #[test]
    fn tick_cleans_expired() {
        let (mut coord, _local, alice, bob) = setup();
        let now = 10_000u64;

        coord.store_message("msg-1".into(), vec![], alice, bob, now, Some(1000));

        // Before expiry — no cleanup
        let actions = coord.tick(now + 500);
        let expired = actions.iter().filter(|a| {
            matches!(a, BackupAction::Event(BackupEvent::MessageExpired { .. }))
        }).count();
        assert_eq!(expired, 0);

        // After expiry
        let actions = coord.tick(now + 1500);
        let expired = actions.iter().filter(|a| {
            matches!(a, BackupAction::Event(BackupEvent::MessageExpired { .. }))
        }).count();
        assert_eq!(expired, 1);
        assert_eq!(coord.store().message_count(), 0);
    }

    #[test]
    fn tick_checks_viability() {
        let (mut coord, _local, alice, bob) = setup();
        let now = 10_000u64;

        coord.store_message("msg-1".into(), vec![], alice, bob, now, None);

        // Degrade host
        coord.store_mut().update_host_factors(HostFactors {
            stability: 0,
            bandwidth: 0,
            contribution: 0,
        });

        let actions = coord.tick(now + 100);
        let replication_needed = actions.iter().filter(|a| {
            matches!(a, BackupAction::Event(BackupEvent::ReplicationNeeded { .. }))
        }).count();
        assert!(replication_needed >= 1);
    }

    #[test]
    fn cleanup_queries_timeout() {
        let (mut coord, _local, alice, _bob) = setup();
        let now = 10_000u64;

        coord.query_pending(alice, now);
        assert_eq!(coord.active_query_count(), 1);

        coord.cleanup_queries(now + QUERY_TIMEOUT_MS + 1);
        assert_eq!(coord.active_query_count(), 0);
    }
}
