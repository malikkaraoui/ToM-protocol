//! L1-001 Presence scenario — REAL network validation (QUIC, 3 nodes).
//!
//! Topology: A ↔ B ↔ C (A and C are NOT directly connected). The scenario
//! validates the full Proof-of-Presence flow over live transport:
//!
//! 1. Relay evidence — A sends chats to C THROUGH B; the signed
//!    RelayForwarded ACKs coming back give A local, cryptographic evidence
//!    that B relays (this is the production anti-Sybil gate input).
//! 2. Challenge round — A challenges B's presence N times; attestations
//!    must come back verified, with median round-trip < 200 ms (M1.1
//!    acceptance criterion).
//! 3. Gate check — A challenges C, for which it holds NO relay evidence:
//!    C's (honest, signed) attestation must be silently dropped by the
//!    local-score gate. Network-level proof that self-reported activity
//!    buys nothing.
//! 4. Entropy window — the aggregation seed is non-empty and the
//!    attestation count matches the accepted round.

use std::time::{Duration, Instant};

use tom_protocol::{
    PeerInfo, PeerRole, PeerStatus, ProtocolEvent, ProtocolRuntime, RuntimeConfig,
};
use tom_transport::{now_ms, TomNode, TomNodeConfig};

use crate::scenario_common::{timed_step_async, ScenarioResult};

/// Challenges issued toward the well-scored relay B.
const CHALLENGE_COUNT: usize = 10;

/// M1.1 acceptance criterion: median challenge→attestation round-trip.
const MEDIAN_LATENCY_BUDGET_MS: u64 = 200;

pub async fn run() -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("presence");
    let start = Instant::now();

    // ── Spawn three nodes: A ↔ B ↔ C ────────────────────────────────
    let node_a = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let node_b = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let node_c = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;

    let id_a = node_a.id();
    let id_b = node_b.id();
    let id_c = node_c.id();
    let addr_a = node_a.addr();
    let addr_b = node_b.addr();
    let addr_c = node_c.addr();

    eprintln!("Node A (challenger): {id_a}");
    eprintln!("Node B (relay):      {id_b}");
    eprintln!("Node C (no-evidence):{id_c}");

    let cfg = |name: &str| RuntimeConfig {
        username: format!("{}{}", tom_protocol::TEST_NODE_PREFIX, name),
        encryption: false,
        enable_dht: false, // red-team: never pollute the shared DHT rendezvous
        ..Default::default()
    };
    let mut channels_a = ProtocolRuntime::spawn(node_a, cfg("alice"));
    let channels_b = ProtocolRuntime::spawn(node_b, cfg("bob"));
    let mut channels_c = ProtocolRuntime::spawn(node_c, cfg("carol"));

    // ── Wire the line topology (A—B, B—C; never A—C) ────────────────
    let step = timed_step_async("wire A-B-C line topology", || async {
        channels_a.handle.add_peer_addr(addr_b.clone()).await;
        channels_b.handle.add_peer_addr(addr_a).await;
        channels_b.handle.add_peer_addr(addr_c).await;
        channels_c.handle.add_peer_addr(addr_b).await;

        // A knows B is a relay candidate and knows C exists (id only, no
        // address): the relay selector must route A→C through B.
        channels_a
            .handle
            .upsert_peer(PeerInfo {
                node_id: id_b,
                role: PeerRole::Relay,
                status: PeerStatus::Online,
                last_seen: now_ms(),
            })
            .await;
        channels_a
            .handle
            .upsert_peer(PeerInfo {
                node_id: id_c,
                role: PeerRole::Peer,
                status: PeerStatus::Online,
                last_seen: now_ms(),
            })
            .await;

        tokio::time::sleep(Duration::from_millis(750)).await;
        Ok("A-B, B-C connected; A holds C as id-only peer".into())
    })
    .await;
    result.add(step);

    // ── Step 1: earn REAL relay evidence for B at A ─────────────────
    //
    // Subtlety seen live: B gossips PeerAnnounce with role=Peer, which
    // overwrites our Relay hint in A's topology — the selector then dials
    // C directly and B never relays. So the Relay hint is re-upserted
    // immediately before EACH send: UpsertPeer and SendMessage are
    // processed back-to-back by the same runtime loop, closing the race.
    // The evidence itself stays real: a signed RelayForwarded ACK from B
    // over live QUIC.
    let step = timed_step_async("earn relay evidence (A→C via B)", || async {
        let mut delivered = 0u32;
        for attempt in 0..8 {
            channels_a
                .handle
                .upsert_peer(PeerInfo {
                    node_id: id_b,
                    role: PeerRole::Relay,
                    status: PeerStatus::Online,
                    last_seen: now_ms(),
                })
                .await;
            channels_a
                .handle
                .send_message(id_c, format!("relayed-{attempt}").into_bytes())
                .await
                .map_err(|e| format!("send failed: {e}"))?;

            // Drain C's inbox (proves the path delivers end-to-end).
            if let Ok(_msg) = crate::scenario_common::recv_timeout(
                &mut channels_c.messages,
                Duration::from_millis(800),
            )
            .await
            {
                delivered += 1;
            }

            let score = channels_a
                .handle
                .get_role_metrics(id_b)
                .await
                .map(|m| m.score)
                .unwrap_or(0.0);
            if score >= 2.0 {
                return Ok(format!(
                    "B local score at A = {score:.2} (gate open) after {} send(s), {delivered} delivered at C",
                    attempt + 1
                ));
            }
            // Stay under the low-score anti-spam baseline.
            tokio::time::sleep(Duration::from_millis(600)).await;
        }
        Err("B local score at A stuck < 2.0 after 8 relayed sends — no relay evidence".into())
    })
    .await;
    result.add(step);

    // ── Step 2: challenge round A → B over live QUIC ────────────────
    let step = timed_step_async("presence challenges A→B (median < 200ms)", || async {
        let mut latencies: Vec<u64> = Vec::new();

        for _ in 0..CHALLENGE_COUNT {
            channels_a.handle.check_presence(id_b).await;

            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                match crate::scenario_common::recv_timeout(
                    &mut channels_a.events,
                    Duration::from_millis(500),
                )
                .await
                {
                    Ok(ProtocolEvent::PresenceAttestationReceived {
                        attester_id,
                        latency_ms,
                        ..
                    }) if attester_id == id_b => {
                        latencies.push(latency_ms);
                        break;
                    }
                    _ => continue, // unrelated event or poll timeout
                }
            }
            // Respect B's responder budget window pacing.
            tokio::time::sleep(Duration::from_millis(150)).await;
        }

        if latencies.len() < CHALLENGE_COUNT * 8 / 10 {
            return Err(format!(
                "only {}/{CHALLENGE_COUNT} attestations accepted",
                latencies.len()
            ));
        }
        latencies.sort_unstable();
        let median = latencies[latencies.len() / 2];
        if median > MEDIAN_LATENCY_BUDGET_MS {
            return Err(format!(
                "median latency {median}ms > {MEDIAN_LATENCY_BUDGET_MS}ms budget"
            ));
        }
        Ok(format!(
            "{}/{CHALLENGE_COUNT} attested, median {median}ms",
            latencies.len()
        ))
    })
    .await;
    result.add(step);

    // ── Step 3: gate check — C has no relay evidence at A ───────────
    let step = timed_step_async("anti-Sybil gate drops no-evidence attester", || async {
        channels_a.handle.check_presence(id_c).await;

        // C is honest and will answer — but A must DROP the attestation:
        // no PresenceAttestationReceived event for C may surface.
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if let Ok(ProtocolEvent::PresenceAttestationReceived { attester_id, .. }) =
                crate::scenario_common::recv_timeout(
                    &mut channels_a.events,
                    Duration::from_millis(500),
                )
                .await
            {
                if attester_id == id_c {
                    return Err("gate FAILED: attestation from zero-score peer accepted".into());
                }
            }
        }
        Ok("attestation from zero-evidence peer silently dropped".into())
    })
    .await;
    result.add(step);

    // ── Step 4: entropy window sanity ───────────────────────────────
    let step = timed_step_async("aggregation window + seed", || async {
        match channels_a.handle.presence_seed().await {
            Some((seed, count)) => {
                if count == 0 {
                    return Err("aggregation window empty after accepted round".into());
                }
                if seed == [0u8; 32] {
                    return Err("seed is all-zero".into());
                }
                Ok(format!("window={count} attestations, seed[0..4]={:02x?}", &seed[..4]))
            }
            None => Err("presence_seed query failed".into()),
        }
    })
    .await;
    result.add(step);

    // ── Shutdown ────────────────────────────────────────────────────
    channels_a.handle.shutdown().await;
    channels_b.handle.shutdown().await;
    channels_c.handle.shutdown().await;

    result.total_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(result)
}
