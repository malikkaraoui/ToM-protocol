//! L1-003 — Signed relay presence view (weak-device presence offload).
//!
//! A weak device (Apple TV, constrained RAM/CPU) stops computing the presence
//! of N peers itself. Instead it subscribes to the SIGNED view of its relay(s)
//! — the strong devices already on its routing path — and only keeps its own
//! first-hand proofs plus those views. See
//! `docs/plans/L1-003-vue-signee-relais.md` (ADR-011 §5).
//!
//! Trust model (NON-NEGOTIABLE, red-team kill-shot #3): a view is signed by its
//! witness, so its ORIGIN is proven — but a signature proves WHO speaks, not
//! that the content is TRUE. A single relay on a weak device's path can be
//! Sybil and lie about the whole view (eclipse). Therefore a peer is only
//! promoted to `Online` when a QUORUM of ≥ N distinct witnesses concur
//! (`required_witnesses`, dynamic per D1). One witness = `Known` at most, never
//! `Online`. Consumption/quorum logic lives elsewhere; this module is the wire
//! type only.
//!
//! Everything here is ephemeral (aligned on `PRESENCE_TTL_MS`), never
//! persisted, never backed up (LOCKED #2). `u64` ms timestamps throughout (no
//! `Instant` in state — deterministic effect-pattern testing).

use ed25519_dalek::Signer;
use serde::{Deserialize, Serialize};

use crate::error::TomProtocolError;
use crate::group::GroupId;
use crate::router::AckType;
use crate::types::NodeId;

/// Hard floor on the witness quorum — a weak device NEVER trusts a single
/// witness for an `Online` promotion (kill-shot #3). `required_witnesses`
/// scales UP from here with network density/activity (D1), never below.
pub const MIN_PRESENCE_WITNESSES: usize = 2;

/// Ceiling on the dynamic witness quorum. Above this the availability cost
/// (needing that many distinct relays on a weak device's path) outweighs the
/// marginal collusion-resistance gain.
pub const MAX_PRESENCE_WITNESSES: usize = 4;

/// Max entries a single view may carry (bounds memory + parse cost on a
/// constrained device; a weak device's scope is small by construction).
pub const MAX_VIEW_ENTRIES: usize = 256;

/// Required distinct witnesses for an `Online` promotion, as a function of
/// network density and activity (D1 — dynamic quorum, not a fixed constant).
///
/// Denser / more active networks raise the bar (2 → 3 → 4) for more power,
/// resilience and security; small/sparse networks stay at the floor so
/// presence stays available. Bounded to
/// `[MIN_PRESENCE_WITNESSES, MAX_PRESENCE_WITNESSES]`.
///
/// `density` = distinct relays reachable by the weak device (its potential
/// witness set). `activity` = a coarse recent-traffic indicator (e.g. views
/// received per window). Both are local observations; the exact calibration is
/// intentionally simple until real fleet data refines it.
pub fn required_witnesses(density: usize, activity: usize) -> usize {
    // Can never demand more witnesses than the device could possibly gather.
    let feasible = density.min(MAX_PRESENCE_WITNESSES);
    // Raise the bar with density AND activity; both must be high to reach 4.
    let desired = if density >= 4 && activity >= 4 {
        4
    } else if density >= 3 && activity >= 2 {
        3
    } else {
        MIN_PRESENCE_WITNESSES
    };
    desired.min(feasible).max(MIN_PRESENCE_WITNESSES)
}

/// What a witness view is scoped to (D3 — explicit subscription). Bounds the
/// view size and states what the weak device asked about. No NEW privacy leak:
/// the relay is already on the routing path and sees `from`/`to` of relayed
/// envelopes (content is E2E-encrypted, routing addresses are not).
///
/// ⚠️ Scope being known/updated/confirmed by the relay is a NOTED future
/// hardening item (see the design doc) — not closed here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresenceScope {
    /// Presence of the members of a specific group.
    Group(GroupId),
    /// Presence of an explicit peer list the weak device subscribed to.
    Peers(Vec<NodeId>),
}

/// One peer the witness attests it saw alive, tied to a REAL signed ACK
/// (verrou #1) it observed — never hearsay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceEntry {
    /// The peer attested present.
    pub peer_id: NodeId,
    /// Reference to a real signed ACK the witness relayed/observed. The
    /// consumer verifies this points to genuine evidence — a forged/absent
    /// ref makes the entry unverifiable and it does NOT count toward quorum.
    pub proof_ref: String,
    /// Which kind of ACK backs this entry.
    pub proof_type: AckType,
    /// Witness clock (Unix ms) when it observed the proof.
    pub seen_at_ms: u64,
}

/// A signed presence view published by a witness (relay) to its subscribers.
///
/// Signature covers everything EXCEPT `signature` itself (same discipline as
/// `Envelope::signing_bytes`). Signed ⇒ origin proven; TRUTH still requires
/// quorum on the consumer side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayPresenceView {
    /// The relay that observed and signs this view.
    pub witness_id: NodeId,
    /// Witness clock (Unix ms) at emission — the consumer judges freshness.
    pub epoch_ms: u64,
    /// What this view covers (D3).
    pub scope: PresenceScope,
    /// Peers the witness saw alive within its window.
    pub present: Vec<PresenceEntry>,
    /// Ed25519 signature of the witness over `signing_bytes()`.
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
}

impl RelayPresenceView {
    /// Canonical bytes to sign/verify — everything except `signature`.
    /// Deterministic: same view always produces the same bytes.
    pub fn signing_bytes(&self) -> Vec<u8> {
        #[derive(Serialize)]
        struct Signable<'a> {
            witness_id: &'a NodeId,
            epoch_ms: u64,
            scope: &'a PresenceScope,
            present: &'a [PresenceEntry],
        }
        rmp_serde::to_vec(&Signable {
            witness_id: &self.witness_id,
            epoch_ms: self.epoch_ms,
            scope: &self.scope,
            present: &self.present,
        })
        .expect("RelayPresenceView signing_bytes serialization cannot fail")
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        rmp_serde::to_vec(self).expect("RelayPresenceView serialization cannot fail")
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, TomProtocolError> {
        rmp_serde::from_slice(data).map_err(Into::into)
    }

    /// Sign the view with the witness's Ed25519 secret seed. Makes the view
    /// INDEPENDENTLY verifiable (not merely "the envelope was signed"): a
    /// consumer aggregating views across witnesses (quorum) can verify each on
    /// its own. Mirrors `Envelope::sign`.
    pub fn sign(&mut self, secret_seed: &[u8; 32]) {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(secret_seed);
        self.signature = signing_key.sign(&self.signing_bytes()).to_bytes().to_vec();
    }

    /// Verify the view's OWN signature against its claimed `witness_id`. Proves
    /// the content was attested by that witness — a prerequisite before the
    /// entry may count toward quorum. Strict (rejects non-canonical sigs).
    pub fn verify_signature(&self) -> bool {
        if self.signature.len() != 64 {
            return false;
        }
        let Ok(verifying_key) =
            ed25519_dalek::VerifyingKey::from_bytes(&self.witness_id.as_bytes())
        else {
            return false;
        };
        let Ok(sig_bytes): Result<[u8; 64], _> = self.signature[..64].try_into() else {
            return false;
        };
        let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify_strict(&self.signing_bytes(), &signature)
            .is_ok()
    }

    /// Structural validation before any crypto/quorum work (cheap gate).
    ///
    /// - `witness_id` must not be empty of entries beyond the cap;
    /// - entries must be non-empty (a view with no `present` carries no
    ///   signal and only costs a signature verification).
    pub fn validate(&self) -> Result<(), TomProtocolError> {
        if self.present.is_empty() {
            return Err(TomProtocolError::InvalidEnvelope {
                reason: "relay presence view carries no entries".into(),
            });
        }
        if self.present.len() > MAX_VIEW_ENTRIES {
            return Err(TomProtocolError::InvalidEnvelope {
                reason: format!(
                    "relay presence view exceeds {MAX_VIEW_ENTRIES} entries (got {})",
                    self.present.len()
                ),
            });
        }
        if self.signature.len() != 64 {
            return Err(TomProtocolError::InvalidEnvelope {
                reason: "relay presence view signature must be 64 bytes".into(),
            });
        }
        Ok(())
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

    fn entry(seed: u8, at: u64) -> PresenceEntry {
        PresenceEntry {
            peer_id: node_id(seed),
            proof_ref: format!("msg-{seed}"),
            proof_type: AckType::RelayForwarded,
            seen_at_ms: at,
        }
    }

    fn view() -> RelayPresenceView {
        RelayPresenceView {
            witness_id: node_id(1),
            epoch_ms: 10_000,
            scope: PresenceScope::Peers(vec![node_id(2), node_id(3)]),
            present: vec![entry(2, 9_800), entry(3, 9_900)],
            signature: vec![7u8; 64],
        }
    }

    #[test]
    fn view_roundtrip() {
        let v = view();
        let back = RelayPresenceView::from_bytes(&v.to_bytes()).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn scope_group_roundtrip() {
        let mut v = view();
        v.scope = PresenceScope::Group(GroupId("g-1".into()));
        let back = RelayPresenceView::from_bytes(&v.to_bytes()).unwrap();
        assert_eq!(back.scope, PresenceScope::Group(GroupId("g-1".into())));
    }

    #[test]
    fn signing_bytes_excludes_signature() {
        let mut v = view();
        let sb1 = v.signing_bytes();
        // Mutating ONLY the signature must not change signing_bytes.
        v.signature = vec![9u8; 64];
        assert_eq!(sb1, v.signing_bytes());
    }

    #[test]
    fn signing_bytes_changes_with_content() {
        let v = view();
        let sb1 = v.signing_bytes();
        let mut v2 = v.clone();
        v2.present.push(entry(4, 9_950));
        assert_ne!(sb1, v2.signing_bytes());
    }

    #[test]
    fn validate_rejects_empty() {
        let mut v = view();
        v.present.clear();
        assert!(v.validate().is_err());
    }

    #[test]
    fn validate_rejects_oversize() {
        let mut v = view();
        v.present = (0..=MAX_VIEW_ENTRIES as u16)
            .map(|i| entry((i % 250) as u8, 9_000 + i as u64))
            .collect();
        assert!(v.validate().is_err());
    }

    #[test]
    fn validate_rejects_bad_signature_len() {
        let mut v = view();
        v.signature = vec![0u8; 32];
        assert!(v.validate().is_err());
    }

    #[test]
    fn validate_accepts_wellformed() {
        assert!(view().validate().is_ok());
    }

    fn keypair(seed: u8) -> ([u8; 32], NodeId) {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        let id: NodeId = secret.public().to_string().parse().unwrap();
        (secret.to_bytes(), id)
    }

    #[test]
    fn sign_then_verify_roundtrip() {
        let (seed, witness) = keypair(1);
        let mut v = RelayPresenceView {
            witness_id: witness,
            epoch_ms: 10_000,
            scope: PresenceScope::Peers(vec![node_id(2)]),
            present: vec![entry(2, 9_800)],
            signature: Vec::new(),
        };
        v.sign(&seed);
        assert!(v.verify_signature(), "own signature must verify");
    }

    #[test]
    fn verify_fails_on_tampered_content() {
        let (seed, witness) = keypair(1);
        let mut v = RelayPresenceView {
            witness_id: witness,
            epoch_ms: 10_000,
            scope: PresenceScope::Peers(vec![node_id(2)]),
            present: vec![entry(2, 9_800)],
            signature: Vec::new(),
        };
        v.sign(&seed);
        // Tamper with an entry after signing → verification must fail.
        v.present.push(entry(3, 9_900));
        assert!(!v.verify_signature(), "tampered view must not verify");
    }

    #[test]
    fn verify_fails_when_witness_id_swapped() {
        let (seed, _witness) = keypair(1);
        let (_other_seed, other) = keypair(2);
        let mut v = RelayPresenceView {
            witness_id: other, // claim a different witness than the signer
            epoch_ms: 10_000,
            scope: PresenceScope::Peers(vec![node_id(2)]),
            present: vec![entry(2, 9_800)],
            signature: Vec::new(),
        };
        v.sign(&seed); // signed by keypair(1), but witness_id says keypair(2)
        assert!(!v.verify_signature(), "signature must bind to witness_id");
    }

    #[test]
    fn quorum_floor_is_two() {
        // Sparse/quiet network → floor.
        assert_eq!(required_witnesses(0, 0), MIN_PRESENCE_WITNESSES);
        assert_eq!(required_witnesses(1, 10), MIN_PRESENCE_WITNESSES);
        assert_eq!(required_witnesses(2, 10), MIN_PRESENCE_WITNESSES);
    }

    #[test]
    fn quorum_scales_with_density_and_activity() {
        // Dense + active → 3 then 4.
        assert_eq!(required_witnesses(3, 2), 3);
        assert_eq!(required_witnesses(4, 4), 4);
        // Dense but quiet stays lower.
        assert_eq!(required_witnesses(4, 1), MIN_PRESENCE_WITNESSES);
    }

    #[test]
    fn quorum_never_exceeds_feasible_density() {
        // Can't demand 4 witnesses when only 3 relays are reachable.
        assert_eq!(required_witnesses(3, 10), 3);
    }

    #[test]
    fn quorum_never_below_floor_even_if_density_zero() {
        assert!(required_witnesses(0, 0) >= MIN_PRESENCE_WITNESSES);
    }
}
