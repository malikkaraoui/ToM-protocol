//! Banc « courbe de masse » — Phase 1, brique in-process (hermétique).
//!
//! But (charte `docs/plans/charte-cibles-agressives.md` §0, banc
//! `docs/plans/banc-courbe-masse.md` §2bis) : juger l'**INTÉGRITÉ de livraison**
//! (perte réelle après drain à quiescence, doublons par clé unique (nœud,seq),
//! herméticité) à charge par nœud FIXE quand N croît. Le débit à saturation et
//! la latence sont RAPPORTÉS, jamais jugés ici (runtime tokio partagé).
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

/// Garde-fou contention côté ÉMISSION : si la cadence d'émission réelle dérive
/// au-delà de ce facteur de l'intervalle visé, le runtime n'a pas suivi côté
/// envoi → point INVALIDE (on ne mesure plus le réseau mais l'ordonnanceur).
const DRIFT_INVALID_RATIO: f64 = 1.5;

/// Drain jusqu'à QUIESCENCE : après la phase de charge, on continue de recevoir
/// tant qu'un message arrive ; on ne clôt le comptage qu'après ce silence.
/// → le taux de livraison est VRAI (rien n'est coupé « en vol »), donc une perte
/// est une VRAIE perte, pas un artefact de fenêtre trop courte (contre le piège
/// « garde-fou commode qui invalide le point gênant » — revue avocat 20/07).
const QUIESCENCE: Duration = Duration::from_secs(2);

/// Filet de sécurité : borne dure du drain (au-delà, on arrête même si ça arrive
/// encore — évite un blocage si un backlog ne se résorbe jamais).
const MAX_DRAIN: Duration = Duration::from_secs(120);

/// Mesures d'un point de courbe (une valeur de N).
struct PointMetrics {
    n: usize,
    /// Débit livré par nœud (msg/s), moyenné sur les nœuds récepteurs.
    delivered_per_node_hz: f64,
    /// Taux de livraison global (reçus / envoyés).
    delivery_rate: f64,
    latency_p50_ms: f64,
    latency_p95_ms: f64,
    latency_max_ms: f64,
    /// Pire ratio (intervalle d'émission réel moyen / intervalle visé) sur les
    /// nœuds — > DRIFT_INVALID_RATIO ⇒ contention côté émission ⇒ INVALIDE.
    worst_cadence_drift: f64,
    total_offered: u64,
    total_errs: u64,
    total_recv: u64,
    total_dups: u64,
    /// Messages reçus dans le canal applicatif qui ne sont PAS du banc.
    total_foreign: u64,
}

impl PointMetrics {
    /// INVALIDE = seulement la contention CÔTÉ ÉMISSION (dérive de cadence).
    /// On ne juge plus la latence de réception comme critère de validité (avec
    /// le drain jusqu'à quiescence, la livraison est VRAIE) : la latence est
    /// RAPPORTÉE, jamais transformée en cause d'exclusion « commode ».
    fn invalid(&self) -> bool {
        self.worst_cadence_drift > DRIFT_INVALID_RATIO
    }

    fn invalid_reason(&self) -> &'static str {
        if self.worst_cadence_drift > DRIFT_INVALID_RATIO {
            "dérive d'émission (ordonnanceur saturé côté envoi)"
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

// ── Logiques de MESURE extraites en fonctions pures (testables) ──────────────
// L'instrument qui juge le protocole doit être verrouillé par des tests : la V1
// de ce banc accusait le protocole à tort (clé de dédup ambiguë + troncature du
// drain). Chaque mécanisme de mesure vit ici, hors I/O, couvert par un test.

/// Signature du banc dans le payload :
/// `[0..8]` horodatage ns | `[8..16]` clé unique (nœud<<32|seq) | `[16..20]`
/// magic | `[20..]` remplissage `'A'`. Le magic 4 octets rend le filtre
/// d'herméticité non ambigu (un padding accidentel `'A'` ne suffit plus).
const BANC_MAGIC: [u8; 4] = *b"CRBE";

/// Taille minimale d'un payload du banc (ts + clé + magic + ≥ 4 de padding).
const MIN_PAYLOAD: usize = 24;

/// Construit un payload du banc (miroir exact de `parse_mine`).
fn build_payload(sent_ns: u64, key: u64, expect_len: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(expect_len);
    payload.extend_from_slice(&sent_ns.to_le_bytes());
    payload.extend_from_slice(&key.to_le_bytes());
    payload.extend_from_slice(&BANC_MAGIC);
    payload.resize(expect_len, b'A');
    payload
}

/// Reconnaît un payload du banc → `Some((sent_ns, clé))`, sinon `None`
/// (message « étranger » : chatter protocolaire dans le canal applicatif).
fn parse_mine(payload: &[u8], expect_len: usize) -> Option<(u64, u64)> {
    if payload.len() != expect_len || expect_len < MIN_PAYLOAD {
        return None;
    }
    if payload[16..20] != BANC_MAGIC || payload[expect_len - 1] != b'A' {
        return None;
    }
    let sent_ns = u64::from_le_bytes(payload[..8].try_into().ok()?);
    let key = u64::from_le_bytes(payload[8..16].try_into().ok()?);
    Some((sent_ns, key))
}

/// Comptage de livraison unique/doublons — point de mutation UNIQUE (leçon
/// « classe de bug compteur qui dérive » : l'état et ses compteurs mutent
/// ensemble, jamais séparément).
#[derive(Default)]
struct DeliveryLedger {
    seen: std::collections::HashSet<u64>,
    unique: u64,
    dups: u64,
}

impl DeliveryLedger {
    /// Enregistre une clé reçue. `true` = première livraison de cette clé.
    fn record(&mut self, key: u64) -> bool {
        if self.seen.insert(key) {
            self.unique += 1;
            true
        } else {
            self.dups += 1;
            false
        }
    }
}

/// Décision de clôture du drain (pure). `last_msg` DOIT être initialisé à
/// `duration` par l'appelant : la fenêtre de quiescence court alors au plus tôt
/// à partir de la FIN de charge — même si aucun message n'est jamais arrivé, on
/// accorde ≥ QUIESCENCE aux tardifs avant de conclure (un 0 % doit être vrai).
fn drain_should_close(now: Duration, duration: Duration, last_msg: Duration, hard_cap: Duration) -> bool {
    if now >= hard_cap {
        return true; // filet de sécurité : backlog qui ne se résorbe jamais
    }
    now >= duration && now.saturating_sub(last_msg) >= QUIESCENCE
}

/// Dérive de cadence d'émission (pure) : moyenne des intervalles réels (hors
/// premier, qui mesure le démarrage) rapportée à l'intervalle visé.
/// `None` = pas assez d'intervalles pour une moyenne honnête.
fn cadence_drift(intervals_ns: &[u64], interval: Duration) -> Option<f64> {
    let body = intervals_ns.get(1..)?;
    if body.is_empty() {
        return None;
    }
    let mean_ns = body.iter().sum::<u64>() as f64 / body.len() as f64;
    Some(mean_ns / interval.as_nanos() as f64)
}

/// Verdict d'intégrité (pur, extrait pour testabilité).
#[derive(Debug, PartialEq)]
enum IntegrityVerdict {
    /// 0 point valide → on ne conclut RIEN (jamais de PASS silencieux).
    Indeterminate,
    Judged {
        pass: bool,
        worst_delivery: f64,
        worst_n: usize,
        total_dups: u64,
    },
}

fn integrity_verdict(points: &[PointMetrics]) -> IntegrityVerdict {
    let valid: Vec<&PointMetrics> = points.iter().filter(|p| !p.invalid()).collect();
    if valid.is_empty() {
        return IntegrityVerdict::Indeterminate;
    }
    // Les doublons de TOUS les points comptent (même invalides) : un doublon est
    // une maladie d'intégrité quelle que soit la validité du point de cadence.
    let total_dups: u64 = points.iter().map(|p| p.total_dups).sum();
    let worst = valid
        .iter()
        .min_by(|a, b| a.delivery_rate.total_cmp(&b.delivery_rate))
        .unwrap();
    IntegrityVerdict::Judged {
        pass: worst.delivery_rate >= 0.999 && total_dups == 0,
        worst_delivery: worst.delivery_rate,
        worst_n: worst.n,
        total_dups,
    }
}

/// Lance un point de courbe : N nœuds, charge fixe/nœud pendant `duration`.
async fn run_point(
    n: usize,
    duration: Duration,
    interval: Duration,
    payload_bytes: usize,
    seed: u64,
) -> anyhow::Result<PointMetrics> {
    if n < 2 {
        anyhow::bail!(
            "N doit être ≥ 2 : à N=1 la seule cible possible est soi-même \
             (self-send = interception locale, pas un chemin réseau — tautologie)"
        );
    }
    eprintln!("\n── Point N={n} (charge {:.1} msg/s/nœud, {}o, {}s) ──",
        1.0 / interval.as_secs_f64(), payload_bytes, duration.as_secs());

    // 1. Spawn N nœuds VRAIMENT hermétiques : n0 OFF + mDNS OFF (transport) ET
    //    DHT rendez-vous OFF (runtime, voir cfg plus bas). Sans ce trio, un nœud
    //    de test PUBLIE au rendez-vous PARTAGÉ et pollue la vraie flotte de
    //    fantômes loopback injoignables (incident 20/07 : ~150 fantômes sur le
    //    Mac) — cf runbook « nœud de test → --isolated ».
    let mut nodes = Vec::with_capacity(n);
    for _ in 0..n {
        nodes.push(
            TomNode::bind(TomNodeConfig::new().hermetic()).await?,
        );
    }
    let ids: Vec<NodeId> = nodes.iter().map(|nd| nd.id()).collect();
    let addrs: Vec<_> = nodes.iter().map(|nd| nd.addr()).collect();

    // Collect node ids for whitelisting
    let node_ids: std::collections::HashSet<_> = ids.iter().copied().collect();

    // 2. Runtimes + canaux.
    let mut channels = Vec::with_capacity(n);
    for (i, node) in nodes.into_iter().enumerate() {
        // Set allowed peers whitelist before spawning runtime
        node.set_allowed_peers(Some(node_ids.clone()));

        let cfg = RuntimeConfig {
            username: format!("{}courbe-{i}", tom_protocol::TEST_NODE_PREFIX),
            encryption: true,
            // CRITIQUE : couper le rendez-vous DHT PARTAGÉ. `..Default::default()`
            // le laisse à `true` (runtime/mod.rs:130) → sinon publication de
            // fantômes dans le rendez-vous de la vraie flotte (incident 20/07).
            enable_dht: false,
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
    // Handles conservés pour le teardown explicite de fin de point.
    let handles: Vec<_> = channels.iter().map(|c| c.handle.clone()).collect();

    // Chauffe : laisser les premières connexions s'établir.
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Époque commune : latence = delta de nanos sur CETTE horloge (pas de skew).
    let epoch = Instant::now();
    let ids = Arc::new(ids);
    // Format du payload : voir `BANC_MAGIC` (ts | clé unique | magic | 'A').
    let expect_len = payload_bytes.max(MIN_PAYLOAD);

    // 4. Une tâche émettrice + une réceptrice par nœud.
    let mut send_tasks = Vec::with_capacity(n);
    let mut recv_tasks = Vec::with_capacity(n);
    for (i, ch) in channels.into_iter().enumerate() {
        let handle = ch.handle.clone();
        let mut messages = ch.messages;
        let ids = ids.clone();
        let n_ids = ids.len();

        // Récepteur : reçoit pendant la charge PUIS draine jusqu'à QUIESCENCE
        // (plus rien pendant `QUIESCENCE`) → livraison VRAIE, pas tronquée.
        // Ne compte QUE les messages du banc (signature de payload), isole les
        // « étrangers » (chatter protocolaire éventuel dans le canal applicatif).
        let hard_cap = duration + MAX_DRAIN;
        recv_tasks.push(tokio::spawn(async move {
            let mut mine = 0u64;
            let mut foreign = 0u64;
            let mut ledger = DeliveryLedger::default();
            let mut lat_ms: Vec<f64> = Vec::new();
            // Init à `duration` (contrat de `drain_should_close`) : la fenêtre
            // de quiescence court au plus tôt à partir de la FIN de charge —
            // même à zéro message reçu, les tardifs ont leur fenêtre complète.
            let mut last_msg = duration;
            loop {
                let now = epoch.elapsed();
                if drain_should_close(now, duration, last_msg, hard_cap) {
                    break;
                }
                match tokio::time::timeout(Duration::from_millis(250), messages.recv()).await {
                    Ok(Some(msg)) => {
                        let Some((sent_ns, key)) = parse_mine(&msg.payload, expect_len) else {
                            foreign += 1; // non-banc : chatter protocolaire
                            continue;
                        };
                        mine += 1;
                        last_msg = epoch.elapsed();
                        // Doublon = même clé UNIQUE (nœud, seq) revue → incontestable.
                        ledger.record(key);
                        let now_ns = epoch.elapsed().as_nanos() as u64;
                        lat_ms.push((now_ns.saturating_sub(sent_ns)) as f64 / 1e6);
                    }
                    Ok(None) => break,        // canal fermé (shutdown)
                    Err(_) => {}              // tick de 250 ms → reboucle (teste la quiescence)
                }
            }
            (mine, ledger.dups, foreign, lat_ms)
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
            let mut seq = 0u32;
            let mut intervals_ns: Vec<u64> = Vec::new();
            let mut last = Instant::now();
            while epoch.elapsed() < duration {
                // cible ≠ soi
                let mut j = rng.random_range(0..n_ids);
                if j == i { j = (j + 1) % n_ids; }
                let sent_ns = epoch.elapsed().as_nanos() as u64;
                let key: u64 = ((i as u64) << 32) | (seq as u64); // unique par (nœud, seq)
                seq += 1;
                let payload = build_payload(sent_ns, key, expect_len);
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
        if let Some(drift) = cadence_drift(&intervals, interval) {
            worst_drift = worst_drift.max(drift);
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
    all_lat.sort_by(f64::total_cmp); // NaN-safe (les latences ne peuvent pas être NaN, mais robuste)
    let latency_max_ms = all_lat.last().copied().unwrap_or(0.0);

    // Livraison UNIQUE = reçus moins doublons (dédup = santé I8, doit être 0).
    let unique = total_recv.saturating_sub(total_dups);
    let dur_s = duration.as_secs_f64();
    let delivered_per_node_hz = if recv_nodes > 0 {
        unique as f64 / recv_nodes as f64 / dur_s
    } else {
        0.0
    };

    // Teardown explicite borné : réduit le bruit de teardown massif (findings
    // §2bis) et évite les fuites de tâches si run_point est appelé en boucle.
    for h in handles {
        let _ = tokio::time::timeout(Duration::from_secs(5), h.shutdown()).await;
    }

    Ok(PointMetrics {
        n,
        delivered_per_node_hz,
        delivery_rate: if total_offered > 0 { unique as f64 / total_offered as f64 } else { 0.0 },
        latency_p50_ms: percentile(&all_lat, 50.0),
        latency_p95_ms: percentile(&all_lat, 95.0),
        latency_max_ms,
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
    for (cycle, n) in sizes.iter().enumerate() {
        // TEST B : logguer octets alloués avant/après run_point
        let bytes_before = crate::get_allocated_bytes();
        eprintln!("[TEST B CYCLE {}] Octets AVANT : {} Mo", cycle + 1, bytes_before / 1_000_000);
        points.push(run_point(*n, duration, interval, payload_bytes, seed).await?);
        let bytes_after = crate::get_allocated_bytes();
        eprintln!("[TEST B CYCLE {}] Octets APRÈS : {} Mo (diff: {} Mo)",
            cycle + 1, bytes_after / 1_000_000, (bytes_after as i64 - bytes_before as i64) / 1_000_000);
    }

    // ── Rapport ──
    // AXE JUGÉ = intégrité de livraison (perte RÉELLE, mesurée par drain complet
    // jusqu'à quiescence) + dédup. La latence est RAPPORTÉE, pas jugée : sur un
    // runtime tokio PARTAGÉ elle mêle « latence protocolaire ∝ N » et
    // « ordonnanceur saturé ». Le débit/nœud à charge FAIBLE ≈ la charge offerte
    // (tautologique) : le point de SATURATION n'est pas sondé ici — il faut
    // monter --rate (et, pour la latence, l'isolation par PROCESSUS, banc §3).
    eprintln!("\n=== COURBE ===");
    eprintln!("{:>4}  {:>10}  {:>12}  {:>11}  {:>8}  {:>8}  {:>8}  {:>6}  {:>4}  {:>8}",
        "N", "livraison", "offert→reçu", "livré/nœud", "p50ms", "p95ms", "maxms", "dérive", "dup", "état");
    for p in &points {
        eprintln!("{:>4}  {:>8.1}%  {:>5}→{:<6}  {:>9.2}Hz  {:>8.1}  {:>8.1}  {:>8.1}  {:>5.2}x  {:>4}  {:>8}",
            p.n, p.delivery_rate * 100.0,
            p.total_offered, p.total_recv, p.delivered_per_node_hz,
            p.latency_p50_ms, p.latency_p95_ms, p.latency_max_ms, p.worst_cadence_drift,
            p.total_dups,
            if p.invalid() { "INVALIDE" } else { "ok" });
        if p.invalid() {
            eprintln!("       └ N={} INVALIDE : {}", p.n, p.invalid_reason());
        }
    }
    let any_errs: u64 = points.iter().map(|p| p.total_errs).sum();
    let any_foreign: u64 = points.iter().map(|p| p.total_foreign).sum();
    if any_errs > 0 {
        eprintln!("ℹ️ {any_errs} rejet(s) API à l'émission (retries transport possibles derrière — non compté comme perte).");
    }
    if any_foreign > 0 {
        eprintln!("ℹ️ {any_foreign} message(s) non-banc filtré(s) du canal applicatif (chatter protocolaire — exclus du calcul).");
    }

    // ── Verdict : INTÉGRITÉ DE LIVRAISON (le seul axe que le in-process juge,
    //    logique pure `integrity_verdict` — testée) ──
    let valid: Vec<&PointMetrics> = points.iter().filter(|p| !p.invalid()).collect();
    eprintln!();
    let (worst_delivery, worst_n, verdict_dups, integrity_ok) = match integrity_verdict(&points) {
        IntegrityVerdict::Indeterminate => {
            eprintln!("VERDICT : indéterminé — 0 point valide (dérive d'émission partout).");
            return Ok(());
        }
        IntegrityVerdict::Judged { pass, worst_delivery, worst_n, total_dups } => {
            (worst_delivery, worst_n, total_dups, pass)
        }
    };
    eprintln!("VERDICT INTÉGRITÉ (N valides {}→{}) : livraison min {:.2}% (N={}), dédup {} → {}",
        valid.first().unwrap().n, valid.last().unwrap().n,
        worst_delivery * 100.0, worst_n,
        if verdict_dups == 0 { "0 doublon" } else { "DOUBLONS !" },
        if integrity_ok { "PASS (0 perte, 0 doublon, hermétique)" } else { "FAIL — perte ou doublon RÉEL à investiguer" });

    // Latence + débit-à-l'échelle : RAPPORTÉS, explicitement NON jugés ici.
    let lat_ratio = if valid.first().unwrap().latency_p50_ms > 0.0 {
        valid.last().unwrap().latency_p50_ms / valid.first().unwrap().latency_p50_ms
    } else { f64::INFINITY };
    eprintln!("LATENCE (rapportée, NON jugée) : p50 ×{:.1} de N={} à N={} sur runtime PARTAGÉ.",
        lat_ratio, valid.first().unwrap().n, valid.last().unwrap().n);
    eprintln!("⚠️ Portée : ce banc prouve l'INTÉGRITÉ (perte/doublon/herméticité) à charge {:.0} msg/s/nœud \
        sur loopback. Il ne prouve NI le débit à saturation (monter --rate) NI la latence-vs-N \
        (runtime partagé → multi-process, banc §3). Pas de conclusion « la masse tient » sur la \
        perf sans ces deux-là.", rate_hz);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(drift: f64, p95: f64) -> PointMetrics {
        PointMetrics {
            n: 5,
            delivered_per_node_hz: 1.0,
            delivery_rate: 1.0,
            latency_p50_ms: 1.0,
            latency_p95_ms: p95,
            latency_max_ms: p95,
            worst_cadence_drift: drift,
            total_offered: 100,
            total_errs: 0,
            total_recv: 100,
            total_dups: 0,
            total_foreign: 0,
        }
    }

    #[test]
    fn percentile_empty_is_zero() {
        assert_eq!(percentile(&[], 50.0), 0.0);
        assert_eq!(percentile(&[], 95.0), 0.0);
    }

    #[test]
    fn percentile_single_element() {
        assert_eq!(percentile(&[42.0], 50.0), 42.0);
        assert_eq!(percentile(&[42.0], 95.0), 42.0);
    }

    #[test]
    fn percentile_p50_p95_on_sorted() {
        let d = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert_eq!(percentile(&d, 50.0), 6.0); // idx round(0.5*9)=5 → 6.0
        assert_eq!(percentile(&d, 95.0), 10.0); // idx round(0.95*9)=9 → 10.0
        assert_eq!(percentile(&d, 0.0), 1.0);
        assert_eq!(percentile(&d, 100.0), 10.0);
    }

    #[test]
    fn invalid_only_on_send_drift_not_latency() {
        // Latence énorme mais cadence d'émission tenue → VALIDE (la latence n'est
        // PLUS un critère d'exclusion depuis le drain jusqu'à quiescence).
        assert!(!metrics(1.0, 60_000.0).invalid());
        // Dérive d'émission au-delà du seuil → INVALIDE.
        assert!(metrics(2.0, 5.0).invalid());
        assert!(!metrics(1.49, 5.0).invalid());
        assert!(metrics(1.51, 5.0).invalid());
    }

    #[test]
    fn invalid_reason_matches_cause() {
        assert_eq!(metrics(1.0, 5.0).invalid_reason(), "");
        assert!(metrics(2.0, 5.0).invalid_reason().contains("dérive"));
    }

    // ── Mécanismes de MESURE (verrouillage oracle 20/07 : l'instrument qui a
    //    déjà accusé le protocole à tort doit être testé pièce par pièce) ──

    fn pt(n: usize, delivery_rate: f64, dups: u64, drift: f64) -> PointMetrics {
        PointMetrics {
            n,
            delivery_rate,
            total_dups: dups,
            ..metrics(drift, 5.0)
        }
    }

    #[test]
    fn payload_roundtrip_and_reject_foreign() {
        let key = (7u64 << 32) | 42;
        let p = build_payload(123_456, key, 64);
        assert_eq!(p.len(), 64);
        assert_eq!(parse_mine(&p, 64), Some((123_456, key)));
        // Longueur inattendue → étranger.
        assert_eq!(parse_mine(&p, 65), None);
        // Magic corrompu → étranger (l'ancien filtre « octet 'A' » d'avant la
        // révision aurait pu l'accepter).
        let mut bad = p.clone();
        bad[17] = b'X';
        assert_eq!(parse_mine(&bad, 64), None);
        // Trop court pour porter la signature → jamais mien.
        assert_eq!(parse_mine(&[b'A'; 8], 8), None);
    }

    #[test]
    fn ledger_counts_unique_and_dup_exactly_once() {
        let mut l = DeliveryLedger::default();
        assert!(l.record(1));
        assert!(l.record(2));
        assert!(!l.record(1)); // doublon revu
        assert!(!l.record(1)); // revu encore
        assert_eq!(l.unique, 2);
        assert_eq!(l.dups, 2);
    }

    #[test]
    fn drain_gives_full_quiescence_window_even_with_zero_messages() {
        let dur = Duration::from_secs(15);
        let cap = dur + MAX_DRAIN;
        // Pendant la charge : jamais de clôture, même silence long.
        assert!(!drain_should_close(Duration::from_secs(14), dur, dur, cap));
        // Zéro message reçu (last_msg initialisé à duration, contrat) : la
        // fenêtre de quiescence COMPLÈTE est accordée après la fin de charge —
        // un « 0 % livré » ne peut pas venir d'une clôture prématurée.
        assert!(!drain_should_close(dur, dur, dur, cap));
        assert!(!drain_should_close(dur + QUIESCENCE - Duration::from_millis(1), dur, dur, cap));
        assert!(drain_should_close(dur + QUIESCENCE, dur, dur, cap));
        // Un message tardif fait glisser la fenêtre.
        let late = Duration::from_secs(16);
        assert!(!drain_should_close(late + QUIESCENCE - Duration::from_millis(1), dur, late, cap));
        assert!(drain_should_close(late + QUIESCENCE, dur, late, cap));
    }

    #[test]
    fn drain_hard_cap_always_closes() {
        let dur = Duration::from_secs(15);
        let cap = dur + MAX_DRAIN;
        // Même en pleine activité (last_msg = now), le filet dur ferme.
        assert!(drain_should_close(cap, dur, cap, cap));
        assert!(drain_should_close(cap + Duration::from_secs(1), dur, cap + Duration::from_secs(1), cap));
    }

    #[test]
    fn cadence_drift_ignores_first_interval_and_measures_contention() {
        let target = Duration::from_millis(100);
        // Nominal : le 1er intervalle (démarrage) est ignoré.
        let d = cadence_drift(&[999_999_999, 100_000_000, 100_000_000], target).unwrap();
        assert!((d - 1.0).abs() < 0.01, "dérive nominale ≈ 1.0, obtenu {d}");
        // Contention ×2.
        let d = cadence_drift(&[0, 200_000_000, 200_000_000], target).unwrap();
        assert!((d - 2.0).abs() < 0.01, "dérive contention ≈ 2.0, obtenu {d}");
        // Pas assez d'échantillons pour une moyenne honnête.
        assert_eq!(cadence_drift(&[], target), None);
        assert_eq!(cadence_drift(&[100], target), None);
    }

    #[test]
    fn verdict_never_passes_silently() {
        // 0 point valide → Indeterminate (jamais un PASS par défaut).
        assert_eq!(
            integrity_verdict(&[pt(5, 1.0, 0, 9.0)]),
            IntegrityVerdict::Indeterminate
        );
        // Zéro offert → delivery_rate 0.0 → FAIL, pas de pass silencieux.
        match integrity_verdict(&[pt(5, 0.0, 0, 1.0)]) {
            IntegrityVerdict::Judged { pass, .. } => assert!(!pass),
            v => panic!("attendu Judged, obtenu {v:?}"),
        }
        // 99,8 % < seuil 99,9 % → FAIL.
        match integrity_verdict(&[pt(5, 0.998, 0, 1.0)]) {
            IntegrityVerdict::Judged { pass, worst_n, .. } => {
                assert!(!pass);
                assert_eq!(worst_n, 5);
            }
            v => panic!("attendu Judged, obtenu {v:?}"),
        }
        // 100 % mais un doublon → FAIL (même si le doublon vient d'un point
        // INVALIDE : un doublon est une maladie d'intégrité, point).
        match integrity_verdict(&[pt(5, 1.0, 0, 1.0), pt(10, 1.0, 1, 9.0)]) {
            IntegrityVerdict::Judged { pass, total_dups, .. } => {
                assert!(!pass);
                assert_eq!(total_dups, 1);
            }
            v => panic!("attendu Judged, obtenu {v:?}"),
        }
        // 100 %, 0 doublon → PASS.
        match integrity_verdict(&[pt(5, 1.0, 0, 1.0), pt(10, 1.0, 0, 1.0)]) {
            IntegrityVerdict::Judged { pass, .. } => assert!(pass),
            v => panic!("attendu Judged, obtenu {v:?}"),
        }
    }

    /// F1 — smoke d'orchestration de bout en bout : 2 nœuds RÉELS in-process,
    /// charge courte. Assertions volontairement robustes (aucun seuil de perf,
    /// un runner CI chargé ne doit pas le faire flaker) : ≥ 1 message livré,
    /// zéro doublon. Hermétique par construction (trio isolated).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn smoke_run_point_n2_hermetic() {
        let p = run_point(2, Duration::from_secs(2), Duration::from_millis(500), 64, 7)
            .await
            .expect("run_point N=2");
        assert!(p.total_offered >= 2, "chaque nœud doit offrir ≥ 1 message");
        assert!(p.total_recv > 0, "aucun message livré sur loopback N=2");
        assert_eq!(p.total_dups, 0, "doublon inattendu");
    }

    #[tokio::test]
    async fn run_point_rejects_n1_self_send_tautology() {
        assert!(
            run_point(1, Duration::from_secs(1), Duration::from_millis(500), 64, 7)
                .await
                .is_err()
        );
    }
}
