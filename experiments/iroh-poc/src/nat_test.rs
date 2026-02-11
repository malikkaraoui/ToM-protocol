//! ToM Protocol - iroh PoC: NAT Traversal Test (v2)
//!
//! Instrumented binary for testing hole punching across real networks.
//! Outputs structured JSON events for analysis.
//!
//! New in v2: --continuous mode (infinite pings, rolling summaries)
//! and automatic reconnection on connection loss.
//!
//! Usage:
//!   # Node A (listener - run on remote machine / NAS / VPS):
//!   ./nat-test --listen --name NAS
//!
//!   # Node B (connector - fixed ping count):
//!   ./nat-test --connect <NODE_A_ID> --name MacBook --pings 20
//!
//!   # Node B (connector - continuous for in-motion testing):
//!   ./nat-test --connect <NODE_A_ID> --name MacBook --continuous
//!
//! The listener waits for incoming connections, responds to pings,
//! and logs path changes. The connector initiates connection, sends
//! pings, and monitors relay-to-direct upgrade (hole punch success).
//! In continuous mode, reconnects automatically on connection loss.

use std::sync::atomic::{AtomicBool, Ordering};
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

    /// Number of ping/pong rounds (ignored in --continuous mode)
    #[arg(short, long, default_value = "20")]
    pings: u32,

    /// Delay between pings in milliseconds
    #[arg(short, long, default_value = "2000")]
    delay: u64,

    /// Continuous mode: ping forever until Ctrl+C (for in-motion testing)
    #[arg(long)]
    continuous: bool,

    /// Emit rolling summary every N pings (continuous mode)
    #[arg(long, default_value = "50")]
    summary_interval: u32,

    /// Max reconnection attempts (0 = unlimited in continuous mode)
    #[arg(long, default_value = "10")]
    max_reconnects: u32,
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
struct EventDisconnected {
    event: &'static str,
    reason: String,
    session_pings: u32,
    elapsed_s: f64,
    timestamp: String,
}

#[derive(Serialize)]
struct EventReconnecting {
    event: &'static str,
    attempt: u32,
    elapsed_s: f64,
    timestamp: String,
}

#[derive(Serialize)]
struct EventReconnected {
    event: &'static str,
    attempt: u32,
    reconnect_time_ms: f64,
    elapsed_s: f64,
    timestamp: String,
}

#[derive(Serialize)]
struct EventSummary {
    event: &'static str,
    name: String,
    total_pings: u32,
    successful_pings: u32,
    failed_pings: u32,
    direct_pings: u32,
    relay_pings: u32,
    direct_pct: f64,
    avg_rtt_direct_ms: f64,
    avg_rtt_relay_ms: f64,
    hole_punch_success: bool,
    time_to_direct_s: f64,
    reconnections: u32,
    total_disconnected_s: f64,
    elapsed_s: f64,
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

// --- Connector state (shared across reconnections) ---

struct ConnectorState {
    first_direct_time: Option<Duration>,
    first_relay_rtt: Option<f64>,
    direct_rtts: Vec<f64>,
    relay_rtts: Vec<f64>,
    successful_pings: u32,
    failed_pings: u32,
    seq: u32,
    reconnections: u32,
    total_disconnected: Duration,
}

impl ConnectorState {
    fn new() -> Self {
        Self {
            first_direct_time: None,
            first_relay_rtt: None,
            direct_rtts: Vec::new(),
            relay_rtts: Vec::new(),
            successful_pings: 0,
            failed_pings: 0,
            seq: 0,
            reconnections: 0,
            total_disconnected: Duration::ZERO,
        }
    }

    fn emit_summary(&self, name: &str, start: Instant) {
        let direct_count = self.direct_rtts.len() as u32;
        let relay_count = self.relay_rtts.len() as u32;
        let direct_pct = if self.successful_pings > 0 {
            (direct_count as f64 / self.successful_pings as f64) * 100.0
        } else {
            0.0
        };
        let avg_direct = if self.direct_rtts.is_empty() {
            0.0
        } else {
            self.direct_rtts.iter().sum::<f64>() / self.direct_rtts.len() as f64
        };
        let avg_relay = if self.relay_rtts.is_empty() {
            0.0
        } else {
            self.relay_rtts.iter().sum::<f64>() / self.relay_rtts.len() as f64
        };

        emit(&EventSummary {
            event: "summary",
            name: name.to_string(),
            total_pings: self.seq,
            successful_pings: self.successful_pings,
            failed_pings: self.failed_pings,
            direct_pings: direct_count,
            relay_pings: relay_count,
            direct_pct,
            avg_rtt_direct_ms: avg_direct,
            avg_rtt_relay_ms: avg_relay,
            hole_punch_success: self.first_direct_time.is_some(),
            time_to_direct_s: self.first_direct_time
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0),
            reconnections: self.reconnections,
            total_disconnected_s: self.total_disconnected.as_secs_f64(),
            elapsed_s: start.elapsed().as_secs_f64(),
        });
    }
}

// --- Connector logic ---

/// Spawn a path watcher for a connection, returns a watch receiver for the current path type
fn spawn_path_watcher(
    connection: &Connection,
    start: Instant,
) -> tokio::sync::watch::Receiver<String> {
    let paths_watcher = connection.paths();
    let (path_tx, path_rx) = tokio::sync::watch::channel(String::from("UNKNOWN"));
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
                let _ = path_tx.send(ptype.clone());
                last_type = ptype;
            }
        }
    });
    path_rx
}

/// Try to establish a connection, with retry logic
async fn connect_with_retry(
    endpoint: &Endpoint,
    target_id: EndpointId,
    max_attempts: u32,
    start: Instant,
) -> Result<Connection> {
    let addr = EndpointAddr::new(target_id);
    let connect_start = Instant::now();

    // First attempt — no retry delay
    match endpoint.connect(addr.clone(), NAT_TEST_ALPN).await {
        Ok(conn) => {
            let ms = connect_start.elapsed().as_secs_f64() * 1000.0;
            eprintln!("[{:.1}s] Connected in {:.0}ms", start.elapsed().as_secs_f64(), ms);
            return Ok(conn);
        }
        Err(e) => {
            if max_attempts <= 1 {
                return Err(e.into());
            }
            eprintln!("[{:.1}s] Initial connect failed: {e}", start.elapsed().as_secs_f64());
        }
    }

    // Retry with exponential backoff
    let limit = if max_attempts == 0 { u32::MAX } else { max_attempts };
    for attempt in 2..=limit {
        let backoff = Duration::from_millis(1000 * (1 << (attempt - 2).min(5)));
        emit(&EventReconnecting {
            event: "reconnecting",
            attempt,
            elapsed_s: start.elapsed().as_secs_f64(),
            timestamp: now_iso(),
        });
        eprintln!("[{:.1}s] Reconnecting (attempt {attempt})...", start.elapsed().as_secs_f64());
        tokio::time::sleep(backoff).await;

        match endpoint.connect(addr.clone(), NAT_TEST_ALPN).await {
            Ok(conn) => return Ok(conn),
            Err(e) => {
                eprintln!("[{:.1}s] Attempt {attempt} failed: {e}", start.elapsed().as_secs_f64());
            }
        }
    }

    anyhow::bail!("Failed to connect after {limit} attempts")
}

/// Send a single ping over a connection, return Ok(rtt_ms) or Err on failure
async fn send_ping(
    connection: &Connection,
    seq: u32,
    name: &str,
) -> Result<f64, String> {
    let ping_start = Instant::now();

    let (mut send, mut recv) = connection
        .open_bi()
        .await
        .map_err(|e| format!("open_bi: {e}"))?;

    let payload = format!("PING|{seq}|{name}");
    send.write_all(payload.as_bytes())
        .await
        .map_err(|e| format!("write: {e}"))?;
    send.finish().map_err(|e| format!("finish: {e}"))?;

    recv.read_to_end(1024)
        .await
        .map_err(|e| format!("read: {e}"))?;

    Ok(ping_start.elapsed().as_secs_f64() * 1000.0)
}

#[allow(clippy::too_many_arguments)]
async fn run_connector(
    endpoint: &Endpoint,
    target_id: EndpointId,
    name: &str,
    num_pings: u32,
    delay_ms: u64,
    continuous: bool,
    summary_interval: u32,
    max_reconnects: u32,
    running: Arc<AtomicBool>,
    start: Instant,
) -> Result<()> {
    eprintln!("[{:.1}s] Connecting to {}...", start.elapsed().as_secs_f64(), &target_id.to_string()[..12]);
    if continuous {
        eprintln!("Mode: CONTINUOUS (Ctrl+C to stop)");
    }

    let mut state = ConnectorState::new();

    // Initial connection
    let mut connection = connect_with_retry(endpoint, target_id, 3, start).await?;
    let mut path_rx = spawn_path_watcher(&connection, start);

    // Main ping loop
    while running.load(Ordering::Relaxed) {
        // Check if we've reached the ping limit (non-continuous mode)
        if !continuous && state.seq >= num_pings {
            break;
        }

        state.seq += 1;
        let seq = state.seq;

        match send_ping(&connection, seq, name).await {
            Ok(rtt) => {
                state.successful_pings += 1;
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
                        state.direct_rtts.push(rtt);
                        if state.first_direct_time.is_none() {
                            state.first_direct_time = Some(start.elapsed());
                            emit(&EventHolePunch {
                                event: "hole_punch",
                                success: true,
                                time_to_direct_s: start.elapsed().as_secs_f64(),
                                relay_rtt_ms: state.first_relay_rtt.unwrap_or(0.0),
                                direct_rtt_ms: rtt,
                            });
                        }
                    }
                    "RELAY" => {
                        state.relay_rtts.push(rtt);
                        if state.first_relay_rtt.is_none() {
                            state.first_relay_rtt = Some(rtt);
                        }
                    }
                    _ => {}
                }

                // Rolling summary in continuous mode
                if continuous && summary_interval > 0 && seq.is_multiple_of(summary_interval) {
                    state.emit_summary(name, start);
                }
            }
            Err(e) => {
                state.failed_pings += 1;
                eprintln!("[{:.1}s] Ping {seq} failed: {e}", start.elapsed().as_secs_f64());

                // Emit disconnection event
                emit(&EventDisconnected {
                    event: "disconnected",
                    reason: e.to_string(),
                    session_pings: state.successful_pings,
                    elapsed_s: start.elapsed().as_secs_f64(),
                    timestamp: now_iso(),
                });

                if !continuous && !running.load(Ordering::Relaxed) {
                    break;
                }

                // Reconnection
                let max = if continuous && max_reconnects == 0 {
                    0 // unlimited
                } else if continuous {
                    max_reconnects
                } else {
                    3 // limited mode: only 3 retries
                };

                let reconn_start = Instant::now();
                match connect_with_retry(endpoint, target_id, max, start).await {
                    Ok(new_conn) => {
                        let reconn_time = reconn_start.elapsed();
                        state.reconnections += 1;
                        state.total_disconnected += reconn_time;

                        emit(&EventReconnected {
                            event: "reconnected",
                            attempt: state.reconnections,
                            reconnect_time_ms: reconn_time.as_secs_f64() * 1000.0,
                            elapsed_s: start.elapsed().as_secs_f64(),
                            timestamp: now_iso(),
                        });

                        connection = new_conn;
                        path_rx = spawn_path_watcher(&connection, start);
                    }
                    Err(e) => {
                        eprintln!("[{:.1}s] Reconnection failed: {e}", start.elapsed().as_secs_f64());
                        state.total_disconnected += reconn_start.elapsed();
                        break;
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    // Final summary
    state.emit_summary(name, start);
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

    let mode = if args.listen {
        "listen"
    } else if args.continuous {
        "connect-continuous"
    } else {
        "connect"
    };

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
            .accept(NAT_TEST_ALPN, Arc::new(handler))
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

        // Graceful shutdown flag — Ctrl+C sets this to false
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let name_clone = args.name.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            eprintln!("\n[{}] Ctrl+C received, finishing...", name_clone);
            running_clone.store(false, Ordering::Relaxed);
        });

        run_connector(
            &endpoint,
            target_id,
            &args.name,
            args.pings,
            args.delay,
            args.continuous,
            args.summary_interval,
            args.max_reconnects,
            running,
            start,
        )
        .await?;
    } else {
        anyhow::bail!("Specify --listen or --connect <PEER_ID>");
    }

    Ok(())
}
