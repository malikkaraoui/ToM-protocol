//! L1-003 — witness (relay) side: accumulate first-hand presence observations
//! and build a (yet-unsigned) `RelayPresenceView` for publication.
//!
//! A relay is a witness because the network put it on a routing path (LOCKED
//! #6) — never by self-appointment. As it relays signed ACKs it learns, at
//! first hand, that a peer was alive at a timestamp (ADR-011 §2: "vu-par-X",
//! never hearsay). This module holds those observations, bounded and
//! self-purging, and packs them into a view. Signing happens one layer up
//! (the runtime owns the secret key); this module is pure, deterministic
//! logic — `u64` ms injected, no `Instant`, testable under the effect pattern.
//!
//! Memory is bounded GLOBALLY, not per-peer (lesson 347421b): a scope is small
//! by construction, but a misbehaving/compromised feed must never grow the log
//! without bound. At capacity, the stalest observation is evicted.

use std::collections::HashMap;

use super::relay_view::{PresenceEntry, PresenceScope, RelayPresenceView, MAX_VIEW_ENTRIES};
use super::PRESENCE_TTL_MS;
use crate::router::AckType;
use crate::types::NodeId;

/// Accumulates a witness's first-hand presence observations, keyed by peer.
///
/// Only the MOST RECENT observation per peer is kept — presence is a "seen
/// alive at" fact, older evidence for the same peer is superseded.
#[derive(Debug, Default)]
pub struct WitnessLog {
    observations: HashMap<NodeId, PresenceEntry>,
    /// Hard cap on distinct peers tracked (global memory bound).
    max_entries: usize,
}

impl WitnessLog {
    pub fn new() -> Self {
        Self {
            observations: HashMap::new(),
            max_entries: MAX_VIEW_ENTRIES,
        }
    }

    /// Record a first-hand observation that `peer_id` was alive at `now`,
    /// evidenced by a REAL signed ACK (`proof_ref` = its message id) with its
    /// raw cryptographic proof (`ack_proof_bytes` = the signed Envelope). The
    /// caller (runtime) is responsible for only feeding genuine evidence — the
    /// wiring anchors this on ACKs the witness actually relayed.
    ///
    /// Supersedes any older observation for the same peer. At capacity, evicts
    /// the stalest peer (oldest `seen_at_ms`) to admit the newcomer, so a fresh
    /// live peer is never starved by dead entries (same discipline as
    /// `Topology::upsert`).
    pub fn record(
        &mut self,
        peer_id: NodeId,
        proof_ref: String,
        proof_type: AckType,
        now: u64,
        ack_proof_bytes: Vec<u8>,
    ) {
        let entry = PresenceEntry {
            peer_id,
            proof_ref,
            proof_type,
            seen_at_ms: now,
            ack_proof: ack_proof_bytes,
        };
        if !self.observations.contains_key(&peer_id) && self.observations.len() >= self.max_entries {
            if let Some(stalest) = self
                .observations
                .values()
                .min_by_key(|e| e.seen_at_ms)
                .map(|e| e.peer_id)
            {
                self.observations.remove(&stalest);
            }
        }
        self.observations.insert(peer_id, entry);
    }

    /// Drop observations older than the presence TTL (30s). Deterministic —
    /// driven entirely by the injected `now`.
    pub fn purge_expired(&mut self, now: u64) {
        self.observations
            .retain(|_, e| now.saturating_sub(e.seen_at_ms) < PRESENCE_TTL_MS);
    }

    /// Number of live observations currently held.
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    /// Build an UNSIGNED view of the fresh observations, restricted to `scope`.
    /// The caller signs it (owns the secret key) before publication.
    ///
    /// Returns `None` when no in-scope, non-expired observation exists — a view
    /// with no entries carries no signal and must not be emitted (mirrors
    /// `RelayPresenceView::validate`).
    ///
    /// `now` purges expired entries first, so a stale observation never leaks
    /// into a fresh view even if `purge_expired` has not run yet.
    pub fn build_view(
        &self,
        witness_id: NodeId,
        scope: PresenceScope,
        now: u64,
    ) -> Option<RelayPresenceView> {
        let in_scope: Box<dyn Fn(&NodeId) -> bool> = match &scope {
            // Group scope: membership is not known at this layer; the runtime
            // resolves group→members and passes a Peers scope. A bare Group
            // scope here means "no explicit filter" and includes all.
            PresenceScope::Group(_) => Box::new(|_| true),
            PresenceScope::Peers(peers) => {
                let set: std::collections::HashSet<NodeId> = peers.iter().copied().collect();
                Box::new(move |id| set.contains(id))
            }
        };

        let mut present: Vec<PresenceEntry> = self
            .observations
            .values()
            .filter(|e| now.saturating_sub(e.seen_at_ms) < PRESENCE_TTL_MS)
            .filter(|e| in_scope(&e.peer_id))
            .cloned()
            .collect();

        if present.is_empty() {
            return None;
        }
        // Deterministic order (by peer_id bytes) so the same observation set
        // yields the same signing bytes regardless of HashMap iteration order.
        present.sort_by_key(|e| e.peer_id.as_bytes());

        Some(RelayPresenceView {
            witness_id,
            epoch_ms: now,
            scope,
            present,
            signature: Vec::new(),
        })
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
    fn record_then_build_view() {
        let mut log = WitnessLog::new();
        log.record(
            node_id(2),
            "m-2".into(),
            AckType::RecipientReceived,
            1_000,
            Vec::new(),
        );
        log.record(
            node_id(3),
            "m-3".into(),
            AckType::RelayForwarded,
            1_100,
            Vec::new(),
        );

        let view = log
            .build_view(node_id(1), PresenceScope::Group(crate::group::GroupId("g".into())), 1_200)
            .expect("view with 2 fresh entries");
        assert_eq!(view.present.len(), 2);
        assert_eq!(view.witness_id, node_id(1));
        assert!(view.signature.is_empty(), "witness layer leaves signing to runtime");
    }

    #[test]
    fn latest_observation_supersedes() {
        let mut log = WitnessLog::new();
        log.record(
            node_id(2),
            "m-old".into(),
            AckType::RelayForwarded,
            1_000,
            Vec::new(),
        );
        log.record(
            node_id(2),
            "m-new".into(),
            AckType::RecipientReceived,
            1_500,
            Vec::new(),
        );
        assert_eq!(log.len(), 1, "same peer keeps one entry");
        let view = log
            .build_view(node_id(1), PresenceScope::Group(crate::group::GroupId("g".into())), 1_600)
            .unwrap();
        assert_eq!(view.present[0].proof_ref, "m-new");
        assert_eq!(view.present[0].proof_type, AckType::RecipientReceived);
    }

    #[test]
    fn expired_observations_excluded_from_view() {
        let mut log = WitnessLog::new();
        log.record(
            node_id(2),
            "m-2".into(),
            AckType::RecipientReceived,
            1_000,
            Vec::new(),
        );
        // now is > TTL after the observation → excluded.
        assert!(log
            .build_view(node_id(1), PresenceScope::Group(crate::group::GroupId("g".into())), 1_000 + PRESENCE_TTL_MS)
            .is_none());
    }

    #[test]
    fn purge_expired_drops_stale() {
        let mut log = WitnessLog::new();
        log.record(
            node_id(2),
            "m-2".into(),
            AckType::RecipientReceived,
            1_000,
            Vec::new(),
        );
        log.record(
            node_id(3),
            "m-3".into(),
            AckType::RecipientReceived,
            1_000 + PRESENCE_TTL_MS - 1,
            Vec::new(),
        );
        log.purge_expired(1_000 + PRESENCE_TTL_MS);
        assert_eq!(log.len(), 1, "only the still-fresh observation survives");
    }

    #[test]
    fn scope_peers_filters() {
        let mut log = WitnessLog::new();
        log.record(
            node_id(2),
            "m-2".into(),
            AckType::RecipientReceived,
            1_000,
            Vec::new(),
        );
        log.record(
            node_id(3),
            "m-3".into(),
            AckType::RecipientReceived,
            1_000,
            Vec::new(),
        );
        // Only ask about peer 2.
        let view = log
            .build_view(node_id(1), PresenceScope::Peers(vec![node_id(2)]), 1_100)
            .unwrap();
        assert_eq!(view.present.len(), 1);
        assert_eq!(view.present[0].peer_id, node_id(2));
    }

    #[test]
    fn empty_or_all_expired_yields_no_view() {
        let log = WitnessLog::new();
        assert!(log
            .build_view(node_id(1), PresenceScope::Peers(vec![node_id(2)]), 1_000)
            .is_none());
    }

    #[test]
    fn capacity_evicts_stalest() {
        let mut log = WitnessLog {
            observations: HashMap::new(),
            max_entries: 2,
        };
        log.record(
            node_id(2),
            "m-2".into(),
            AckType::RelayForwarded,
            1_000,
            Vec::new(),
        ); // stalest
        log.record(
            node_id(3),
            "m-3".into(),
            AckType::RelayForwarded,
            2_000,
            Vec::new(),
        );
        log.record(
            node_id(4),
            "m-4".into(),
            AckType::RelayForwarded,
            3_000,
            Vec::new(),
        ); // evicts peer 2
        assert_eq!(log.len(), 2);
        let view = log
            .build_view(node_id(1), PresenceScope::Group(crate::group::GroupId("g".into())), 3_100)
            .unwrap();
        let ids: Vec<NodeId> = view.present.iter().map(|e| e.peer_id).collect();
        assert!(!ids.contains(&node_id(2)), "stalest peer evicted");
        assert!(ids.contains(&node_id(3)) && ids.contains(&node_id(4)));
    }

    #[test]
    fn view_present_is_deterministically_ordered() {
        let mut log = WitnessLog::new();
        for s in [5u8, 2, 9, 3] {
            log.record(
                node_id(s),
                format!("m-{s}"),
                AckType::RelayForwarded,
                1_000,
                Vec::new(),
            );
        }
        let v1 = log
            .build_view(node_id(1), PresenceScope::Group(crate::group::GroupId("g".into())), 1_100)
            .unwrap();
        // Sorted by peer_id bytes → same order every build.
        let mut sorted = v1.present.clone();
        sorted.sort_by_key(|e| e.peer_id.as_bytes());
        assert_eq!(v1.present, sorted);
    }
}
