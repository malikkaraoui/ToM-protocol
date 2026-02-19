use crate::common::{elapsed_s, setup_ctrlc, spawn_path_monitor};
use crate::events::*;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tom_transport::{MessageEnvelope, NodeId, TomNode};

pub struct PingConfig {
    pub target: NodeId,
    pub count: u32,
    pub delay_ms: u64,
    pub continuous: bool,
    pub summary_interval: u32,
    pub name: String,
}

struct State {
    seq: u32,
    successful: u32,
    failed: u32,
    rtts: Vec<f64>,
    reconnections: u32,
    total_disconnected: Duration,
    consecutive_timeouts: u32,
}

impl State {
    fn new() -> Self {
        Self {
            seq: 0,
            successful: 0,
            failed: 0,
            rtts: Vec::new(),
            reconnections: 0,
            total_disconnected: Duration::ZERO,
            consecutive_timeouts: 0,
        }
    }
}

pub async fn run(
    mut node: TomNode,
    config: PingConfig,
    start: Instant,
) -> anyhow::Result<()> {
    let my_id = node.id();

    emit(&EventStarted::new(&config.name, &my_id.to_string(), "ping"));
    eprintln!("Ping mode → target: {}", config.target);

    let running = setup_ctrlc();
    spawn_path_monitor(&node, start);

    let mut state = State::new();
    let ping_timeout = Duration::from_secs(10);

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }
        if !config.continuous && state.seq >= config.count {
            break;
        }

        state.seq += 1;
        let seq = state.seq;

        // Build ping envelope
        let envelope = MessageEnvelope::new(
            my_id,
            config.target,
            "stress-ping",
            serde_json::json!({
                "seq": seq,
                "name": config.name,
                "ts": now_ms(),
            }),
        );
        let msg_id = envelope.id.clone();
        // Send
        match node.send(config.target, &envelope).await {
            Ok(()) => {}
            Err(e) => {
                state.failed += 1;
                let reason = format!("{e}");
                emit(&EventDisconnected {
                    event: "disconnected",
                    reason: reason.clone(),
                    elapsed_s: elapsed_s(start),
                    timestamp: now_iso(),
                });
                eprintln!("  ping #{seq} send failed: {reason}");

                // Try to reconnect with backoff
                if try_reconnect(&node, config.target, &mut state, start, &running, config.continuous).await {
                    continue; // retry this seq
                } else {
                    break; // gave up
                }
            }
        }

        // Wait for pong
        match wait_for_pong(&mut node, &msg_id, ping_timeout).await {
            Ok(rtt) => {
                state.successful += 1;
                state.consecutive_timeouts = 0;
                let rtt_ms = rtt.as_secs_f64() * 1000.0;
                state.rtts.push(rtt_ms);
                emit(&EventPing {
                    event: "ping",
                    seq,
                    rtt_ms,
                    path: String::new(), // filled by path_change events
                    elapsed_s: elapsed_s(start),
                });
            }
            Err(e) => {
                state.failed += 1;
                state.consecutive_timeouts += 1;
                eprintln!("  ping #{seq} recv failed: {e}");

                // After 3 consecutive timeouts, the connection is likely
                // a zombie (alive at QUIC level but network path is dead).
                // Evict it so the next send() triggers fresh discovery.
                if state.consecutive_timeouts >= 3 {
                    node.disconnect(config.target).await;
                    state.consecutive_timeouts = 0;
                    eprintln!("  evicted zombie connection, will reconnect on next send");
                }
            }
        }

        // Rolling summary
        if config.summary_interval > 0
            && state.seq.is_multiple_of(config.summary_interval)
            && state.seq > 0
        {
            emit_summary(&state, &config.name, start);
        }

        // Delay
        if config.delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(config.delay_ms)).await;
        }
    }

    // Final summary
    emit_summary(&state, &config.name, start);
    eprintln!(
        "\nDone: {}/{} pings OK, avg RTT: {:.1}ms",
        state.successful,
        state.seq,
        avg(&state.rtts),
    );

    node.shutdown().await?;
    Ok(())
}

/// Wait for a stress-pong matching `expected_id`.
async fn wait_for_pong(
    node: &mut TomNode,
    expected_id: &str,
    timeout: Duration,
) -> anyhow::Result<Duration> {
    let start = Instant::now();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let result = tokio::time::timeout_at(deadline, node.recv()).await;

        match result {
            Ok(Ok((_from, envelope))) => {
                if envelope.msg_type == "stress-pong" {
                    if let Some(echo_id) = envelope.payload.get("echo_id").and_then(|v| v.as_str())
                    {
                        if echo_id == expected_id {
                            return Ok(start.elapsed());
                        }
                    }
                }
                // Not our pong, keep waiting
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err(anyhow::anyhow!("pong timeout ({timeout:?})")),
        }
    }
}

/// Try reconnecting with exponential backoff.
/// In continuous mode, retries indefinitely. Otherwise caps at 10 attempts.
async fn try_reconnect(
    node: &TomNode,
    target: NodeId,
    state: &mut State,
    start: Instant,
    running: &std::sync::atomic::AtomicBool,
    continuous: bool,
) -> bool {
    let disconnect_start = Instant::now();
    let mut attempt = 0u32;

    loop {
        attempt += 1;

        if !continuous && attempt > 10 {
            break;
        }

        if !running.load(Ordering::Relaxed) {
            return false;
        }

        emit(&EventReconnecting {
            event: "reconnecting",
            attempt,
            elapsed_s: elapsed_s(start),
            timestamp: now_iso(),
        });

        // Backoff: 1s, 2s, 4s, 8s, 16s, 32s (capped at 32s)
        let backoff = Duration::from_millis(1000 * 2u64.pow(attempt.min(5) - 1));
        tokio::time::sleep(backoff).await;

        // Force-evict every 5 failed attempts to trigger fresh discovery
        if attempt.is_multiple_of(5) {
            node.disconnect(target).await;
        }

        // Try a probe send
        let probe = MessageEnvelope::new(
            node.id(),
            target,
            "stress-ping",
            serde_json::json!({"seq": 0, "name": "probe", "ts": now_ms()}),
        );
        match node.send(target, &probe).await {
            Ok(()) => {
                let reconnect_time = disconnect_start.elapsed();
                state.reconnections += 1;
                state.total_disconnected += reconnect_time;

                emit(&EventReconnected {
                    event: "reconnected",
                    attempt,
                    reconnect_time_ms: reconnect_time.as_secs_f64() * 1000.0,
                    elapsed_s: elapsed_s(start),
                    timestamp: now_iso(),
                });
                eprintln!("  reconnected after {attempt} attempts ({:.0}ms)", reconnect_time.as_secs_f64() * 1000.0);
                return true;
            }
            Err(_) => continue,
        }
    }

    eprintln!("  gave up after 10 reconnect attempts");
    false
}

fn emit_summary(state: &State, name: &str, start: Instant) {
    emit(&EventSummary {
        event: "summary",
        name: name.to_string(),
        mode: "ping".to_string(),
        total_pings: state.seq,
        successful: state.successful,
        failed: state.failed,
        direct_pings: 0,   // tracked via path events
        relay_pings: 0,    // tracked via path events
        direct_pct: 0.0,   // tracked via path events
        avg_rtt_ms: avg(&state.rtts),
        reconnections: state.reconnections,
        elapsed_s: elapsed_s(start),
    });
}

fn avg(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}
