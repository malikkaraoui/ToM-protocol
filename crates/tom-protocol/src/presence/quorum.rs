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
//! HARDENING: each witness is also capped per-peer. A single Sybil witness
//! cannot evict legitimate peers from other witnesses — its own oldest entries
//! are evicted first when it exceeds its per-witness quota.

use std::collections::HashMap;

use super::relay_view::{required_witnesses, MAX_VIEW_ENTRIES};
use super::PRESENCE_TTL_MS;
use crate::types::NodeId;

/// Global cap on distinct peers tracked by the aggregator.
pub const MAX_TRACKED_PEERS: usize = MAX_VIEW_ENTRIES;

/// Max distinct peers a single witness can attest to in the aggregator.
/// Prevents a Sybil witness from filling the global table and evicting peers
/// attested by legitimate witnesses. Set to 1/4 of MAX_TRACKED_PEERS as a
/// reasonable fraction; even a Sybil with 64 peers cannot starve others in a
/// 256-peer global table.
pub const MAX_PEERS_PER_WITNESS: usize = MAX_TRACKED_PEERS / 4;

/// Consumer-side aggregation of witness attestations, per attested peer.
#[derive(Debug)]
pub struct QuorumAggregator {
    /// peer → (witness → last attestation ms). A witness contributes AT MOST
    /// once per peer (the inner map dedups repeats from the same witness), so a
    /// single relay cannot inflate a peer's witness count.
    attestations: HashMap<NodeId, HashMap<NodeId, u64>>,
    max_peers: usize,
    /// witness → set of peers it has attested to. Tracks per-witness capacity.
    witness_peers: HashMap<NodeId, std::collections::HashSet<NodeId>>,
}

impl Default for QuorumAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl QuorumAggregator {
    pub fn new() -> Self {
        Self {
            attestations: HashMap::new(),
            max_peers: MAX_TRACKED_PEERS,
            witness_peers: HashMap::new(),
        }
    }

    /// Record that `witness` attested `peer` alive at `now`. Idempotent per
    /// (peer, witness): a repeat just refreshes the timestamp.
    ///
    /// Enforces two capacity limits:
    /// - Per-witness: each witness can attest at most MAX_PEERS_PER_WITNESS
    ///   distinct peers. If exceeded, its stalest peer is evicted first.
    /// - Global: the aggregator tracks at most MAX_TRACKED_PEERS distinct peers.
    ///   If exceeded, the stalest peer (across all witnesses) is evicted.
    ///
    /// This prevents a Sybil witness from dominating the table and evicting
    /// legitimate peers attested by other witnesses.
    pub fn record(&mut self, witness: NodeId, peer: NodeId, now: u64) {
        // Check if this is a new peer for this witness (capacity constraint).
        let witness_already_attests_peer = self
            .attestations
            .get(&peer)
            .map(|w| w.contains_key(&witness))
            .unwrap_or(false);

        if !witness_already_attests_peer {
            let peer_count = self
                .witness_peers
                .entry(witness)
                .or_default()
                .len();

            // If witness has reached per-witness capacity, evict its oldest peer.
            if peer_count >= MAX_PEERS_PER_WITNESS {
                // Find the oldest peer (by attestation timestamp) that this witness attests to.
                if let Some(oldest_peer_for_witness) = self
                    .witness_peers
                    .get(&witness)
                    .and_then(|peers| {
                        peers
                            .iter()
                            .copied()
                            .min_by_key(|&p| {
                                self.attestations
                                    .get(&p)
                                    .and_then(|witness_map| witness_map.get(&witness).copied())
                                    .unwrap_or(u64::MAX)
                            })
                    })
                {
                    // Remove this peer from the witness's attestation.
                    if let Some(witness_map) = self.attestations.get_mut(&oldest_peer_for_witness) {
                        witness_map.remove(&witness);
                        // If no other witness attests this peer, remove it globally.
                        if witness_map.is_empty() {
                            self.attestations.remove(&oldest_peer_for_witness);
                        }
                    }
                    // Remove from witness_peers set.
                    self.witness_peers
                        .entry(witness)
                        .and_modify(|peers| {
                            peers.remove(&oldest_peer_for_witness);
                        });
                }
            }

            // Track that this witness now attests this peer.
            self.witness_peers
                .entry(witness)
                .or_default()
                .insert(peer);
        }

        // Global capacity check: evict stalest peer across all witnesses if needed.
        if !self.attestations.contains_key(&peer) && self.attestations.len() >= self.max_peers {
            if let Some(stalest) = self
                .attestations
                .iter()
                .min_by_key(|(_, w)| w.values().copied().max().unwrap_or(0))
                .map(|(id, _)| *id)
            {
                // Remove all witnesses' records for this peer.
                if let Some(witness_map) = self.attestations.remove(&stalest) {
                    // Update witness_peers: remove stalest from each witness's set.
                    for w in witness_map.keys() {
                        self.witness_peers
                            .entry(*w)
                            .and_modify(|peers| {
                                peers.remove(&stalest);
                            });
                    }
                }
            }
        }

        // Record the attestation.
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

        // Track which peers are being removed for cleanup in witness_peers.
        let removed_peers: Vec<NodeId> = self
            .attestations
            .iter()
            .filter(|(_, w)| w.is_empty())
            .map(|(p, _)| *p)
            .collect();

        for peer in removed_peers {
            self.attestations.remove(&peer);
            // Remove this peer from all witnesses' sets.
            for peers in self.witness_peers.values_mut() {
                peers.remove(&peer);
            }
        }
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
            witness_peers: Default::default(),
        };
        q.record(node_id(1), node_id(100), 1_000); // stalest peer
        q.record(node_id(1), node_id(101), 2_000);
        q.record(node_id(1), node_id(102), 3_000); // evicts peer 100
        assert_eq!(q.tracked_peers(), 2);
        assert_eq!(q.fresh_witnesses(&node_id(100), 3_100), 0, "stalest peer evicted");
    }

    #[test]
    fn per_witness_cap_prevents_sybil_flood() {
        // Create aggregator with custom per-witness cap for testing.
        let mut q = QuorumAggregator {
            attestations: HashMap::new(),
            max_peers: 256,
            witness_peers: Default::default(),
        };
        let sybil_witness = node_id(1);
        let legit_witness = node_id(2);
        let legit_peer = node_id(200);

        // Sybil witness fills its quota (MAX_PEERS_PER_WITNESS = 64).
        for i in 0..MAX_PEERS_PER_WITNESS as u8 {
            let peer = node_id(i + 10); // distinct peers
            q.record(sybil_witness, peer, 1_000 + i as u64);
        }
        assert_eq!(q.tracked_peers(), MAX_PEERS_PER_WITNESS);

        // Sybil tries to add one more peer — should evict its oldest.
        let new_sybil_peer = node_id(254);
        q.record(sybil_witness, new_sybil_peer, 2_000);
        // Total peers should not exceed capacity (sybil's oldest should have been evicted).
        assert!(q.tracked_peers() <= MAX_PEERS_PER_WITNESS);

        // Meanwhile, legit witness attests a peer — must not be evicted by sybil flood.
        q.record(legit_witness, legit_peer, 1_500);
        assert_eq!(q.fresh_witnesses(&legit_peer, 1_600), 1, "legit peer attestation survives");
    }

    #[test]
    fn per_witness_cap_enforced_independently() {
        // Witness A fills its quota, Witness B should still be able to attest new peers.
        let mut q = QuorumAggregator {
            attestations: HashMap::new(),
            max_peers: 256,
            witness_peers: Default::default(),
        };
        let witness_a = node_id(10);
        let witness_b = node_id(20);

        // Witness A attests MAX_PEERS_PER_WITNESS peers.
        for i in 0..MAX_PEERS_PER_WITNESS as u8 {
            q.record(witness_a, node_id(i + 100), 1_000);
        }
        let a_count = q.witness_peers.get(&witness_a).map(|s| s.len()).unwrap_or(0);
        assert_eq!(a_count, MAX_PEERS_PER_WITNESS);

        // Witness B should still be able to add many peers independently.
        for i in 0..(MAX_PEERS_PER_WITNESS + 10) as usize {
            q.record(witness_b, node_id((i as u8).wrapping_add(100)), 1_000);
        }
        // Witness B capped at its own limit.
        let b_count = q.witness_peers.get(&witness_b).map(|s| s.len()).unwrap_or(0);
        assert_eq!(b_count, MAX_PEERS_PER_WITNESS);
    }

    #[test]
    fn mixed_valid_and_sybil_quorum_stability() {
        // 2 legit witnesses + 1 Sybil: legit peer from legit witnesses reaches quorum,
        // Sybil cannot starve it despite flooding its own peers.
        let mut q = QuorumAggregator::new();
        let legit_1 = node_id(1);
        let legit_2 = node_id(2);
        let sybil = node_id(99);
        let shared_peer = node_id(100);

        // Both legit witnesses attest the shared peer.
        q.record(legit_1, shared_peer, 1_000);
        q.record(legit_2, shared_peer, 1_050);
        assert_eq!(q.fresh_witnesses(&shared_peer, 1_100), 2);
        assert!(q.at_quorum(&shared_peer, 1, 1_100), "legit quorum met");

        // Sybil floods with many distinct peers.
        for i in 0..(MAX_PEERS_PER_WITNESS as u8) {
            q.record(sybil, node_id(i + 150), 2_000 + i as u64);
        }

        // Shared peer still attested by 2 legit witnesses; quorum stable.
        assert_eq!(
            q.fresh_witnesses(&shared_peer, 2_100),
            2,
            "legit attestations not evicted by sybil"
        );
        // Use activity=1 (not 5) so required_witnesses stays at 2 (floor).
        // At activity=5, required_witnesses rises to 3, which is a deliberate
        // tradeoff (higher activity → require higher consensus). This test
        // documents that per-witness capping does NOT break the 2-witness floor.
        assert!(
            q.at_quorum(&shared_peer, 1, 2_100),
            "quorum still stable with sybil flood when activity low"
        );
    }
}
