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

/// Maximum total PAYLOAD BYTES held across all backups (memory protection).
///
/// `MAX_TOTAL_MESSAGES` alone bounds the message COUNT, never the VOLUME — and
/// payloads live in RAM (`BackupEntry.payload: Vec<u8>`), so ~100 messages of
/// 8 MiB reach 800 MiB long before the 10_000-entry cap is approached. Measured
/// on 2026-07-19: the NAS relay held 688 MiB on a 920 MiB VM, the OOM killer had
/// already reaped a node (771 MiB RSS), and under that pressure the node lost
/// every peer (8_366 send failures) while still reporting itself connected.
/// A legitimate load campaign was enough to trigger it.
///
/// 64 MiB keeps a relay useful (a healthy node's whole RSS is ~24 MiB) while
/// leaving headroom on the smallest host we run on. Same bug class as the
/// large-message and reassembly DoS fixes: a per-unit cap needs a global budget.
const MAX_TOTAL_BYTES: usize = 64 * 1024 * 1024;

/// Stores backup messages for offline recipients.
pub struct BackupStore {
    /// Messages by ID.
    messages: HashMap<String, BackupEntry>,
    /// Index: recipient → message IDs.
    by_recipient: HashMap<NodeId, HashSet<String>>,
    /// Host factors for viability computation.
    host_factors: HostFactors,
    /// Sum of `payload.len()` over `messages`. Maintained incrementally through
    /// the single insert/remove pair below — never recomputed by scanning.
    total_bytes: usize,
}

impl BackupStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
            by_recipient: HashMap::new(),
            host_factors: HostFactors::default(),
            total_bytes: 0,
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

        // Memory protection — count AND volume. A locally-originated backup is
        // by definition from a known depositor (ourselves), so it may displace
        // older entries; it still cannot exceed the byte budget.
        if self.messages.len() >= MAX_TOTAL_MESSAGES {
            self.evict_oldest(now);
        }
        if !self.make_room_for(payload.len(), true, now) {
            // NEVER refuse silently. The byte budget introduces a refusal path
            // that did not exist before (`store` always evicted its way to
            // success), and a mute refusal is exactly the class of blindness
            // that let the 688 MiB failure run for a whole night. The backup
            // net simply did not catch this message — the sender still learns
            // the truth from the absence of an ACK (LOCKED #1), but an operator
            // must be able to see it.
            tracing::warn!(
                message_id = %message_id,
                payload_bytes = payload.len(),
                total_bytes = self.total_bytes,
                budget = MAX_TOTAL_BYTES,
                "backup refused: byte budget exhausted"
            );
            return events;
        }

        let entry = BackupEntry::new(message_id.clone(), payload, recipient_id, sender_id, now, ttl_ms);

        events.push(BackupEvent::MessageStored {
            message_id,
            recipient_id,
        });

        self.insert_entry(entry);
        events
    }

    /// Store from a replication payload (received from another node).
    ///
    /// `depositor_known` is true when we hold real local contribution evidence
    /// for the peer that sent us this replica. Under capacity pressure, stranger
    /// (unknown) deposits are evicted first and are refused outright when the
    /// store is full of legitimate (known-deposited) backups — so a
    /// BackupReplicate flood from fresh identities can neither grow the store
    /// without bound nor displace real backups (red-team FINDING #9).
    pub fn store_replica(
        &mut self,
        payload: &ReplicationPayload,
        depositor_known: bool,
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
                // Borne le HashSet : un pair malveillant peut envoyer un
                // replicated_to géant (anti-DoS mémoire, cf. MAX_REPLICATED_TO).
                for node in &payload.replicated_to {
                    if entry.replicated_to.len() >= MAX_REPLICATED_TO {
                        break;
                    }
                    entry.replicated_to.insert(*node);
                }
            }
            return vec![];
        }

        // Memory protection + fairness (red-team FINDING #9): the NETWORK
        // replication path was UNBOUNDED — only the local `store()` enforced
        // MAX_TOTAL_MESSAGES, so a BackupReplicate flood could grow the store
        // without limit (OOM). Enforce the hard cap here, and make room fairly:
        //  - drop a stranger-deposited entry first (a flood churns its own);
        //  - else, only a KNOWN depositor may displace the oldest legit entry;
        //  - a STRANGER deposit into a store full of legit backups is refused,
        //    so a flood can never evict real backups.
        if self.messages.len() >= MAX_TOTAL_MESSAGES && !self.evict_oldest_stranger() {
            if depositor_known {
                self.evict_oldest(now);
            } else {
                // Store full of legitimate backups, depositor is a stranger:
                // refuse rather than displace a real backup.
                return vec![];
            }
        }
        // Same guard on VOLUME: without it the network path could hold a few
        // dozen multi-MiB replicas and exhaust host memory well under the
        // 10_000-entry cap (the failure actually observed on the NAS relay).
        if !self.make_room_for(payload.payload.len(), depositor_known, now) {
            tracing::warn!(
                message_id = %payload.message_id,
                payload_bytes = payload.payload.len(),
                total_bytes = self.total_bytes,
                budget = MAX_TOTAL_BYTES,
                depositor_known,
                "backup replica refused: byte budget exhausted"
            );
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
        entry.from_known_depositor = depositor_known;
        // Borne le HashSet dès la création (anti-DoS mémoire, cf. MAX_REPLICATED_TO).
        for node in &payload.replicated_to {
            if entry.replicated_to.len() >= MAX_REPLICATED_TO {
                break;
            }
            entry.replicated_to.insert(*node);
        }

        let events = vec![BackupEvent::MessageStored {
            message_id: payload.message_id.clone(),
            recipient_id: payload.recipient_id,
        }];

        self.insert_entry(entry);
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
        let Some(entry) = self.remove_entry(message_id) else {
            return vec![];
        };

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
            if let Some(entry) = self.remove_entry(&id) {
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
        self.remove_entry(message_id).is_some()
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

    /// Payload bytes currently held (memory footprint of the store).
    ///
    /// Exposed because the count alone hid the failure: the NAS relay looked
    /// healthy by `message_count()` while holding 688 MiB.
    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    /// Byte budget ceiling — for reporting saturation before it bites.
    pub fn max_total_bytes() -> usize {
        MAX_TOTAL_BYTES
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

    /// Insert an entry — the ONLY place `messages` grows. Keeps `total_bytes`
    /// and the recipient index in sync with the map.
    fn insert_entry(&mut self, entry: BackupEntry) {
        self.total_bytes = self.total_bytes.saturating_add(entry.payload.len());
        self.by_recipient
            .entry(entry.recipient_id)
            .or_default()
            .insert(entry.message_id.clone());
        self.messages.insert(entry.message_id.clone(), entry);
    }

    /// Remove an entry — the ONLY place `messages` shrinks. Centralising this
    /// is what makes `total_bytes` impossible to desynchronise: previously the
    /// `remove` + index-cleanup pair was duplicated in four places, and any new
    /// call site that forgot to decrement would leak the budget silently.
    fn remove_entry(&mut self, message_id: &str) -> Option<BackupEntry> {
        let entry = self.messages.remove(message_id)?;
        self.total_bytes = self.total_bytes.saturating_sub(entry.payload.len());
        if let Some(ids) = self.by_recipient.get_mut(&entry.recipient_id) {
            ids.remove(message_id);
            if ids.is_empty() {
                self.by_recipient.remove(&entry.recipient_id);
            }
        }
        Some(entry)
    }

    /// Evict until `incoming` bytes fit within `MAX_TOTAL_BYTES`.
    ///
    /// Preserves the fairness rule of FINDING #9: strangers are evicted first,
    /// and a stranger deposit may never displace legitimate backups. Returns
    /// false when the payload cannot be accommodated — the caller must refuse
    /// rather than exceed the budget.
    fn make_room_for(&mut self, incoming: usize, depositor_known: bool, now: u64) -> bool {
        // A single payload larger than the whole budget is never storable;
        // refusing here also guarantees the loop below terminates.
        if incoming > MAX_TOTAL_BYTES {
            return false;
        }
        while self.total_bytes.saturating_add(incoming) > MAX_TOTAL_BYTES {
            if self.evict_oldest_stranger() {
                continue;
            }
            if !depositor_known {
                // Budget is full of legitimate backups and the depositor is a
                // stranger: refuse instead of displacing real backups.
                return false;
            }
            let before = self.messages.len();
            self.evict_oldest(now);
            if self.messages.len() == before {
                // Nothing left to evict (store already empty) — give up rather
                // than spin forever.
                return false;
            }
        }
        true
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

    /// Evict the oldest STRANGER-deposited message (from a peer we hold no
    /// contribution evidence for). Returns true if one was evicted. Used to
    /// make room without touching legitimate (known-deposited) backups under a
    /// replication flood (FINDING #9).
    fn evict_oldest_stranger(&mut self) -> bool {
        if let Some((oldest_id, _)) = self
            .messages
            .iter()
            .filter(|(_, entry)| !entry.from_known_depositor)
            .min_by_key(|(_, entry)| entry.stored_at)
        {
            let oldest_id = oldest_id.clone();
            self.delete(&oldest_id);
            true
        } else {
            false
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
        let secret = tom_connect::SecretKey::generate(&mut rng);
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

        let events = store.store_replica(&payload, true, 15_000);
        assert_eq!(events.len(), 1);
        assert!(store.has("msg-1"));

        let entry = store.get("msg-1").unwrap();
        assert_eq!(entry.viability_score, 80);
        assert!(entry.replicated_to.contains(&relay));
        // Remaining TTL: 20_000 - 15_000 = 5_000
        assert_eq!(entry.remaining_ttl(15_000), 5_000);
    }

    #[test]
    fn store_replica_is_bounded_against_flood() {
        // FINDING #9: the network replication path must enforce
        // MAX_TOTAL_MESSAGES (it previously did not → unbounded memory / OOM
        // under a BackupReplicate flood from a hostile peer).
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);
        for i in 0..(MAX_TOTAL_MESSAGES + 500) {
            let payload = ReplicationPayload {
                message_id: format!("flood-{i}"),
                payload: vec![0u8; 8],
                recipient_id: recipient,
                sender_id: sender,
                expires_at: 1_000_000,
                viability_score: 0,
                replicated_to: vec![],
            };
            store.store_replica(&payload, false, 1_000);
        }
        assert!(
            store.message_count() <= MAX_TOTAL_MESSAGES,
            "store_replica must stay bounded, got {}",
            store.message_count()
        );
    }

    /// 1 MiB payload — big enough that a handful blows past a byte budget long
    /// before the 10_000-entry count cap is anywhere near.
    fn heavy_payload() -> Vec<u8> {
        vec![0u8; 1024 * 1024]
    }

    #[test]
    fn store_is_bounded_by_bytes_not_just_count() {
        // The NAS failure of 2026-07-19: MAX_TOTAL_MESSAGES bounds the COUNT,
        // never the VOLUME. 200 × 1 MiB stays far under 10_000 entries yet held
        // 688 MiB on a 920 MiB VM until the OOM killer reaped the node.
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);
        for i in 0..200 {
            store.store(
                format!("heavy-{i}"),
                heavy_payload(),
                recipient,
                sender,
                1_000,
                None,
            );
        }
        assert!(
            store.message_count() < MAX_TOTAL_MESSAGES,
            "precondition: the count cap must NOT be what saves us here"
        );
        assert!(
            store.total_bytes() <= MAX_TOTAL_BYTES,
            "byte budget exceeded: {} > {}",
            store.total_bytes(),
            MAX_TOTAL_BYTES
        );
    }

    #[test]
    fn replica_flood_is_bounded_by_bytes() {
        // Same guard on the NETWORK path — a hostile peer sending large
        // replicas must not be able to exhaust host memory either.
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);
        for i in 0..200 {
            let payload = ReplicationPayload {
                message_id: format!("heavy-replica-{i}"),
                payload: heavy_payload(),
                recipient_id: recipient,
                sender_id: sender,
                expires_at: 1_000_000,
                viability_score: 0,
                replicated_to: vec![],
            };
            store.store_replica(&payload, false, 1_000);
        }
        assert!(
            store.total_bytes() <= MAX_TOTAL_BYTES,
            "byte budget exceeded on replication path: {}",
            store.total_bytes()
        );
    }

    #[test]
    fn total_bytes_returns_to_zero_after_removals() {
        // The counter is maintained incrementally, so a missed decrement would
        // leak the budget until the store refuses everything. Exercise all
        // three removal paths: delivered, expired, self-deleted.
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);
        store.store("a".into(), vec![0u8; 4096], recipient, sender, 1_000, Some(5_000));
        store.store("b".into(), vec![0u8; 4096], recipient, sender, 1_000, Some(5_000));
        store.store("c".into(), vec![0u8; 4096], recipient, sender, 1_000, Some(5_000));
        assert_eq!(store.total_bytes(), 3 * 4096);

        store.mark_delivered("a");
        store.delete("b");
        store.cleanup_expired(1_000_000); // expires "c"

        assert_eq!(store.message_count(), 0);
        assert_eq!(store.total_bytes(), 0, "byte counter leaked on removal");
    }

    #[test]
    fn payload_larger_than_budget_is_refused_not_looped() {
        // A single payload over the whole budget can never fit; the store must
        // refuse it outright rather than spin evicting everything forever.
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);
        store.store("small".into(), vec![0u8; 1024], recipient, sender, 1_000, None);

        let events = store.store(
            "oversized".into(),
            vec![0u8; MAX_TOTAL_BYTES + 1],
            recipient,
            sender,
            1_000,
            None,
        );

        assert!(events.is_empty(), "oversized payload must be refused");
        assert!(!store.has("oversized"));
        assert!(store.has("small"), "refusal must not evict existing backups");
    }

    #[test]
    fn eviction_of_the_only_entry_makes_room_instead_of_refusing() {
        // Cas limite soulevé en review : le store contient UNE entrée occupant
        // tout le budget, et un dépôt légitime minuscule se présente. La garde
        // de progression de `make_room_for` compare `messages.len()` avant et
        // après éviction — il fallait vérifier qu'elle n'interprète pas
        // « j'ai évincé la dernière entrée » comme « rien à évincer, je refuse ».
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);
        store.store(
            "hog".into(),
            vec![0u8; MAX_TOTAL_BYTES],
            recipient,
            sender,
            1_000,
            None,
        );
        assert_eq!(store.total_bytes(), MAX_TOTAL_BYTES);

        let events = store.store("tiny".into(), vec![0u8; 1], recipient, sender, 2_000, None);

        assert!(!events.is_empty(), "le petit dépôt doit être accepté");
        assert!(store.has("tiny"));
        assert!(!store.has("hog"), "l'occupant du budget doit avoir été évincé");
        assert_eq!(store.total_bytes(), 1);
    }

    #[test]
    fn refusal_by_saturation_stores_nothing_and_keeps_existing() {
        // Refus par SATURATION (et non par taille unitaire) : le budget est
        // plein de backups légitimes non expirés, et l'espace libéré par
        // éviction ne suffit pas. Vérifie qu'aucun état partiel ne subsiste.
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);
        let stranger_free = MAX_TOTAL_BYTES / (1024 * 1024);
        for i in 0..stranger_free {
            let payload = ReplicationPayload {
                message_id: format!("legit-{i}"),
                payload: heavy_payload(),
                recipient_id: recipient,
                sender_id: sender,
                expires_at: 1_000_000,
                viability_score: 0,
                replicated_to: vec![],
            };
            store.store_replica(&payload, true, 1_000); // known depositor
        }
        let bytes_before = store.total_bytes();
        let count_before = store.message_count();

        // Un inconnu tente de déposer : refus attendu (il ne peut pas déplacer
        // des backups légitimes), et le store doit rester strictement intact.
        let intruder = ReplicationPayload {
            message_id: "intruder".into(),
            payload: heavy_payload(),
            recipient_id: recipient,
            sender_id: sender,
            expires_at: 1_000_000,
            viability_score: 0,
            replicated_to: vec![],
        };
        let events = store.store_replica(&intruder, false, 1_000);

        assert!(events.is_empty(), "dépôt inconnu refusé → aucun événement");
        assert!(!store.has("intruder"));
        assert_eq!(store.total_bytes(), bytes_before, "octets modifiés malgré le refus");
        assert_eq!(store.message_count(), count_before, "entrées modifiées malgré le refus");
    }

    #[test]
    fn record_replication_does_not_move_the_byte_counter() {
        // `record_replication` mute les métadonnées d'une entrée sans passer par
        // insert/remove_entry. C'est volontaire — le compteur ne suit que
        // `payload.len()` — mais l'invariant mérite d'être verrouillé par un
        // test : si quelqu'un touche un jour au payload ici, le budget fuirait.
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);
        store.store("m".into(), vec![0u8; 2048], recipient, sender, 1_000, None);
        let before = store.total_bytes();

        store.record_replication("m", node_id(3));
        store.record_replication("m", node_id(4));

        assert_eq!(store.total_bytes(), before);
    }

    #[test]
    fn stranger_cannot_evict_legit_backups_via_byte_pressure() {
        // FINDING #9 fairness must hold for the BYTE budget too, otherwise a
        // flood of large stranger replicas evicts real backups — the same
        // attack, just denominated in bytes instead of entries.
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);

        // Fill the budget with legitimate, locally-stored backups.
        let legit = MAX_TOTAL_BYTES / (1024 * 1024);
        for i in 0..legit {
            store.store(
                format!("legit-{i}"),
                heavy_payload(),
                recipient,
                sender,
                1_000,
                None,
            );
        }
        let legit_before = store.message_count();

        for i in 0..50 {
            let payload = ReplicationPayload {
                message_id: format!("stranger-{i}"),
                payload: heavy_payload(),
                recipient_id: recipient,
                sender_id: sender,
                expires_at: 1_000_000,
                viability_score: 0,
                replicated_to: vec![],
            };
            store.store_replica(&payload, false, 1_000);
        }

        assert!(store.total_bytes() <= MAX_TOTAL_BYTES);
        for i in 0..legit {
            assert!(
                store.has(&format!("legit-{i}")),
                "stranger flood evicted legitimate backup legit-{i}"
            );
        }
        assert_eq!(store.message_count(), legit_before);
    }

    #[test]
    fn known_backups_survive_stranger_flood() {
        // FINDING #9 fairness: legitimate (known-deposited) backups must NOT be
        // evicted by a flood of stranger replicas, and a stranger deposit into a
        // store full of known backups is refused.
        let mut store = BackupStore::new();
        let recipient = node_id(1);
        let sender = node_id(2);
        let mk = |id: &str| ReplicationPayload {
            message_id: id.into(),
            payload: vec![0u8; 8],
            recipient_id: recipient,
            sender_id: sender,
            expires_at: 1_000_000,
            viability_score: 0,
            replicated_to: vec![],
        };

        // Fill the store completely with KNOWN-deposited backups.
        for i in 0..MAX_TOTAL_MESSAGES {
            store.store_replica(&mk(&format!("legit-{i}")), true, 1_000);
        }
        assert_eq!(store.message_count(), MAX_TOTAL_MESSAGES);

        // A stranger now floods: every deposit must be REFUSED (store full of
        // legit), so not a single legit backup is displaced.
        for i in 0..2_000 {
            store.store_replica(&mk(&format!("flood-{i}")), false, 2_000);
        }
        assert_eq!(
            store.message_count(),
            MAX_TOTAL_MESSAGES,
            "stranger flood must not grow the store"
        );
        for i in 0..MAX_TOTAL_MESSAGES {
            assert!(
                store.has(&format!("legit-{i}")),
                "legit backup legit-{i} was evicted by the stranger flood"
            );
        }
        assert!(!store.has("flood-0"), "no stranger entry should have been admitted");
    }

    #[test]
    fn store_replica_borne_un_replicated_to_geant() {
        // Anti-DoS mémoire : un pair malveillant envoie un replicated_to énorme.
        // Le HashSet persistant doit rester borné à MAX_REPLICATED_TO.
        let mut store = BackupStore::new();
        let huge: Vec<NodeId> = (0..200u8).map(node_id).collect(); // 200 > MAX_REPLICATED_TO
        let payload = ReplicationPayload {
            message_id: "msg-dos".into(),
            payload: vec![1, 2, 3],
            recipient_id: node_id(201),
            sender_id: node_id(202),
            expires_at: 20_000,
            viability_score: 50,
            replicated_to: huge,
        };
        store.store_replica(&payload, true, 15_000);
        let entry = store.get("msg-dos").unwrap();
        assert!(
            entry.replicated_to.len() <= MAX_REPLICATED_TO,
            "replicated_to doit être borné à {MAX_REPLICATED_TO}, obtenu {}",
            entry.replicated_to.len()
        );
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

        let events = store.store_replica(&payload, true, 15_000); // Already expired
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
