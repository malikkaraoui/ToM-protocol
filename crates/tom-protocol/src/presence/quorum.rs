//! L1-003 — consumer-side quorum aggregator (the defensive heart).
//!
//! A weak device receives signed presence views from several witnesses (its
//! relays). A signature proves WHO published a view, never that its content is
//! TRUE — a single relay on the device's path can be Sybil and lie (eclipse,
//! red-team kill-shot #3). So a peer is promoted `Known → Online` ONLY when a
//! QUORUM of ≥ N DISTINCT witnesses concur that it is alive within the
//! freshness window. One witness — even repeating — never promotes.
//!
//! N is dynamic (`required_witnesses`): it rises with the consumer's witness
//! density and view activity (2 → 3 → 4), floors at 2, and never exceeds the
//! witnesses actually available.
//!
//! Pure logic — `u64` ms injected, no `Instant`. Memory bounded GLOBALLY
//! (lesson 347421b): a flood of views from forged witnesses must never grow
//! the maps without bound; at capacity the stalest peer is evicted, and
//! everything self-purges by TTL.
//!
//! NOTE (future hardening, tracked): each `PresenceEntry.proof_ref` is the
//! witness's ASSERTED evidence (a real signed ACK id). Full spot-check of a
//! proof the consumer did not itself observe is deferred; the current defense
//! is signed-view + distinct-witness quorum. Do not claim proof_ref is
//! independently verified here.

use std::collections::HashMap;

use super::relay_view::{required_witnesses, MAX_VIEW_ENTRIES};
use super::PRESENCE_TTL_MS;
use crate::types::NodeId;

/// Global cap on distinct peers tracked by the aggregator.
pub const MAX_TRACKED_PEERS: usize = MAX_VIEW_ENTRIES;

/// Consumer-side aggregation of witness attestations, per attested peer.
#[derive(Debug, Default)]
pub struct QuorumAggregator {
    /// peer → (witness → last attestation ms). A witness contributes AT MOST
    /// once per peer (the inner map dedups repeats from the same witness), so a
    /// single relay cannot inflate a peer's witness count.
    attestations: HashMap<NodeId, HashMap<NodeId, u64>>,
    max_peers: usize,
}

impl QuorumAggregator {
    pub fn new() -> Self {
        Self {
            attestations: HashMap::new(),
            max_peers: MAX_TRACKED_PEERS,
        }
    }

    /// Record that `witness` attested `peer` alive at `now`. Idempotent per
    /// (peer, witness): a repeat just refreshes the timestamp. At capacity,
    /// evicts the stalest peer (oldest freshest-attestation) to admit a
    /// newcomer.
    pub fn record(&mut self, witness: NodeId, peer: NodeId, now: u64) {
        if !self.attestations.contains_key(&peer) && self.attestations.len() >= self.max_peers {
            if let Some(stalest) = self
                .attestations
                .iter()
                .min_by_key(|(_, w)| w.values().copied().max().unwrap_or(0))
                .map(|(id, _)| *id)
            {
                self.attestations.remove(&stalest);
            }
        }
        self.attestations.entry(peer).or_default().insert(witness, now);
    }

    /// Number of DISTINCT witnesses that attested `peer` within the freshness
    /// window. Stale attestations do not count.
    pub fn fresh_witnesses(&self, peer: &NodeId, now: u64) -> usize {
        self.attestations
            .get(peer)
            .map(|w| {
                w.values()
                    .filter(|&&ts| now.saturating_sub(ts) < PRESENCE_TTL_MS)
                    .count()
            })
            .unwrap_or(0)
    }

    /// Total distinct witnesses the consumer has heard fresh attestations from,
    /// across all peers — the witness DENSITY that drives the dynamic quorum.
    pub fn distinct_witnesses(&self, now: u64) -> usize {
        let mut set = std::collections::HashSet::new();
        for w in self.attestations.values() {
            for (witness, &ts) in w {
                if now.saturating_sub(ts) < PRESENCE_TTL_MS {
                    set.insert(*witness);
                }
            }
        }
        set.len()
    }

    /// Is `peer` at quorum, i.e. attested by ≥ `required_witnesses` DISTINCT
    /// fresh witnesses? `activity` (fresh views observed this window) is passed
    /// in by the caller. Density is derived from the aggregator itself.
    pub fn at_quorum(&self, peer: &NodeId, activity: usize, now: u64) -> bool {
        let density = self.distinct_witnesses(now);
        let required = required_witnesses(density, activity);
        self.fresh_witnesses(peer, now) >= required
    }

    /// Drop attestations past the TTL; forget peers left with no fresh witness.
    pub fn purge_expired(&mut self, now: u64) {
        for w in self.attestations.values_mut() {
            w.retain(|_, ts| now.saturating_sub(*ts) < PRESENCE_TTL_MS);
        }
        self.attestations.retain(|_, w| !w.is_empty());
    }

    pub fn tracked_peers(&self) -> usize {
        self.attestations.len()
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
    fn single_witness_never_reaches_quorum() {
        let mut q = QuorumAggregator::new();
        let peer = node_id(100);
        // One witness attests the peer many times — must NOT promote (floor 2).
        q.record(node_id(1), peer, 1_000);
        q.record(node_id(1), peer, 1_100);
        q.record(node_id(1), peer, 1_200);
        assert_eq!(q.fresh_witnesses(&peer, 1_300), 1, "same witness counts once");
        assert!(!q.at_quorum(&peer, 10, 1_300), "one witness can never eclipse-promote");
    }

    #[test]
    fn two_distinct_witnesses_reach_quorum() {
        let mut q = QuorumAggregator::new();
        let peer = node_id(100);
        q.record(node_id(1), peer, 1_000);
        q.record(node_id(2), peer, 1_050);
        assert_eq!(q.fresh_witnesses(&peer, 1_100), 2);
        // Sparse network (density 2, low activity) → required = floor 2 → met.
        assert!(q.at_quorum(&peer, 1, 1_100));
    }

    #[test]
    fn expired_attestation_drops_below_quorum() {
        let mut q = QuorumAggregator::new();
        let peer = node_id(100);
        q.record(node_id(1), peer, 1_000);
        q.record(node_id(2), peer, 1_000);
        assert!(q.at_quorum(&peer, 1, 1_100));
        // Advance past TTL for one witness only (refresh the other).
        q.record(node_id(2), peer, 1_000 + PRESENCE_TTL_MS);
        let now = 1_000 + PRESENCE_TTL_MS + 1; // witness 1 now stale
        assert_eq!(q.fresh_witnesses(&peer, now), 1);
        assert!(!q.at_quorum(&peer, 1, now), "losing a witness drops below quorum");
    }

    #[test]
    fn dynamic_quorum_rises_with_density_and_activity() {
        let mut q = QuorumAggregator::new();
        let peer = node_id(100);
        // 3 distinct witnesses attest the peer; density will be 3.
        q.record(node_id(1), peer, 1_000);
        q.record(node_id(2), peer, 1_000);
        q.record(node_id(3), peer, 1_000);
        // Dense + active → required rises to 3, exactly met by 3 witnesses.
        assert_eq!(q.distinct_witnesses(1_050), 3);
        assert!(q.at_quorum(&peer, 5, 1_050));
        // If only 2 of the 3 had attested THIS peer while density stays 3,
        // required 3 would NOT be met — check via a second peer.
        let peer2 = node_id(200);
        q.record(node_id(1), peer2, 1_000);
        q.record(node_id(2), peer2, 1_000);
        assert_eq!(q.fresh_witnesses(&peer2, 1_050), 2);
        assert!(!q.at_quorum(&peer2, 5, 1_050), "2 witnesses < required 3 in a dense/active net");
    }

    #[test]
    fn purge_forgets_fully_stale_peer() {
        let mut q = QuorumAggregator::new();
        let peer = node_id(100);
        q.record(node_id(1), peer, 1_000);
        q.record(node_id(2), peer, 1_000);
        q.purge_expired(1_000 + PRESENCE_TTL_MS);
        assert_eq!(q.tracked_peers(), 0, "peer with no fresh witness is forgotten");
    }

    #[test]
    fn capacity_evicts_stalest_peer() {
        let mut q = QuorumAggregator {
            attestations: HashMap::new(),
            max_peers: 2,
        };
        q.record(node_id(1), node_id(100), 1_000); // stalest peer
        q.record(node_id(1), node_id(101), 2_000);
        q.record(node_id(1), node_id(102), 3_000); // evicts peer 100
        assert_eq!(q.tracked_peers(), 2);
        assert_eq!(q.fresh_witnesses(&node_id(100), 3_100), 0, "stalest peer evicted");
    }
}
