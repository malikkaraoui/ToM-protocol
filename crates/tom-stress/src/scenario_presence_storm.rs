//! L1-001 Presence STORM — network stress + measurement (build 20).
//!
//! Spins up a small mesh of REAL nodes over live QUIC and hammers the
//! Proof-of-Presence path: every node challenges every other node, in
//! repeated bursts, for a fixed duration. Then it reads the per-outcome
//! presence counters (build 20) from each node and reports the aggregate:
//! acceptance ratio, latency min/mean/max, and the full drop breakdown.
//!
//! Gate is set to 0.0 (plumbing mode) so acceptances happen at scale
//! without needing relay evidence for every pair — the gate's real-network
//! behavior WITH evidence is already proven by `scenario_presence`. Here we
//! measure throughput, latency under load, and that nothing crashes or
//! leaks memory on constrained devices.
//!
//! Everything is emitted as JSONL (`emit_jsonl`) so an autonomous harness
//! can collect relevés across many runs.

use std::time::{Duration, Instant};

use tom_protocol::{PresenceMetrics, ProtocolRuntime, RuntimeConfig, RuntimeHandle};
use tom_transport::{TomNode, TomNodeConfig};

use crate::scenario_common::{timed_step_async, ScenarioResult};

/// Nodes in the mesh.
const NODE_COUNT: usize = 4;
/// Challenge bursts (each burst = every node challenges all its Online peers).
const BURSTS: usize = 20;
/// Pause between bursts (keeps us under the responder budget of 10/30s/peer).
const BURST_PACE_MS: u64 = 400;

struct StormNode {
    handle: RuntimeHandle,
}

pub async fn run() -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("presence-storm");
    let start = Instant::now();

    // ── Spawn NODE_COUNT real nodes (gate 0.0 = plumbing) ───────────
    let mut nodes: Vec<StormNode> = Vec::with_capacity(NODE_COUNT);
    let mut addrs = Vec::with_capacity(NODE_COUNT);
    let mut ids = Vec::with_capacity(NODE_COUNT);

    for i in 0..NODE_COUNT {
        let node = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
        ids.push(node.id());
        addrs.push(node.addr());
        let cfg = RuntimeConfig {
            username: format!("storm-{i}"),
            encryption: false,
            enable_dht: false, // red-team: never pollute the shared DHT rendezvous
            presence_contribution_min: 0.0,
            ..Default::default()
        };
        let channels = ProtocolRuntime::spawn(node, cfg);
        nodes.push(StormNode {
            handle: channels.handle,
        });
    }

    // ── Mesh: every node learns every other node's address ──────────
    let step = timed_step_async("mesh N nodes", || async {
        for (i, n) in nodes.iter().enumerate() {
            for (j, addr) in addrs.iter().enumerate() {
                if i != j {
                    n.handle.add_peer_addr(addr.clone()).await;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(1200)).await;
        Ok(format!("{NODE_COUNT} nodes meshed"))
    })
    .await;
    result.add(step);

    // ── Warmup: one gentle challenge round so QUIC paths establish ──
    // (the FIRST challenge to a cold peer pays connection-setup latency —
    // a real fleet effect we surface separately from steady-state load).
    let step = timed_step_async("warmup connections", || async {
        for n in &nodes {
            n.handle.check_presence_all_online().await;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok("paths warmed".into())
    })
    .await;
    result.add(step);

    // ── Storm: repeated challenge-all bursts across the whole mesh ───
    let step = timed_step_async("challenge storm", || async {
        for _ in 0..BURSTS {
            for n in &nodes {
                n.handle.check_presence_all_online().await;
            }
            tokio::time::sleep(Duration::from_millis(BURST_PACE_MS)).await;
        }
        // Let the last attestations settle.
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(format!(
            "{BURSTS} bursts × {NODE_COUNT} nodes = {} challenge waves",
            BURSTS * NODE_COUNT
        ))
    })
    .await;
    result.add(step);

    // ── Collect per-node metrics and aggregate ──────────────────────
    let mut agg = PresenceMetrics::default();
    let mut per_node = Vec::new();
    for (i, n) in nodes.iter().enumerate() {
        if let Some(m) = n.handle.presence_metrics().await {
            per_node.push((i, m));
            agg.issued += m.issued;
            agg.accepted += m.accepted;
            agg.drop_unknown_challenge += m.drop_unknown_challenge;
            agg.drop_stale += m.drop_stale;
            agg.drop_wrong_attester += m.drop_wrong_attester;
            agg.drop_bad_signature += m.drop_bad_signature;
            agg.drop_incoherent += m.drop_incoherent;
            agg.drop_gate += m.drop_gate;
            agg.drop_store_full += m.drop_store_full;
            agg.challenges_received += m.challenges_received;
            agg.signed += m.signed;
            agg.latency_max_ms = agg.latency_max_ms.max(m.latency_max_ms);
            agg.latency_sum_ms = agg.latency_sum_ms.saturating_add(m.latency_sum_ms);
            agg.latency_min_ms = if agg.latency_min_ms == 0 {
                m.latency_min_ms
            } else if m.latency_min_ms == 0 {
                agg.latency_min_ms
            } else {
                agg.latency_min_ms.min(m.latency_min_ms)
            };
        }
    }

    let step = timed_step_async("throughput + latency", || async {
        if agg.issued == 0 {
            return Err("no challenges issued — mesh never connected".into());
        }
        if agg.accepted == 0 {
            return Err("zero acceptances despite gate 0.0 — plumbing broken".into());
        }
        let ratio = agg.accepted as f64 / agg.issued as f64;
        let mean = agg.latency_sum_ms.checked_div(agg.accepted).unwrap_or(0);
        // The storm DELIBERATELY exceeds the responder budget
        // (RESPONDER_BUDGET_PER_WINDOW = 10/challenger/30s), so a low
        // acceptance ratio is the budget WORKING, not a failure. What must
        // hold under load:
        //  (a) plumbing alive — acceptances actually happen;
        //  (b) the budget bites — some received challenges go unsigned;
        //  (c) steady-state latency stays sane after warmup.
        if agg.accepted == 0 {
            return Err("plumbing dead: zero acceptances at gate 0.0".into());
        }
        if agg.signed >= agg.challenges_received {
            return Err(format!(
                "responder budget never engaged: signed {} of {} received",
                agg.signed, agg.challenges_received
            ));
        }
        // Steady-state gate: mean latency after warmup. The MAX is reported
        // (cold-connection outliers are a real fleet effect, not a failure);
        // the warm-median <200ms criterion lives in `scenario_presence`.
        if mean > 1500 {
            return Err(format!(
                "mean latency {mean}ms > 1500ms steady-state (max {}ms)",
                agg.latency_max_ms
            ));
        }
        Ok(format!(
            "{}/{} accepted ({:.0}%, budget-capped), latency {}/{}/{}ms (min/mean/max)",
            agg.accepted,
            agg.issued,
            ratio * 100.0,
            agg.latency_min_ms,
            mean,
            agg.latency_max_ms
        ))
    })
    .await;
    result.add(step);

    // ── Integrity: aggregation window bounded, no crash ─────────────
    let step = timed_step_async("bounded windows + seed", || async {
        let mut total_window = 0u64;
        for n in &nodes {
            if let Some((seed, count)) = n.handle.presence_seed().await {
                total_window += count as u64;
                // MAX_STORED_ATTESTATIONS = 512 per node.
                if count > 512 {
                    return Err(format!("window {count} exceeds cap 512"));
                }
                let _ = seed;
            }
        }
        Ok(format!("aggregation windows healthy (total {total_window})"))
    })
    .await;
    result.add(step);

    // Emit the full aggregate as a machine-readable relevé line.
    eprintln!(
        "[storm] issued={} accepted={} received={} signed={} refused_budget(via received-signed)={} drops(unknown={},stale={},wrong={},badsig={},incoh={},gate={},full={}) lat(min/mean/max)={}/{}/{}ms",
        agg.issued,
        agg.accepted,
        agg.challenges_received,
        agg.signed,
        agg.challenges_received.saturating_sub(agg.signed),
        agg.drop_unknown_challenge,
        agg.drop_stale,
        agg.drop_wrong_attester,
        agg.drop_bad_signature,
        agg.drop_incoherent,
        agg.drop_gate,
        agg.drop_store_full,
        agg.latency_min_ms,
        agg.latency_sum_ms.checked_div(agg.accepted).unwrap_or(0),
        agg.latency_max_ms,
    );
    for (i, m) in &per_node {
        eprintln!(
            "[storm]   node {i}: issued={} accepted={} received={} signed={}",
            m.issued, m.accepted, m.challenges_received, m.signed
        );
    }

    // ── Shutdown ────────────────────────────────────────────────────
    for n in &nodes {
        n.handle.shutdown().await;
    }

    result.total_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(result)
}
