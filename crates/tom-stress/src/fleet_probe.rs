//! Fleet probe — autonomous live orchestrator for real-device testing.
//!
//! Binds ONE controllable node that joins the REAL network (relay + local
//! discovery, like the fleet), then, as devices plug in and are discovered,
//! it exercises the API "in all directions" against each of them and prints
//! a live per-device table plus machine-readable JSONL relevés:
//!
//! - **Presence** (build 20): challenges every Online peer on a cadence,
//!   counts accepted attestations per attester and their latency.
//! - **Reachability**: sends a small chat ping to each discovered peer; the
//!   apps auto-echo, so a returning message confirms two-way transport.
//! - **Metrics**: dumps the full per-outcome presence counters each report.
//!
//! Gate is 0.0 (plumbing) so the probe accepts attestations from any real
//! device regardless of whether it has relayed for us yet — this measures
//! reach and liveness, not the anti-Sybil gate (that is phase 2, and the
//! gate's real-evidence behavior is proven by `scenario_presence`).
//!
//! Runs until Ctrl+C (or `--duration-secs`). Point it at the NAS relay:
//!   tom-stress fleet-probe --relay-url http://192.168.0.21:3340
//!   tom-stress fleet-probe --relay-url http://82.67.95.8:3340 --duration-secs 600

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tom_protocol::{ProtocolEvent, ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

/// First 8 chars of a NodeId string (display only).
fn short_id(id: &str) -> &str {
    &id[..id.len().min(8)]
}

pub struct FleetProbeConfig {
    pub relay_url: Option<String>,
    /// None = run until Ctrl+C.
    pub duration_secs: Option<u64>,
    /// Seconds between challenge/reachability rounds.
    pub probe_interval_secs: u64,
    /// Seconds between printed reports.
    pub report_interval_secs: u64,
    /// Also send a chat ping to each peer each round (reachability).
    pub with_reachability: bool,
    /// SIM: inject a presence clock offset (ms) on the probe node — validates
    /// the anti-NTP hardening against real, skewed devices.
    pub clock_offset_ms: i64,
}

#[derive(Default, Clone)]
struct PeerStat {
    username: String,
    source: String,
    attestations: u64,
    last_latency_ms: u64,
    echoes: u64,
}

pub async fn run(cfg: FleetProbeConfig) -> anyhow::Result<()> {
    let running = crate::common::setup_ctrlc();
    let start = Instant::now();

    // Join the real network exactly like a fleet node.
    let mut node_config = TomNodeConfig::new().n0_discovery(false).local_discovery(true);
    if let Some(ref url) = cfg.relay_url {
        node_config = node_config.relay_url(url.parse()?);
    }
    let node = TomNode::bind(node_config).await?;
    let my_id = node.id();
    eprintln!("Fleet probe node ID: {my_id}");
    if let Some(ref url) = cfg.relay_url {
        eprintln!("Relay: {url}");
    }
    eprintln!("Probing every {}s, reporting every {}s\n", cfg.probe_interval_secs, cfg.report_interval_secs);

    // Gate 0.0 so we accept attestations from any real device; encryption on
    // to match the fleet; auto-probe off (we drive it explicitly).
    let runtime_config = RuntimeConfig {
        username: "fleet-probe".into(),
        encryption: true,
        presence_contribution_min: 0.0,
        presence_clock_offset_ms: cfg.clock_offset_ms,
        ..Default::default()
    };
    if cfg.clock_offset_ms != 0 {
        eprintln!("SIM: probe presence clock offset = {}ms", cfg.clock_offset_ms);
    }
    let mut channels = ProtocolRuntime::spawn(node, runtime_config);
    let handle = channels.handle.clone();

    let mut peers: HashMap<String, PeerStat> = HashMap::new();
    let mut probe_tick = tokio::time::interval(Duration::from_secs(cfg.probe_interval_secs));
    let mut report_tick = tokio::time::interval(Duration::from_secs(cfg.report_interval_secs));
    probe_tick.tick().await;
    report_tick.tick().await;

    loop {
        if !running.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        if let Some(limit) = cfg.duration_secs {
            if start.elapsed().as_secs() >= limit {
                break;
            }
        }

        tokio::select! {
            // ── Drain protocol events: discovery, attestations, echoes ──
            Some(event) = channels.events.recv() => {
                match event {
                    ProtocolEvent::PeerDiscovered { node_id, username, source } => {
                        let e = peers.entry(node_id.to_string()).or_default();
                        if !username.is_empty() { e.username = username; }
                        e.source = format!("{source:?}");
                    }
                    ProtocolEvent::PresenceAttestationReceived { attester_id, latency_ms, .. } => {
                        let e = peers.entry(attester_id.to_string()).or_default();
                        e.attestations += 1;
                        e.last_latency_ms = latency_ms;
                    }
                    _ => {}
                }
            }

            // ── Reachability echoes ──
            Some(msg) = channels.messages.recv() => {
                if let Ok(text) = String::from_utf8(msg.payload.clone()) {
                    if text.starts_with("recu 5/5") || text.starts_with("PONG") {
                        let e = peers.entry(msg.from.to_string()).or_default();
                        e.echoes += 1;
                    }
                }
            }

            // ── Probe round: challenge all + optional reachability ping ──
            _ = probe_tick.tick() => {
                handle.check_presence_all_online().await;
                if cfg.with_reachability {
                    for peer in handle.connected_peers().await {
                        let _ = handle.send_message(peer, b"PING:fleet-probe".to_vec()).await;
                    }
                }
            }

            // ── Report round: live table + JSONL relevé ──
            _ = report_tick.tick() => {
                print_report(&handle, &peers, start).await;
            }

            else => break,
        }
    }

    eprintln!("\nFleet probe stopping — final report:");
    print_report(&handle, &peers, start).await;
    handle.shutdown().await;
    Ok(())
}

async fn print_report(
    handle: &tom_protocol::RuntimeHandle,
    peers: &HashMap<String, PeerStat>,
    start: Instant,
) {
    let elapsed = start.elapsed().as_secs();
    let connected = handle.connected_peers().await;
    let connected_set: std::collections::HashSet<String> =
        connected.iter().map(|n| n.to_string()).collect();

    eprintln!("\n── Fleet report @ {elapsed}s — {} discovered, {} connected ──", peers.len(), connected.len());
    eprintln!("  {:<10} {:<16} {:<8} {:>6} {:>9} {:>6} {:>5}", "id", "username", "source", "attest", "lat(ms)", "echo", "conn");
    let mut rows: Vec<(&String, &PeerStat)> = peers.iter().collect();
    rows.sort_by_key(|(_, s)| std::cmp::Reverse(s.attestations));
    for (id, s) in rows {
        eprintln!(
            "  {:<10} {:<16} {:<8} {:>6} {:>9} {:>6} {:>5}",
            short_id(id),
            if s.username.is_empty() { "?" } else { &s.username },
            s.source,
            s.attestations,
            s.last_latency_ms,
            s.echoes,
            if connected_set.contains(id) { "yes" } else { "no" },
        );
    }

    // Machine-readable relevé line (presence counters).
    if let Some(m) = handle.presence_metrics().await {
        if let Ok(json) = serde_json::to_string(&serde_json::json!({
            "event": "fleet-probe-relevé",
            "elapsed_s": elapsed,
            "peers_discovered": peers.len(),
            "peers_connected": connected.len(),
            "presence": {
                "issued": m.issued,
                "accepted": m.accepted,
                "challenges_received": m.challenges_received,
                "signed": m.signed,
                "drop_gate": m.drop_gate,
                "drop_bad_signature": m.drop_bad_signature,
                "drop_unknown_challenge": m.drop_unknown_challenge,
                "refused_budget": m.refused_budget,
                "latency_min_ms": m.latency_min_ms,
                "latency_mean_ms": m.mean_latency_ms(),
                "latency_max_ms": m.latency_max_ms,
            }
        })) {
            println!("{json}");
        }
    }
}
