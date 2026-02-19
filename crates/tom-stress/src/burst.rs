use crate::common::{elapsed_s, generate_payload};
use crate::events::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tom_transport::{MessageEnvelope, NodeId, TomNode};

pub struct BurstConfig {
    pub target: NodeId,
    pub count: u32,
    pub payload_size: usize,
    pub rounds: u32,
    pub round_delay_ms: u64,
    pub name: String,
}

pub async fn run(
    mut node: TomNode,
    config: BurstConfig,
    start: Instant,
) -> anyhow::Result<()> {
    let my_id = node.id();

    emit(&EventStarted::new(&config.name, &my_id.to_string(), "burst"));
    eprintln!(
        "Burst mode → target: {}, {} msgs x {} bytes, {} rounds",
        config.target, config.count, config.payload_size, config.rounds
    );

    for round in 1..=config.rounds {
        eprintln!("\n  Round {round}/{} ...", config.rounds);
        run_single_burst(&mut node, &config, round, my_id, start).await?;

        if round < config.rounds && config.round_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(config.round_delay_ms)).await;
        }
    }

    node.shutdown().await?;
    Ok(())
}

async fn run_single_burst(
    node: &mut TomNode,
    config: &BurstConfig,
    round: u32,
    my_id: tom_transport::NodeId,
    start: Instant,
) -> anyhow::Result<()> {
    let mut pending: HashMap<String, Instant> = HashMap::new();
    let mut rtts: Vec<f64> = Vec::new();
    let mut send_failures = 0u32;
    let burst_start = Instant::now();

    // Phase 1: Send all envelopes
    for seq in 1..=config.count {
        let payload = generate_payload(config.payload_size, seq);
        let envelope = MessageEnvelope::new(my_id, config.target, "stress-burst", payload);
        let msg_id = envelope.id.clone();

        match node.send(config.target, &envelope).await {
            Ok(()) => {
                pending.insert(msg_id, Instant::now());
            }
            Err(e) => {
                send_failures += 1;
                if seq <= 3 || send_failures <= 3 {
                    eprintln!("    send #{seq} failed: {e}");
                }
            }
        }
    }

    let send_elapsed = burst_start.elapsed();
    let messages_sent = config.count - send_failures;
    eprintln!(
        "    sent {messages_sent}/{} in {:.1}ms",
        config.count,
        send_elapsed.as_secs_f64() * 1000.0
    );

    // Phase 2: Collect responses (30s idle timeout — 4G relay can be slow)
    let collect_timeout = Duration::from_secs(30);
    let mut last_recv = Instant::now();

    loop {
        let remaining = collect_timeout.saturating_sub(last_recv.elapsed());
        if remaining.is_zero() || pending.is_empty() {
            break;
        }

        match tokio::time::timeout(remaining, node.recv()).await {
            Ok(Ok((_from, envelope))) => {
                last_recv = Instant::now();
                if envelope.msg_type == "stress-pong" {
                    if let Some(echo_id) =
                        envelope.payload.get("echo_id").and_then(|v| v.as_str())
                    {
                        if let Some(send_time) = pending.remove(echo_id) {
                            rtts.push(send_time.elapsed().as_secs_f64() * 1000.0);
                        }
                    }
                }
            }
            Ok(Err(e)) => {
                eprintln!("    recv error during collect: {e}");
                break;
            }
            Err(_) => break, // idle timeout
        }
    }

    let total_elapsed = burst_start.elapsed();
    let messages_acked = rtts.len() as u32;
    let lost = messages_sent.saturating_sub(messages_acked);
    let elapsed_ms = total_elapsed.as_secs_f64() * 1000.0;
    let messages_per_sec = if elapsed_ms > 0.0 {
        messages_acked as f64 / (elapsed_ms / 1000.0)
    } else {
        0.0
    };
    let total_bytes = messages_acked as u64 * config.payload_size as u64;
    let bytes_per_sec = if elapsed_ms > 0.0 {
        total_bytes as f64 / (elapsed_ms / 1000.0)
    } else {
        0.0
    };

    emit(&EventBurstResult {
        event: "burst_result",
        round,
        messages_sent,
        messages_acked,
        lost,
        payload_size: config.payload_size,
        total_bytes,
        elapsed_ms,
        messages_per_sec,
        bytes_per_sec,
        rtt_min_ms: rtts.iter().copied().fold(f64::INFINITY, f64::min),
        rtt_max_ms: rtts.iter().copied().fold(0.0f64, f64::max),
        rtt_avg_ms: if rtts.is_empty() {
            0.0
        } else {
            rtts.iter().sum::<f64>() / rtts.len() as f64
        },
        elapsed_s: elapsed_s(start),
    });

    eprintln!(
        "    burst round {round}: {messages_acked}/{messages_sent} acked, {lost} lost, {messages_per_sec:.0} msg/s, avg RTT {:.1}ms",
        if rtts.is_empty() { 0.0 } else { rtts.iter().sum::<f64>() / rtts.len() as f64 }
    );

    // If we sent messages but got zero pongs, the connection is likely
    // a zombie. Evict so the next round triggers fresh discovery.
    if messages_sent > 0 && messages_acked == 0 {
        node.disconnect(config.target).await;
        eprintln!("    evicted zombie connection, will reconnect on next round");
    }

    Ok(())
}
