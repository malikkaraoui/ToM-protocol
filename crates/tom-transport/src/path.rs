use crate::NodeId;
use std::time::{Duration, Instant};

/// The kind of network path to a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathKind {
    /// Traffic goes through a relay server.
    Relay,
    /// Direct UDP connection (hole-punched).
    Direct,
    /// Path type not yet determined.
    Unknown,
}

impl std::fmt::Display for PathKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathKind::Relay => write!(f, "RELAY"),
            PathKind::Direct => write!(f, "DIRECT"),
            PathKind::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Famille d'adresse d'un chemin. Sert à filtrer le bruit de re-sélection de
/// port à chemin constant sans perdre le signal v4↔v6↔relais (R14 Lot A).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddrFamily {
    None,
    Relay,
    V4,
    V6,
}

impl AddrFamily {
    /// Dérive la famille depuis la représentation affichable produite par
    /// `format_transport_addr` ("relay:…", "[2a01:…]:port" IPv6, sinon IPv4).
    pub fn of(addr: &str) -> Self {
        if addr.is_empty() {
            AddrFamily::None
        } else if addr.starts_with("relay:") {
            AddrFamily::Relay
        } else if addr.starts_with('[') {
            AddrFamily::V6
        } else {
            AddrFamily::V4
        }
    }
}

impl std::fmt::Display for AddrFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddrFamily::None => write!(f, "none"),
            AddrFamily::Relay => write!(f, "relay"),
            AddrFamily::V4 => write!(f, "v4"),
            AddrFamily::V6 => write!(f, "v6"),
        }
    }
}

/// A path change event for a connected peer.
#[derive(Debug, Clone)]
pub struct PathEvent {
    /// Current path kind (relay or direct).
    pub kind: PathKind,
    /// Round-trip time on this path.
    pub rtt: Duration,
    /// The remote peer.
    pub remote: NodeId,
    /// When this event occurred.
    pub timestamp: Instant,
    /// Remote address of the selected path (e.g., "192.168.0.82:61240", "relay:http://...").
    pub addr: String,
    /// Famille du chemin courant (dérivée de `addr`).
    pub family: AddrFamily,
    /// Famille du chemin PRÉCÉDENT de cette connexion — `Some` ⟺ cet
    /// événement est une bascule observée par le watcher (pas un premier
    /// chemin). Par-connexion : non pollué par les connexions multiples
    /// d'un même pair.
    pub prev_family: Option<AddrFamily>,
    /// RTT du chemin précédent au moment de la bascule.
    pub prev_rtt: Option<Duration>,
}
