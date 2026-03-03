/// Protocol runtime metrics — lightweight counters and gauges.
///
/// All fields are atomic — safe to read from any thread without locking.
/// Updated by the runtime loop; read by the application via RuntimeHandle.
use std::sync::Arc;
use tom_metrics::{Counter, Gauge};

/// Snapshot of all protocol metrics at a point in time.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_failed: u64,
    pub groups_count: u64,
    pub peers_known: u64,
    pub uptime_seconds: u64,
}

/// Shared, clonable metrics handle.
///
/// Internally uses `Arc` so the runtime and app can both hold references.
#[derive(Clone)]
pub struct ProtocolMetrics {
    inner: Arc<Inner>,
}

struct Inner {
    messages_sent: Counter,
    messages_received: Counter,
    messages_failed: Counter,
    groups_count: Gauge,
    peers_known: Gauge,
    start_time: std::time::Instant,
}

impl ProtocolMetrics {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                messages_sent: Counter::new(),
                messages_received: Counter::new(),
                messages_failed: Counter::new(),
                groups_count: Gauge::new(),
                peers_known: Gauge::new(),
                start_time: std::time::Instant::now(),
            }),
        }
    }

    // ── Increment methods (called by runtime) ───────────────────────

    pub fn inc_messages_sent(&self) {
        self.inner.messages_sent.inc();
    }

    pub fn inc_messages_received(&self) {
        self.inner.messages_received.inc();
    }

    pub fn inc_messages_failed(&self) {
        self.inner.messages_failed.inc();
    }

    pub fn set_groups_count(&self, n: u64) {
        self.inner.groups_count.set(n);
    }

    pub fn set_peers_known(&self, n: u64) {
        self.inner.peers_known.set(n);
    }

    // ── Read method (called by app via RuntimeHandle) ────────────────

    /// Take a consistent snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            messages_sent: self.inner.messages_sent.get(),
            messages_received: self.inner.messages_received.get(),
            messages_failed: self.inner.messages_failed.get(),
            groups_count: self.inner.groups_count.get(),
            peers_known: self.inner.peers_known.get(),
            uptime_seconds: self.inner.start_time.elapsed().as_secs(),
        }
    }
}

impl Default for ProtocolMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ProtocolMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProtocolMetrics")
            .field("snapshot", &self.snapshot())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_increment_and_snapshot() {
        let m = ProtocolMetrics::new();
        m.inc_messages_sent();
        m.inc_messages_sent();
        m.inc_messages_received();
        m.inc_messages_failed();
        m.set_groups_count(3);
        m.set_peers_known(7);

        let snap = m.snapshot();
        assert_eq!(snap.messages_sent, 2);
        assert_eq!(snap.messages_received, 1);
        assert_eq!(snap.messages_failed, 1);
        assert_eq!(snap.groups_count, 3);
        assert_eq!(snap.peers_known, 7);
    }

    #[test]
    fn metrics_clone_shares_state() {
        let m = ProtocolMetrics::new();
        let m2 = m.clone();
        m.inc_messages_sent();
        assert_eq!(m2.snapshot().messages_sent, 1);
    }

    #[test]
    fn metrics_snapshot_serializes() {
        let m = ProtocolMetrics::new();
        m.inc_messages_sent();
        let json = serde_json::to_string(&m.snapshot()).unwrap();
        assert!(json.contains("\"messages_sent\":1"));
    }
}
