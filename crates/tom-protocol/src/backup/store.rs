/// BackupStore — message storage for offline recipients.
///
/// Pure state machine: store messages, track replicas, expire TTL.
/// No I/O — caller drives cleanup timer and handles actions.
///
/// The "virus" lives here: messages self-replicate across nodes
/// and self-delete when delivered or when viability drops too low.
use std::collections::{HashMap, HashSet};

use crate::backup::types::*;
use crate::types::NodeId;

/// Maximum total messages across all recipients (memory protection).
const MAX_TOTAL_MESSAGES: usize = 10_000;

/// Stores backup messages for offline recipients.
pub struct BackupStore {
    /// Messages by ID.
    messages: HashMap<String, BackupEntry>,
    /// Index: recipient → message IDs.
    by_recipient: HashMap<NodeId, HashSet<String>>,
    /// Host factors for viability computation.
    host_factors: HostFactors,
}

impl BackupStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
            by_recipient: HashMap::new(),
            host_factors: HostFactors::default(),
        }
    }

    /// Store a message for an offline recipient.
    pub fn store(
        &mut self,
        message_id: String,
        payload: Vec<u8>,
        recipient_id: NodeId,
        sender_id: NodeId,
        now: u64,
        ttl_ms: Option<u64>,
    ) -> Vec<BackupEvent> {
        let mut events = vec![];

        // Dedup
        if self.messages.contains_key(&message_id) {
            return events;
        }

        // Memory protection
        if self.messages.len() >= MAX_TOTAL_MESSAGES {
            self.evict_oldest(now);
        }

        let entry = BackupEntry::new(message_id.clone(), payload, recipient_id, sender_id, now, ttl_ms);

        self.by_recipient
            .entry(recipient_id)
            .or_default()
            .insert(message_id.clone());

        events.push(BackupEvent::MessageStored {
            message_id: message_id.clone(),
            recipient_id,
        });

        self.messages.insert(message_id, entry);
        events
    }

    /// Store from a replication payload (received from another node).
    pub fn store_replica(
        &mut self,
        payload: &ReplicationPayload,
        now: u64,
    ) -> Vec<BackupEvent> {
        // Don't store expired messages
        if now >= payload.expires_at {
            return vec![];
        }

        // Already have it
        if self.messages.contains_key(&payload.message_id) {
            // But record the replications we didn't know about
            if let Some(entry) = self.messages.get_mut(&payload.message_id) {
                for node in &payload.replicated_to {
                    entry.replicated_to.insert(*node);
                }
            }
            return vec![];
        }

        // Calculate remaining TTL from absolute expiry
        let remaining_ttl = payload.expires_at.saturating_sub(now);

        let mut entry = BackupEntry::new(
            payload.message_id.clone(),
            payload.payload.clone(),
            payload.recipient_id,
            payload.sender_id,
            now,
            Some(remaining_ttl),
        );
        entry.viability_score = payload.viability_score;
        for node in &payload.replicated_to {
            entry.replicated_to.insert(*node);
        }

        self.by_recipient
            .entry(payload.recipient_id)
            .or_default()
            .insert(payload.message_id.clone());

        let events = vec![BackupEvent::MessageStored {
            message_id: payload.message_id.clone(),
            recipient_id: payload.recipient_id,
        }];

        self.messages.insert(payload.message_id.clone(), entry);
        events
    }

    /// Get all messages for a recipient.
    pub fn get_for_recipient(&self, recipient_id: &NodeId) -> Vec<&BackupEntry> {
        let Some(ids) = self.by_recipient.get(recipient_id) else {
            return vec![];
        };
        ids.iter()
            .filter_map(|id| self.messages.get(id))
            .collect()
    }

    /// Get a specific message.
    pub fn get(&self, message_id: &str) -> Option<&BackupEntry> {
        self.messages.get(message_id)
    }

    /// Check if we have a message.
    pub fn has(&self, message_id: &str) -> bool {
        self.messages.contains_key(message_id)
    }

    /// Mark message as delivered — remove from store.
    pub fn mark_delivered(&mut self, message_id: &str) -> Vec<BackupEvent> {
        let Some(entry) = self.messages.remove(message_id) else {
            return vec![];
        };

        if let Some(ids) = self.by_recipient.get_mut(&entry.recipient_id) {
            ids.remove(message_id);
            if ids.is_empty() {
                self.by_recipient.remove(&entry.recipient_id);
            }
        }

        vec![BackupEvent::MessageDelivered {
            message_id: message_id.to_string(),
            recipient_id: entry.recipient_id,
        }]
    }

    /// Mark multiple messages as delivered.
    pub fn mark_delivered_batch(&mut self, message_ids: &[String]) -> Vec<BackupEvent> {
        let mut events = vec![];
        for id in message_ids {
            events.extend(self.mark_delivered(id));
        }
        events
    }

    /// Record that a message was replicated to a node.
    pub fn record_replication(&mut self, message_id: &str, target: NodeId) {
        if let Some(entry) = self.messages.get_mut(message_id) {
            entry.replicated_to.insert(target);
        }
    }

    /// Update viability score for a message.
    pub fn update_viability(&mut self, message_id: &str, score: u8) {
        if let Some(entry) = self.messages.get_mut(message_id) {
            entry.viability_score = score.min(100);
        }
    }

    /// Update host factors (affects all viability computations).
    pub fn update_host_factors(&mut self, factors: HostFactors) {
        self.host_factors = factors;
    }

    /// Get current host factors.
    pub fn host_factors(&self) -> &HostFactors {
        &self.host_factors
    }

    /// Run cleanup: remove expired messages. Returns events.
    pub fn cleanup_expired(&mut self, now: u64) -> Vec<BackupEvent> {
        let mut events = vec![];
        let expired: Vec<String> = self
            .messages
            .iter()
            .filter(|(_, entry)| entry.is_expired(now))
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired {
            if let Some(entry) = self.messages.remove(&id) {
                if let Some(ids) = self.by_recipient.get_mut(&entry.recipient_id) {
                    ids.remove(&id);
                    if ids.is_empty() {
                        self.by_recipient.remove(&entry.recipient_id);
                    }
                }
                events.push(BackupEvent::MessageExpired {
                    message_id: id,
                    recipient_id: entry.recipient_id,
                });
            }
        }

        events
    }

    /// Check viability of all messages. Returns events for those
    /// needing replication or self-deletion.
    pub fn check_viability(&self) -> Vec<BackupEvent> {
        let mut events = vec![];

        // Compute viability from host factors
        let score = self.compute_host_viability();

        for entry in self.messages.values() {
            let effective_score = score.min(entry.viability_score);

            if effective_score <= DELETION_THRESHOLD {
                events.push(BackupEvent::SelfDeleteRecommended {
                    message_id: entry.message_id.clone(),
                    recipient_id: entry.recipient_id,
                    score: effective_score,
                });
            } else if effective_score <= REPLICATION_THRESHOLD
                && entry.replica_count() < MAX_REPLICAS
            {
                events.push(BackupEvent::ReplicationNeeded {
                    message_id: entry.message_id.clone(),
                    recipient_id: entry.recipient_id,
                    score: effective_score,
                });
            }
        }

        events
    }

    /// Delete a message (self-deletion when viability is too low).
    pub fn delete(&mut self, message_id: &str) -> bool {
        if let Some(entry) = self.messages.remove(message_id) {
            if let Some(ids) = self.by_recipient.get_mut(&entry.recipient_id) {
                ids.remove(message_id);
                if ids.is_empty() {
                    self.by_recipient.remove(&entry.recipient_id);
                }
            }
            true
        } else {
            false
        }
    }

    /// Create a replication payload for sending to another node.
    pub fn create_replication_payload(&self, message_id: &str) -> Option<ReplicationPayload> {
        let entry = self.messages.get(message_id)?;
        Some(ReplicationPayload {
            message_id: entry.message_id.clone(),
            payload: entry.payload.clone(),
            recipient_id: entry.recipient_id,
            sender_id: entry.sender_id,
            expires_at: entry.expires_at,
            viability_score: entry.viability_score,
            replicated_to: entry.replicated_to.iter().copied().collect(),
        })
    }

    /// Total stored messages.
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Number of unique recipients with pending messages.
    pub fn recipient_count(&self) -> usize {
        self.by_recipient.len()
    }

    /// All message IDs (for iteration).
    pub fn message_ids(&self) -> Vec<String> {
        self.messages.keys().cloned().collect()
    }

    // ── Internal ─────────────────────────────────────────────────────────

    /// Compute host viability score (0–100) from host factors.
    /// Weighted: stability 30%, bandwidth 25%, contribution 20%, base 25%.
    fn compute_host_viability(&self) -> u8 {
        let f = &self.host_factors;
        let score = f.stability as f64 * 0.30
            + f.bandwidth as f64 * 0.25
            + f.contribution as f64 * 0.20
            + 25.0; // Base 25% (always have some viability)
        (score as u8).min(100)
    }

    /// Evict the oldest message to make room.
    fn evict_oldest(&mut self, _now: u64) {
        if let Some((oldest_id, _)) = self
            .messages
            .iter()
            .min_by_key(|(_, entry)| entry.stored_at)
        {
            let oldest_id = oldest_id.clone();
            self.delete(&oldest_id);
        }
    }
}

impl Default for BackupStore {
    fn default() -> Self {
        Self::new()
    }
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
    fn store_and_retrieve() {
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);

        let events = store.store("msg-1".into(), vec![1, 2], recipient, sender, 10_000, None);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], BackupEvent::MessageStored { .. }));

        assert!(store.has("msg-1"));
        assert_eq!(store.message_count(), 1);

        let msgs = store.get_for_recipient(&recipient);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].payload, vec![1, 2]);
    }

    #[test]
    fn dedup_same_message() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);

        store.store("msg-1".into(), vec![1], r, s, 10_000, None);
        let events = store.store("msg-1".into(), vec![1], r, s, 10_000, None);
        assert!(events.is_empty()); // No duplicate stored
        assert_eq!(store.message_count(), 1);
    }

    #[test]
    fn cleanup_expired() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);

        store.store("msg-1".into(), vec![], r, s, 10_000, Some(1000));
        store.store("msg-2".into(), vec![], r, s, 10_000, Some(5000));

        // At 11_500: msg-1 expired, msg-2 still alive
        let events = store.cleanup_expired(11_500);
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], BackupEvent::MessageExpired { message_id, .. } if message_id == "msg-1"));
        assert_eq!(store.message_count(), 1);
        assert!(store.has("msg-2"));
    }

    #[test]
    fn mark_delivered() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);

        store.store("msg-1".into(), vec![], r, s, 10_000, None);
        store.store("msg-2".into(), vec![], r, s, 10_000, None);

        let events = store.mark_delivered("msg-1");
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], BackupEvent::MessageDelivered { message_id, .. } if message_id == "msg-1"));
        assert!(!store.has("msg-1"));
        assert!(store.has("msg-2"));
        assert_eq!(store.recipient_count(), 1); // Still has msg-2 for recipient
    }

    #[test]
    fn mark_delivered_cleans_recipient_index() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);

        store.store("msg-1".into(), vec![], r, s, 10_000, None);
        store.mark_delivered("msg-1");
        assert_eq!(store.recipient_count(), 0);
    }

    #[test]
    fn store_replica() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);
        let relay = node_id(3);

        let payload = ReplicationPayload {
            message_id: "msg-1".into(),
            payload: vec![1, 2, 3],
            recipient_id: r,
            sender_id: s,
            expires_at: 20_000,
            viability_score: 80,
            replicated_to: vec![relay],
        };

        let events = store.store_replica(&payload, 15_000);
        assert_eq!(events.len(), 1);
        assert!(store.has("msg-1"));

        let entry = store.get("msg-1").unwrap();
        assert_eq!(entry.viability_score, 80);
        assert!(entry.replicated_to.contains(&relay));
        // Remaining TTL: 20_000 - 15_000 = 5_000
        assert_eq!(entry.remaining_ttl(15_000), 5_000);
    }

    #[test]
    fn store_replica_expired_rejected() {
        let mut store = BackupStore::new();
        let payload = ReplicationPayload {
            message_id: "msg-1".into(),
            payload: vec![],
            recipient_id: node_id(1),
            sender_id: node_id(2),
            expires_at: 10_000,
            viability_score: 50,
            replicated_to: vec![],
        };

        let events = store.store_replica(&payload, 15_000); // Already expired
        assert!(events.is_empty());
        assert!(!store.has("msg-1"));
    }

    #[test]
    fn record_replication() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);
        let target = node_id(3);

        store.store("msg-1".into(), vec![], r, s, 10_000, None);
        store.record_replication("msg-1", target);

        let entry = store.get("msg-1").unwrap();
        assert!(entry.replicated_to.contains(&target));
        assert_eq!(entry.replica_count(), 1);
    }

    #[test]
    fn create_replication_payload() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);
        let target = node_id(3);

        store.store("msg-1".into(), vec![42], r, s, 10_000, Some(5000));
        store.record_replication("msg-1", target);

        let payload = store.create_replication_payload("msg-1").unwrap();
        assert_eq!(payload.message_id, "msg-1");
        assert_eq!(payload.payload, vec![42]);
        assert_eq!(payload.recipient_id, r);
        assert_eq!(payload.expires_at, 15_000);
        assert_eq!(payload.replicated_to, vec![target]);
    }

    #[test]
    fn viability_replication_needed() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);

        store.store("msg-1".into(), vec![], r, s, 10_000, None);

        // Default host factors (50/50/50) → score ~50 → no replication
        let events = store.check_viability();
        assert!(events.is_empty());

        // Degrade host → low viability
        store.update_host_factors(HostFactors {
            stability: 0,
            bandwidth: 0,
            contribution: 0,
        });

        let events = store.check_viability();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], BackupEvent::ReplicationNeeded { .. }));
    }

    #[test]
    fn viability_self_delete_recommended() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);

        store.store("msg-1".into(), vec![], r, s, 10_000, None);

        // Very bad host + bad message viability
        store.update_host_factors(HostFactors {
            stability: 0,
            bandwidth: 0,
            contribution: 0,
        });
        store.update_viability("msg-1", 5);

        let events = store.check_viability();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], BackupEvent::SelfDeleteRecommended { .. }));
    }

    #[test]
    fn delete_message() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);

        store.store("msg-1".into(), vec![], r, s, 10_000, None);
        assert!(store.delete("msg-1"));
        assert!(!store.has("msg-1"));
        assert_eq!(store.message_count(), 0);
    }

    #[test]
    fn batch_delivery() {
        let mut store = BackupStore::new();
        let r = node_id(1);
        let s = node_id(2);

        store.store("msg-1".into(), vec![], r, s, 10_000, None);
        store.store("msg-2".into(), vec![], r, s, 10_000, None);
        store.store("msg-3".into(), vec![], r, s, 10_000, None);

        let events = store.mark_delivered_batch(&["msg-1".into(), "msg-3".into()]);
        assert_eq!(events.len(), 2);
        assert_eq!(store.message_count(), 1);
        assert!(store.has("msg-2"));
    }

    #[test]
    fn multiple_recipients() {
        let mut store = BackupStore::new();
        let r1 = node_id(1);
        let r2 = node_id(2);
        let s = node_id(3);

        store.store("msg-1".into(), vec![], r1, s, 10_000, None);
        store.store("msg-2".into(), vec![], r2, s, 10_000, None);
        store.store("msg-3".into(), vec![], r1, s, 10_000, None);

        assert_eq!(store.get_for_recipient(&r1).len(), 2);
        assert_eq!(store.get_for_recipient(&r2).len(), 1);
        assert_eq!(store.recipient_count(), 2);

        // Deliver all for r1
        store.mark_delivered("msg-1");
        store.mark_delivered("msg-3");
        assert_eq!(store.recipient_count(), 1);
    }
}
