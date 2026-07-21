//! R4 scénario : rendez-vous DHT (deux inconnus se trouvent).
//!
//! Design : `docs/plans/banc-roles-sous-charge.md` §2-R4 (lignes 108-124).
//!
//! Montage hermétique (in-process) : deux nœuds avec identités neuves, AUCUN
//! pair configuré, state.db vierges, branchés au runtime avec le MÊME
//! `rendezvous_namespace` TEST → chacun publie et découvre.
//!
//! Verdicts :
//! (a) Aller-retour : probe→echo en ≤ timeout_secs
//! (b) Source rendez-vous : event DiscoveryTiming mentionne « Rendez-vous »
//! (c) Entries authentifiées (informatif) : compte pairs rejetés (signature)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tom_protocol::{NodeId, ProtocolEvent, ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

#[derive(Default, Clone)]
struct ProbeState {
    /// Messages envoyés : nonce → timestamp.
    sent: HashMap<String, Instant>,
    /// Échos reçus (nonce → timestamp d'arrivée).
    echoes: HashMap<String, Instant>,
}

#[derive(Default)]
struct NodeObs {
    /// Probes reçus (nonce).
    probes_rcvd: Vec<String>,
    /// Rendezvous découvert via event.
    rendezvous_seen: bool,
    /// Pairs gardés/rejetés (informatif).
    rendezvous_detail: String,
}

/// Génère un nonce 16 hex aléatoire.
fn gen_nonce() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    format!("{:016x}", nanos ^ (rand::random::<u64>()))
}

/// Envoie un probe depuis `handle` vers `target` avec un nonce.
async fn send_probe(
    handle: &tom_protocol::RuntimeHandle,
    target: NodeId,
    nonce: &str,
) -> anyhow::Result<()> {
    let payload = format!("r4-probe:{}", nonce).into_bytes();
    if let Err(e) = handle.send_message(target, payload).await {
        eprintln!("  [warn] envoi probe vers {target} échoué : {e}");
    }
    Ok(())
}

/// Envoie un echo en réponse à un probe reçu.
async fn send_echo(
    handle: &tom_protocol::RuntimeHandle,
    target: NodeId,
    nonce: &str,
) -> anyhow::Result<()> {
    let payload = format!("r4-echo:{}", nonce).into_bytes();
    if let Err(e) = handle.send_message(target, payload).await {
        eprintln!("  [warn] envoi echo vers {target} échoué : {e}");
    }
    Ok(())
}

/// Parse probe payload, retourne nonce si valide.
fn parse_probe(payload: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(payload).ok()?;
    s.strip_prefix("r4-probe:").map(|s| s.to_string())
}

/// Parse echo payload, retourne nonce si valide.
fn parse_echo(payload: &[u8]) -> Option<String> {
    let s = std::str::from_utf8(payload).ok()?;
    s.strip_prefix("r4-echo:").map(|s| s.to_string())
}

pub async fn run(
    namespace: Option<String>,
    timeout_secs: u64,
    probe_interval_secs: u64,
    linger_secs: u64,
) -> anyhow::Result<()> {
    // None = rendez-vous RÉEL (capstone) : le probing applicatif est coupé pour
    // ne jamais injecter de r4-probe dans les chats des vraies apps ; le
    // verdict se réduit à (b) source-rendezvous. Some(label) = étage isolé.
    let probing = namespace.is_some();
    let ns_label = namespace
        .clone()
        .unwrap_or_else(|| "PRODUCTION (capstone)".to_string());
    eprintln!("\n=== R4 : Rendez-vous DHT — deux inconnus se trouvent (namespace={}) ===", ns_label);
    eprintln!(
        "   Timeout total: {} s, probe interval: {} s, linger: {} s, probing: {}",
        timeout_secs, probe_interval_secs, linger_secs,
        if probing { "on" } else { "OFF (mode capstone)" }
    );

    // ── Spawn un seul nœud hermétique sans pair configuré ──
    let node = TomNode::bind(TomNodeConfig::new().hermetic()).await?;
    let node_id = node.id();
    eprintln!("  Node: {}", node_id);

    // ── Attach runtime avec rendezvous_namespace ──
    let config = RuntimeConfig {
        username: format!("{}r4-{:08x}", tom_protocol::TEST_NODE_PREFIX, rand::random::<u32>()),
        encryption: true,
        enable_dht: true,
        rendezvous_namespace: namespace.clone(),
        ..Default::default()
    };

    let channels = ProtocolRuntime::spawn(node, config);
    let handle = channels.handle.clone();

    let obs = Arc::new(Mutex::new(NodeObs::default()));
    let probe_state = Arc::new(Mutex::new(ProbeState::default()));

    // ── Collecteur d'événements
    let mut messages = channels.messages;
    let mut events = channels.events;
    let obs_task = obs.clone();
    let probe_task = probe_state.clone();
    let handle_echo = handle.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = messages.recv() => {
                    let Some(msg) = msg else { break };
                    if let Some(nonce) = parse_probe(&msg.payload) {
                        obs_task.lock().unwrap().probes_rcvd.push(nonce.clone());
                        let _ = send_echo(&handle_echo, msg.from, &nonce).await;
                    } else if let Some(nonce) = parse_echo(&msg.payload) {
                        probe_task.lock().unwrap().echoes.insert(nonce, Instant::now());
                    }
                }
                ev = events.recv() => {
                    let Some(ev) = ev else { break };
                    if let ProtocolEvent::DiscoveryTiming { detail, .. } = ev {
                        // Détail émis par loop.rs : « Rendez-vous (…) : N pairs gardés, M rejetés (signature) ».
                        // L'oracle (b) exige N ≥ 1 : un event « 0 gardés, M rejetés » ne prouve
                        // aucune découverte (que des entrées squattées/mortes filtrées).
                        if detail.contains("Rendez-vous") {
                            let kept = detail
                                .split(':')
                                .nth(1)
                                .and_then(|s| s.trim().split(' ').next())
                                .and_then(|n| n.parse::<u32>().ok());
                            if kept.is_none() {
                                // Garde F1 (review) : libellé de loop.rs changé →
                                // le compte ne se parse plus. WARN visible plutôt
                                // qu'un faux-FAIL silencieux de l'oracle (b).
                                eprintln!(
                                    "  [warn] détail rendez-vous non parsable \
                                     (libellé loop.rs changé ?) : {detail}"
                                );
                            }
                            let mut o = obs_task.lock().unwrap();
                            if kept.is_some_and(|n| n >= 1) {
                                o.rendezvous_seen = true;
                            }
                            o.rendezvous_detail = detail.clone();
                        }
                    }
                }
            }
        }
    });

    // ── Boucle de découverte + probing ──
    let t0 = Instant::now();
    let mut last_probe_time = Instant::now();
    let mut peer_discovered = false;
    let mut verdict_echo = false;
    let mut done_time: Option<Instant> = None;

    loop {
        // Timeout global
        if t0.elapsed() > Duration::from_secs(timeout_secs) {
            break;
        }

        // Mode linger : reste vivant après verdict local pour servir d'écho à l'autre nœud
        if let Some(done) = done_time {
            if done.elapsed() > Duration::from_secs(linger_secs) {
                break;
            }
        }

        // Poll statuses pour découvrir les pairs (via rendez-vous).
        let statuses = handle.get_peer_statuses().await;

        // Découverte : au premier pair ≠ nous-mêmes
        if !peer_discovered && statuses.iter().any(|(id, _)| *id != node_id) {
            peer_discovered = true;
            eprintln!(
                "  [OK] Premier pair vu à t+{:.1} s (baseline froide découverte)",
                t0.elapsed().as_secs_f64()
            );
        }

        // Mode capstone (probing off) : le verdict local tombe dès la découverte —
        // la preuve de connexion est relevée CÔTÉ FLOTTE par l'orchestrateur
        // (node_id de l'inconnu dans pairs_connectes des :9091).
        if !probing && peer_discovered && done_time.is_none() {
            done_time = Some(Instant::now());
        }

        // Envoyer probes toutes les `probe_interval_secs`.
        if probing && last_probe_time.elapsed() > Duration::from_secs(probe_interval_secs) {
            if peer_discovered {
                if let Some((peer_id, _)) = statuses.iter().find(|(id, _)| *id != node_id) {
                    let nonce = gen_nonce();
                    probe_state.lock().unwrap().sent.insert(nonce.clone(), Instant::now());
                    let _ = send_probe(&handle, *peer_id, &nonce).await;
                }
            }
            last_probe_time = Instant::now();
        }

        // Vérifier les échos (RTT = arrivée réelle de l'écho − envoi du probe,
        // pas l'instant du poll — sinon le RTT absorbe jusqu'à 500 ms de tick).
        if !verdict_echo {
            let ps = probe_state.lock().unwrap();
            let best = ps
                .echoes
                .iter()
                .filter_map(|(nonce, arrival)| {
                    ps.sent.get(nonce).map(|sent| arrival.duration_since(*sent))
                })
                .min();
            if let Some(rtt) = best {
                eprintln!(
                    "  [OK] Aller-retour réussi : RTT {:.3} s — verdict à t+{:.1} s depuis le bind",
                    rtt.as_secs_f64(),
                    t0.elapsed().as_secs_f64()
                );
                verdict_echo = true;
                done_time = Some(Instant::now());
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // ── Verdicts ──
    eprintln!("\n── Verdicts ──");

    let ok_aller_retour = verdict_echo;
    let (ok_rendezvous, rendezvous_detail) = {
        let obs_data = obs.lock().unwrap();
        (obs_data.rendezvous_seen, obs_data.rendezvous_detail.clone())
    };

    if probing {
        eprintln!(
            "  [{}] R4.aller-retour : {}",
            if ok_aller_retour { "PASS" } else { "FAIL" },
            if ok_aller_retour {
                "probe→echo validé"
            } else {
                "AUCUN echo reçu"
            }
        );
    } else {
        eprintln!(
            "  [N/A ] R4.aller-retour : probing OFF (capstone) — preuve de \
             connexion relevée côté flotte par l'orchestrateur"
        );
    }

    eprintln!(
        "  [{}] R4.source-rendez-vous : {}",
        if ok_rendezvous { "PASS" } else { "FAIL" },
        if ok_rendezvous {
            "DiscoveryTiming mentionne « Rendez-vous »"
        } else {
            "Aucun événement rendez-vous reçu"
        }
    );

    // Informatif
    eprintln!(
        "  [INFO] R4.anti-squat : {}",
        if !rendezvous_detail.is_empty() {
            &rendezvous_detail
        } else {
            "(pas de détail)"
        }
    );

    let all_ok = ok_rendezvous && (ok_aller_retour || !probing);
    eprintln!(
        "\n=== VERDICT R4 ({}) : {} ===",
        ns_label,
        if all_ok { "PASS" } else { "FAIL" }
    );

    // Teardown borné
    let _ = tokio::time::timeout(Duration::from_secs(5), handle.shutdown()).await;

    if !all_ok {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_probe_valid() {
        let payload = b"r4-probe:abc123def456";
        assert_eq!(parse_probe(payload), Some("abc123def456".to_string()));
    }

    #[test]
    fn test_parse_probe_invalid() {
        let payload = b"r4-other:abc123def456";
        assert_eq!(parse_probe(payload), None);
    }

    #[test]
    fn test_parse_echo_valid() {
        let payload = b"r4-echo:abc123def456";
        assert_eq!(parse_echo(payload), Some("abc123def456".to_string()));
    }

    #[test]
    fn test_parse_echo_invalid() {
        let payload = b"r4-probe:abc123def456";
        assert_eq!(parse_echo(payload), None);
    }

    #[test]
    fn test_gen_nonce_uniqueness() {
        let n1 = gen_nonce();
        let n2 = gen_nonce();
        assert_ne!(n1, n2);
        assert_eq!(n1.len(), 16);
        assert_eq!(n2.len(), 16);
    }
}
