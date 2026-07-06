//! Chaos Monkey for ToM presence (build 21) — Simian-Army style resilience.
//!
//! Spins up a controlled fleet doing honest Proof-of-Presence, then unleashes
//! RANDOM, AGGRESSIVE faults on it — the kind a real fleet suffers but that
//! are hard to reproduce by hand:
//!
//! - **KILL**  — a node vanishes abruptly (battery dead / OS force-quit /
//!   network gone): its runtime is shut down mid-flight, no goodbye.
//! - **REVIVE** — a dead slot comes back as a fresh node and must re-join and
//!   resume attesting (a device rebooting / reconnecting).
//! - **SKEW**  — a live node's presence clock is jerked by a random offset
//!   (build 20 sim) to stress the anti-NTP freshness path under churn.
//!
//! Faults are drawn from a SEEDED RNG so a failing run is reproducible
//! (`--seed`). Throughout, the survivors keep challenging each other; at the
//! end we heal the fleet and assert presence RESUMES — the network must
//! self-repair, never wedge. Timeline + per-fault relevés are emitted as
//! JSONL for autonomous collection.

use std::time::{Duration, Instant};

use rand::{Rng, SeedableRng};
use tom_protocol::{ProtocolRuntime, RuntimeConfig, RuntimeHandle};
use tom_transport::{EndpointAddr, TomNode, TomNodeConfig};

use crate::scenario_common::{timed_step_async, ScenarioResult};

const NODE_COUNT: usize = 4;
/// Number of chaos events to inject.
const CHAOS_EVENTS: usize = 16;
/// Pause between chaos events (presence traffic flows in between).
const EVENT_PACE_MS: u64 = 500;
/// Never let the fleet drop below this many live nodes (keep a network alive).
const MIN_ALIVE: usize = 2;

struct Slot {
    handle: RuntimeHandle,
    addr: EndpointAddr,
    id: tom_protocol::NodeId,
}

async fn spawn_node(index: usize) -> anyhow::Result<Slot> {
    // Empty relay list + no n0 discovery → fast local binds (no DNS-failing
    // default-relay probing on every revive).
    let node = TomNode::bind(
        TomNodeConfig::new()
            .n0_discovery(false)
            .relay_urls(vec![]),
    )
    .await?;
    let id = node.id();
    let addr = node.addr();
    let cfg = RuntimeConfig {
        username: format!("chaos-{index}"),
        encryption: false,
        presence_contribution_min: 0.0,
        ..Default::default()
    };
    let channels = ProtocolRuntime::spawn(node, cfg);
    Ok(Slot {
        handle: channels.handle,
        addr,
        id,
    })
}

/// Cross-register `newcomer` with every currently-live slot.
async fn remesh(slots: &[Option<Slot>], newcomer_idx: usize) {
    let newcomer_addr = slots[newcomer_idx].as_ref().unwrap().addr.clone();
    for (i, s) in slots.iter().enumerate() {
        if i == newcomer_idx {
            continue;
        }
        if let Some(other) = s {
            other.handle.add_peer_addr(newcomer_addr.clone()).await;
            slots[newcomer_idx]
                .as_ref()
                .unwrap()
                .handle
                .add_peer_addr(other.addr.clone())
                .await;
        }
    }
}

pub async fn run_with_seed(seed: u64) -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("chaos-monkey");
    let start = Instant::now();
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // ── Spawn + mesh the initial fleet ──────────────────────────────
    let mut slots: Vec<Option<Slot>> = Vec::with_capacity(NODE_COUNT);
    for i in 0..NODE_COUNT {
        slots.push(Some(spawn_node(i).await?));
    }
    let step = timed_step_async("spawn + mesh fleet", || async {
        for i in 0..NODE_COUNT {
            remesh(&slots, i).await;
        }
        tokio::time::sleep(Duration::from_millis(1200)).await;
        Ok(format!("{NODE_COUNT} nodes live (seed {seed})"))
    })
    .await;
    result.add(step);

    // ── Chaos loop ──────────────────────────────────────────────────
    let mut kills = 0u32;
    let mut revives = 0u32;
    let mut skews = 0u32;
    let mut min_alive_seen = NODE_COUNT;

    let step = timed_step_async("inject chaos", || async {
        for evt in 0..CHAOS_EVENTS {
            let alive: Vec<usize> = slots
                .iter()
                .enumerate()
                .filter_map(|(i, s)| s.as_ref().map(|_| i))
                .collect();
            let dead: Vec<usize> = slots
                .iter()
                .enumerate()
                .filter_map(|(i, s)| if s.is_none() { Some(i) } else { None })
                .collect();

            // Choose an action, respecting MIN_ALIVE.
            let roll = rng.random_range(0u8..100);
            let action = if !dead.is_empty() && roll < 35 {
                "revive"
            } else if alive.len() > MIN_ALIVE && roll < 75 {
                "kill"
            } else {
                "skew"
            };

            match action {
                "kill" => {
                    let victim = alive[rng.random_range(0..alive.len())];
                    if let Some(slot) = slots[victim].take() {
                        let dead_id = slot.id;
                        // Abrupt death: shut the runtime, drop the handle.
                        slot.handle.shutdown().await;
                        // Survivors drop the ghost from topology so the test
                        // doesn't drown in timing-out dials to a corpse (the
                        // 45s liveness-detection path is a separate scenario).
                        for s in slots.iter().flatten() {
                            s.handle.remove_peer(dead_id).await;
                        }
                        kills += 1;
                        eprintln!("[chaos {evt}] KILL node {victim} ({} left)", alive.len() - 1);
                    }
                }
                "revive" => {
                    let slot_idx = dead[rng.random_range(0..dead.len())];
                    match spawn_node(slot_idx).await {
                        Ok(s) => {
                            slots[slot_idx] = Some(s);
                            remesh(&slots, slot_idx).await;
                            revives += 1;
                            eprintln!("[chaos {evt}] REVIVE node {slot_idx}");
                        }
                        Err(e) => eprintln!("[chaos {evt}] revive failed: {e}"),
                    }
                }
                _ => {
                    let victim = alive[rng.random_range(0..alive.len())];
                    let offset = rng.random_range(-90_000i64..90_000);
                    if let Some(slot) = slots[victim].as_ref() {
                        slot.handle.set_presence_clock_offset(offset).await;
                        skews += 1;
                        eprintln!("[chaos {evt}] SKEW node {victim} by {offset}ms");
                    }
                }
            }

            min_alive_seen = min_alive_seen.min(slots.iter().filter(|s| s.is_some()).count());

            // Presence traffic flows between chaos events.
            for s in slots.iter().flatten() {
                s.handle.check_presence_all_online().await;
            }
            tokio::time::sleep(Duration::from_millis(EVENT_PACE_MS)).await;
        }
        Ok(format!(
            "{CHAOS_EVENTS} events: {kills} kills, {revives} revives, {skews} skews; min alive={min_alive_seen}"
        ))
    })
    .await;
    result.add(step);

    // ── Heal: revive every dead slot, reset any skew, settle ────────
    let step = timed_step_async("heal fleet", || async {
        for i in 0..NODE_COUNT {
            if slots[i].is_none() {
                if let Ok(s) = spawn_node(i).await {
                    slots[i] = Some(s);
                    remesh(&slots, i).await;
                }
            } else if let Some(s) = slots[i].as_ref() {
                s.handle.set_presence_clock_offset(0).await; // clear skew
            }
        }
        // Re-mesh everyone with everyone (revived nodes have fresh identities;
        // survivors dropped ghosts on kill, so re-teach the full topology).
        for i in 0..NODE_COUNT {
            remesh(&slots, i).await;
        }
        // Warm the QUIC paths (freshly-bound nodes pay cold-connection setup).
        // Guarded by a timeout: FINDING #1 — a node's runtime loop can stop
        // replying after churn+skew (see docs/redteam/JOURNAL.md). Without the
        // guard the whole scenario hangs on that one wedged node.
        for s in slots.iter().flatten() {
            let _ = tokio::time::timeout(
                Duration::from_secs(3),
                s.handle.check_presence_all_online(),
            )
            .await;
        }
        tokio::time::sleep(Duration::from_millis(4000)).await;
        let alive = slots.iter().filter(|s| s.is_some()).count();
        if alive != NODE_COUNT {
            return Err(format!("only {alive}/{NODE_COUNT} nodes revived"));
        }
        Ok(format!("{NODE_COUNT} nodes healed + rewired"))
    })
    .await;
    result.add(step);

    // ── Prove presence RESUMES after the storm of faults ────────────
    let before: u64 = collect_accepted(&slots).await;
    let step = timed_step_async("presence resumes after chaos", || async {
        // Clean challenge rounds on the healed, warmed fleet.
        for _ in 0..8 {
            for s in slots.iter().flatten() {
                s.handle.check_presence_all_online().await;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let after = collect_accepted(&slots).await;
        if after <= before {
            return Err(format!(
                "presence did NOT resume: accepted stuck at {before} → {after}"
            ));
        }
        Ok(format!("network self-healed: accepted {before} → {after}"))
    })
    .await;
    result.add(step);

    // ── Never fully died ────────────────────────────────────────────
    let step = timed_step_async("network never fully died", || async {
        if min_alive_seen < 1 {
            return Err("fleet reached zero live nodes".into());
        }
        Ok(format!("min alive across run: {min_alive_seen}"))
    })
    .await;
    result.add(step);

    // JSONL relevé.
    eprintln!(
        "[chaos] seed={seed} kills={kills} revives={revives} skews={skews} min_alive={min_alive_seen}"
    );

    for s in slots.iter().flatten() {
        // Guarded: a wedged node (FINDING #1) must not hang teardown either.
        let _ = tokio::time::timeout(Duration::from_secs(3), s.handle.shutdown()).await;
    }
    result.total_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(result)
}

async fn collect_accepted(slots: &[Option<Slot>]) -> u64 {
    let mut total = 0;
    for (i, s) in slots.iter().enumerate() {
        if let Some(s) = s {
            match tokio::time::timeout(Duration::from_secs(3), s.handle.presence_metrics()).await {
                Ok(Some(m)) => total += m.accepted,
                Ok(None) => eprintln!("[collect] node {i}: metrics None"),
                Err(_) => eprintln!("[collect] node {i}: presence_metrics() TIMEOUT — runtime loop not replying"),
            }
        }
    }
    total
}
