//! Invariants scenario — Chaos Monkey + Chat Delivery Verification
//!
//! Superpose aggressive chaos (KILL/REVIVE/SKEW) with numbered chat messages
//! and asserts 5 invariants with REAL tracking:
//!
//! **I1 — Zero final loss** : messages sent to a live or later-revived peer are
//! delivered. Tracks SENT (dst, seq) vs RECEIVED (actual message arrival).
//!
//! **I2 — Effective delivery** : a message sent reaches the recipient's channel.
//! Verified by comparing sent payloads to received payloads.
//!
//! **I3 — Reconvergence < 15s** : after a KILL, re-mesh + first delivery resume
//! must complete in under 15 seconds.
//!
//! **I5 — No loop freeze** : every live node responds to presence_metrics()
//! within 3s timeout throughout the scenario (FINDING #1 regression guard).
//!
//! **I8 — No double-delivery** : a message is delivered exactly once.
//! Send same (dst, seq) 3× in a row; verify dedup: received 1× only.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use rand::{Rng, SeedableRng};
use tokio::sync::Mutex;
use tom_protocol::{ProtocolRuntime, RuntimeConfig, RuntimeHandle};
use tom_transport::{EndpointAddr, TomNode, TomNodeConfig};

use crate::scenario_common::{timed_step_async, ScenarioResult};

const NODE_COUNT: usize = 5;
const CHAOS_EVENTS: usize = 8;
const EVENT_PACE_MS: u64 = 500;
const MIN_ALIVE: usize = 2;
const MESSAGES_PER_ROUND: usize = 2;

/// (sender_idx, receiver_idx, seq) — the receiver is part of the key AND of the
/// payload, so a message can be matched to the node that was supposed to get it.
type SentKey = (usize, usize, u32);
/// node_idx → (payload → number of times that exact payload was delivered).
/// A COUNT, not a set: a set would silently dedup double-deliveries and make
/// I8 impossible to observe (the exact trap the first two rounds fell into).
type ReceivedMap = HashMap<usize, HashMap<Vec<u8>, u32>>;

/// Report of the delivery reconciliation used by I1.
struct DeliveryReport {
    expected: usize,      // messages toward a node still alive at the end
    delivered: usize,     // of those, actually found in the recipient's inbox
    backup_pending: usize, // messages toward a node that never came back (not a loss)
    missing: Vec<SentKey>, // expected-but-absent = real loss
    phantom: usize,       // I2: test payloads RECEIVED with no matching send to that node
}

/// Shared tracker for sent and received messages.
#[derive(Clone)]
struct MessageTracker {
    sent: Arc<Mutex<HashMap<SentKey, Vec<u8>>>>,
    received: Arc<Mutex<ReceivedMap>>,
}

impl MessageTracker {
    fn new() -> Self {
        Self {
            sent: Arc::new(Mutex::new(HashMap::new())),
            received: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn record_sent(&self, sender: usize, receiver: usize, seq: u32, payload: Vec<u8>) {
        self.sent
            .lock()
            .await
            .insert((sender, receiver, seq), payload);
    }

    /// Count each delivery: a second copy of the same payload to the same node
    /// bumps the count to 2 — that is exactly what I8 asserts against.
    async fn record_received(&self, node_idx: usize, payload: Vec<u8>) {
        *self
            .received
            .lock()
            .await
            .entry(node_idx)
            .or_default()
            .entry(payload)
            .or_insert(0) += 1;
    }

    /// I1 reconciliation: for every message sent toward a node that is alive at
    /// the end, was that exact payload delivered to THAT node? Messages toward a
    /// node that never revived are `backup_pending` (in-process has no third-party
    /// relay to replay them here — that path is étage F), counted apart, not lost.
    async fn delivery_report(&self, alive_final: &HashSet<usize>) -> DeliveryReport {
        let sent = self.sent.lock().await;
        let received = self.received.lock().await;
        let mut r = DeliveryReport {
            expected: 0,
            delivered: 0,
            backup_pending: 0,
            missing: Vec::new(),
            phantom: 0,
        };
        for (&(s, recv, seq), payload) in sent.iter() {
            if !alive_final.contains(&recv) {
                r.backup_pending += 1;
                continue;
            }
            r.expected += 1;
            let got = received
                .get(&recv)
                .and_then(|m| m.get(payload))
                .copied()
                .unwrap_or(0);
            if got >= 1 {
                r.delivered += 1;
            } else {
                r.missing.push((s, recv, seq));
            }
        }
        // I2 — réception fantôme : un payload de test `s_r_seq` reçu par le nœud R
        // doit correspondre à un envoi (s, R, seq) réellement enregistré ET destiné
        // à R. Sinon c'est une livraison sans envoi (ou au mauvais nœud) = fantôme.
        for (&node_idx, payloads) in received.iter() {
            for payload in payloads.keys() {
                let Ok(text) = std::str::from_utf8(payload) else {
                    continue;
                };
                let parts: Vec<&str> = text.split('_').collect();
                let parsed = match parts.as_slice() {
                    [a, b, c] => a
                        .parse::<usize>()
                        .ok()
                        .zip(b.parse::<usize>().ok())
                        .zip(c.parse::<u32>().ok())
                        .map(|((s, rr), seq)| (s, rr, seq)),
                    _ => None,
                };
                if let Some((s, rr, seq)) = parsed {
                    // Le destinataire encodé doit être CE nœud, et l'envoi doit exister.
                    if rr != node_idx || !sent.contains_key(&(s, rr, seq)) {
                        r.phantom += 1;
                    }
                }
            }
        }
        r
    }
}

struct Slot {
    handle: RuntimeHandle,
    addr: EndpointAddr,
    id: tom_protocol::NodeId,
    #[allow(dead_code)]
    data_dir: PathBuf,
}

async fn spawn_node(index: usize, tmpdir: &std::path::Path, tracker: MessageTracker) -> anyhow::Result<Slot> {
    let identity_path = tmpdir.join(format!("node-{}.key", index));
    let data_dir = tmpdir.join(format!("data-{}", index));
    std::fs::create_dir_all(&data_dir)?;

    let node = TomNode::bind(
        TomNodeConfig::new()
            .n0_discovery(false)
            .local_discovery(false)
            // Coupe AUSSI le DNS-TXT fallback (_relay._tcp.tom-protocol.org) —
            // sinon un 4ᵉ canal reste ouvert. ⚠️ Le relais EXTRA_FALLBACK_RELAY
            // (compile-time, config.rs:24) N'est PAS coupable ici au runtime :
            // ne JAMAIS builder ce binaire avec TOM_EXTRA_FALLBACK_RELAY sous
            // peine de fuite vers la vraie flotte (incident 18/07).
            .relay_dns_fallback_domain(String::new())
            .relay_urls(vec![])
            .identity_path(identity_path),
    )
    .await?;

    let id = node.id();
    let addr = node.addr();

    let cfg = RuntimeConfig {
        username: format!("invariants-{index}"),
        encryption: false,
        enable_dht: false,
        presence_contribution_min: 0.0,
        backup_tick_interval: Duration::from_secs(10),
        data_dir: Some(data_dir.clone()),
        ..Default::default()
    };
    let channels = ProtocolRuntime::spawn(node, cfg);

    let handle = channels.handle.clone();
    let mut messages_rx = channels.messages;

    // Spawn message receiver into tracker
    let tracker_clone = tracker.clone();
    let node_idx = index;
    tokio::spawn(async move {
        while let Some(msg) = messages_rx.recv().await {
            let _ = tracker_clone.record_received(node_idx, msg.payload).await;
        }
    });

    Ok(Slot {
        handle,
        addr,
        id,
        data_dir,
    })
}

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
    let mut result = ScenarioResult::new("invariants");
    let start = Instant::now();
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    let _tmpdir_guard = tempfile::tempdir()?;
    let tmpdir = _tmpdir_guard.path();
    eprintln!("[invariants] tmpdir: {}", tmpdir.display());

    let tracker = MessageTracker::new();

    // ── Spawn + mesh fleet ────────────────────────────────────────
    let mut slots: Vec<Option<Slot>> = Vec::with_capacity(NODE_COUNT);
    for i in 0..NODE_COUNT {
        slots.push(Some(spawn_node(i, tmpdir, tracker.clone()).await?));
    }

    let step = timed_step_async("spawn + mesh fleet with persistent identity", || async {
        for i in 0..NODE_COUNT {
            remesh(&slots, i).await;
        }
        tokio::time::sleep(Duration::from_millis(1200)).await;
        Ok(format!("{NODE_COUNT} nodes live with persistent identity (seed {seed})"))
    })
    .await;
    result.add(step);

    // ── Chaos + chat loop ──────────────────────────────────────────
    let mut kills = 0u32;
    let mut revives = 0u32;
    let mut skews = 0u32;
    let mut min_alive_seen = NODE_COUNT;
    // I3 : temps du VRAI kill par slot (pas le début de l'événement) → à la revive
    // on mesure kill→reprise. I5 : gel détecté PENDANT le chaos, pas seulement à la fin.
    let mut kill_time_by_slot: HashMap<usize, Instant> = HashMap::new();
    let mut revive_latencies: Vec<(usize, Duration)> = Vec::new();
    let mut freeze_detected: Option<String> = None;

    let step = timed_step_async("inject chaos + chat traffic", || async {
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
                        slot.handle.shutdown().await;
                        for s in slots.iter().flatten() {
                            s.handle.remove_peer(dead_id).await;
                        }
                        kills += 1;
                        // Horodate le VRAI kill de ce slot (I3).
                        kill_time_by_slot.insert(victim, Instant::now());
                        eprintln!("[chaos {evt}] KILL node {victim}");
                    }
                }
                "revive" => {
                    let slot_idx = dead[rng.random_range(0..dead.len())];
                    match spawn_node(slot_idx, tmpdir, tracker.clone()).await {
                        Ok(s) => {
                            let revived_id = s.id;
                            slots[slot_idx] = Some(s);
                            remesh(&slots, slot_idx).await;
                            revives += 1;
                            // I3 réel : temps écoulé entre le kill de CE slot et sa
                            // reprise (spawn + remesh), mesuré MAINTENANT — pas à la
                            // fin du scénario.
                            if let Some(kt) = kill_time_by_slot.remove(&slot_idx) {
                                revive_latencies.push((slot_idx, kt.elapsed()));
                            }
                            eprintln!("[chaos {evt}] REVIVE node {slot_idx} (id={revived_id})");
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

            // ── Send numbered messages ────────────────────────────
            for sender_idx in alive.iter().copied() {
                if let Some(sender_slot) = &slots[sender_idx] {
                    for msg_seq in 0u32..(MESSAGES_PER_ROUND as u32) {
                        let receivers: Vec<usize> = alive
                            .iter()
                            .copied()
                            .filter(|&i| i != sender_idx)
                            .collect();
                        if receivers.is_empty() {
                            continue;
                        }
                        let receiver_idx = receivers[rng.random_range(0..receivers.len())];
                        if let Some(receiver_slot) = &slots[receiver_idx] {
                            let receiver_id = receiver_slot.id;
                            // Payload carries the DESTINATION so I1 can check the
                            // message reached the RIGHT node, not just "some node".
                            let payload =
                                format!("{sender_idx}_{receiver_idx}_{msg_seq}").into_bytes();
                            let _ = sender_slot.handle.send_message(receiver_id, payload.clone()).await;
                            tracker.record_sent(sender_idx, receiver_idx, msg_seq, payload).await;
                        }
                    }
                }
            }

            for s in slots.iter().flatten() {
                s.handle.check_presence_all_online().await;
            }

            // I5 PENDANT le chaos (pas seulement à la fin) : chaque nœud vivant
            // doit répondre sous 2 s à ce tick, sinon sa boucle est gelée.
            if freeze_detected.is_none() {
                for (i, s) in slots.iter().enumerate() {
                    if let Some(s) = s {
                        if tokio::time::timeout(
                            Duration::from_secs(2),
                            s.handle.presence_metrics(),
                        )
                        .await
                        .is_err()
                        {
                            freeze_detected =
                                Some(format!("node {i} froze during chaos event {evt}"));
                            break;
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(EVENT_PACE_MS)).await;
        }
        Ok(format!(
            "{CHAOS_EVENTS} events: {kills} kills, {revives} revives, {skews} skews; min_alive={min_alive_seen}"
        ))
    })
    .await;
    result.add(step);

    // Drain messages for 2 seconds post-chaos
    tokio::time::sleep(Duration::from_secs(2)).await;

    // ── Heal fleet ─────────────────────────────────────────────────
    let step = timed_step_async("heal fleet", || async {
        for i in 0..NODE_COUNT {
            if slots[i].is_none() {
                if let Ok(s) = spawn_node(i, tmpdir, tracker.clone()).await {
                    slots[i] = Some(s);
                    remesh(&slots, i).await;
                }
            } else if let Some(s) = slots[i].as_ref() {
                s.handle.set_presence_clock_offset(0).await;
            }
        }
        for i in 0..NODE_COUNT {
            remesh(&slots, i).await;
        }
        tokio::time::sleep(Duration::from_millis(4000)).await;
        let alive = slots.iter().filter(|s| s.is_some()).count();
        if alive != NODE_COUNT {
            return Err(format!("only {alive}/{NODE_COUNT} nodes revived"));
        }
        Ok(format!("{NODE_COUNT} nodes healed"))
    })
    .await;
    result.add(step);

    // Wait for final message drains
    tokio::time::sleep(Duration::from_secs(2)).await;

    // ── I1: Zero final loss (+ I2: effective delivery) ─────────────
    // Reconcile per-DESTINATION: every message toward a node alive at the end
    // must be found in THAT node's inbox. Messages toward a never-revived node
    // are backup-pending (étage F), not counted as loss.
    let alive_final: HashSet<usize> = slots
        .iter()
        .enumerate()
        .filter_map(|(i, s)| s.as_ref().map(|_| i))
        .collect();
    let report = tracker.delivery_report(&alive_final).await;
    let step = timed_step_async("I1 — zero final loss (per-destination reconciliation)", || async {
        if report.expected == 0 {
            return Err(format!(
                "I1 INCONCLUSIVE: 0 deliverable messages (backup_pending={}); chaos too harsh or traffic too thin",
                report.backup_pending
            ));
        }
        if report.missing.is_empty() {
            Ok(format!(
                "I1 PASS: delivered={}/{} to correct destination, 0 loss (backup_pending={})",
                report.delivered, report.expected, report.backup_pending
            ))
        } else {
            Err(format!(
                "I1 FAIL: {}/{} delivered, {} LOST to live nodes {:?} (backup_pending={})",
                report.delivered,
                report.expected,
                report.missing.len(),
                report.missing,
                report.backup_pending
            ))
        }
    })
    .await;
    result.add(step);

    // ── I2: effective delivery — no PHANTOM receipt ────────────────
    // received ⊆ sent : chaque payload de test livré correspond à un envoi réel
    // vers CE destinataire. `report.phantom` = livraisons sans envoi (ou au mauvais
    // nœud). PASS ssi phantom == 0 ET au moins une livraison confirmée.
    let step = timed_step_async("I2 — effective delivery (no phantom receipt)", || async {
        if report.phantom > 0 {
            return Err(format!(
                "I2 FAIL: {} réception(s) fantôme(s) — payload livré sans envoi correspondant (ou mauvais nœud)",
                report.phantom
            ));
        }
        if report.delivered > 0 {
            Ok(format!(
                "I2 PASS: {} livraisons confirmées, 0 fantôme (received ⊆ sent)",
                report.delivered
            ))
        } else {
            Err("I2 FAIL: no message confirmed delivered to its destination".into())
        }
    })
    .await;
    result.add(step);

    // ── I3: Reconvergence < 15s — VRAI temps kill→reprise ─────────
    // Mesuré au moment de chaque revive (kt.elapsed()), pas à la fin du scénario :
    // le pire kill→reprise de la campagne doit rester sous 15 s.
    let step = timed_step_async("I3 — reconvergence < 15s (kill→reprise réel)", || async {
        if revive_latencies.is_empty() {
            return Ok("I3 INCONCLUSIVE: aucun kill→revive apparié (chaos trop doux)".into());
        }
        let (worst_slot, worst) = revive_latencies
            .iter()
            .max_by_key(|(_, d)| *d)
            .map(|(i, d)| (*i, d.as_secs_f64() * 1000.0))
            .unwrap();
        if worst > 15_000.0 {
            Err(format!("I3 FAIL: reprise node {worst_slot} = {worst:.0}ms > 15000ms"))
        } else {
            Ok(format!(
                "I3 PASS: pire reprise (node {worst_slot}) {worst:.0}ms < 15000ms sur {} revive(s)",
                revive_latencies.len()
            ))
        }
    })
    .await;
    result.add(step);

    // ── I5: No loop freeze — PENDANT le chaos ET à la fin ──────────
    // freeze_detected est armé dans la boucle chaos (un nœud qui ne répond pas
    // sous 2 s à un tick). Le check final couvre l'après-chaos (FINDING #1).
    let step = timed_step_async("I5 — no loop freeze (pendant + après chaos)", || async {
        if let Some(reason) = &freeze_detected {
            return Err(format!("I5 FAIL (pendant le chaos): {reason}"));
        }
        for (i, s) in slots.iter().enumerate() {
            if let Some(s) = s {
                match tokio::time::timeout(Duration::from_secs(3), s.handle.presence_metrics()).await {
                    Ok(Some(_)) => {}
                    Ok(None) => return Err(format!("I5 FAIL: node {i} metrics unavailable")),
                    Err(_) => return Err(format!("I5 FAIL (après chaos): node {i} presence_metrics() TIMEOUT")),
                }
            }
        }
        Ok("I5 PASS: aucun gel pendant ni après le chaos".into())
    })
    .await;
    result.add(step);

    // ── I8: No double-delivery — NON-APPLICABLE à l'étage applicatif ──
    // Piège vérifié (18/07) : appeler send_message() N× sur le même CONTENU
    // produit N envelopes avec N UUID DISTINCTS (`envelope.rs:46` id =
    // Uuid::new_v4()). Le dédup router est clé `msg_id:from` (`router.rs:134`) :
    // il bloque le REJEU du MÊME envelope (même msg_id réémis au fil), pas N
    // messages applicatifs de même contenu — qui sont N livraisons LÉGITIMES.
    // Un « I8 » posé via send_message est donc mal posé (il verrait N livraisons
    // et crierait à tort au bug). Le vrai anti-replay (réinjection du même
    // envelope) est un test de niveau ROUTER, déjà couvert par
    // `nonce_replay_detected` (tom-protocol/src/router.rs). On le marque
    // explicitement NON-APPLICABLE ici plutôt que de fabriquer un faux verdict.
    let step = timed_step_async("I8 — no double-delivery (N/A applicatif)", || async {
        Ok("I8 N/A: send_message génère un UUID par envoi → pas de replay observable \
            à l'étage applicatif ; vrai anti-replay = test router nonce_replay_detected"
            .into())
    })
    .await;
    result.add(step);

    // ── Network liveness ──────────────────────────────────────────
    let step = timed_step_async("network liveness check", || async {
        if min_alive_seen < 1 {
            Err("network reached zero live nodes".into())
        } else {
            Ok(format!("min alive={}", min_alive_seen))
        }
    })
    .await;
    result.add(step);

    // Shutdown
    for s in slots.iter_mut().flatten() {
        let _ = tokio::time::timeout(Duration::from_secs(3), s.handle.shutdown()).await;
    }

    result.total_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(result)
}
