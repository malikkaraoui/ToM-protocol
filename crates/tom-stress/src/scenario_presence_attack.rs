//! Presence RED TEAM — adversarial injection over LIVE QUIC (build 22+).
//!
//! I am the attacker. A raw `TomNode` (no ProtocolRuntime, so it is bound by
//! no protocol rules) crafts MALICIOUS protocol envelopes and fires them at a
//! real target node via `send_raw`. The target receives them through its
//! normal `recv_raw → handle_incoming` path — exactly as a real hostile peer
//! on the network would. We then read the target's per-outcome presence
//! counters (build 20) to VERIFY every attack was defended, and that nothing
//! crashed, hung, or accepted an illegitimate attestation.
//!
//! This moves A1 (forge), A2 (replay), malformed, and the responder budget
//! from in-process unit tests onto the real wire. Each attack is a permanent
//! corpus entry (docs/redteam/corpus.jsonl).
//!
//! Note: the target ignores the QUIC-authenticated transport `from` and
//! validates the ENVELOPE's `from` (see loop.rs `recv_raw` → handle_incoming),
//! so the attacker can forge `from` freely — the signature must still match
//! that `from`'s key, which is the whole point of the defense.

use std::time::{Duration, Instant};

use tom_protocol::presence::{
    PresenceAttestationPayload, PresenceChallengePayload, RelayProof, RelayProofType, NONCE_LEN,
};
use tom_protocol::{
    EnvelopeBuilder, MessageType, NodeId, PresenceMetrics, ProtocolRuntime, RuntimeConfig,
};
use tom_transport::{TomNode, TomNodeConfig};

use crate::scenario_common::{timed_step_async, ScenarioResult};

fn keypair(seed: u8) -> (NodeId, [u8; 32]) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = tom_connect::SecretKey::generate(&mut rng);
    let node_id: NodeId = secret.public().to_string().parse().unwrap();
    (node_id, secret.to_bytes())
}

/// Build a signed presence ATTESTATION envelope (bytes ready for send_raw).
/// `sign_key` may be the wrong key on purpose (forge).
fn forged_attestation(
    from: NodeId,
    to: NodeId,
    challenge_id: &str,
    sign_key: &[u8; 32],
) -> Vec<u8> {
    let payload = PresenceAttestationPayload {
        challenge_id: challenge_id.into(),
        nonce: vec![7u8; NONCE_LEN],
        timestamp: tom_protocol::now_ms(),
        attester_id: from,
        challenger_id: to,
        relay_proof: RelayProof {
            proof_type: RelayProofType::SelfObserved,
            observer_id: from,
            observed_at: tom_protocol::now_ms(),
            bytes_relayed: u64::MAX / 2, // "I relayed terabytes, trust me"
            observer_signature: vec![],
            reliability_score: Some(10.0), // lie — must be ignored
        },
    };
    EnvelopeBuilder::new(from, to, MessageType::PresenceAttestation, payload.to_bytes())
        .sign(sign_key)
        .to_bytes()
        .unwrap()
}

pub async fn run() -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("presence-attack");
    let start = Instant::now();

    // ── Target: a real protocol node (gate 0.0 so we prove drops happen even
    //    at the most permissive setting — nothing illegit must slip through). ─
    let target_node = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let target_id = target_node.id();
    let target_addr = target_node.addr();
    let channels = ProtocolRuntime::spawn(
        target_node,
        RuntimeConfig {
            username: "victim".into(),
            encryption: false,
            enable_dht: false, // red-team: never pollute the shared DHT rendezvous
            presence_contribution_min: 0.0,
            ..Default::default()
        },
    );
    let target = channels.handle;

    // ── Attacker: a RAW node bound by no protocol rules. ────────────────────
    let attacker = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    attacker.add_peer_addr(target_addr).await;
    let (evil_id, evil_key) = keypair(0xEE);
    let (_, wrong_key) = keypair(0x66); // a key that does NOT match evil_id

    // Give QUIC a moment to be dialable.
    tokio::time::sleep(Duration::from_millis(600)).await;

    let baseline: PresenceMetrics = target.presence_metrics().await.unwrap_or_default();

    // ── A1 — forged signature (unsolicited, signed with the WRONG key) ──────
    let step = timed_step_async("A1 forged attestation over QUIC", || async {
        for i in 0..20 {
            let bytes = forged_attestation(evil_id, target_id, &format!("forge-{i}"), &wrong_key);
            let _ = attacker.send_raw(target_id, &bytes).await;
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        let m = target.presence_metrics().await.unwrap_or_default();
        if m.accepted > baseline.accepted {
            return Err(format!("BREACH: target accepted a forged attestation ({} accepted)", m.accepted));
        }
        // Unsolicited attestation → no matching challenge → drop_unknown_challenge.
        let drops = (m.drop_unknown_challenge + m.drop_bad_signature) - (baseline.drop_unknown_challenge + baseline.drop_bad_signature);
        Ok(format!("20 forged sent, 0 accepted, {drops} counted-drops"))
    })
    .await;
    result.add(step);

    // ── A2 — replay: same forged envelope fired 30× ─────────────────────────
    let step = timed_step_async("A2 replay same attestation over QUIC", || async {
        let bytes = forged_attestation(evil_id, target_id, "replay-const", &evil_key);
        let before = target.presence_metrics().await.unwrap_or_default();
        for _ in 0..30 {
            let _ = attacker.send_raw(target_id, &bytes).await;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
        let m = target.presence_metrics().await.unwrap_or_default();
        if m.accepted > before.accepted {
            return Err("BREACH: replayed attestation accepted".into());
        }
        Ok(format!("30 replays, 0 accepted (drops now {})", m.drop_unknown_challenge))
    })
    .await;
    result.add(step);

    // ── Malformed: PresenceAttestation msg_type, garbage payload ────────────
    let step = timed_step_async("malformed presence payload over QUIC", || async {
        for _ in 0..20 {
            let bytes = EnvelopeBuilder::new(
                evil_id,
                target_id,
                MessageType::PresenceAttestation,
                vec![0xFFu8; 64], // not valid MessagePack for the payload
            )
            .sign(&evil_key)
            .to_bytes()
            .unwrap();
            let _ = attacker.send_raw(target_id, &bytes).await;
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
        // Survival is the assertion: the target must still answer a query.
        match tokio::time::timeout(Duration::from_secs(3), target.presence_metrics()).await {
            Ok(Some(_)) => Ok("garbage payloads absorbed, target still responsive".into()),
            _ => Err("BREACH: target unresponsive after malformed flood".into()),
        }
    })
    .await;
    result.add(step);

    // ── Challenge flood: signed challenges beyond the responder budget ──────
    let step = timed_step_async("challenge flood over QUIC (budget)", || async {
        let before = target.presence_metrics().await.unwrap_or_default();
        for i in 0..40 {
            let payload = PresenceChallengePayload {
                challenge_id: format!("flood-{i}"),
                nonce: vec![9u8; NONCE_LEN],
                timestamp: tom_protocol::now_ms(),
                challenger_id: evil_id,
            };
            let bytes = EnvelopeBuilder::new(
                evil_id,
                target_id,
                MessageType::PresenceChallenge,
                payload.to_bytes(),
            )
            .sign(&evil_key)
            .to_bytes()
            .unwrap();
            let _ = attacker.send_raw(target_id, &bytes).await;
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        let m = target.presence_metrics().await.unwrap_or_default();
        let received = m.challenges_received - before.challenges_received;
        let signed = m.signed - before.signed;
        if received == 0 {
            return Err("no challenges reached the target (connectivity)".into());
        }
        if signed >= received && received > 10 {
            return Err(format!(
                "BREACH: responder budget never engaged ({signed} signed of {received})"
            ));
        }
        Ok(format!("{received} challenges reached, {signed} signed (budget capped)"))
    })
    .await;
    result.add(step);

    // ── SYBIL SWARM: budget bypass via identity rotation ────────────────────
    // Hypothesis: the responder budget (RESPONDER_BUDGET_PER_WINDOW) is keyed
    // by CHALLENGER identity (envelope.from). An attacker rotating N forged
    // identities would get N × budget signatures — no GLOBAL cap on signing
    // work → CPU DoS. Each identity is a real keypair with a valid signature,
    // so every challenge is individually legitimate.
    let step = timed_step_async("sybil swarm (budget bypass via identity rotation)", || async {
        const SYBILS: u8 = 20;
        const PER_SYBIL: usize = 15; // exceed the per-identity budget (10)
        let before = target.presence_metrics().await.unwrap_or_default();
        for s in 0..SYBILS {
            let (sid, skey) = keypair(100 + s); // a distinct forged identity
            for i in 0..PER_SYBIL {
                let payload = PresenceChallengePayload {
                    challenge_id: format!("sybil-{s}-{i}"),
                    nonce: vec![5u8; NONCE_LEN],
                    timestamp: tom_protocol::now_ms(),
                    challenger_id: sid,
                };
                let bytes = EnvelopeBuilder::new(
                    sid,
                    target_id,
                    MessageType::PresenceChallenge,
                    payload.to_bytes(),
                )
                .sign(&skey)
                .to_bytes()
                .unwrap();
                let _ = attacker.send_raw(target_id, &bytes).await;
                tokio::time::sleep(Duration::from_millis(4)).await;
            }
        }
        tokio::time::sleep(Duration::from_millis(800)).await;
        let m = target.presence_metrics().await.unwrap_or_default();
        let signed = m.signed - before.signed;
        let received = m.challenges_received - before.challenges_received;
        // Per-identity budget is 10; global budget window count is bounded
        // (MAX_RESPONDER_WINDOWS). With 20 identities, a per-identity-only cap
        // yields ~200 signatures. A GLOBAL cap would keep it far lower.
        let per_id = tom_protocol::presence::RESPONDER_BUDGET_PER_WINDOW as u64;
        let global = tom_protocol::presence::RESPONDER_GLOBAL_BUDGET_PER_WINDOW as u64;
        let per_identity_worst = per_id * SYBILS as u64;
        // FINDING #4 fix: signing must be bounded by the GLOBAL cap, NOT scale
        // with attacker identities. If signed approaches per_identity_worst
        // (10×20=200), the global cap is absent/broken → CPU DoS.
        if signed > global + global / 4 {
            return Err(format!(
                "BREACH: sybil swarm extracted {signed} signatures ({received} recv) — \
                 exceeds global cap {global}; scales toward {per_identity_worst} with {SYBILS} ids (CPU DoS)"
            ));
        }
        Ok(format!(
            "{received} recv, {signed} signed — GLOBAL cap {global} held under {SYBILS} identities \
             (per-identity worst would be {per_identity_worst})"
        ))
    })
    .await;
    result.add(step);

    // ── Final: target fully responsive + zero illegit acceptances ───────────
    let step = timed_step_async("target intact after full assault", || async {
        let m = target.presence_metrics().await.unwrap_or_default();
        if m.accepted > baseline.accepted {
            return Err(format!("BREACH: {} illegit acceptances", m.accepted - baseline.accepted));
        }
        Ok(format!(
            "0 illegit accepted; drops unknown={} badsig={} incoherent={}; signed={}",
            m.drop_unknown_challenge, m.drop_bad_signature, m.drop_incoherent, m.signed
        ))
    })
    .await;
    result.add(step);

    eprintln!(
        "[attack] target defended: forge+replay+malformed+flood over live QUIC, 0 illegit accepted"
    );

    let _ = tokio::time::timeout(Duration::from_secs(3), target.shutdown()).await;
    result.total_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(result)
}
