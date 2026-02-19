use crate::events::{emit, EventPathChange};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tom_transport::{NodeId, PathKind, TomNode};

/// Setup Ctrl+C handler, returns a flag that goes false on signal.
pub fn setup_ctrlc() -> Arc<AtomicBool> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        eprintln!("\nCtrl+C received, shutting down...");
        r.store(false, Ordering::Relaxed);
    });
    running
}

/// Spawn a background task that emits JSONL on path changes.
pub fn spawn_path_monitor(node: &TomNode, start: Instant) {
    let mut rx = node.path_events();
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let kind_str = match event.kind {
                PathKind::Direct => "DIRECT",
                PathKind::Relay => "RELAY",
                PathKind::Unknown => "UNKNOWN",
            };
            emit(&EventPathChange {
                event: "path_change",
                kind: kind_str.to_string(),
                rtt_ms: event.rtt.as_secs_f64() * 1000.0,
                remote: event.remote.to_string(),
                elapsed_s: start.elapsed().as_secs_f64(),
            });
        }
    });
}

/// Parse a NodeId from a hex string.
pub fn parse_node_id(s: &str) -> anyhow::Result<NodeId> {
    s.parse::<NodeId>()
        .map_err(|e| anyhow::anyhow!("invalid NodeId '{s}': {e}"))
}

/// Generate a JSON payload of approximately `size` bytes.
pub fn generate_payload(size: usize, seq: u32) -> serde_json::Value {
    // Overhead for JSON structure: {"seq":N,"data":"..."}
    let overhead = 30;
    let fill = if size > overhead {
        "X".repeat(size - overhead)
    } else {
        String::new()
    };
    serde_json::json!({
        "seq": seq,
        "data": fill,
    })
}

/// Elapsed seconds since `start`.
pub fn elapsed_s(start: Instant) -> f64 {
    start.elapsed().as_secs_f64()
}
