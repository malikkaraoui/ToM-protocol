use crate::common::{setup_ctrlc, spawn_path_monitor};
use crate::events::{emit, now_ms, EventStarted};
use std::sync::atomic::Ordering;
use std::time::Instant;
use tom_transport::{MessageEnvelope, TomNode};

pub async fn run(mut node: TomNode, name: &str, start: Instant) -> anyhow::Result<()> {
    let my_id = node.id();

    emit(&EventStarted::new(name, &my_id.to_string(), "listen"));
    eprintln!("Listening as {name} — ID: {my_id}");
    eprintln!("Press Ctrl+C to stop.\n");

    let running = setup_ctrlc();
    spawn_path_monitor(&node, start);

    let mut count: u64 = 0;
    while running.load(Ordering::Relaxed) {
        // Use a short timeout so we can check the running flag
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            node.recv(),
        )
        .await;

        let (from, envelope) = match result {
            Ok(Ok(msg)) => msg,
            Ok(Err(e)) => {
                eprintln!("recv error: {e}");
                break;
            }
            Err(_) => continue, // timeout, check running flag
        };

        count += 1;

        // Echo back with stress-pong
        match envelope.msg_type.as_str() {
            "stress-ping" | "stress-burst" | "stress-ladder" => {
                let response = MessageEnvelope::new(
                    my_id,
                    from,
                    "stress-pong",
                    serde_json::json!({
                        "echo_id": envelope.id,
                        "received_at": now_ms(),
                    }),
                );
                if let Err(e) = node.send(from, &response).await {
                    eprintln!("echo send to {from} failed: {e}");
                }
            }
            other => {
                eprintln!("unknown msg_type from {from}: {other}");
            }
        }

        if count.is_multiple_of(100) {
            eprintln!("  [{name}] echoed {count} messages (elapsed: {:.1}s)", start.elapsed().as_secs_f64());
        }
    }

    eprintln!("\n{name}: echoed {count} messages total.");
    node.shutdown().await?;
    Ok(())
}
