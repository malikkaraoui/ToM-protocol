//! ToM Protocol - iroh PoC: NAT Traversal Test
//!
//! Instrumented binary for testing hole punching across real networks.
//! Outputs structured JSON events for analysis.
//!
//! Usage:
//!   # Node A (listener - run on remote machine / NAS / VPS):
//!   ./nat-test --listen --name NAS
//!
//!   # Node B (connector - run on MacBook):
//!   ./nat-test --connect <NODE_A_ID> --name MacBook --pings 20
//!
//! The listener waits for incoming connections, responds to pings,
//! and logs path changes. The connector initiates connection, sends
//! pings, and monitors relay-to-direct upgrade (hole punch success).

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use iroh::endpoint::{Connection, PathInfoList};
use iroh::protocol::{AcceptError, Router};
use iroh::{Endpoint, EndpointAddr, EndpointId};
use n0_future::StreamExt;
use n0_watcher::Watcher;
use serde::Serialize;

/// ALPN for NAT test ping/pong protocol
const NAT_TEST_ALPN: &[u8] = b"tom-protocol/poc/nat-test/0";

#[derive(Parser, Debug)]
#[command(name = "nat-test", about = "ToM NAT traversal test — measures hole punching")]
struct Args {
    /// Display name for this node
    #[arg(short, long, default_value = "Node")]
    name: String,

    /// Run in listen mode (wait for incoming connections)
    #[arg(short, long)]
    listen: bool,

    /// Connect to this peer's EndpointId
    #[arg(short, long)]
    connect: Option<String>,

    /// Number of ping/pong rounds
    #[arg(short, long, default_value = "20")]
    pings: u32,

    /// Delay between pings in milliseconds
    #[arg(short, long, default_value = "2000")]
    delay: u64,
}

// --- JSON event types ---

#[derive(Serialize)]
struct EventStarted {
    event: &'static str,
    name: String,
    id: String,
    mode: String,
    timestamp: String,
}

#[derive(Serialize)]
struct EventPathChange {
    event: &'static str,
    selected: String,
    rtt_ms: f64,
    paths: Vec<String>,
    elapsed_s: f64,
}

#[derive(Serialize)]
struct EventPing {
    event: &'static str,
    seq: u32,
    rtt_ms: f64,
    via: String,
    elapsed_s: f64,
}

#[derive(Serialize)]
struct EventHolePunch {
    event: &'static str,
    success: bool,
    time_to_direct_s: f64,
    relay_rtt_ms: f64,
    direct_rtt_ms: f64,
}

#[derive(Serialize)]
struct EventSummary {
    event: &'static str,
    name: String,
    total_pings: u32,
    successful_pings: u32,
    direct_pings: u32,
    relay_pings: u32,
    direct_pct: f64,
    avg_rtt_direct_ms: f64,
    avg_rtt_relay_ms: f64,
    hole_punch_success: bool,
    time_to_direct_s: f64,
}

fn emit<T: Serialize>(event: &T) {
    if let Ok(json) = serde_json::to_string(event) {
        println!("{json}");
    }
}

fn now_iso() -> String {
    // Simple ISO timestamp without chrono dependency
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}s", d.as_secs())
}

/// Determine the currently selected path type from a PathInfoList
fn selected_path_type(paths: &PathInfoList) -> (String, f64) {
    for path in paths.iter() {
        if path.is_selected() {
            let ptype = if path.is_relay() {
                "RELAY"
            } else if path.is_ip() {
                "DIRECT"
            } else {
                "UNKNOWN"
            };
            return (ptype.to_string(), path.rtt().as_secs_f64() * 1000.0);
        }
    }
    ("NONE".to_string(), 0.0)
}

/// Format all paths as strings for logging
fn format_paths(paths: &PathInfoList) -> Vec<String> {
    paths
        .iter()
        .map(|p| {
            let kind = if p.is_relay() {
                "relay"
            } else if p.is_ip() {
                "ip"
            } else {
                "?"
            };
            let sel = if p.is_selected() { "*" } else { "" };
            format!("{kind}:{:?}{sel}", p.remote_addr())
        })
        .collect()
}

// --- Protocol handler (listener side) ---

#[derive(Debug, Clone)]
struct PingHandler {
    start: Instant,
}

impl iroh::protocol::ProtocolHandler for PingHandler {
    async fn accept(
        &self,
        connection: Connection,
    ) -> Result<(), AcceptError> {
        let remote = connection.remote_id();
        let elapsed = self.start.elapsed().as_secs_f64();
        eprintln!("[{elapsed:.1}s] Accepted connection from {}", &remote.to_string()[..12]);

        // Spawn path watcher for this connection
        let paths_watcher = connection.paths();
        let start = self.start;
        tokio::spawn(async move {
            let mut stream = paths_watcher.stream();
            let mut last_type = String::new();
            while let Some(paths) = stream.next().await {
                let (ptype, rtt) = selected_path_type(&paths);
                if ptype != last_type {
                    emit(&EventPathChange {
                        event: "path_change",
                        selected: ptype.clone(),
                        rtt_ms: rtt,
                        paths: format_paths(&paths),
                        elapsed_s: start.elapsed().as_secs_f64(),
                    });
                    last_type = ptype;
                }
            }
        });

        // Respond to pings
        loop {
            let bi = connection.accept_bi().await;
            match bi {
                Ok((mut send, mut recv)) => {
                    let data = recv
                        .read_to_end(1024)
                        .await
                        .map_err(AcceptError::from_err)?;

                    // Echo back (pong)
                    send.write_all(&data)
                        .await
                        .map_err(AcceptError::from_err)?;
                    send.finish().map_err(AcceptError::from_err)?;
                }
                Err(_) => break, // Connection closed
            }
        }

        Ok(())
    }
}

// --- Connector logic ---

async fn run_connector(
    endpoint: &Endpoint,
    target_id: EndpointId,
    name: &str,
    num_pings: u32,
    delay_ms: u64,
    start: Instant,
) -> Result<()> {
    eprintln!("[{:.1}s] Connecting to {}...", start.elapsed().as_secs_f64(), &target_id.to_string()[..12]);

    let addr = EndpointAddr::new(target_id);
    let connect_start = Instant::now();
    let connection = endpoint.connect(addr, NAT_TEST_ALPN).await?;
    let connect_time = connect_start.elapsed();

    eprintln!(
        "[{:.1}s] Connected in {:.0}ms",
        start.elapsed().as_secs_f64(),
        connect_time.as_secs_f64() * 1000.0,
    );

    // Track hole punch state
    let mut first_direct_time: Option<Duration> = None;
    let mut first_relay_rtt: Option<f64> = None;
    let mut direct_rtts: Vec<f64> = Vec::new();
    let mut relay_rtts: Vec<f64> = Vec::new();
    let mut successful_pings = 0u32;
    // Spawn path watcher
    let paths_watcher = connection.paths();
    let (path_tx, path_rx) = tokio::sync::watch::channel(String::from("UNKNOWN"));
    let start_clone = start;
    tokio::spawn(async move {
        let mut stream = paths_watcher.stream();
        let mut last_type = String::new();
        while let Some(paths) = stream.next().await {
            let (ptype, rtt) = selected_path_type(&paths);
            if ptype != last_type {
                emit(&EventPathChange {
                    event: "path_change",
                    selected: ptype.clone(),
                    rtt_ms: rtt,
                    paths: format_paths(&paths),
                    elapsed_s: start_clone.elapsed().as_secs_f64(),
                });
                let _ = path_tx.send(ptype.clone());
                last_type = ptype;
            }
        }
    });

    // Ping loop
    for seq in 1..=num_pings {
        let ping_start = Instant::now();

        // Open bi-stream, send ping, read pong
        match connection.open_bi().await {
            Ok((mut send, mut recv)) => {
                let payload = format!("PING|{seq}|{name}");
                send.write_all(payload.as_bytes()).await?;
                send.finish()?;

                match recv.read_to_end(1024).await {
                    Ok(_) => {
                        let rtt = ping_start.elapsed().as_secs_f64() * 1000.0;
                        successful_pings += 1;

                        let via = path_rx.borrow().clone();

                        emit(&EventPing {
                            event: "ping",
                            seq,
                            rtt_ms: rtt,
                            via: via.clone(),
                            elapsed_s: start.elapsed().as_secs_f64(),
                        });

                        match via.as_str() {
                            "DIRECT" => {
                                direct_rtts.push(rtt);
                                if first_direct_time.is_none() {
                                    first_direct_time = Some(start.elapsed());
                                    emit(&EventHolePunch {
                                        event: "hole_punch",
                                        success: true,
                                        time_to_direct_s: start.elapsed().as_secs_f64(),
                                        relay_rtt_ms: first_relay_rtt.unwrap_or(0.0),
                                        direct_rtt_ms: rtt,
                                    });
                                }
                            }
                            "RELAY" => {
                                relay_rtts.push(rtt);
                                if first_relay_rtt.is_none() {
                                    first_relay_rtt = Some(rtt);
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        eprintln!("[ping {seq}] read error: {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("[ping {seq}] open_bi error: {e}");
                break;
            }
        }

        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    // Summary
    let total = num_pings;
    let direct_count = direct_rtts.len() as u32;
    let relay_count = relay_rtts.len() as u32;
    let direct_pct = if successful_pings > 0 {
        (direct_count as f64 / successful_pings as f64) * 100.0
    } else {
        0.0
    };
    let avg_direct = if direct_rtts.is_empty() {
        0.0
    } else {
        direct_rtts.iter().sum::<f64>() / direct_rtts.len() as f64
    };
    let avg_relay = if relay_rtts.is_empty() {
        0.0
    } else {
        relay_rtts.iter().sum::<f64>() / relay_rtts.len() as f64
    };

    emit(&EventSummary {
        event: "summary",
        name: name.to_string(),
        total_pings: total,
        successful_pings,
        direct_pings: direct_count,
        relay_pings: relay_count,
        direct_pct,
        avg_rtt_direct_ms: avg_direct,
        avg_rtt_relay_ms: avg_relay,
        hole_punch_success: first_direct_time.is_some(),
        time_to_direct_s: first_direct_time
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0),
    });

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let start = Instant::now();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Create endpoint
    let endpoint = Endpoint::bind().await?;
    let my_id = endpoint.id();

    let mode = if args.listen { "listen" } else { "connect" };

    emit(&EventStarted {
        event: "started",
        name: args.name.clone(),
        id: my_id.to_string(),
        mode: mode.to_string(),
        timestamp: now_iso(),
    });

    eprintln!("=== ToM NAT Test ({}) ===", args.name);
    eprintln!("ID: {my_id}");
    eprintln!("Mode: {mode}");
    eprintln!();

    if args.listen {
        // --- Listener mode ---
        let handler = PingHandler { start };

        let _router = Router::builder(endpoint.clone())
            .accept(NAT_TEST_ALPN.to_vec(), Arc::new(handler))
            .spawn();

        eprintln!("Listening... Share this ID with the connector:");
        eprintln!("{my_id}");
        eprintln!();
        eprintln!("Press Ctrl+C to stop.");

        tokio::signal::ctrl_c().await?;
        eprintln!("\nShutting down.");
    } else if let Some(ref peer_str) = args.connect {
        // --- Connector mode ---
        let target_id: EndpointId = peer_str.parse().context("Invalid peer ID")?;

        // We don't need a Router for outgoing connections, but iroh needs
        // to accept connections for hole punching coordination.
        // Use a dummy router to keep the endpoint alive.
        let _router = Router::builder(endpoint.clone()).spawn();

        run_connector(
            &endpoint,
            target_id,
            &args.name,
            args.pings,
            args.delay,
            start,
        )
        .await?;
    } else {
        anyhow::bail!("Specify --listen or --connect <PEER_ID>");
    }

    Ok(())
}
