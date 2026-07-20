//! Banc « courbe de masse » — Phase 1, brique in-process (hermétique).
//!
//! But (charte `docs/plans/charte-cibles-agressives.md` §0, banc
//! `docs/plans/banc-courbe-masse.md`) : mesurer si, à **charge par nœud FIXE**,
//! quand N croît, le **débit livré PAR NŒUD** ne s'effondre pas et la **latence**
//! reste bornée. L'agrégat brut serait tautologique (avocat mesure) → tout est
//! par nœud.
//!
//! Pourquoi in-process : les nœuds sont câblés par `add_peer_addr` (aucune
//! découverte n0/mDNS/DHT), donc **hermétique par construction** — la vraie
//! flotte ne peut pas fausser la courbe (piège #1 de l'avocat mesure). Bonus :
//! horloge de process UNIQUE → latence par message sans skew d'horloge.
//!
//! Limites ASSUMÉES (le rapport ne doit jamais les cacher) :
//! - RTT loopback (~0), PAS du LAN/WAN → validité externe = Phase 2 (netem).
//! - runtime tokio PARTAGÉ entre les N nœuds → la contention est réelle et
//!   MESURÉE (dérive de cadence) ; un point en contention est INVALIDE, pas un
//!   résultat. Plafond du banc local ≈ 20-24 nœuds (au-delà : matériel distribué).
//! - topologie all-pairs directe (régime « tout le monde se joint ») ; le
//!   routage par hubs est une variante ultérieure.

use std::sync::Arc;
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tom_protocol::{NodeId, ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

/// Garde-fou contention #1 : si la cadence d'émission réelle dérive au-delà de
/// ce facteur de l'intervalle visé, le runtime partagé n'a pas suivi côté
/// émission → point INVALIDE (on ne mesure plus le réseau mais l'ordonnanceur).
const DRIFT_INVALID_RATIO: f64 = 1.5;

/// Fenêtre de drain après la phase de charge : temps laissé aux derniers
/// messages pour arriver avant de clore le comptage.
//
// Garde-fou contention #2 (le décisif), appliqué dans `PointMetrics::invalid` :
// si la latence p95 DÉPASSE `DRAIN_GRACE`, des messages en vol sont coupés du
// comptage → le taux de livraison est TRONQUÉ, pas fiable → point INVALIDE.
// C'est ce qui distingue « contention du banc » (la dérive d'émission peut
// rester basse alors que la RÉCEPTION est saturée) d'une vraie perte réseau.
const DRAIN_GRACE: Duration = Duration::from_secs(3);

/// Mesures d'un point de courbe (une valeur de N).
struct PointMetrics {
    n: usize,
    /// Débit livré par nœud (msg/s), moyenné sur les nœuds récepteurs.
    delivered_per_node_hz: f64,
    /// Taux de livraison global (reçus / envoyés).
    delivery_rate: f64,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    /// Pire ratio (intervalle d'émission réel moyen / intervalle visé) sur les
    /// nœuds — > DRIFT_INVALID_RATIO ⇒ contention ⇒ point INVALIDE.
    worst_cadence_drift: f64,
    total_offered: u64,
    total_errs: u64,
    total_recv: u64,
    total_dups: u64,
    /// Messages reçus dans le canal applicatif qui ne sont PAS du banc.
    total_foreign: u64,
}

impl PointMetrics {
    fn invalid(&self) -> bool {
        self.worst_cadence_drift > DRIFT_INVALID_RATIO
            || self.latency_p95_ms > DRAIN_GRACE.as_millis() as f64
    }

    /// Raison lisible de l'invalidité (pour le rapport).
    fn invalid_reason(&self) -> &'static str {
        if self.worst_cadence_drift > DRIFT_INVALID_RATIO {
            "dérive d'émission (ordonnanceur saturé côté envoi)"
        } else if self.latency_p95_ms > DRAIN_GRACE.as_millis() as f64 {
            "p95 > fenêtre de drain (comptage tronqué, réception saturée)"
        } else {
            ""
        }
    }
}

fn percentile(sorted_ms: &[f64], p: f64) -> f64 {
    if sorted_ms.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted_ms.len() as f64 - 1.0)).round() as usize;
    sorted_ms[idx.min(sorted_ms.len() - 1)]
}

/// Lance un point de courbe : N nœuds, charge fixe/nœud pendant `duration`.
async fn run_point(
    n: usize,
    duration: Duration,
    interval: Duration,
    payload_bytes: usize,
    seed: u64,
) -> anyhow::Result<PointMetrics> {
    eprintln!("\n── Point N={n} (charge {:.1} msg/s/nœud, {}o, {}s) ──",
        1.0 / interval.as_secs_f64(), payload_bytes, duration.as_secs());

    // 1. Spawn N nœuds hermétiques (aucune découverte).
    let mut nodes = Vec::with_capacity(n);
    for _ in 0..n {
        nodes.push(TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?);
    }
    let ids: Vec<NodeId> = nodes.iter().map(|nd| nd.id()).collect();
    let addrs: Vec<_> = nodes.iter().map(|nd| nd.addr()).collect();

    // 2. Runtimes + canaux.
    let mut channels = Vec::with_capacity(n);
    for (i, node) in nodes.into_iter().enumerate() {
        let cfg = RuntimeConfig {
            username: format!("{}courbe-{i}", tom_protocol::TEST_NODE_PREFIX),
            encryption: true,
            ..Default::default()
        };
        channels.push(ProtocolRuntime::spawn(node, cfg));
    }

    // 3. Câblage all-pairs (adresses directes ; aucune connexion active tant
    //    qu'on n'a pas envoyé).
    for (i, ch) in channels.iter().enumerate() {
        for (j, addr) in addrs.iter().enumerate() {
            if i != j {
                ch.handle.add_peer_addr(addr.clone()).await;
            }
        }
    }
    // Chauffe : laisser les premières connexions s'établir.
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Époque commune : latence = delta de nanos sur CETTE horloge (pas de skew).
    let epoch = Instant::now();
    let ids = Arc::new(ids);

    // 4. Une tâche émettrice + une réceptrice par nœud.
    let mut send_tasks = Vec::with_capacity(n);
    let mut recv_tasks = Vec::with_capacity(n);
    for (i, ch) in channels.into_iter().enumerate() {
        let handle = ch.handle.clone();
        let mut messages = ch.messages;
        let ids = ids.clone();
        let n_ids = ids.len();

        // Récepteur : draine jusqu'à `duration` + grâce. Ne compte QUE les
        // messages du banc (signature de payload), isole les « étrangers »
        // (chatter protocolaire éventuel dans le canal applicatif).
        let recv_deadline = duration + DRAIN_GRACE;
        let expect_len = payload_bytes.max(8);
        recv_tasks.push(tokio::spawn(async move {
            let mut mine = 0u64;
            let mut foreign = 0u64;
            let mut dups = 0u64;
            let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
            let mut lat_ms: Vec<f64> = Vec::new();
            loop {
                let remaining = recv_deadline.checked_sub(epoch.elapsed());
                let Some(remaining) = remaining else { break };
                match tokio::time::timeout(remaining.min(Duration::from_millis(500)), messages.recv()).await {
                    Ok(Some(msg)) => {
                        // « Mien » = longueur exacte + remplissage 'A' (le corps
                        // après l'horodatage). Sinon = trafic non-banc.
                        let is_mine = msg.payload.len() == expect_len
                            && (expect_len == 8 || msg.payload[expect_len - 1] == b'A');
                        if !is_mine {
                            foreign += 1;
                            continue;
                        }
                        mine += 1;
                        let sent_ns = u64::from_le_bytes(msg.payload[..8].try_into().unwrap());
                        // Doublon = même horodatage d'émission revu (collision
                        // inter-émetteurs au nanosecond ≈ impossible → fiable).
                        if !seen.insert(sent_ns) {
                            dups += 1;
                        }
                        let now_ns = epoch.elapsed().as_nanos() as u64;
                        lat_ms.push((now_ns.saturating_sub(sent_ns)) as f64 / 1e6);
                    }
                    Ok(None) => break,        // canal fermé (shutdown)
                    Err(_) => {
                        if epoch.elapsed() >= recv_deadline { break; }
                    }
                }
            }
            (mine, dups, foreign, lat_ms)
        }));

        // Émetteur : 1 message / `interval` vers un pair aléatoire, jusqu'à
        // `duration`. Mesure la dérive de cadence (contention).
        let mut rng = StdRng::seed_from_u64(seed.wrapping_add(i as u64));
        send_tasks.push(tokio::spawn(async move {
            // `offered` = tentatives (dénominateur honnête du débit) ; `errs` =
            // rejets API. Un Err n'implique PAS non-livraison (un retry transport
            // peut livrer) → on compte l'offre, pas le succès API.
            let mut offered = 0u64;
            let mut errs = 0u64;
            let mut intervals_ns: Vec<u64> = Vec::new();
            let mut last = Instant::now();
            while epoch.elapsed() < duration {
                // cible ≠ soi
                let mut j = rng.random_range(0..n_ids);
                if j == i { j = (j + 1) % n_ids; }
                let sent_ns = epoch.elapsed().as_nanos() as u64;
                let mut payload = Vec::with_capacity(payload_bytes.max(8));
                payload.extend_from_slice(&sent_ns.to_le_bytes());
                payload.resize(payload_bytes.max(8), b'A');
                offered += 1;
                if handle.send_message(ids[j], payload).await.is_err() {
                    errs += 1;
                }
                let now = Instant::now();
                intervals_ns.push(now.duration_since(last).as_nanos() as u64);
                last = now;
                tokio::time::sleep(interval).await;
            }
            (offered, errs, intervals_ns)
        }));
    }

    // 5. Collecte.
    let mut total_offered = 0u64;
    let mut total_errs = 0u64;
    let mut worst_drift = 0.0_f64;
    for t in send_tasks {
        let (offered, errs, intervals) = t.await.unwrap_or((0, 0, Vec::new()));
        total_offered += offered;
        total_errs += errs;
        if !intervals.is_empty() {
            // 1er intervalle = temps jusqu'au 1er tick, ignoré.
            let body = &intervals[1.min(intervals.len())..];
            if !body.is_empty() {
                let mean_ns = body.iter().sum::<u64>() as f64 / body.len() as f64;
                let drift = mean_ns / interval.as_nanos() as f64;
                worst_drift = worst_drift.max(drift);
            }
        }
    }

    let mut total_recv = 0u64;
    let mut total_dups = 0u64;
    let mut total_foreign = 0u64;
    let mut recv_nodes = 0usize;
    let mut all_lat: Vec<f64> = Vec::new();
    for t in recv_tasks {
        let (count, dups, foreign, lat) = t.await.unwrap_or((0, 0, 0, Vec::new()));
        total_recv += count;
        total_dups += dups;
        total_foreign += foreign;
        if count > 0 { recv_nodes += 1; }
        all_lat.extend(lat);
    }
    all_lat.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Livraison UNIQUE = reçus moins doublons (dédup = santé I8, doit être 0).
    let unique = total_recv.saturating_sub(total_dups);
    let dur_s = duration.as_secs_f64();
    let delivered_per_node_hz = if recv_nodes > 0 {
        unique as f64 / recv_nodes as f64 / dur_s
    } else {
        0.0
    };

    Ok(PointMetrics {
        n,
        delivered_per_node_hz,
        delivery_rate: if total_offered > 0 { unique as f64 / total_offered as f64 } else { 0.0 },
        latency_p50_ms: percentile(&all_lat, 50.0),
        latency_p95_ms: percentile(&all_lat, 95.0),
        worst_cadence_drift: worst_drift,
        total_offered,
        total_errs,
        total_recv,
        total_dups,
        total_foreign,
    })
}

/// Balaye plusieurs N et imprime la courbe + un verdict pré-enregistré.
pub async fn run(
    sizes: Vec<usize>,
    duration_secs: u64,
    rate_hz: f64,
    payload_bytes: usize,
    seed: u64,
) -> anyhow::Result<()> {
    eprintln!("=== Banc courbe de masse — Phase 1 (in-process, hermétique) ===");
    eprintln!("seed={seed} · loopback (pas LAN/WAN) · runtime tokio partagé");

    let interval = Duration::from_secs_f64(1.0 / rate_hz);
    let duration = Duration::from_secs(duration_secs);
    let mut points = Vec::new();
    for n in sizes {
        points.push(run_point(n, duration, interval, payload_bytes, seed).await?);
    }

    // ── Rapport ──
    eprintln!("\n=== COURBE ===");
    eprintln!("{:>4}  {:>14}  {:>10}  {:>9}  {:>9}  {:>7}  {:>13}  {:>5}  {:>8}",
        "N", "livré/nœud(Hz)", "livraison", "p50(ms)", "p95(ms)", "dérive", "offert→reçu", "dup", "verdict");
    for p in &points {
        eprintln!("{:>4}  {:>14.3}  {:>8.1}%  {:>9.2}  {:>9.2}  {:>6.2}x  {:>6}→{:<6}  {:>5}  {:>8}",
            p.n, p.delivered_per_node_hz, p.delivery_rate * 100.0,
            p.latency_p50_ms, p.latency_p95_ms, p.worst_cadence_drift,
            p.total_offered, p.total_recv, p.total_dups,
            if p.invalid() { "INVALIDE" } else { "ok" });
        if p.invalid() {
            eprintln!("       └ N={} INVALIDE : {}", p.n, p.invalid_reason());
        }
    }
    let any_dups: u64 = points.iter().map(|p| p.total_dups).sum();
    let any_errs: u64 = points.iter().map(|p| p.total_errs).sum();
    let any_foreign: u64 = points.iter().map(|p| p.total_foreign).sum();
    if any_dups > 0 {
        eprintln!("⚠️ {any_dups} DOUBLON(S) de livraison au total — dédup I8 à investiguer (ne devrait jamais arriver).");
    }
    if any_errs > 0 {
        eprintln!("ℹ️ {any_errs} rejet(s) API à l'émission (retries transport possibles derrière — non compté comme perte).");
    }
    if any_foreign > 0 {
        eprintln!("ℹ️ {any_foreign} message(s) non-banc filtré(s) du canal applicatif (chatter protocolaire — exclus du calcul).");
    }

    // Verdict pré-enregistré (banc §0). AXE JUGÉ IN-PROCESS = débit/nœud + dédup
    // uniquement. La latence est INDICATIVE : le runtime tokio partagé ne permet
    // pas de séparer « latence protocolaire ∝ N » de « ordonnanceur saturé »
    // (constaté : p50 explose alors que la cadence d'émission tient). L'axe
    // latence-vs-N exige l'isolation par PROCESSUS (banc §3, multi-process).
    let valid: Vec<&PointMetrics> = points.iter().filter(|p| !p.invalid()).collect();
    eprintln!();
    if valid.len() < 2 {
        eprintln!("VERDICT : indéterminé — {} point(s) valide(s) (dérive de cadence). \
            Réduire N ou passer au matériel distribué.", valid.len());
        return Ok(());
    }
    let base = valid.first().unwrap();
    let top = valid.last().unwrap();
    let thr_ratio = if base.delivered_per_node_hz > 0.0 {
        top.delivered_per_node_hz / base.delivered_per_node_hz
    } else { 0.0 };
    let dedup_ok = any_dups == 0;
    let pass = thr_ratio >= 0.80 && dedup_ok;
    eprintln!("VERDICT DÉBIT/DÉDUP (N={}→{}) : débit/nœud ×{:.2} (seuil ≥0.80), dédup {} → {}",
        base.n, top.n, thr_ratio,
        if dedup_ok { "0 doublon" } else { "DOUBLONS" },
        if pass { "PASS" } else { "FAIL (bug archi à localiser)" });
    // Latence : rapportée, jamais transformée en verdict ici.
    let lat_ratio = if base.latency_p50_ms > 0.0 {
        top.latency_p50_ms / base.latency_p50_ms
    } else { f64::INFINITY };
    eprintln!("LATENCE (indicative, runtime partagé) : p50 ×{:.1} de N={} à N={} — NON concluant in-process.",
        lat_ratio, base.n, top.n);
    if lat_ratio > 3.0 {
        eprintln!("   → forte inflation = signature de contention du banc, PAS un verdict protocole. \
            Axe latence-vs-N ⇒ harnais multi-process (banc §3).");
    }
    eprintln!("⚠️ loopback + runtime partagé : signal de PENTE sur le débit, pas une preuve à grande échelle (banc §5).");
    Ok(())
}
