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
    addr: tom_transport::EndpointAddr,
    handle: RuntimeHandle,
    obs: Arc<Mutex<NodeObs>>,
}

impl BenchNode {
    /// Câble ce nœud vers un autre (adresse directe, sans découverte).
    async fn connect_to(&self, other: &BenchNode) {
        self.handle.add_peer_addr(other.addr.clone()).await;
    }
}

/// Attache runtime + collecteur d'observations à un `TomNode` déjà bindé.
/// Extrait pour que R2 puisse RE-créer un nœud (même identité) après un arrêt.
fn attach_node(node: TomNode, cfg: RuntimeConfig) -> BenchNode {
    let id = node.id();
    let addr = node.addr();
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
                    obs_task.lock().unwrap().recv_from.entry(msg.from).and_modify(|c| *c += 1).or_insert(1);
                }
                ev = events.recv() => {
                    let Some(ev) = ev else { break };
                    let mut o = obs_task.lock().unwrap();
                    match ev {
                        ProtocolEvent::SubnetFormed { subnet_id, members } => o.formed.push((subnet_id, members)),
                        ProtocolEvent::SubnetDissolved { subnet_id, reason } => o.dissolved.push((subnet_id, reason)),
                        ProtocolEvent::SenderThrottled { node_id, .. } => { *o.throttled.entry(node_id).or_default() += 1; }
                        ProtocolEvent::Forwarded { .. } => o.forwarded += 1,
                        ProtocolEvent::RolePromoted { node_id, score } => o.promoted.push((node_id, score)),
                        _ => {}
                    }
                }
            }
        }
    });

    BenchNode { id, addr, handle: channels.handle, obs }
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
        let mut cfg = RuntimeConfig {
            username: format!("{}rolch-{i}", tom_protocol::TEST_NODE_PREFIX),
            encryption: true,
            // CRITIQUE : jamais le rendez-vous partagé (incident 20/07).
            enable_dht: false,
            ..Default::default()
        };
        tune(&mut cfg);
        out.push(attach_node(node, cfg));
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

// ── R7-L : PoP — les fantômes ne votent pas ────────────────────────────────

async fn r7_pop_ghosts() -> anyhow::Result<bool> {
    eprintln!("\n── R7-L : PoP (ADR-011) — un mort découvert ne passe JAMAIS Online ──");
    eprintln!("   O ↔ V échangent du trafic RÉEL ; un fantôme G est « re-découvert » 6×");
    eprintln!("   (add_peer = mark_known) sans jamais travailler → doit rester Known.");

    // Fantôme : une identité VALIDE jamais connectée (nœud jetable → on garde
    // juste son NodeId). C'est un « mort » plausible côté carnet d'adresses.
    let ghost_id = {
        let throwaway =
            TomNode::bind(TomNodeConfig::new().n0_discovery(false).local_discovery(false)).await?;
        let id = throwaway.id();
        drop(throwaway);
        id
    };

    let nodes = spawn_mesh(2, |_| {}).await?; // O = 0, V = 1
    let (v_id, _o) = (nodes[1].id, nodes[0].id);

    // « Ré-annonces gossip du mort » : O le re-découvre en boucle. La découverte
    // (ADR-011) ne doit accorder AUCUN crédit de présence — juste le carnet.
    for _ in 0..6 {
        nodes[0].handle.add_peer(ghost_id).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    // Trafic RÉEL O ↔ V (dans les deux sens : chacun devient Online chez l'autre).
    for i in 0..10u32 {
        let _ = nodes[0].handle.send_message(v_id, format!("r7-{i}").into_bytes()).await;
        let _ = nodes[1].handle.send_message(nodes[0].id, format!("r7b-{i}").into_bytes()).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Vue PoP de O (nouvelle API get_peer_statuses).
    use tom_protocol::PeerStatus;
    let statuses = nodes[0].handle.get_peer_statuses().await;
    let ghost_status = statuses.iter().find(|(id, _)| *id == ghost_id).map(|(_, s)| *s);
    let v_online = statuses
        .iter()
        .any(|(id, s)| *id == v_id && *s == PeerStatus::Online);
    // NOTE herméticité : si des apps de la flotte tournent sur la MÊME machine,
    // elles trouvent O par mDNS ENTRANT (le trio isolated ne coupe que la
    // découverte SORTANTE) et lui poussent leur carnet → O les liste en Known.
    // Ces Known « étrangers » ne faussent PAS le contrat R7 (ils restent Known,
    // pas Online, faute de travail constaté), mais rendent un `online_count`
    // ABSOLU non déterministe. On juge donc RELATIVEMENT : G ne vote pas, V vote.
    // Trou d'herméticité documenté (banc-roles-sous-charge.md §1) — à traiter
    // pour l'étage L en présence d'une flotte locale.
    let foreign_known = statuses
        .iter()
        .filter(|(id, s)| *id != v_id && *id != ghost_id && *s == PeerStatus::Known)
        .count();
    if foreign_known > 0 {
        eprintln!(
            "  [info] {foreign_known} pair(s) étranger(s) en Known (flotte locale via mDNS entrant — \
             n'affecte pas le contrat R7, cf note herméticité)"
        );
    }

    let mut ok = true;
    ok &= verdict(
        "R7.fantôme-jamais-online",
        ghost_status != Some(PeerStatus::Online),
        &format!("G re-découvert 6× reste {ghost_status:?} (jamais Online — PoP : pas de travail constaté)"),
    );
    ok &= verdict(
        "R7.vivant-online",
        v_online,
        "V, avec qui O a VRAIMENT échangé, est Online (le travail constaté, lui, compte)",
    );

    teardown(nodes).await;
    Ok(ok)
}

// ── R2-L : backup virus — l'absent est servi à son retour ──────────────────

async fn r2_backup_absent_return() -> anyhow::Result<bool> {
    eprintln!("\n── R2-L : backup (ADR-009) — A garde pour B absent, livre à son retour ──");
    eprintln!("   B a une identité STABLE (identity_path) → le MÊME node_id revient après arrêt.");

    // Identité persistante de B dans un fichier temporaire du scratchpad.
    let id_path = std::env::temp_dir().join(format!(
        "tom-bench-r2-identity-{}.key",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&id_path);

    let a = attach_node(
        TomNode::bind(TomNodeConfig::new().n0_discovery(false).local_discovery(false)).await?,
        RuntimeConfig {
            username: format!("{}r2-A", tom_protocol::TEST_NODE_PREFIX),
            enable_dht: false,
            ..Default::default()
        },
    );
    let b_node =
        TomNode::bind(
            TomNodeConfig::new()
                .n0_discovery(false)
                .local_discovery(false)
                .identity_path(id_path.clone()),
        )
        .await?;
    let b_id = b_node.id();
    let b = attach_node(
        b_node,
        RuntimeConfig {
            username: format!("{}r2-B", tom_protocol::TEST_NODE_PREFIX),
            enable_dht: false,
            ..Default::default()
        },
    );

    // Câblage A↔B + échauffement (B devient connu/online chez A).
    a.connect_to(&b).await;
    b.connect_to(&a).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = a.handle.send_message(b_id, b"r2-warmup".to_vec()).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let warmup_ok = count_from(&b, a.id) >= 1;

    // B s'éteint (teardown borné) → offline.
    let _ = tokio::time::timeout(Duration::from_secs(5), b.handle.shutdown()).await;
    drop(b);
    tokio::time::sleep(Duration::from_secs(1)).await;

    // A envoie 3 messages à B ABSENT → l'envoi transport échoue → backup local.
    const N: u32 = 3;
    for i in 0..N {
        let _ = a.handle.send_message(b_id, format!("r2-offline-{i}").into_bytes()).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    tokio::time::sleep(Duration::from_secs(2)).await; // laisse l'échec + backup se produire
    // Preuve DIRECTE que le backup ADR-009 a retenu (pas le cache de réémission
    // pending_envelopes R9.2) : le store de A contient les messages pour B.
    let backup_pending_during_absence = a.handle.get_backup_pending_count().await;

    // B REVIENT : re-bind avec le MÊME identity_path → MÊME node_id.
    let b2_node =
        TomNode::bind(
            TomNodeConfig::new()
                .n0_discovery(false)
                .local_discovery(false)
                .identity_path(id_path.clone()),
        )
        .await?;
    let b2_id = b2_node.id();
    let b2 = attach_node(
        b2_node,
        RuntimeConfig {
            username: format!("{}r2-B", tom_protocol::TEST_NODE_PREFIX),
            enable_dht: false,
            ..Default::default()
        },
    );
    let same_identity = b2_id == b_id;

    // Re-câbler A↔B' (nouvelle adresse transport, même identité) et faire signe
    // à A pour qu'il voie B' Online → prepare_backup_delivery → drain.
    a.connect_to(&b2).await;
    b2.connect_to(&a).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let _ = b2.handle.send_message(a.id, b"r2-imback".to_vec()).await;

    // Laisser la re-livraison (grâce 3 s + drain 5 msg/s/pair) opérer.
    let redelivered = wait_for_count(&b2, a.id, N as u64, Duration::from_secs(20)).await;
    tokio::time::sleep(Duration::from_secs(3)).await; // laisse l'ACK purger le store
    let received = count_from(&b2, a.id);
    // Preuve DIRECTE de la purge : le store de A se vide après ACK (auto-délétion).
    let backup_pending_after = a.handle.get_backup_pending_count().await;

    let mut ok = true;
    ok &= verdict("R2.échauffement", warmup_ok, "B a bien reçu le message initial (online)");
    ok &= verdict(
        "R2.même-identité",
        same_identity,
        &format!("B revient avec le MÊME node_id ({}…)", &b2_id.to_string()[..8]),
    );
    ok &= verdict(
        "R2.backup-à-absence",
        backup_pending_during_absence >= N as usize,
        &format!("le store de A retient {backup_pending_during_absence} message(s) pour B absent (≥ {N} — backup ADR-009, pas le cache R9.2)"),
    );
    ok &= verdict(
        "R2.livraison-différée",
        redelivered && received >= N as u64,
        &format!("B' a reçu les {received}/{N} messages différés à son retour (virus ADR-009)"),
    );
    ok &= verdict(
        "R2.exactement-une-fois",
        received == N as u64,
        &format!("exactement {N} livrés, pas de doublon ({received})"),
    );
    ok &= verdict(
        "R2.purge-après-ack",
        backup_pending_after == 0,
        &format!("le store de A se vide après livraison+ACK ({backup_pending_after} restant — auto-délétion)"),
    );

    let _ = tokio::time::timeout(Duration::from_secs(5), a.handle.shutdown()).await;
    let _ = tokio::time::timeout(Duration::from_secs(5), b2.handle.shutdown()).await;
    let _ = std::fs::remove_file(&id_path);
    Ok(ok)
}

/// Attend que `node` ait reçu au moins `target` messages de `from` (poll 500 ms).
async fn wait_for_count(node: &BenchNode, from: NodeId, target: u64, budget: Duration) -> bool {
    let t0 = Instant::now();
    while t0.elapsed() < budget {
        if count_from(node, from) >= target {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

// ── Entrée ──────────────────────────────────────────────────────────────────

pub async fn run(scenario: String) -> anyhow::Result<()> {
    eprintln!("=== Banc rôles sous charge — Phase A étage L (hermétique, in-process) ===");
    let wanted = scenario.as_str();
    if !matches!(wanted, "r1" | "r2" | "r5" | "r7" | "r8" | "all") {
        anyhow::bail!("scénario inconnu '{wanted}' — valeurs : r1, r2, r5, r7, r8, all");
    }
    let mut all_ok = true;
    if matches!(wanted, "r1" | "all") {
        all_ok &= r1_r3_multihop_promotion().await?;
    }
    if matches!(wanted, "r2" | "all") {
        all_ok &= r2_backup_absent_return().await?;
    }
    if matches!(wanted, "r5" | "all") {
        all_ok &= r5_subnets().await?;
    }
    if matches!(wanted, "r7" | "all") {
        all_ok &= r7_pop_ghosts().await?;
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
