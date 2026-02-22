use crate::output;
use serde::Serialize;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

/// Emit a JSONL event to stdout (flushed immediately for piped output).
/// If --output-dir was provided, also writes to the JSONL file.
pub fn emit<T: Serialize>(event: &T) {
    if let Ok(json) = serde_json::to_string(event) {
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        let _ = writeln!(lock, "{json}");
        let _ = lock.flush();

        output::write_jsonl_line(&json);
    }
}

pub use tom_transport::now_ms;

/// ISO-ish timestamp string for JSONL events.
pub fn now_iso() -> String {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}s", d.as_secs())
}

// ── Session events ──────────────────────────────────────────────

#[derive(Serialize)]
pub struct EventStarted {
    pub event: &'static str,
    pub name: String,
    pub id: String,
    pub mode: String,
    pub timestamp: String,
}

impl EventStarted {
    pub fn new(name: &str, id: &str, mode: &str) -> Self {
        Self {
            event: "started",
            name: name.to_string(),
            id: id.to_string(),
            mode: mode.to_string(),
            timestamp: now_iso(),
        }
    }
}

// ── Ping events ─────────────────────────────────────────────────

#[derive(Serialize)]
pub struct EventPing {
    pub event: &'static str,
    pub seq: u32,
    pub rtt_ms: f64,
    pub path: String,
    pub elapsed_s: f64,
}

// ── Path events ─────────────────────────────────────────────────

#[derive(Serialize)]
pub struct EventPathChange {
    pub event: &'static str,
    pub kind: String,
    pub rtt_ms: f64,
    pub remote: String,
    pub elapsed_s: f64,
}

// ── Disconnection / reconnection ────────────────────────────────

#[derive(Serialize)]
pub struct EventDisconnected {
    pub event: &'static str,
    pub reason: String,
    pub elapsed_s: f64,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct EventReconnecting {
    pub event: &'static str,
    pub attempt: u32,
    pub elapsed_s: f64,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct EventReconnected {
    pub event: &'static str,
    pub attempt: u32,
    pub reconnect_time_ms: f64,
    pub elapsed_s: f64,
    pub timestamp: String,
}

// ── Burst result ────────────────────────────────────────────────

#[derive(Serialize)]
pub struct EventBurstResult {
    pub event: &'static str,
    pub round: u32,
    pub messages_sent: u32,
    pub messages_acked: u32,
    pub lost: u32,
    pub payload_size: usize,
    pub total_bytes: u64,
    pub elapsed_ms: f64,
    pub messages_per_sec: f64,
    pub bytes_per_sec: f64,
    pub rtt_min_ms: f64,
    pub rtt_max_ms: f64,
    pub rtt_avg_ms: f64,
    pub elapsed_s: f64,
}

// ── Ladder result ───────────────────────────────────────────────

#[derive(Serialize)]
pub struct EventLadderResult {
    pub event: &'static str,
    pub step: u32,
    pub size: usize,
    pub reps: u32,
    pub successful: u32,
    pub failed: u32,
    pub rtt_min_ms: f64,
    pub rtt_max_ms: f64,
    pub rtt_avg_ms: f64,
    pub elapsed_s: f64,
}

// ── Fanout result ───────────────────────────────────────────────

#[derive(Serialize)]
pub struct EventFanoutResult {
    pub event: &'static str,
    pub target_count: u32,
    pub envelopes_per_target: u32,
    pub total_sent: u32,
    pub total_delivered: u32,
    pub total_failed: u32,
    pub avg_rtt_ms: f64,
    pub max_rtt_ms: f64,
    pub elapsed_ms: f64,
    pub elapsed_s: f64,
}

// ── Summary ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct EventSummary {
    pub event: &'static str,
    pub name: String,
    pub mode: String,
    pub total_pings: u32,
    pub successful: u32,
    pub failed: u32,
    pub direct_pings: u32,
    pub relay_pings: u32,
    pub direct_pct: f64,
    pub avg_rtt_ms: f64,
    pub reconnections: u32,
    pub elapsed_s: f64,
}
