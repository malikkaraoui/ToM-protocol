//! Banc « rôles sous charge » — Phase A, étage L (in-process, hermétique).
//!
//! Mandat (recadrage 20/07, `docs/plans/PROMPT-REPRISE-ROLES.md`) : le réseau
//! est un organisme à rôles, pas un tuyau send/ACK. Ce banc exerce les rôles
//! QUI EXISTENT, un scénario = un oracle à verdict honnête.
//! Design : `docs/plans/banc-roles-sous-charge.md` · grille :
//! `docs/plans/prisme-des-roles.md` §3. (Le smoke informatif `scenario_roles`
//! de l'étage F reste séparé — lui n'a pas d'oracle dur.)
//!
//! Phase A livrée ici :
//! - **R5 subnets** : un trio qui converse dense FORME un subnet
//!   (`SubnetFormed`), l'inactivité le DISSOUT (`SubnetDissolved`), les nœuds
//!   silencieux n'y entrent jamais (wp §3.3 — le territoire de rôles respire).
//! - **R8 arroseur arrosé** : un spammeur est freiné (`SenderThrottled`) mais
//!   JAMAIS exclu (LOCKED #5, `min_rate` > 0), les victimes ne voient rien
//!   (débit stable), et la rédemption est immédiate à cadence normale
//!   (LOCKED #4, fade réversible).
//!
//! Hermétique par construction : trio isolated (`n0_discovery(false)`,
//! `local_discovery(false)`, `enable_dht: false`) — un nœud de banc ne touche
//! JAMAIS le rendez-vous partagé (incident 20/07).
//!
//! Limites assumées (imprimées au rapport) :
//! - R5 dure ~2 min : l'évaluation des subnets est un tick runtime de 30 s
//!   (`EVALUATION_INTERVAL_MS`, non configurable — seul le timeout de
//!   dissolution l'est, P3).
//! - runtime tokio PARTAGÉ : aucun verdict de latence/débit absolu — R8 ne
//!   compare que des RATIOS internes au même run.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tom_protocol::{NodeId, ProtocolEvent, ProtocolRuntime, RuntimeConfig, RuntimeHandle};
use tom_transport::{TomNode, TomNodeConfig};

/// Câblage transport du mesh de banc.
enum Wiring {
    /// Tous connectés à tous (r5, r8).
    AllPairs,
    /// Arêtes explicites (r1 : A⇄R, R⇄B — surtout PAS A⇄B).
    Links(Vec<(usize, usize)>),
}

/// Observations d'un nœud du banc (alimentées par sa tâche collectrice).
#[derive(Default)]
struct NodeObs {
    /// Messages applicatifs reçus, par expéditeur.
    recv_from: HashMap<NodeId, u64>,
    /// `SubnetFormed` : (subnet_id, membres).
    formed: Vec<(String, Vec<NodeId>)>,
    /// `SubnetDissolved` : (subnet_id, raison).
    dissolved: Vec<(String, String)>,
    /// `SenderThrottled` : expéditeur → occurrences.
    throttled: HashMap<NodeId, u64>,
    /// `Forwarded` (le nœud a relayé) : occurrences.
    forwarded: u64,
    /// `RolePromoted` : node_id promu → score.
    promoted: Vec<(NodeId, f64)>,
}

struct BenchNode {
    id: NodeId,
    handle: RuntimeHandle,
    obs: Arc<Mutex<NodeObs>>,
}

/// Spawn N nœuds hermétiques câblés all-pairs + une tâche collectrice par
/// nœud (messages + événements → `NodeObs`). Pattern du banc courbe.
async fn spawn_mesh(
    n: usize,
    tune: impl Fn(&mut RuntimeConfig),
) -> anyhow::Result<Vec<BenchNode>> {
    spawn_mesh_wired(n, Wiring::AllPairs, tune).await
}

async fn spawn_mesh_wired(
    n: usize,
    wiring: Wiring,
    tune: impl Fn(&mut RuntimeConfig),
) -> anyhow::Result<Vec<BenchNode>> {
    let mut nodes = Vec::with_capacity(n);
    for _ in 0..n {
        nodes.push(
            TomNode::bind(TomNodeConfig::new().n0_discovery(false).local_discovery(false)).await?,
        );
    }
    let addrs: Vec<_> = nodes.iter().map(|nd| nd.addr()).collect();

    let mut out = Vec::with_capacity(n);
    for (i, node) in nodes.into_iter().enumerate() {
        let id = node.id();
        let mut cfg = RuntimeConfig {
            username: format!("{}rolch-{i}", tom_protocol::TEST_NODE_PREFIX),
            encryption: true,
            // CRITIQUE : jamais le rendez-vous partagé (incident 20/07).
            enable_dht: false,
            ..Default::default()
        };
        tune(&mut cfg);
        let channels = ProtocolRuntime::spawn(node, cfg);
        let obs = Arc::new(Mutex::new(NodeObs::default()));

        let mut messages = channels.messages;
        let mut events = channels.events;
        let obs_task = obs.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = messages.recv() => {
                        let Some(msg) = msg else { break };
                        let mut o = obs_task.lock().unwrap();
                        *o.recv_from.entry(msg.from).or_default() += 1;
                    }
                    ev = events.recv() => {
                        let Some(ev) = ev else { break };
                        let mut o = obs_task.lock().unwrap();
                        match ev {
                            ProtocolEvent::SubnetFormed { subnet_id, members } => {
                                o.formed.push((subnet_id, members));
                            }
                            ProtocolEvent::SubnetDissolved { subnet_id, reason } => {
                                o.dissolved.push((subnet_id, reason));
                            }
                            ProtocolEvent::SenderThrottled { node_id, .. } => {
                                *o.throttled.entry(node_id).or_default() += 1;
                            }
                            ProtocolEvent::Forwarded { .. } => {
                                o.forwarded += 1;
                            }
                            ProtocolEvent::RolePromoted { node_id, score } => {
                                o.promoted.push((node_id, score));
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        out.push(BenchNode { id, handle: channels.handle, obs });
    }

    // Câblage transport (adresses directes, aucune découverte).
    match &wiring {
        Wiring::AllPairs => {
            for (i, bn) in out.iter().enumerate() {
                for (j, addr) in addrs.iter().enumerate() {
                    if i != j {
                        bn.handle.add_peer_addr(addr.clone()).await;
                    }
                }
            }
        }
        Wiring::Links(links) => {
            for &(a, b) in links {
                out[a].handle.add_peer_addr(addrs[b].clone()).await;
                out[b].handle.add_peer_addr(addrs[a].clone()).await;
            }
        }
    }
    tokio::time::sleep(Duration::from_secs(2)).await; // chauffe connexions

    Ok(out)
}

/// Teardown borné (leçon banc courbe : sans lui, les nœuds d'un scénario
/// contaminent le suivant).
async fn teardown(nodes: Vec<BenchNode>) {
    for bn in nodes {
        let _ = tokio::time::timeout(Duration::from_secs(5), bn.handle.shutdown()).await;
    }
}

fn verdict(name: &str, ok: bool, detail: &str) -> bool {
    eprintln!("  [{}] {name} — {detail}", if ok { "PASS" } else { "FAIL" });
    ok
}

/// Attend qu'un prédicat devienne vrai sur AU MOINS un des nœuds observés.
async fn wait_for(
    obs: &[&Arc<Mutex<NodeObs>>],
    budget: Duration,
    pred: impl Fn(&NodeObs) -> bool,
) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < budget {
        if obs.iter().any(|o| pred(&o.lock().unwrap())) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

fn count_from(node: &BenchNode, from: NodeId) -> u64 {
    node.obs.lock().unwrap().recv_from.get(&from).copied().unwrap_or(0)
}

/// Envoie un tick vers `to` à cadence fixe pendant `dur`.
async fn drive_traffic(handle: &RuntimeHandle, to: NodeId, dur: Duration, every: Duration) {
    let t0 = Instant::now();
    while t0.elapsed() < dur {
        let _ = handle.send_message(to, b"tick".to_vec()).await;
        tokio::time::sleep(every).await;
    }
}

// ── R5 : subnets éphémères ──────────────────────────────────────────────────

async fn r5_subnets() -> anyhow::Result<bool> {
    eprintln!("\n── R5 : subnets éphémères (formation → frontière → dissolution) ──");
    eprintln!("   6 nœuds ; trio 0-1-2 dense, 3-4-5 silencieux ; dissolution 3 s (P3).");
    eprintln!("   ⏱ éval subnets = tick 30 s (non configurable) → scénario ~2 min.");

    let nodes = spawn_mesh(6, |cfg| {
        cfg.subnet_inactivity_timeout_ms = Some(3_000);
    })
    .await?;
    let trio: Vec<NodeId> = nodes[..3].iter().map(|b| b.id).collect();

    // Trafic dense bidirectionnel sur les 3 arêtes du trio, entretenu ~35 s
    // (≥ MIN_EDGE_WEIGHT=3 par arête, couvre au moins une évaluation).
    let t0 = Instant::now();
    let mut round = 0u64;
    while t0.elapsed() < Duration::from_secs(35) {
        for (a, b) in [(0usize, 1usize), (1, 2), (0, 2)] {
            let payload = format!("r5-{round}").into_bytes();
            let _ = nodes[a].handle.send_message(nodes[b].id, payload.clone()).await;
            let _ = nodes[b].handle.send_message(nodes[a].id, payload).await;
        }
        round += 1;
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let trio_obs: Vec<&Arc<Mutex<NodeObs>>> = nodes[..3].iter().map(|b| &b.obs).collect();
    let formed_seen = wait_for(&trio_obs, Duration::from_secs(45), |o| {
        o.formed.iter().any(|(_, members)| trio.iter().all(|m| members.contains(m)))
    })
    .await;

    // Frontière : les silencieux n'apparaissent dans AUCUN subnet.
    let outsiders: Vec<NodeId> = nodes[3..].iter().map(|b| b.id).collect();
    let outsider_in_subnet = nodes.iter().any(|b| {
        b.obs
            .lock()
            .unwrap()
            .formed
            .iter()
            .any(|(_, members)| members.iter().any(|m| outsiders.contains(m)))
    });

    // Silence total → dissolution (timeout 3 s + éval ≤ 30 s + marge).
    let dissolved_seen = wait_for(&trio_obs, Duration::from_secs(75), |o| {
        o.dissolved.iter().any(|(_, reason)| reason.to_lowercase().contains("inactiv"))
    })
    .await;

    let mut ok = true;
    ok &= verdict("R5.formation", formed_seen, "SubnetFormed couvrant le trio dense observé");
    ok &= verdict(
        "R5.frontière",
        !outsider_in_subnet,
        "aucun nœud silencieux embarqué dans un subnet",
    );
    ok &= verdict(
        "R5.dissolution",
        dissolved_seen,
        "SubnetDissolved (inactivité) après arrêt du trafic — auto-purge wp §3.3",
    );

    teardown(nodes).await;
    Ok(ok)
}

// ── R8 : l'arroseur arrosé ──────────────────────────────────────────────────

async fn r8_arroseur() -> anyhow::Result<bool> {
    eprintln!("\n── R8 : arroseur arrosé (LOCKED #5 — freiné, jamais exclu) ──");
    eprintln!("   4 nœuds : S(0) spamme V1(1) ; V2(2)→V3(3) conversent ; antispam défaut (min 30/s, max 100/s).");

    let nodes = spawn_mesh(4, |_| {}).await?;
    let (s_id, v1_id, v2_id, v3_id) = (nodes[0].id, nodes[1].id, nodes[2].id, nodes[3].id);

    // Baseline victimes : V2 → V3 à ~2 msg/s pendant 8 s, sans spam.
    let v3_t0 = count_from(&nodes[3], v2_id);
    drive_traffic(&nodes[2].handle, v3_id, Duration::from_secs(8), Duration::from_millis(500)).await;
    tokio::time::sleep(Duration::from_secs(2)).await; // drain
    let baseline = count_from(&nodes[3], v2_id) - v3_t0;

    // Spam : S → V1 à ~200 msg/s offerts pendant 10 s, victimes en parallèle.
    let spam_handle = nodes[0].handle.clone();
    let spam = tokio::spawn(async move {
        let t0 = Instant::now();
        let mut offered = 0u64;
        while t0.elapsed() < Duration::from_secs(10) {
            let _ = spam_handle.send_message(v1_id, b"spam".to_vec()).await;
            offered += 1;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        offered
    });
    let v1_t0 = count_from(&nodes[1], s_id);
    let v3_t1 = count_from(&nodes[3], v2_id);
    drive_traffic(&nodes[2].handle, v3_id, Duration::from_secs(10), Duration::from_millis(500)).await;
    let offered = spam.await.unwrap_or(0);
    tokio::time::sleep(Duration::from_secs(2)).await; // drain
    let spam_delivered = count_from(&nodes[1], s_id) - v1_t0;
    let victims_during = count_from(&nodes[3], v2_id) - v3_t1;

    let throttled = nodes[1].obs.lock().unwrap().throttled.get(&s_id).copied().unwrap_or(0);

    // Rédemption : S revient à cadence normale (2 msg/s, 6 s → 12 offerts).
    let v1_t1 = count_from(&nodes[1], s_id);
    drive_traffic(&nodes[0].handle, v1_id, Duration::from_secs(6), Duration::from_millis(500)).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let redemption = count_from(&nodes[1], s_id) - v1_t1;

    let mut ok = true;
    ok &= verdict(
        "R8.throttle",
        throttled > 0,
        &format!("SenderThrottled émis par V1 contre S ({throttled} fois, {offered} offerts en 10 s)"),
    );
    ok &= verdict(
        "R8.jamais-exclu",
        spam_delivered > 0,
        &format!("S livre encore PENDANT le freinage ({spam_delivered} reçus — min_rate > 0, pas de ban)"),
    );
    // Le freinage doit être RÉEL, pas cosmétique (bug 21/07 : 1608/1609
    // livrés parce que le rejet était dédupliqué avec l'alerte). Borne large :
    // burst + max_rate×10 s ≪ offert/2 à ~160 msg/s offerts.
    ok &= verdict(
        "R8.freinage-réel",
        spam_delivered * 2 < offered,
        &format!("spam livré {spam_delivered} < offert {offered}/2 (le budget mord vraiment)"),
    );
    ok &= verdict(
        "R8.victimes-intactes",
        victims_during * 10 >= baseline * 9,
        &format!("V2→V3 pendant le spam : {victims_during} vs baseline {baseline} (≥ 90 % exigé)"),
    );
    ok &= verdict(
        "R8.rédemption",
        redemption >= 8,
        &format!("S à cadence normale post-spam : {redemption}/12 livrés (fade réversible, LOCKED #4)"),
    );

    teardown(nodes).await;
    Ok(ok)
}

// ── R1+R3 : relais multi-hop → promotion par contribution constatée ────────

async fn r1_r3_multihop_promotion() -> anyhow::Result<bool> {
    eprintln!("\n── R1+R3 : multi-hop A→R→B puis promotion de R par crédits CONSTATÉS ──");
    eprintln!("   Câblage transport : A⇄R, R⇄B — AUCUNE connexion A⇄B possible.");
    eprintln!("   Amorce : R est déclaré Relay/Online chez A et B (upsert API — le rôle");
    eprintln!("   de départ ; la PREUVE R3 est le score gagné par relais réels ensuite).");

    use tom_protocol::{PeerInfo, PeerRole, PeerStatus};
    fn wall_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    let nodes = spawn_mesh_wired(3, Wiring::Links(vec![(0, 1), (1, 2)]), |_| {}).await?;
    let (a_id, r_id, b_id) = (nodes[0].id, nodes[1].id, nodes[2].id);

    // Amorce du rôle : R relais aux yeux d'A (sélection sortante) ET de B
    // (le chemin retour des ACKs de B passe aussi par R — B n'a pas de
    // connexion vers A).
    for n in [&nodes[0], &nodes[2]] {
        n.handle
            .upsert_peer(PeerInfo {
                node_id: r_id,
                role: PeerRole::Relay,
                status: PeerStatus::Online,
                last_seen: wall_ms(),
            })
            .await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 25 messages A→B à 2 msg/s : chaque forward constaté crédite R chez A
    // (RelayForwarded ACK signé, verrou anti-pumping #7).
    for i in 0..25u32 {
        let _ = nodes[0]
            .handle
            .send_message(b_id, format!("r1-{i}").into_bytes())
            .await;
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    tokio::time::sleep(Duration::from_secs(3)).await; // drain forwards + ACKs

    let b_received = count_from(&nodes[2], a_id);
    let r_delivered_leak = count_from(&nodes[1], a_id);
    let r_forwarded = nodes[1].obs.lock().unwrap().forwarded;
    let score_r_at_a = nodes[0]
        .handle
        .get_role_metrics(r_id)
        .await
        .map(|m| m.score)
        .unwrap_or(0.0);
    let score_a_at_r = nodes[1]
        .handle
        .get_role_metrics(a_id)
        .await
        .map(|m| m.score)
        .unwrap_or(0.0);
    let promoted_at_a = nodes[0]
        .obs
        .lock()
        .unwrap()
        .promoted
        .iter()
        .any(|(id, _)| *id == r_id);

    let mut ok = true;
    ok &= verdict(
        "R1.livraison-multi-hop",
        b_received >= 20,
        &format!("B a reçu {b_received}/25 SANS connexion directe A⇄B (tout via R)"),
    );
    ok &= verdict(
        "R1.pass-through",
        r_delivered_leak == 0,
        &format!("R ne DÉLIVRE pas ce qu'il relaie ({r_delivered_leak} fuite(s) applicative(s))"),
    );
    ok &= verdict(
        "R1.forward-constaté",
        r_forwarded >= 20,
        &format!("R a émis {r_forwarded} événements Forwarded"),
    );
    ok &= verdict(
        "R1.usage-non-crédité",
        score_a_at_r < 2.0,
        &format!(
            "le score de l'ORIGINE A chez R reste sous le gate anti-Sybil ({score_a_at_r:.2} < 2.0 — anti-régression fix 2937330)"
        ),
    );
    ok &= verdict(
        "R3.score-par-constat",
        score_r_at_a >= 10.0,
        &format!(
            "score de R chez A = {score_r_at_a:.2} ≥ 10.0 (PROMOTION_THRESHOLD) par RelayForwarded signés"
        ),
    );
    // La promotion EFFECTIVE tombe au tick d'évaluation (300 s) — si le run
    // l'attrape tant mieux, sinon le SCORE est la preuve (le déclenchement
    // seuil→upsert est couvert par les tests unitaires du RoleManager).
    eprintln!(
        "  [info] RolePromoted observé pendant le run : {} (tick d'éval = 300 s, non exigé)",
        if promoted_at_a { "OUI" } else { "non" }
    );

    teardown(nodes).await;
    Ok(ok)
}

// ── Entrée ──────────────────────────────────────────────────────────────────

pub async fn run(scenario: String) -> anyhow::Result<()> {
    eprintln!("=== Banc rôles sous charge — Phase A étage L (hermétique, in-process) ===");
    let wanted = scenario.as_str();
    if !matches!(wanted, "r1" | "r5" | "r8" | "all") {
        anyhow::bail!("scénario inconnu '{wanted}' — valeurs : r1, r5, r8, all");
    }
    let mut all_ok = true;
    if matches!(wanted, "r1" | "all") {
        all_ok &= r1_r3_multihop_promotion().await?;
    }
    if matches!(wanted, "r5" | "all") {
        all_ok &= r5_subnets().await?;
    }
    if matches!(wanted, "r8" | "all") {
        all_ok &= r8_arroseur().await?;
    }
    eprintln!(
        "\n=== VERDICT PHASE A ({wanted}) : {} ===",
        if all_ok { "PASS" } else { "FAIL — voir les oracles ci-dessus" }
    );
    eprintln!(
        "⚠️ Portée : rôles exercés sous trafic RÉEL in-process (loopback, runtime partagé). \
         Aucun verdict de perf absolue ; R1+R3 (multi-hop → promotion) et R2/R7 suivent."
    );
    if !all_ok {
        std::process::exit(1);
    }
    Ok(())
}
