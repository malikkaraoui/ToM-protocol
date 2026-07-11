//! L1-003 — relay-side subscription table (D3, explicit subscription).
//!
//! A weak device tells its relay(s) which peers/groups it cares about (its
//! `PresenceScope`). The relay remembers `subscriber → scope` and, at each
//! publish tick, builds+signs+pushes a `RelayPresenceView` scoped to that
//! subscription (D2, push). No NEW privacy leak: the relay already sees
//! `from`/`to` of the traffic it routes (see the design doc). The
//! scope-known-to-relay concern is a NOTED future hardening item, not closed
//! here.
//!
//! Pure logic — `u64` ms injected, no `Instant` — deterministic under the
//! effect pattern. Memory bounded GLOBALLY (lesson 347421b): a swarm of fake
//! subscribers must never grow the table without bound; at capacity the
//! stalest subscription is evicted, and everything self-purges by TTL.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::relay_view::PresenceScope;
use crate::error::TomProtocolError;
use crate::types::NodeId;

/// Payload of a `MessageType::PresenceSubscribe` envelope (weak device → relay).
/// The subscriber declares the scope it wants presence views for. Wrapped in a
/// struct (not a bare `PresenceScope`) so the wire format can grow without a
/// breaking change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceSubscribePayload {
    pub scope: PresenceScope,
}

impl PresenceSubscribePayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        rmp_serde::to_vec(self).expect("PresenceSubscribePayload serialization cannot fail")
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, TomProtocolError> {
        rmp_serde::from_slice(data).map_err(Into::into)
    }
}

/// How long a subscription stays live without a refresh. Longer than the 30s
/// presence tick so a weak device need not re-subscribe every round, short
/// enough that a departed subscriber is forgotten promptly (bounds the table
/// and stops publishing to the absent). Tunable.
pub const SUBSCRIPTION_TTL_MS: u64 = 120_000;

/// Hard cap on tracked subscribers (global memory bound).
pub const MAX_SUBSCRIBERS: usize = 1024;

#[derive(Debug, Clone)]
struct Subscription {
    scope: PresenceScope,
    refreshed_at_ms: u64,
}

/// Relay-side table of who subscribes to our presence views, and to what scope.
#[derive(Debug, Default)]
pub struct SubscriptionTable {
    subs: HashMap<NodeId, Subscription>,
    max_subscribers: usize,
}

impl SubscriptionTable {
    pub fn new() -> Self {
        Self {
            subs: HashMap::new(),
            max_subscribers: MAX_SUBSCRIBERS,
        }
    }

    /// Record or refresh a subscription. Supersedes any prior scope for the
    /// same subscriber (a weak device can change what it cares about). At
    /// capacity, evicts the stalest subscription (oldest refresh) to admit a
    /// newcomer — a live subscriber is never starved by a dead one (same
    /// discipline as `Topology::upsert` / `WitnessLog::record`).
    pub fn subscribe(&mut self, subscriber: NodeId, scope: PresenceScope, now: u64) {
        if !self.subs.contains_key(&subscriber) && self.subs.len() >= self.max_subscribers {
            if let Some(stalest) = self
                .subs
                .iter()
                .min_by_key(|(_, s)| s.refreshed_at_ms)
                .map(|(id, _)| *id)
            {
                self.subs.remove(&stalest);
            }
        }
        self.subs.insert(
            subscriber,
            Subscription {
                scope,
                refreshed_at_ms: now,
            },
        );
    }

    /// Explicit unsubscribe (a weak device leaving / changing relay).
    pub fn unsubscribe(&mut self, subscriber: &NodeId) {
        self.subs.remove(subscriber);
    }

    /// Drop subscriptions past their TTL. Deterministic on `now`.
    pub fn purge_expired(&mut self, now: u64) {
        self.subs
            .retain(|_, s| now.saturating_sub(s.refreshed_at_ms) < SUBSCRIPTION_TTL_MS);
    }

    /// Live (non-expired) subscriptions at `now`, as `(subscriber, scope)` —
    /// what the publish tick iterates to build one view per subscriber.
    pub fn active(&self, now: u64) -> Vec<(NodeId, PresenceScope)> {
        self.subs
            .iter()
            .filter(|(_, s)| now.saturating_sub(s.refreshed_at_ms) < SUBSCRIPTION_TTL_MS)
            .map(|(id, s)| (*id, s.scope.clone()))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.subs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.subs.is_empty()
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

    fn peers_scope(seed: u8) -> PresenceScope {
        PresenceScope::Peers(vec![node_id(seed)])
    }

    #[test]
    fn subscribe_then_active() {
        let mut t = SubscriptionTable::new();
        t.subscribe(node_id(2), peers_scope(3), 1_000);
        let active = t.active(1_100);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].0, node_id(2));
    }

    #[test]
    fn refresh_supersedes_scope() {
        let mut t = SubscriptionTable::new();
        t.subscribe(node_id(2), peers_scope(3), 1_000);
        t.subscribe(node_id(2), peers_scope(4), 1_500);
        assert_eq!(t.len(), 1, "same subscriber keeps one entry");
        let active = t.active(1_600);
        assert_eq!(active[0].1, peers_scope(4), "latest scope wins");
    }

    #[test]
    fn unsubscribe_removes() {
        let mut t = SubscriptionTable::new();
        t.subscribe(node_id(2), peers_scope(3), 1_000);
        t.unsubscribe(&node_id(2));
        assert!(t.is_empty());
    }

    #[test]
    fn expired_excluded_from_active() {
        let mut t = SubscriptionTable::new();
        t.subscribe(node_id(2), peers_scope(3), 1_000);
        assert!(t.active(1_000 + SUBSCRIPTION_TTL_MS).is_empty());
        assert!(!t.active(1_000 + SUBSCRIPTION_TTL_MS - 1).is_empty());
    }

    #[test]
    fn purge_drops_expired() {
        let mut t = SubscriptionTable::new();
        t.subscribe(node_id(2), peers_scope(3), 1_000);
        t.subscribe(node_id(3), peers_scope(4), 1_000 + SUBSCRIPTION_TTL_MS - 1);
        t.purge_expired(1_000 + SUBSCRIPTION_TTL_MS);
        assert_eq!(t.len(), 1, "only the still-fresh subscription survives");
    }

    #[test]
    fn capacity_evicts_stalest() {
        let mut t = SubscriptionTable {
            subs: HashMap::new(),
            max_subscribers: 2,
        };
        t.subscribe(node_id(2), peers_scope(9), 1_000); // stalest
        t.subscribe(node_id(3), peers_scope(9), 2_000);
        t.subscribe(node_id(4), peers_scope(9), 3_000); // evicts node 2
        assert_eq!(t.len(), 2);
        let ids: Vec<NodeId> = t.active(3_100).into_iter().map(|(id, _)| id).collect();
        assert!(!ids.contains(&node_id(2)), "stalest subscriber evicted");
        assert!(ids.contains(&node_id(3)) && ids.contains(&node_id(4)));
    }
}
