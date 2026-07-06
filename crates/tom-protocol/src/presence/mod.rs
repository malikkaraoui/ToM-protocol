//! L1-001 — Proof of Presence primitive (challenge → signed attestation).
//!
//! `PresenceManager` holds the ONLY presence state, all of it ephemeral:
//! - `pending`: challenges we (as challenger A) issued, ONE-SHOT — consumed
//!   on acceptance, so a replayed attestation has nothing to match against.
//! - `accepted`: attestations accepted in the current 30s window, kept in
//!   memory only for seed aggregation. NEVER persisted, NEVER backed up
//!   (LOCKED #2).
//! - `responder_windows`: per-challenger response budget (B side), so a
//!   challenge flood cannot extract unlimited Ed25519 signatures from us.
//!
//! All timestamps are `u64` Unix ms injected by the caller — never
//! `Instant` — so purge/TTL behavior is deterministic in tests (same rule
//! as `PeerInfo.last_seen`).
//!
//! Memory is bounded GLOBALLY (not just per-peer): lesson from the DoS
//! hardening pass (347421b) — per-peer caps alone leave collections
//! unbounded under a swarm of attackers. Worst case here:
//! 512 × ~400 B ≈ 200 KB, fine for constrained devices (Apple TV).

pub mod aggregator;
pub mod attestation;

use std::collections::HashMap;

use crate::types::NodeId;

pub use attestation::{
    PresenceAttestationPayload, PresenceChallengePayload, RelayProof, RelayProofType, NONCE_LEN,
};

/// Lifetime of every presence artifact (challenge, attestation), in ms.
pub const PRESENCE_TTL_MS: u64 = 30_000;

/// Anti-Sybil gate: minimum LOCAL contribution score of the attester as
/// observed by the challenger. Same value as the relay demotion threshold.
pub const RELAY_CONTRIBUTION_MIN: f64 = 2.0;

/// Max in-flight challenges toward a single target (challenger side).
pub const MAX_CONCURRENT_CHALLENGES_PER_PEER: usize = 10;

/// Global cap on in-flight challenges (challenger side).
pub const MAX_PENDING_CHALLENGES: usize = 256;

/// Global cap on accepted attestations kept for aggregation.
pub const MAX_STORED_ATTESTATIONS: usize = 512;

/// Max attestations we sign per challenger per TTL window (responder side).
pub const RESPONDER_BUDGET_PER_WINDOW: u32 = 10;

/// Global cap on tracked responder windows (responder side).
const MAX_RESPONDER_WINDOWS: usize = 512;

/// A challenge we issued, awaiting its attestation (one-shot).
#[derive(Debug, Clone)]
pub struct PendingChallenge {
    pub challenge_id: String,
    pub nonce: Vec<u8>,
    /// The node we challenged — the attestation MUST come from it.
    pub target: NodeId,
    /// OUR local clock at emission; freshness is judged against this,
    /// never against the attester's claimed timestamp.
    pub issued_at: u64,
}

/// An accepted attestation, kept only for seed aggregation (≤ 30s).
#[derive(Debug, Clone)]
pub struct StoredAttestation {
    pub payload: PresenceAttestationPayload,
    /// Ed25519 envelope signature of the attester — the aggregation input.
    pub envelope_signature: Vec<u8>,
    pub accepted_at: u64,
}

/// Outcome of a single presence decision — the unit the metrics count.
///
/// Every early-return in the runtime presence handlers maps to exactly one
/// of these, so the counters below fully partition what happened (an
/// invariant checked by `metrics_partition` test). This is what makes drop
/// reasons observable for stress relevés without leaking anything on the
/// wire (drops stay silent to the attacker; only WE count them locally).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceOutcome {
    // ── Challenger side (A) ──
    /// We issued a challenge.
    Issued,
    /// An attestation was accepted (with its round-trip latency in ms).
    Accepted(u64),
    /// Attestation for an unknown/already-consumed challenge (replay, A2).
    DropUnknownChallenge,
    /// Challenge too old on OUR clock (A3).
    DropStale,
    /// Attestation from a node other than the one we challenged (A8).
    DropWrongAttester,
    /// Attestation signature invalid (A1).
    DropBadSignature,
    /// Payload/nonce/id incoherent with the challenge.
    DropIncoherent,
    /// Attester's LOCAL score below the anti-Sybil gate (A5).
    DropGate,
    /// Aggregation store full (cap reached).
    DropStoreFull,
    // ── Responder side (B) ──
    /// We received a challenge.
    ChallengeReceived,
    /// We signed and returned an attestation.
    Signed,
    /// Challenge signature invalid / reflection attempt (A9).
    RefusedBadSignature,
    /// challenger_id ≠ signer, or malformed payload.
    RefusedIncoherent,
    /// Per-challenger signing budget exhausted (A7).
    RefusedBudget,
}

/// Monotonic presence counters (never reset for the node's lifetime).
///
/// Plain `u64` in pure state → deterministic and snapshot-cheap. Latency is
/// accumulated (sum + count + min + max) so a harness can derive mean/min/max
/// without an on-node histogram.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PresenceMetrics {
    pub issued: u64,
    pub accepted: u64,
    pub drop_unknown_challenge: u64,
    pub drop_stale: u64,
    pub drop_wrong_attester: u64,
    pub drop_bad_signature: u64,
    pub drop_incoherent: u64,
    pub drop_gate: u64,
    pub drop_store_full: u64,
    pub challenges_received: u64,
    pub signed: u64,
    pub refused_bad_signature: u64,
    pub refused_incoherent: u64,
    pub refused_budget: u64,
    /// Sum of accepted-attestation latencies (ms) — divide by `accepted`.
    pub latency_sum_ms: u64,
    pub latency_min_ms: u64,
    pub latency_max_ms: u64,
}

impl PresenceMetrics {
    fn record(&mut self, outcome: PresenceOutcome) {
        use PresenceOutcome::*;
        match outcome {
            Issued => self.issued += 1,
            Accepted(latency) => {
                self.accepted += 1;
                self.latency_sum_ms = self.latency_sum_ms.saturating_add(latency);
                self.latency_max_ms = self.latency_max_ms.max(latency);
                self.latency_min_ms = if self.accepted == 1 {
                    latency
                } else {
                    self.latency_min_ms.min(latency)
                };
            }
            DropUnknownChallenge => self.drop_unknown_challenge += 1,
            DropStale => self.drop_stale += 1,
            DropWrongAttester => self.drop_wrong_attester += 1,
            DropBadSignature => self.drop_bad_signature += 1,
            DropIncoherent => self.drop_incoherent += 1,
            DropGate => self.drop_gate += 1,
            DropStoreFull => self.drop_store_full += 1,
            ChallengeReceived => self.challenges_received += 1,
            Signed => self.signed += 1,
            RefusedBadSignature => self.refused_bad_signature += 1,
            RefusedIncoherent => self.refused_incoherent += 1,
            RefusedBudget => self.refused_budget += 1,
        }
    }

    /// Mean accepted-attestation latency in ms (0 if none accepted).
    pub fn mean_latency_ms(&self) -> u64 {
        self.latency_sum_ms
            .checked_div(self.accepted)
            .unwrap_or(0)
    }
}

/// Ephemeral presence state. See module docs for invariants.
#[derive(Default)]
pub struct PresenceManager {
    pending: HashMap<String, PendingChallenge>,
    accepted: HashMap<String, StoredAttestation>,
    /// challenger → (window_start_ms, responses_signed_in_window)
    responder_windows: HashMap<NodeId, (u64, u32)>,
    metrics: PresenceMetrics,
}

impl PresenceManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a challenge we are about to send. Returns `false` (caller
    /// must NOT send) when a cap would be exceeded — global first, then
    /// per-target.
    pub fn register_challenge(&mut self, challenge: PendingChallenge) -> bool {
        if self.pending.len() >= MAX_PENDING_CHALLENGES {
            return false;
        }
        let per_target = self
            .pending
            .values()
            .filter(|c| c.target == challenge.target)
            .count();
        if per_target >= MAX_CONCURRENT_CHALLENGES_PER_PEER {
            return false;
        }
        self.pending
            .insert(challenge.challenge_id.clone(), challenge);
        true
    }

    /// Look up a pending (not yet consumed) challenge.
    pub fn pending(&self, challenge_id: &str) -> Option<&PendingChallenge> {
        self.pending.get(challenge_id)
    }

    /// Consume the challenge (one-shot) and store its attestation for
    /// aggregation. Returns `false` if the challenge was already consumed
    /// or the attestation store is full.
    pub fn consume_and_store(
        &mut self,
        challenge_id: &str,
        payload: PresenceAttestationPayload,
        envelope_signature: &[u8],
        now: u64,
    ) -> bool {
        if self.pending.remove(challenge_id).is_none() {
            return false;
        }
        if self.accepted.len() >= MAX_STORED_ATTESTATIONS {
            return false;
        }
        self.accepted.insert(
            challenge_id.to_string(),
            StoredAttestation {
                payload,
                envelope_signature: envelope_signature.to_vec(),
                accepted_at: now,
            },
        );
        true
    }

    /// Responder-side budget: may we sign one more attestation for this
    /// challenger? Consumes one budget unit on success.
    pub fn allow_response(&mut self, challenger: NodeId, now: u64) -> bool {
        // Bound the map itself before touching the entry.
        if !self.responder_windows.contains_key(&challenger)
            && self.responder_windows.len() >= MAX_RESPONDER_WINDOWS
        {
            // Opportunistic purge of expired windows, then re-check.
            self.responder_windows
                .retain(|_, (start, _)| now.saturating_sub(*start) < PRESENCE_TTL_MS);
            if self.responder_windows.len() >= MAX_RESPONDER_WINDOWS {
                return false;
            }
        }
        let entry = self.responder_windows.entry(challenger).or_insert((now, 0));
        if now.saturating_sub(entry.0) >= PRESENCE_TTL_MS {
            *entry = (now, 0);
        }
        if entry.1 >= RESPONDER_BUDGET_PER_WINDOW {
            return false;
        }
        entry.1 += 1;
        true
    }

    /// Purge everything older than [`PRESENCE_TTL_MS`]. Deterministic:
    /// driven entirely by the injected `now`.
    pub fn cleanup(&mut self, now: u64) {
        self.pending
            .retain(|_, c| now.saturating_sub(c.issued_at) < PRESENCE_TTL_MS);
        self.accepted
            .retain(|_, a| now.saturating_sub(a.accepted_at) < PRESENCE_TTL_MS);
        self.responder_windows
            .retain(|_, (start, _)| now.saturating_sub(*start) < PRESENCE_TTL_MS);
    }

    /// Aggregate the current attestation window into an entropy seed
    /// (order-independent; see `aggregator` module for the honest limits).
    pub fn aggregate_seed(&self) -> [u8; 32] {
        aggregator::aggregate_seed(
            self.accepted
                .values()
                .map(|a| a.envelope_signature.as_slice()),
        )
    }

    /// Number of attestations in the current aggregation window.
    pub fn accepted_count(&self) -> usize {
        self.accepted.len()
    }

    /// Number of in-flight challenges.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Record a presence decision outcome (observability, never on the wire).
    pub fn record(&mut self, outcome: PresenceOutcome) {
        self.metrics.record(outcome);
    }

    /// Snapshot of the lifetime presence counters.
    pub fn metrics(&self) -> PresenceMetrics {
        self.metrics
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

    fn challenge(id: &str, target: NodeId, now: u64) -> PendingChallenge {
        PendingChallenge {
            challenge_id: id.into(),
            nonce: vec![7u8; NONCE_LEN],
            target,
            issued_at: now,
        }
    }

    fn attestation(id: &str, attester: NodeId, challenger: NodeId) -> PresenceAttestationPayload {
        PresenceAttestationPayload {
            challenge_id: id.into(),
            nonce: vec![7u8; NONCE_LEN],
            timestamp: 0,
            attester_id: attester,
            challenger_id: challenger,
            relay_proof: RelayProof {
                proof_type: RelayProofType::SelfObserved,
                observer_id: attester,
                observed_at: 0,
                bytes_relayed: 0,
                observer_signature: vec![],
                reliability_score: None,
            },
        }
    }

    #[test]
    fn one_shot_consume() {
        let mut m = PresenceManager::new();
        let target = node_id(2);
        assert!(m.register_challenge(challenge("c1", target, 1_000)));

        let att = attestation("c1", target, node_id(1));
        assert!(m.consume_and_store("c1", att.clone(), &[1u8; 64], 1_500));
        // Second acceptance of the same challenge: structurally impossible.
        assert!(!m.consume_and_store("c1", att, &[1u8; 64], 1_600));
        assert_eq!(m.accepted_count(), 1);
    }

    #[test]
    fn per_peer_and_global_caps() {
        let mut m = PresenceManager::new();
        let target = node_id(2);
        for i in 0..MAX_CONCURRENT_CHALLENGES_PER_PEER {
            assert!(m.register_challenge(challenge(&format!("c{i}"), target, 1_000)));
        }
        // 11th toward the same peer: refused.
        assert!(!m.register_challenge(challenge("c-extra", target, 1_000)));

        // Fill up to the global cap with distinct targets.
        let mut seed = 3u8;
        'outer: while m.pending_count() < MAX_PENDING_CHALLENGES {
            let t = node_id(seed);
            seed = seed.wrapping_add(1);
            for i in 0..MAX_CONCURRENT_CHALLENGES_PER_PEER {
                if m.pending_count() >= MAX_PENDING_CHALLENGES {
                    break 'outer;
                }
                assert!(m.register_challenge(challenge(&format!("g{seed}-{i}"), t, 1_000)));
            }
        }
        // Global cap reached: any further registration is refused.
        assert!(!m.register_challenge(challenge("overflow", node_id(250), 1_000)));
    }

    #[test]
    fn cleanup_is_deterministic_with_injected_now() {
        let mut m = PresenceManager::new();
        let target = node_id(2);
        assert!(m.register_challenge(challenge("c1", target, 1_000)));
        m.cleanup(1_000 + PRESENCE_TTL_MS - 1);
        assert_eq!(m.pending_count(), 1, "still inside TTL");
        m.cleanup(1_000 + PRESENCE_TTL_MS);
        assert_eq!(m.pending_count(), 0, "purged at TTL");
        // A new challenge with the same id is accepted after purge (T6).
        assert!(m.register_challenge(challenge("c1", target, 40_000)));
    }

    #[test]
    fn metrics_record_partitions_outcomes() {
        use PresenceOutcome::*;
        let mut m = PresenceMetrics::default();
        for o in [
            Issued,
            Accepted(10),
            Accepted(30),
            DropUnknownChallenge,
            DropStale,
            DropWrongAttester,
            DropBadSignature,
            DropIncoherent,
            DropGate,
            DropStoreFull,
            ChallengeReceived,
            Signed,
            RefusedBadSignature,
            RefusedIncoherent,
            RefusedBudget,
        ] {
            m.record(o);
        }
        // Each counter incremented exactly once (accepted twice).
        assert_eq!(m.issued, 1);
        assert_eq!(m.accepted, 2);
        assert_eq!(m.drop_gate, 1);
        assert_eq!(m.refused_budget, 1);
        // Latency accounting.
        assert_eq!(m.latency_min_ms, 10);
        assert_eq!(m.latency_max_ms, 30);
        assert_eq!(m.mean_latency_ms(), 20);
    }

    #[test]
    fn responder_budget_limits_signatures() {
        let mut m = PresenceManager::new();
        let challenger = node_id(1);
        for _ in 0..RESPONDER_BUDGET_PER_WINDOW {
            assert!(m.allow_response(challenger, 1_000));
        }
        assert!(!m.allow_response(challenger, 1_000), "budget exhausted");
        // New window after TTL: budget resets (no permanent ban — LOCKED #4).
        assert!(m.allow_response(challenger, 1_000 + PRESENCE_TTL_MS));
    }
}
