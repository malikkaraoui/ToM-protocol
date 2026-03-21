//! Relay registry — local store of discovered relay-ready peers.
//!
//! Consumes validated RelayReadyAnnounce data. No crypto, no validation —
//! that's the caller's job. This is pure storage + TTL expiration.

use std::collections::HashMap;
use tom_connect::RelayUrl;

use crate::types::NodeId;

/// Default TTL for relay registry entries (10 minutes).
pub const DEFAULT_RELAY_REGISTRY_TTL_MS: u64 = 10 * 60 * 1000;

/// Result of an upsert operation on the relay registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpsertResult {
    /// New entry inserted (node_id was unknown).
    Inserted,
    /// Existing entry refreshed with the same URL.
    RefreshedSameUrl,
    /// Existing entry updated with a different URL.
    UpdatedUrl {
        old_url: RelayUrl,
    },
}

impl UpsertResult {
    pub fn is_new(&self) -> bool {
        matches!(self, UpsertResult::Inserted)
    }
}

/// A single entry in the relay registry.
#[derive(Debug, Clone)]
pub struct RelayRegistryEntry {
    pub node_id: NodeId,
    pub relay_url: RelayUrl,
    /// Timestamp from the RelayReadyAnnounce (remote clock, observability only).
    pub announced_at: u64,
    /// Local now_ms() at reception — used for TTL expiration.
    pub refreshed_at: u64,
    /// refreshed_at + ttl_ms — deterministic, computed only in upsert().
    pub expires_at: u64,
}

/// Local registry of relays discovered via RelayReadyAnnounce gossip.
///
/// Pure storage + TTL. No crypto, no routing, no auto-selection.
pub struct RelayRegistry {
    entries: HashMap<NodeId, RelayRegistryEntry>,
    ttl_ms: u64,
}

impl RelayRegistry {
    pub fn new(ttl_ms: u64) -> Self {
        Self {
            entries: HashMap::new(),
            ttl_ms,
        }
    }

    /// Upsert a relay entry. Returns rich result for transport decisions.
    pub fn upsert(
        &mut self,
        node_id: NodeId,
        relay_url: RelayUrl,
        announced_at: u64,
        now: u64,
    ) -> UpsertResult {
        let result = match self.entries.get(&node_id) {
            None => UpsertResult::Inserted,
            Some(existing) if existing.relay_url == relay_url => UpsertResult::RefreshedSameUrl,
            Some(existing) => UpsertResult::UpdatedUrl {
                old_url: existing.relay_url.clone(),
            },
        };
        self.entries.insert(
            node_id,
            RelayRegistryEntry {
                node_id,
                relay_url,
                announced_at,
                refreshed_at: now,
                expires_at: now + self.ttl_ms,
            },
        );
        result
    }

    /// Remove expired entries. Returns the removed entries (for event emission).
    pub fn prune(&mut self, now: u64) -> Vec<RelayRegistryEntry> {
        let mut expired = Vec::new();
        self.entries.retain(|_, entry| {
            if entry.expires_at < now {
                expired.push(entry.clone());
                false
            } else {
                true
            }
        });
        expired
    }

    pub fn get(&self, node_id: &NodeId) -> Option<&RelayRegistryEntry> {
        self.entries.get(node_id)
    }

    pub fn all(&self) -> impl Iterator<Item = &RelayRegistryEntry> {
        self.entries.values()
    }

    /// Returns true if at least one active entry points to this URL.
    pub fn has_active_url(&self, url: &RelayUrl) -> bool {
        self.entries.values().any(|e| &e.relay_url == url)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for RelayRegistry {
    fn default() -> Self {
        Self::new(DEFAULT_RELAY_REGISTRY_TTL_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_url() -> RelayUrl {
        "http://127.0.0.1:3340".parse().unwrap()
    }

    fn test_node_id(seed: u64) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn new_registry_is_empty() {
        let reg = RelayRegistry::new(600_000);
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn upsert_new_returns_inserted() {
        let mut reg = RelayRegistry::new(600_000);
        let id = test_node_id(1);
        let result = reg.upsert(id, test_url(), 1000, 2000);
        assert_eq!(result, UpsertResult::Inserted);
        assert!(result.is_new());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn upsert_same_url_returns_refreshed() {
        let mut reg = RelayRegistry::new(600_000);
        let id = test_node_id(1);
        let url = test_url();
        reg.upsert(id, url.clone(), 1000, 2000);
        let result = reg.upsert(id, url, 3000, 4000);
        assert_eq!(result, UpsertResult::RefreshedSameUrl);
        assert!(!result.is_new());
    }

    #[test]
    fn upsert_different_url_returns_updated() {
        let mut reg = RelayRegistry::new(600_000);
        let id = test_node_id(1);
        let url1 = test_url();
        let url2: RelayUrl = "http://10.0.0.1:4444".parse().unwrap();
        reg.upsert(id, url1.clone(), 1000, 2000);
        let result = reg.upsert(id, url2.clone(), 3000, 4000);
        assert_eq!(result, UpsertResult::UpdatedUrl { old_url: url1 });
        let entry = reg.get(&id).unwrap();
        assert_eq!(entry.relay_url, url2);
        assert_eq!(entry.refreshed_at, 4000);
    }

    #[test]
    fn upsert_sets_expires_at() {
        let mut reg = RelayRegistry::new(600_000);
        let id = test_node_id(1);
        reg.upsert(id, test_url(), 1000, 2000);
        let entry = reg.get(&id).unwrap();
        assert_eq!(entry.expires_at, 2000 + 600_000);
    }

    #[test]
    fn prune_removes_expired_entries() {
        let mut reg = RelayRegistry::new(1000); // 1s TTL
        let id1 = test_node_id(1);
        let id2 = test_node_id(2);
        reg.upsert(id1, test_url(), 100, 100);
        reg.upsert(id2, "http://10.0.0.1:5555".parse().unwrap(), 200, 200);

        // At t=1200, id1 expired (100+1000=1100 < 1200), id2 still alive (200+1000=1200 == 1200)
        let expired = reg.prune(1200);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].node_id, id1);
        assert_eq!(reg.len(), 1);
        assert!(reg.get(&id2).is_some());
    }

    #[test]
    fn prune_returns_empty_when_nothing_expired() {
        let mut reg = RelayRegistry::new(600_000);
        let id = test_node_id(1);
        reg.upsert(id, test_url(), 1000, 1000);
        let expired = reg.prune(2000);
        assert!(expired.is_empty());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn prune_returns_full_entries() {
        let mut reg = RelayRegistry::new(100);
        let id = test_node_id(1);
        let url = test_url();
        reg.upsert(id, url.clone(), 50, 50);
        let expired = reg.prune(200);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].relay_url, url);
        assert_eq!(expired[0].announced_at, 50);
    }

    #[test]
    fn default_uses_10_min_ttl() {
        let reg = RelayRegistry::default();
        let id = test_node_id(1);
        reg.get(&id); // just verify it works
        let mut reg = RelayRegistry::default();
        reg.upsert(id, test_url(), 0, 0);
        let entry = reg.get(&id).unwrap();
        assert_eq!(entry.expires_at, DEFAULT_RELAY_REGISTRY_TTL_MS);
    }

    // ── has_active_url tests ────────────────────────────────────────────

    #[test]
    fn has_active_url_true_when_present() {
        let mut reg = RelayRegistry::new(600_000);
        let url = test_url();
        reg.upsert(test_node_id(1), url.clone(), 100, 100);
        assert!(reg.has_active_url(&url));
    }

    #[test]
    fn has_active_url_false_when_absent() {
        let mut reg = RelayRegistry::new(600_000);
        reg.upsert(test_node_id(1), test_url(), 100, 100);
        let other: RelayUrl = "http://10.0.0.1:9999".parse().unwrap();
        assert!(!reg.has_active_url(&other));
    }

    #[test]
    fn has_active_url_false_after_prune() {
        let mut reg = RelayRegistry::new(100);
        let url = test_url();
        reg.upsert(test_node_id(1), url.clone(), 50, 50);
        reg.prune(200);
        assert!(!reg.has_active_url(&url));
    }

    #[test]
    fn has_active_url_shared_url_survives_single_prune() {
        let mut reg = RelayRegistry::new(1000);
        let url = test_url();
        let id1 = test_node_id(1);
        let id2 = test_node_id(2);
        // id1 inserted at t=100, id2 at t=500 — both point to same URL
        reg.upsert(id1, url.clone(), 100, 100);
        reg.upsert(id2, url.clone(), 500, 500);
        // Prune at t=1200: id1 expires (100+1000=1100 < 1200), id2 alive (500+1000=1500)
        reg.prune(1200);
        assert!(reg.has_active_url(&url), "URL still active via id2");
        assert_eq!(reg.len(), 1);
    }
}
