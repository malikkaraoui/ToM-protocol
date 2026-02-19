use crate::common::{elapsed_s, generate_payload};
use crate::events::*;
use std::time::{Duration, Instant};
use tom_transport::{MessageEnvelope, NodeId, TomNode};

pub struct LadderConfig {
    pub target: NodeId,
    pub sizes: Vec<usize>,
    pub reps: u32,
    pub delay_ms: u64,
    pub name: String,
}

/// Default size ladder (geometric progression up to max_message_size).
pub fn default_sizes(max: usize) -> Vec<usize> {
    let candidates = [
        1_024,     //   1 KB
        4_096,     //   4 KB
        16_384,    //  16 KB
        65_536,    //  64 KB
        131_072,   // 128 KB
        262_144,   // 256 KB
        524_288,   // 512 KB
        1_048_576, //   1 MB
    ];
    candidates.iter().copied().filter(|&s| s <= max).collect()
}

pub async fn run(
    mut node: TomNode,
    config: LadderConfig,
    start: Instant,
) -> anyhow::Result<()> {
    let my_id = node.id();

    emit(&EventStarted::new(&config.name, &my_id.to_string(), "ladder"));
    eprintln!(
        "Ladder mode → target: {}, {} sizes, {} reps each",
        config.target,
        config.sizes.len(),
        config.reps
    );

    let ping_timeout = Duration::from_secs(15);

    for (step_idx, &size) in config.sizes.iter().enumerate() {
        let step = (step_idx + 1) as u32;
        let mut rtts: Vec<f64> = Vec::new();
        let mut failures = 0u32;

        eprintln!("\n  Step {step}: {} bytes ({} reps)", size, config.reps);

        for rep in 1..=config.reps {
            let payload = generate_payload(size, rep);
            let envelope =
                MessageEnvelope::new(my_id, config.target, "stress-ladder", payload);
            let msg_id = envelope.id.clone();
            // Send
            if let Err(e) = node.send(config.target, &envelope).await {
                failures += 1;
                eprintln!("    rep {rep}: send failed: {e}");
                continue;
            }

            // Wait for pong
            match wait_for_pong(&mut node, &msg_id, ping_timeout).await {
                Ok(rtt) => {
                    let rtt_ms = rtt.as_secs_f64() * 1000.0;
                    rtts.push(rtt_ms);
                }
                Err(e) => {
                    failures += 1;
                    eprintln!("    rep {rep}: {e}");
                }
            }
        }

        emit(&EventLadderResult {
            event: "ladder_result",
            step,
            size,
            reps: config.reps,
            successful: rtts.len() as u32,
            failed: failures,
            rtt_min_ms: rtts.iter().copied().fold(f64::INFINITY, f64::min),
            rtt_max_ms: rtts.iter().copied().fold(0.0f64, f64::max),
            rtt_avg_ms: if rtts.is_empty() {
                0.0
            } else {
                rtts.iter().sum::<f64>() / rtts.len() as f64
            },
            elapsed_s: elapsed_s(start),
        });

        let avg = if rtts.is_empty() {
            0.0
        } else {
            rtts.iter().sum::<f64>() / rtts.len() as f64
        };
        eprintln!(
            "    → {}/{} OK, avg RTT: {avg:.1}ms",
            rtts.len(),
            config.reps
        );

        // If every rep failed, the connection is likely a zombie.
        // Evict so the next step triggers fresh discovery.
        if rtts.is_empty() && config.reps > 0 {
            node.disconnect(config.target).await;
            eprintln!("    evicted zombie connection, will reconnect on next step");
        }

        // Delay between steps
        if config.delay_ms > 0 && step_idx + 1 < config.sizes.len() {
            tokio::time::sleep(Duration::from_millis(config.delay_ms)).await;
        }
    }

    node.shutdown().await?;
    Ok(())
}

/// Wait for a stress-pong matching `expected_id`.
async fn wait_for_pong(
    node: &mut TomNode,
    expected_id: &str,
    timeout: Duration,
) -> anyhow::Result<Duration> {
    let pong_start = Instant::now();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        match tokio::time::timeout_at(deadline, node.recv()).await {
            Ok(Ok((_from, envelope))) => {
                if envelope.msg_type == "stress-pong" {
                    if let Some(echo_id) =
                        envelope.payload.get("echo_id").and_then(|v| v.as_str())
                    {
                        if echo_id == expected_id {
                            return Ok(pong_start.elapsed());
                        }
                    }
                }
            }
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err(anyhow::anyhow!("pong timeout ({timeout:?})")),
        }
    }
}
