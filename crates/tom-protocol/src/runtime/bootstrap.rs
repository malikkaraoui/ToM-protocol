/// Internal bootstrap phase — tracks where the node is in its startup discovery sequence.
///
/// Purely informational: used for logging and future telemetry. Does not gate
/// any protocol behaviour; the node remains active in every phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[allow(dead_code)] // RelayAssist / DhtAssist used in future phases
pub enum BootstrapPhase {
    /// No peer discovered yet; LAN probe in progress.
    LanProbe,
    /// LAN probe timed out without results; waiting on relay-assisted discovery.
    RelayAssist,
    /// Relay-assisted discovery also silent; DHT lookup in progress.
    DhtAssist,
    /// At least one peer joined the gossip mesh — bootstrap complete.
    Converged,
}

impl std::fmt::Display for BootstrapPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapPhase::LanProbe => write!(f, "amorcage"),
            BootstrapPhase::RelayAssist => write!(f, "amorcage"),
            BootstrapPhase::DhtAssist => write!(f, "amorcage"),
            BootstrapPhase::Converged => write!(f, "connecte"),
        }
    }
}

impl BootstrapPhase {
    /// Transition to `Converged` on first useful hint, regardless of current phase.
    pub(crate) fn on_hint_accepted(&mut self) {
        *self = BootstrapPhase::Converged;
    }
}

/// Source that produced a bootstrap hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[allow(dead_code)] // Manual used in future
pub enum BootstrapSource {
    Mdns,
    PeerPresent,
    Dht,
    Manual,
}

impl std::fmt::Display for BootstrapSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapSource::Mdns => write!(f, "mDNS"),
            BootstrapSource::PeerPresent => write!(f, "PeerPresent"),
            BootstrapSource::Dht => write!(f, "DHT"),
            BootstrapSource::Manual => write!(f, "Manual"),
        }
    }
}
