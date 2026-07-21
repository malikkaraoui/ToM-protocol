/// Protocol runtime metrics — lightweight counters and gauges.
///
/// All fields are atomic — safe to read from any thread without locking.
/// Updated by the runtime loop; read by the application via RuntimeHandle.
use std::sync::Arc;
use tom_metrics::{Counter, Gauge};

use super::bootstrap::BootstrapPhase;
use crate::relay::PeerRole;

/// Snapshot of all protocol metrics at a point in time.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_failed: u64,
    pub messages_dropped: u64,
    pub groups_count: u64,
    pub peers_known: u64,
    pub uptime_seconds: u64,
    /// Network phase as perceived by this node.
    pub phase: BootstrapPhase,
    /// Number of active peers this node currently sees.
    pub taille_reseau: u64,
    /// Number of peers with relay role in the topology.
    pub relayeurs_connus: u64,
    /// Local node role (Peer or Relay).
    pub role_local: PeerRole,
    /// [DIAG OOM] Clients relais actuellement connectés au serveur embarqué
    /// (accepts − disconnects, compteurs appariés). 0 si pas de self-relay.
    pub clients_relais_actifs: u64,
    /// [DIAG OOM] Total cumulé de connexions acceptées par le relais embarqué.
    pub relais_accepts_total: u64,
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
    messages_dropped: Counter,
    groups_count: Gauge,
    peers_known: Gauge,
    start_time: std::time::Instant,
    /// 0=LanProbe, 1=RelayAssist, 2=DhtAssist, 3=Converged
    phase: std::sync::atomic::AtomicU8,
    taille_reseau: Gauge,
    relayeurs_connus: Gauge,
    /// 0=Peer, 1=Relay
    role_local: std::sync::atomic::AtomicU8,
    /// [DIAG OOM] Clients relais actifs (accepts − disconnects du relais embarqué).
    clients_relais_actifs: Gauge,
    /// [DIAG OOM] Accepts cumulés du relais embarqué.
    relais_accepts_total: Gauge,
}

impl ProtocolMetrics {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                messages_sent: Counter::new(),
                messages_received: Counter::new(),
                messages_failed: Counter::new(),
                messages_dropped: Counter::new(),
                groups_count: Gauge::new(),
                peers_known: Gauge::new(),
                start_time: std::time::Instant::now(),
                phase: std::sync::atomic::AtomicU8::new(0),
                taille_reseau: Gauge::new(),
                relayeurs_connus: Gauge::new(),
                role_local: std::sync::atomic::AtomicU8::new(0),
                clients_relais_actifs: Gauge::new(),
                relais_accepts_total: Gauge::new(),
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

    pub fn inc_messages_dropped(&self) {
        self.inner.messages_dropped.inc();
    }

    pub fn set_groups_count(&self, n: u64) {
        self.inner.groups_count.set(n);
    }

    pub fn set_peers_known(&self, n: u64) {
        self.inner.peers_known.set(n);
    }

    pub fn set_phase(&self, phase: BootstrapPhase) {
        let v = match phase {
            BootstrapPhase::LanProbe => 0,
            BootstrapPhase::RelayAssist => 1,
            BootstrapPhase::DhtAssist => 2,
            BootstrapPhase::Converged => 3,
        };
        self.inner.phase.store(v, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_taille_reseau(&self, n: u64) {
        self.inner.taille_reseau.set(n);
    }

    pub fn set_relayeurs_connus(&self, n: u64) {
        self.inner.relayeurs_connus.set(n);
    }

    pub fn set_role_local(&self, role: PeerRole) {
        let v = match role {
            PeerRole::Peer => 0,
            PeerRole::Relay => 1,
        };
        self.inner.role_local.store(v, std::sync::atomic::Ordering::Relaxed);
    }

    /// [DIAG OOM] Met à jour les stats du relais embarqué (clients actifs + accepts cumulés).
    pub fn set_relais_stats(&self, actifs: u64, accepts_total: u64) {
        self.inner.clients_relais_actifs.set(actifs);
        self.inner.relais_accepts_total.set(accepts_total);
    }

    // ── Read method (called by app via RuntimeHandle) ────────────────

    /// Take a consistent snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let phase_val = self.inner.phase.load(std::sync::atomic::Ordering::Relaxed);
        let phase = match phase_val {
            1 => BootstrapPhase::RelayAssist,
            2 => BootstrapPhase::DhtAssist,
            3 => BootstrapPhase::Converged,
            _ => BootstrapPhase::LanProbe,
        };
        let role_val = self.inner.role_local.load(std::sync::atomic::Ordering::Relaxed);
        let role_local = if role_val == 1 { PeerRole::Relay } else { PeerRole::Peer };

        MetricsSnapshot {
            messages_sent: self.inner.messages_sent.get(),
            messages_received: self.inner.messages_received.get(),
            messages_failed: self.inner.messages_failed.get(),
            messages_dropped: self.inner.messages_dropped.get(),
            groups_count: self.inner.groups_count.get(),
            peers_known: self.inner.peers_known.get(),
            uptime_seconds: self.inner.start_time.elapsed().as_secs(),
            phase,
            taille_reseau: self.inner.taille_reseau.get(),
            relayeurs_connus: self.inner.relayeurs_connus.get(),
            role_local,
            clients_relais_actifs: self.inner.clients_relais_actifs.get(),
            relais_accepts_total: self.inner.relais_accepts_total.get(),
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
        assert_eq!(snap.messages_dropped, 0);
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

    #[test]
    fn metrics_network_state() {
        let m = ProtocolMetrics::new();
        let snap = m.snapshot();
        assert_eq!(snap.phase, BootstrapPhase::LanProbe);
        assert_eq!(snap.taille_reseau, 0);
        assert_eq!(snap.relayeurs_connus, 0);
        assert_eq!(snap.role_local, PeerRole::Peer);

        m.set_phase(BootstrapPhase::Converged);
        m.set_taille_reseau(5);
        m.set_relayeurs_connus(2);
        m.set_role_local(PeerRole::Relay);

        let snap = m.snapshot();
        assert_eq!(snap.phase, BootstrapPhase::Converged);
        assert_eq!(snap.taille_reseau, 5);
        assert_eq!(snap.relayeurs_connus, 2);
        assert_eq!(snap.role_local, PeerRole::Relay);

        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"phase\""));
        assert!(json.contains("\"taille_reseau\":5"));
    }
}
