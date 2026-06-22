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

    /// Revert to active discovery when the node loses all live connections.
    ///
    /// Bootstrap must NOT be a one-way latch: if a converged node drops back to
    /// zero peers (e.g. its only relay/peer disappeared), it has to re-enter the
    /// discovery sequence and keep actively seeking a rendezvous instead of
    /// freezing in a stale `Converged` state. Returns `true` if a transition
    /// happened (so the caller can trigger a fresh discovery round + log it).
    pub(crate) fn on_isolated(&mut self) -> bool {
        if *self == BootstrapPhase::Converged {
            *self = BootstrapPhase::RelayAssist;
            true
        } else {
            false
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── BootstrapPhase — état initial ────────────────────────────────────

    #[test]
    fn initial_phase_is_lan_probe() {
        let phase = BootstrapPhase::LanProbe;
        assert_eq!(phase, BootstrapPhase::LanProbe);
    }

    #[test]
    fn on_hint_accepted_from_lan_probe_converges() {
        let mut phase = BootstrapPhase::LanProbe;
        phase.on_hint_accepted();
        assert_eq!(phase, BootstrapPhase::Converged);
    }

    #[test]
    fn on_hint_accepted_from_relay_assist_converges() {
        let mut phase = BootstrapPhase::RelayAssist;
        phase.on_hint_accepted();
        assert_eq!(phase, BootstrapPhase::Converged);
    }

    #[test]
    fn on_hint_accepted_from_dht_assist_converges() {
        let mut phase = BootstrapPhase::DhtAssist;
        phase.on_hint_accepted();
        assert_eq!(phase, BootstrapPhase::Converged);
    }

    #[test]
    fn on_hint_accepted_from_converged_stays_converged() {
        let mut phase = BootstrapPhase::Converged;
        phase.on_hint_accepted();
        assert_eq!(phase, BootstrapPhase::Converged);
    }

    // ── BootstrapPhase — retour en amorçage sur isolement ────────────────

    #[test]
    fn on_isolated_from_converged_reverts_to_relay_assist() {
        let mut phase = BootstrapPhase::Converged;
        let changed = phase.on_isolated();
        assert!(changed, "une transition doit avoir eu lieu");
        assert_eq!(phase, BootstrapPhase::RelayAssist);
        assert_eq!(phase.to_string(), "amorcage");
    }

    #[test]
    fn on_isolated_when_not_converged_is_noop() {
        for start in [
            BootstrapPhase::LanProbe,
            BootstrapPhase::RelayAssist,
            BootstrapPhase::DhtAssist,
        ] {
            let mut phase = start;
            let changed = phase.on_isolated();
            assert!(!changed, "pas de transition hors Converged");
            assert_eq!(phase, start);
        }
    }

    #[test]
    fn converge_then_isolate_then_reconverge() {
        let mut phase = BootstrapPhase::LanProbe;
        phase.on_hint_accepted();
        assert_eq!(phase, BootstrapPhase::Converged);
        assert!(phase.on_isolated());
        assert_eq!(phase, BootstrapPhase::RelayAssist); // replonge en amorçage
        phase.on_hint_accepted();
        assert_eq!(phase, BootstrapPhase::Converged); // reconverge sur nouveau hint
    }

    // ── BootstrapPhase — Display ─────────────────────────────────────────

    #[test]
    fn display_lan_probe() {
        assert_eq!(BootstrapPhase::LanProbe.to_string(), "amorcage");
    }

    #[test]
    fn display_relay_assist() {
        assert_eq!(BootstrapPhase::RelayAssist.to_string(), "amorcage");
    }

    #[test]
    fn display_dht_assist() {
        assert_eq!(BootstrapPhase::DhtAssist.to_string(), "amorcage");
    }

    #[test]
    fn display_converged() {
        assert_eq!(BootstrapPhase::Converged.to_string(), "connecte");
    }

    // ── BootstrapPhase — Clone / Copy / PartialEq ───────────────────────

    #[test]
    fn phase_clone_is_independent() {
        let phase = BootstrapPhase::LanProbe;
        let mut cloned = phase;          // Copy — no reference kept
        cloned.on_hint_accepted();
        assert_eq!(phase, BootstrapPhase::LanProbe);  // original unchanged
        assert_eq!(cloned, BootstrapPhase::Converged);
    }

    #[test]
    fn phase_equality() {
        assert_eq!(BootstrapPhase::LanProbe, BootstrapPhase::LanProbe);
        assert_ne!(BootstrapPhase::LanProbe, BootstrapPhase::Converged);
        assert_ne!(BootstrapPhase::RelayAssist, BootstrapPhase::DhtAssist);
    }

    // ── BootstrapPhase — Serialize ───────────────────────────────────────

    #[test]
    fn phase_serializes_to_json() {
        let p = BootstrapPhase::Converged;
        let json = serde_json::to_string(&p).expect("serialize");
        assert_eq!(json, "\"Converged\"");
    }

    #[test]
    fn all_phases_serialize_without_panic() {
        for p in [
            BootstrapPhase::LanProbe,
            BootstrapPhase::RelayAssist,
            BootstrapPhase::DhtAssist,
            BootstrapPhase::Converged,
        ] {
            serde_json::to_string(&p).expect("serialize");
        }
    }

    // ── BootstrapSource — Display ────────────────────────────────────────

    #[test]
    fn source_display_mdns() {
        assert_eq!(BootstrapSource::Mdns.to_string(), "mDNS");
    }

    #[test]
    fn source_display_peer_present() {
        assert_eq!(BootstrapSource::PeerPresent.to_string(), "PeerPresent");
    }

    #[test]
    fn source_display_dht() {
        assert_eq!(BootstrapSource::Dht.to_string(), "DHT");
    }

    #[test]
    fn source_display_manual() {
        assert_eq!(BootstrapSource::Manual.to_string(), "Manual");
    }

    // ── BootstrapSource — PartialEq / Clone ─────────────────────────────

    #[test]
    fn source_equality() {
        assert_eq!(BootstrapSource::Mdns, BootstrapSource::Mdns);
        assert_ne!(BootstrapSource::Mdns, BootstrapSource::Dht);
        assert_ne!(BootstrapSource::PeerPresent, BootstrapSource::Manual);
    }

    #[test]
    fn source_clone() {
        let s = BootstrapSource::Dht;
        let c = s;  // Copy
        assert_eq!(s, c);
    }

    // ── BootstrapSource — Serialize ──────────────────────────────────────

    #[test]
    fn source_serializes_to_json() {
        let s = BootstrapSource::Mdns;
        let json = serde_json::to_string(&s).expect("serialize");
        assert_eq!(json, "\"Mdns\"");
    }

    #[test]
    fn all_sources_serialize_without_panic() {
        for s in [
            BootstrapSource::Mdns,
            BootstrapSource::PeerPresent,
            BootstrapSource::Dht,
            BootstrapSource::Manual,
        ] {
            serde_json::to_string(&s).expect("serialize");
        }
    }

    // ── Séquence typique de bootstrap ───────────────────────────────────

    #[test]
    fn typical_bootstrap_sequence() {
        // Démarrage → LanProbe → hint reçu → Converged
        let mut phase = BootstrapPhase::LanProbe;
        assert_eq!(phase.to_string(), "amorcage");

        phase.on_hint_accepted();
        assert_eq!(phase, BootstrapPhase::Converged);
        assert_eq!(phase.to_string(), "connecte");
    }

    #[test]
    fn relay_fallback_sequence() {
        // LAN probe timeout → RelayAssist → hint via PeerPresent → Converged
        // (node starts directly in RelayAssist after LAN probe timeout)
        let mut phase = BootstrapPhase::RelayAssist;
        assert_eq!(phase.to_string(), "amorcage");

        phase.on_hint_accepted();
        assert_eq!(phase, BootstrapPhase::Converged);
    }

    #[test]
    fn dht_last_resort_sequence() {
        let mut phase = BootstrapPhase::DhtAssist;
        assert_eq!(phase.to_string(), "amorcage");
        phase.on_hint_accepted();
        assert_eq!(phase, BootstrapPhase::Converged);
        assert_eq!(phase.to_string(), "connecte");
    }
}
