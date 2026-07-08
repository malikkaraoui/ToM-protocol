//! L1-001 — Presence integration + adversarial tests (runtime level).
//!
//! These tests exercise the FULL runtime state machine: real Ed25519
//! signatures, real MessagePack envelopes, real router/anti-spam/scoring
//! paths — two or more `RuntimeState` instances exchanging raw bytes
//! exactly as the QUIC transport would deliver them. No mocked structs.
//!
//! Time-based purge determinism (T6/A3 windows) is covered in the
//! `presence` module tests with an injected clock; the true-network
//! latency criterion (< 200 ms median) lives in `tom-stress`
//! (`scenario_presence`).
//!
//! Adversarial map (spec §5.2): A1 forge · A2 replay · A5 lying Sybil ·
//! A7 responder budget · A8 wrong attester · A9 reflection · A10 caps.

use tom_protocol::presence::{
    self, PresenceAttestationPayload, PresenceChallengePayload, RelayProof, RelayProofType,
};
use tom_protocol::{
    now_ms, Envelope, EnvelopeBuilder, MessageType, NodeId, PeerInfo, PeerRole, PeerStatus,
    ProtocolEvent, RuntimeCommand, RuntimeConfig, RuntimeEffect, RuntimeState,
};

fn keypair(seed: u8) -> (NodeId, [u8; 32]) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = tom_connect::SecretKey::generate(&mut rng);
    let node_id: NodeId = secret.public().to_string().parse().unwrap();
    (node_id, secret.to_bytes())
}

fn state_with(seed: u8) -> RuntimeState {
    let (id, secret) = keypair(seed);
    RuntimeState::new(
        id,
        secret,
        RuntimeConfig {
            encryption: false,
            username: format!("node-{seed}"),
            ..Default::default()
        },
    )
}

/// Extract all envelopes of a given type from effects, as wire bytes.
fn envelopes_of(effects: &[RuntimeEffect], msg_type: MessageType) -> Vec<Vec<u8>> {
    effects
        .iter()
        .filter_map(|e| match e {
            RuntimeEffect::SendEnvelope(env)
            | RuntimeEffect::SendEnvelopeTo { envelope: env, .. }
            | RuntimeEffect::SendWithBackupFallback { envelope: env, .. }
                if env.msg_type == msg_type =>
            {
                Some(env.to_bytes().unwrap())
            }
            _ => None,
        })
        .collect()
}

fn attestation_events(effects: &[RuntimeEffect]) -> Vec<(NodeId, String)> {
    effects
        .iter()
        .filter_map(|e| match e {
            RuntimeEffect::Emit(ProtocolEvent::PresenceAttestationReceived {
                attester_id,
                challenge_id,
                ..
            }) => Some((*attester_id, challenge_id.clone())),
            _ => None,
        })
        .collect()
}

/// Make `relay` earn REAL relay evidence in `observer`'s local RoleManager:
/// `observer` ORIGINATES a chat through its real send path (so the message is
/// tracked, `to = dest`, `via = relay`), the relay forwards it and returns a
/// signed RelayForwarded ACK, and `observer` credits the relay. This is the
/// exact production path of the anti-Sybil gate — relay evidence is only granted
/// for a message the observer actually routed through that relay (FINDING #7).
fn earn_relay_evidence(observer: &mut RuntimeState, relay: &mut RuntimeState, dest_seed: u8) {
    let (dest, _) = keypair(200 + dest_seed); // a third node, offline
    let relay_id = relay.local_id();

    // Observer's topology: `relay` is an online relay, `dest` a known peer with
    // no direct address, so the relay selector routes the send THROUGH `relay`.
    observer.handle_command(RuntimeCommand::UpsertPeer {
        info: PeerInfo {
            node_id: relay_id,
            role: PeerRole::Relay,
            status: PeerStatus::Online,
            last_seen: now_ms(),
        },
    });
    observer.handle_command(RuntimeCommand::UpsertPeer {
        info: PeerInfo {
            node_id: dest,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: now_ms(),
        },
    });

    // Observer originates the chat via its REAL send path (this tracks it).
    let send_effects = observer.handle_send_message(dest, b"payload".to_vec());
    let outgoing = envelopes_of(&send_effects, MessageType::Chat);
    assert!(
        !outgoing.is_empty(),
        "observer must emit a chat routed through the relay"
    );

    let effects = relay.handle_incoming(&outgoing[0]);
    let acks = envelopes_of(&effects, MessageType::Ack);
    assert!(
        !acks.is_empty(),
        "relay must emit a signed RelayForwarded ACK toward the origin"
    );
    for ack in acks {
        observer.handle_incoming(&ack);
    }
}

/// Seed of an EMPTY aggregation window (reference value for asserts).
fn empty_seed() -> [u8; 32] {
    tom_protocol::presence::aggregator::aggregate_seed(std::iter::empty())
}

/// Full honest flow: relay evidence → challenge → signed attestation →
/// acceptance event → non-trivial entropy seed. (T1/T2/T4/T5 at runtime level)
#[test]
fn honest_roundtrip_with_real_relay_evidence() {
    let mut alice = state_with(1);
    let mut bob = state_with(2);
    let bob_id = bob.local_id();

    // Without relay evidence, Bob's local score at Alice is 0 → gate closed.
    earn_relay_evidence(&mut alice, &mut bob, 1);

    // Alice challenges Bob.
    let effects = alice.initiate_presence_check(bob_id);
    let challenges = envelopes_of(&effects, MessageType::PresenceChallenge);
    assert_eq!(challenges.len(), 1, "exactly one signed challenge");

    // Bob answers through the full incoming pipeline.
    let bob_effects = bob.handle_incoming(&challenges[0]);
    let attestations = envelopes_of(&bob_effects, MessageType::PresenceAttestation);
    assert_eq!(attestations.len(), 1, "Bob signs exactly one attestation");

    // Alice accepts: event emitted, window updated, seed non-trivial.
    let alice_effects = alice.handle_incoming(&attestations[0]);
    let events = attestation_events(&alice_effects);
    assert_eq!(events.len(), 1, "acceptance event expected");
    assert_eq!(events[0].0, bob_id);
    assert_eq!(alice.presence_attestation_count(), 1);
    assert_ne!(
        alice.presence_seed(),
        empty_seed(),
        "seed must differ from the empty-window seed once an attestation lands"
    );
}

/// A2 — replaying an accepted attestation hits a consumed (one-shot)
/// challenge: no event, no growth of the aggregation window.
#[test]
fn replayed_attestation_is_dropped() {
    let mut alice = state_with(1);
    let mut bob = state_with(2);
    earn_relay_evidence(&mut alice, &mut bob, 1);

    let challenges = envelopes_of(
        &alice.initiate_presence_check(bob.local_id()),
        MessageType::PresenceChallenge,
    );
    let attestations = envelopes_of(
        &bob.handle_incoming(&challenges[0]),
        MessageType::PresenceAttestation,
    );

    assert_eq!(attestation_events(&alice.handle_incoming(&attestations[0])).len(), 1);
    // Replay, byte-for-byte identical (valid signature, correct nonce).
    let replay_effects = alice.handle_incoming(&attestations[0]);
    assert!(
        attestation_events(&replay_effects).is_empty(),
        "replay must be silently dropped"
    );
    assert_eq!(alice.presence_attestation_count(), 1);
}

/// A1 — attestation with a tampered signature is dropped by the envelope
/// signature check (before any presence logic).
#[test]
fn forged_signature_is_dropped() {
    let mut alice = state_with(1);
    let mut bob = state_with(2);
    earn_relay_evidence(&mut alice, &mut bob, 1);

    let challenges = envelopes_of(
        &alice.initiate_presence_check(bob.local_id()),
        MessageType::PresenceChallenge,
    );
    let attestations = envelopes_of(
        &bob.handle_incoming(&challenges[0]),
        MessageType::PresenceAttestation,
    );

    let mut env = Envelope::from_bytes(&attestations[0]).unwrap();
    env.signature[0] ^= 0xFF; // corrupt one signature byte
    let effects = alice.handle_incoming(&env.to_bytes().unwrap());
    assert!(attestation_events(&effects).is_empty());
    assert_eq!(alice.presence_attestation_count(), 0);
}

/// A5 — a Sybil that NEVER relayed for Alice self-declares a top score in
/// the payload. The gate reads Alice's LOCAL score only → dropped.
#[test]
fn lying_sybil_self_declared_score_is_ignored() {
    let mut alice = state_with(1);
    let sybil = state_with(3);
    let (_, sybil_secret) = keypair(3);
    let sybil_id = sybil.local_id();

    // Alice challenges the Sybil (no relay evidence exists for it).
    let challenges = envelopes_of(
        &alice.initiate_presence_check(sybil_id),
        MessageType::PresenceChallenge,
    );
    let challenge_env = Envelope::from_bytes(&challenges[0]).unwrap();
    let challenge = PresenceChallengePayload::from_bytes(&challenge_env.payload).unwrap();

    // The Sybil crafts a LYING attestation: reliability_score = 10.0,
    // perfectly signed, correct nonce, correct ids. Everything checks out
    // EXCEPT Alice's local observation of it.
    let lie = PresenceAttestationPayload {
        challenge_id: challenge.challenge_id.clone(),
        nonce: challenge.nonce.clone(),
        timestamp: challenge.timestamp,
        attester_id: sybil_id,
        challenger_id: alice.local_id(),
        relay_proof: RelayProof {
            proof_type: RelayProofType::SelfObserved,
            observer_id: sybil_id,
            observed_at: challenge.timestamp,
            bytes_relayed: u64::MAX / 2, // "I relayed terabytes, trust me"
            observer_signature: vec![],
            reliability_score: Some(10.0),
        },
    };
    let env = EnvelopeBuilder::new(
        sybil_id,
        alice.local_id(),
        MessageType::PresenceAttestation,
        lie.to_bytes(),
    )
    .sign(&sybil_secret);

    let effects = alice.handle_incoming(&env.to_bytes().unwrap());
    assert!(
        attestation_events(&effects).is_empty(),
        "self-declared score must never open the gate"
    );
    assert_eq!(alice.presence_attestation_count(), 0);

    // Sanity: the sybil state was never consulted — the lie was crafted
    // out-of-band, as a real attacker would.
    let _ = sybil.local_id();
}

/// A8 — an on-path eavesdropper M (who sees the cleartext challenge) answers
/// in Bob's place with ITS own valid signature: dropped, wrong attester.
#[test]
fn attestation_from_wrong_node_is_dropped() {
    let mut alice = state_with(1);
    let mut bob = state_with(2);
    let mut mallory = state_with(4);
    let (_, mallory_secret) = keypair(4);

    // Even a well-scored Mallory must not be able to usurp Bob's challenge.
    earn_relay_evidence(&mut alice, &mut bob, 1);
    earn_relay_evidence(&mut alice, &mut mallory, 5);

    let challenges = envelopes_of(
        &alice.initiate_presence_check(bob.local_id()),
        MessageType::PresenceChallenge,
    );
    let challenge_env = Envelope::from_bytes(&challenges[0]).unwrap();
    let challenge = PresenceChallengePayload::from_bytes(&challenge_env.payload).unwrap();

    // Mallory saw the nonce on the wire and answers with a VALID signature.
    let usurped = PresenceAttestationPayload {
        challenge_id: challenge.challenge_id.clone(),
        nonce: challenge.nonce.clone(),
        timestamp: challenge.timestamp,
        attester_id: mallory.local_id(),
        challenger_id: alice.local_id(),
        relay_proof: RelayProof {
            proof_type: RelayProofType::SelfObserved,
            observer_id: mallory.local_id(),
            observed_at: challenge.timestamp,
            bytes_relayed: 0,
            observer_signature: vec![],
            reliability_score: None,
        },
    };
    let env = EnvelopeBuilder::new(
        mallory.local_id(),
        alice.local_id(),
        MessageType::PresenceAttestation,
        usurped.to_bytes(),
    )
    .sign(&mallory_secret);

    let effects = alice.handle_incoming(&env.to_bytes().unwrap());
    assert!(
        attestation_events(&effects).is_empty(),
        "attestation must come from the challenged node, not an on-path observer"
    );
    assert_eq!(alice.presence_attestation_count(), 0);
}

/// A9 — reflection: an UNSIGNED challenge spoofing Alice as `from` must not
/// extract a signed attestation from Bob (CPU + reflection defense). Same
/// for a signed challenge whose payload claims a different challenger.
#[test]
fn forged_challenges_extract_no_signature() {
    let alice = state_with(1);
    let mut bob = state_with(2);
    let (_, mallory_secret) = keypair(4);
    let (mallory_id, _) = keypair(4);

    // 1. Unsigned challenge, from spoofed as Alice.
    let payload = PresenceChallengePayload {
        challenge_id: "spoof-1".into(),
        nonce: vec![9u8; presence::NONCE_LEN],
        timestamp: tom_protocol::now_ms(),
        challenger_id: alice.local_id(),
    };
    let unsigned = Envelope::new(
        alice.local_id(),
        bob.local_id(),
        MessageType::PresenceChallenge,
        payload.to_bytes(),
    ); // never signed
    let effects = bob.handle_incoming(&unsigned.to_bytes().unwrap());
    assert!(
        envelopes_of(&effects, MessageType::PresenceAttestation).is_empty(),
        "unsigned challenge must not be answered"
    );

    // 2. Mallory-signed challenge claiming challenger_id = Alice
    //    (attestations would be reflected onto Alice).
    let payload = PresenceChallengePayload {
        challenge_id: "spoof-2".into(),
        nonce: vec![9u8; presence::NONCE_LEN],
        timestamp: tom_protocol::now_ms(),
        challenger_id: alice.local_id(),
    };
    let mismatched = EnvelopeBuilder::new(
        mallory_id,
        bob.local_id(),
        MessageType::PresenceChallenge,
        payload.to_bytes(),
    )
    .sign(&mallory_secret);
    let effects = bob.handle_incoming(&mismatched.to_bytes().unwrap());
    assert!(
        envelopes_of(&effects, MessageType::PresenceAttestation).is_empty(),
        "challenger_id ≠ envelope signer must not be answered"
    );
}

/// A7 — responder budget: a flood of VALID signed challenges (distinct
/// ids, attacker-crafted) extracts at most RESPONDER_BUDGET_PER_WINDOW
/// signatures per window from Bob.
#[test]
fn responder_budget_caps_signature_extraction() {
    let mut bob = state_with(2);
    let (mallory_id, mallory_secret) = keypair(4);

    let mut signed = 0;
    for i in 0..(presence::RESPONDER_BUDGET_PER_WINDOW as usize * 3) {
        let payload = PresenceChallengePayload {
            challenge_id: format!("flood-{i}"),
            nonce: vec![7u8; presence::NONCE_LEN],
            timestamp: tom_protocol::now_ms(),
            challenger_id: mallory_id,
        };
        let env = EnvelopeBuilder::new(
            mallory_id,
            bob.local_id(),
            MessageType::PresenceChallenge,
            payload.to_bytes(),
        )
        .sign(&mallory_secret);
        let effects = bob.handle_incoming(&env.to_bytes().unwrap());
        signed += envelopes_of(&effects, MessageType::PresenceAttestation).len();
    }
    assert_eq!(
        signed,
        presence::RESPONDER_BUDGET_PER_WINDOW as usize,
        "budget must cap signature extraction exactly"
    );
}

/// A10 — challenger-side caps: per-target (10) and global (256) limits
/// bound the pending-challenge memory under a runaway caller.
#[test]
fn challenger_caps_bound_pending_memory() {
    let mut alice = state_with(1);

    // Per-target cap.
    let (target, _) = keypair(50);
    let mut sent = 0;
    for _ in 0..presence::MAX_CONCURRENT_CHALLENGES_PER_PEER + 5 {
        sent += envelopes_of(
            &alice.initiate_presence_check(target),
            MessageType::PresenceChallenge,
        )
        .len();
    }
    assert_eq!(
        sent,
        presence::MAX_CONCURRENT_CHALLENGES_PER_PEER,
        "per-target cap must hold"
    );

    // Global cap across many targets.
    let mut total = sent;
    let mut seed = 51u8;
    while seed < 51 + 30 {
        let (t, _) = keypair(seed);
        for _ in 0..presence::MAX_CONCURRENT_CHALLENGES_PER_PEER {
            total += envelopes_of(
                &alice.initiate_presence_check(t),
                MessageType::PresenceChallenge,
            )
            .len();
        }
        seed += 1;
    }
    assert!(
        total <= presence::MAX_PENDING_CHALLENGES,
        "global cap must hold: {total} > {}",
        presence::MAX_PENDING_CHALLENGES
    );
}

/// Self-challenge is a no-op (no signature spent, no state created).
#[test]
fn self_challenge_is_noop() {
    let mut alice = state_with(1);
    let id = alice.local_id();
    assert!(alice.initiate_presence_check(id).is_empty());
    assert_eq!(alice.presence_attestation_count(), 0);
}

/// Fleet plumbing mode: gate at 0.0 (config) accepts a well-formed signed
/// attestation WITHOUT relay evidence — phase 1 of the fleet runbook.
#[test]
fn config_gate_zero_accepts_without_evidence() {
    let (id, secret) = keypair(1);
    let mut alice = RuntimeState::new(
        id,
        secret,
        RuntimeConfig {
            encryption: false,
            username: "node-1".into(),
            presence_contribution_min: 0.0,
            ..Default::default()
        },
    );
    let mut bob = state_with(2);

    let challenges = envelopes_of(
        &alice.initiate_presence_check(bob.local_id()),
        MessageType::PresenceChallenge,
    );
    let attestations = envelopes_of(
        &bob.handle_incoming(&challenges[0]),
        MessageType::PresenceAttestation,
    );
    let events = attestation_events(&alice.handle_incoming(&attestations[0]));
    assert_eq!(events.len(), 1, "gate 0.0 must accept without relay evidence");

    // Structural defenses stay armed even at gate 0.0: replay still dropped.
    assert!(attestation_events(&alice.handle_incoming(&attestations[0])).is_empty());
}

/// Auto-probe: disabled by default (no effects), challenges Online peers
/// when enabled.
#[test]
fn auto_probe_is_config_gated() {
    use std::time::Duration;
    use tom_protocol::{PeerInfo, PeerRole, PeerStatus, RuntimeCommand};

    // Default: off.
    let mut off = state_with(1);
    assert!(off.tick_presence_probe().is_empty());

    // Enabled: probes Online peers through the same one-shot pipeline.
    let (id, secret) = keypair(1);
    let mut on = RuntimeState::new(
        id,
        secret,
        RuntimeConfig {
            encryption: false,
            username: "node-1".into(),
            presence_probe_interval: Some(Duration::from_secs(15)),
            ..Default::default()
        },
    );
    let (peer, _) = keypair(9);
    on.handle_command(RuntimeCommand::UpsertPeer {
        info: PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: tom_protocol::now_ms(),
        },
    });
    let challenges = envelopes_of(&on.tick_presence_probe(), MessageType::PresenceChallenge);
    assert_eq!(challenges.len(), 1, "one challenge per Online peer");
}

/// Build 20 — drop counters increment on the right reason (runtime level).
#[test]
fn drop_counters_partition_by_reason() {
    use tom_protocol::PresenceOutcome;
    let mut alice = state_with(1);
    let mut bob = state_with(2);
    earn_relay_evidence(&mut alice, &mut bob, 1);

    // Happy path → accepted counter + issued + challenge received/signed on Bob.
    let challenges = envelopes_of(
        &alice.initiate_presence_check(bob.local_id()),
        MessageType::PresenceChallenge,
    );
    let attestations = envelopes_of(
        &bob.handle_incoming(&challenges[0]),
        MessageType::PresenceAttestation,
    );
    alice.handle_incoming(&attestations[0]);
    // Replay → drop_unknown_challenge.
    alice.handle_incoming(&attestations[0]);

    let am = alice.presence_metrics();
    assert_eq!(am.issued, 1);
    assert_eq!(am.accepted, 1);
    assert_eq!(am.drop_unknown_challenge, 1, "replay must count as unknown-challenge drop");
    assert!(am.latency_max_ms >= am.latency_min_ms);

    let bm = bob.presence_metrics();
    assert_eq!(bm.challenges_received, 1);
    assert_eq!(bm.signed, 1);

    // A forged (wrong-signer) challenge to Bob → refused_bad_signature.
    let (mid, msecret) = keypair(7);
    let payload = tom_protocol::presence::PresenceChallengePayload {
        challenge_id: "x".into(),
        nonce: vec![3u8; tom_protocol::presence::NONCE_LEN],
        timestamp: tom_protocol::now_ms(),
        challenger_id: alice.local_id(), // lies about challenger
    };
    let env = EnvelopeBuilder::new(
        mid,
        bob.local_id(),
        MessageType::PresenceChallenge,
        payload.to_bytes(),
    )
    .sign(&msecret);
    bob.handle_incoming(&env.to_bytes().unwrap());
    let bm2 = bob.presence_metrics();
    assert_eq!(bm2.refused_incoherent, 1, "challenger_id≠signer → incoherent refusal");
    let _ = PresenceOutcome::Issued; // type is exported
}

/// Build 20 SIM — clock-skew injection proves the anti-NTP hardening.
///
/// A challenger judges freshness on its OWN clock, and ignores the
/// attester's declared timestamp. So an attester whose clock is wildly
/// skewed still produces attestations the challenger accepts — provided the
/// RESPONDER acceptance window (loose, 120s) tolerates the skew on the
/// CHALLENGE direction. This test proves both the property AND documents
/// where the responder window is the binding constraint.
#[test]
fn clock_skew_freshness_holds_on_local_clock() {
    // Attester B runs 60s ahead of challenger A (within the 120s window).
    let (aid, asecret) = keypair(1);
    let mut alice = RuntimeState::new(
        aid,
        asecret,
        RuntimeConfig {
            encryption: false,
            username: "node-1".into(),
            presence_contribution_min: 0.0,
            presence_clock_offset_ms: 0,
            ..Default::default()
        },
    );
    let (bid, bsecret) = keypair(2);
    let mut bob = RuntimeState::new(
        bid,
        bsecret,
        RuntimeConfig {
            encryption: false,
            username: "node-2".into(),
            presence_contribution_min: 0.0,
            presence_clock_offset_ms: 60_000, // +60s, inside responder window
            ..Default::default()
        },
    );

    let challenges = envelopes_of(
        &alice.initiate_presence_check(bob.local_id()),
        MessageType::PresenceChallenge,
    );
    // Bob (skewed +60s) still accepts A's challenge (|60s| < 120s window)…
    let attestations = envelopes_of(
        &bob.handle_incoming(&challenges[0]),
        MessageType::PresenceAttestation,
    );
    assert_eq!(attestations.len(), 1, "skew within window → Bob attests");
    // …and A accepts B's attestation: freshness is on A's own clock, and
    // B's skewed declared timestamp is never used as a gate.
    let events = attestation_events(&alice.handle_incoming(&attestations[0]));
    assert_eq!(events.len(), 1, "challenger judges freshness on its own clock");
    assert!(bob.presence_metrics().signed == 1);
    assert!(alice.presence_metrics().accepted == 1);
}

/// Build 20 SIM — skew BEYOND the responder window is the binding limit.
/// This is the honest boundary the anti-NTP hardening leaves: the responder
/// rejects a challenge whose declared timestamp is >120s from its own clock.
#[test]
fn clock_skew_beyond_responder_window_is_rejected() {
    let mut alice = state_with(1);
    let (bid, bsecret) = keypair(2);
    let mut bob = RuntimeState::new(
        bid,
        bsecret,
        RuntimeConfig {
            encryption: false,
            username: "node-2".into(),
            presence_clock_offset_ms: 300_000, // +5 min, well beyond 120s window
            ..Default::default()
        },
    );

    let challenges = envelopes_of(
        &alice.initiate_presence_check(bob.local_id()),
        MessageType::PresenceChallenge,
    );
    let attestations = envelopes_of(
        &bob.handle_incoming(&challenges[0]),
        MessageType::PresenceAttestation,
    );
    assert!(
        attestations.is_empty(),
        "5min skew exceeds the 120s responder window → challenge refused"
    );
    // The refusal is counted as incoherent (validate() window failure).
    assert_eq!(bob.presence_metrics().refused_incoherent, 1);
}
