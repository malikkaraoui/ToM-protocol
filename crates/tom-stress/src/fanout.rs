use crate::common::{elapsed_s, generate_payload};
use crate::events::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tom_transport::{MessageEnvelope, NodeId, TomNode};

pub struct FanoutConfig {
    pub targets: Vec<NodeId>,
    pub count: u32,
    pub payload_size: usize,
    pub name: String,
}

pub async fn run(
    mut node: TomNode,
    config: FanoutConfig,
    start: Instant,
) -> anyhow::Result<()> {
    let my_id = node.id();
    let target_count = config.targets.len() as u32;

    emit(&EventStarted::new(&config.name, &my_id.to_string(), "fanout"));
    eprintln!(
        "Fanout mode → {} targets, {} msgs each, {} byte payload",
        target_count, config.count, config.payload_size
    );

    let mut pending: HashMap<String, Instant> = HashMap::new();
    let mut rtts: Vec<f64> = Vec::new();
    let mut send_failures = 0u32;
    let fanout_start = Instant::now();

    // Send phase: for each round, send to all targets
    for seq in 1..=config.count {
        let payload = generate_payload(config.payload_size, seq);

        // Send to all targets (send takes &self, so we can do this sequentially
        // without ownership issues — could also use join_all for concurrency)
        for &target in &config.targets {
            let envelope = MessageEnvelope::new(my_id, target, "stress-burst", payload.clone());
            let msg_id = envelope.id.clone();

            match node.send(target, &envelope).await {
                Ok(()) => {
                    pending.insert(msg_id, Instant::now());
                }
                Err(e) => {
                    send_failures += 1;
                    if send_failures <= 5 {
                        eprintln!("  send to {target} seq {seq} failed: {e}");
                    }
                }
            }
        }
    }

    let total_sent = (config.count * target_count).saturating_sub(send_failures);
    let send_elapsed = fanout_start.elapsed();
    eprintln!(
        "  sent {total_sent} envelopes in {:.1}ms",
        send_elapsed.as_secs_f64() * 1000.0
    );

    // Collect phase (5s idle timeout)
    let collect_timeout = Duration::from_secs(5);
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
                eprintln!("  recv error: {e}");
                break;
            }
            Err(_) => break,
        }
    }

    let total_delivered = rtts.len() as u32;
    let total_failed = total_sent.saturating_sub(total_delivered);
    let total_elapsed = fanout_start.elapsed();

    emit(&EventFanoutResult {
        event: "fanout_result",
        target_count,
        envelopes_per_target: config.count,
        total_sent,
        total_delivered,
        total_failed,
        avg_rtt_ms: if rtts.is_empty() {
            0.0
        } else {
            rtts.iter().sum::<f64>() / rtts.len() as f64
        },
        max_rtt_ms: rtts.iter().copied().fold(0.0f64, f64::max),
        elapsed_ms: total_elapsed.as_secs_f64() * 1000.0,
        elapsed_s: elapsed_s(start),
    });

    eprintln!(
        "\n  fanout: {total_delivered}/{total_sent} delivered, {total_failed} lost, avg RTT {:.1}ms",
        if rtts.is_empty() { 0.0 } else { rtts.iter().sum::<f64>() / rtts.len() as f64 }
    );

    node.shutdown().await?;
    Ok(())
}
