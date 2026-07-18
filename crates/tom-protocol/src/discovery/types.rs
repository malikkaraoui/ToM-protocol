/// Discovery types for ToM protocol.
///
/// Application-level peer metadata broadcast over the protocol layer.
/// iroh handles low-level address resolution; this module handles
/// what a node announces about itself (username, roles, capabilities).
use serde::{Deserialize, Serialize};

use crate::relay::PeerRole;
use crate::types::{now_ms, NodeId};

// ── Constants ────────────────────────────────────────────────────────────

/// Heartbeat interval (5 seconds).
pub const HEARTBEAT_INTERVAL_MS: u64 = 5_000;

/// Stale threshold — peer becomes stale after missing 2 gossip announces (20s).
pub const STALE_THRESHOLD_MS: u64 = 20_000;

/// Offline threshold — peer becomes offline after missing ~4 gossip announces (45s).
pub const OFFLINE_THRESHOLD_MS: u64 = 45_000;

/// Maximum allowed clock drift for timestamps (5 minutes).
pub const MAX_FUTURE_DRIFT_MS: u64 = 5 * 60 * 1000;

/// Gossip announce interval (10 seconds — acts as keepalive).
pub const GOSSIP_INTERVAL_MS: u64 = 10_000;

/// Max peers returned in a single gossip response.
pub const MAX_PEERS_PER_GOSSIP: usize = 20;

/// Longueur maximale d'un username reçu du réseau (octets UTF-8).
/// Aligné sur `tom_dht::DhtNodeAddr` (32 octets) : les deux canaux de
/// découverte imposent la même borne.
pub const MAX_USERNAME_BYTES: usize = 32;

/// Assainit un username reçu du réseau AVANT tout usage (affichage, log JSON
/// collecteur, stockage heartbeat). Retire les caractères de contrôle (`\n`,
/// `\r`, …) et tronque à `MAX_USERNAME_BYTES` sur une frontière de caractère.
///
/// Brèche fermée (review sécurité 18/07) : la voie DHT sanitisait déjà
/// (`tom_dht::sanitize_username`), mais les voies GOSSIP et QUIC directe
/// acceptaient un username brut — un pair malveillant pouvait envoyer un
/// username géant (bloat mémoire) ou avec des sauts de ligne (casse le JSON
/// du collecteur `:9999`, injection dans l'UptimeLog Swift). Miroir de la
/// logique DHT pour un comportement identique sur les deux canaux.
pub fn sanitize_username(raw: &str) -> String {
    let mut result = String::new();
    for ch in raw.chars().filter(|c| !c.is_control()) {
        if result.len() + ch.len_utf8() > MAX_USERNAME_BYTES {
            break;
        }
        result.push(ch);
    }
    result
}

// ── PeerAnnounce ─────────────────────────────────────────────────────────

/// Payload for PeerAnnounce messages — what a node broadcasts about itself.
///
/// Serialized into `Envelope.payload` with `MessageType::PeerAnnounce`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerAnnounce {
    /// The announcing node's identity.
    pub node_id: NodeId,
    /// Human-readable display name.
    pub username: String,
    /// Application build number (non-authoritative hint, 0 = unknown).
    #[serde(default)]
    pub app_build: u32,
    /// Roles this node can serve.
    pub roles: Vec<PeerRole>,
    /// Ed25519 public key for E2E encryption (32 bytes).
    pub encryption_key: Option<[u8; 32]>,
    /// Announcement timestamp (Unix ms).
    pub timestamp: u64,
}

impl PeerAnnounce {
    /// Create a new peer announcement.
    pub fn new(node_id: NodeId, username: String, app_build: u32, roles: Vec<PeerRole>) -> Self {
        Self {
            node_id,
            username,
            app_build,
            roles,
            encryption_key: Some(node_id.as_bytes()),
            timestamp: now_ms(),
        }
    }

    /// Whether this announcement is within acceptable clock drift.
    pub fn is_timestamp_valid(&self, now: u64) -> bool {
        // Not too far in the future
        if self.timestamp > now + MAX_FUTURE_DRIFT_MS {
            return false;
        }
        // Not absurdly old (1 hour)
        if now > self.timestamp && now - self.timestamp > 60 * 60 * 1000 {
            return false;
        }
        true
    }
}

// ── DiscoveryEvent ───────────────────────────────────────────────────────

/// Events emitted by the discovery system.
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// A new peer was discovered.
    PeerDiscovered {
        node_id: NodeId,
        username: String,
        app_build: u32,
        source: DiscoverySource,
    },

    /// A peer went stale (missed heartbeats but might recover).
    PeerStale {
        node_id: NodeId,
    },

    /// A peer went offline (confirmed departed).
    PeerOffline {
        node_id: NodeId,
    },

    /// A peer came back online.
    PeerOnline {
        node_id: NodeId,
    },
}

/// How we learned about a peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscoverySource {
    /// Direct connection / bootstrap.
    Direct,
    /// Learned via gossip from another peer.
    Gossip,
    /// Peer announced itself.
    Announce,
    /// Discovered via DHT (BEP-0044) lookup.
    Dht,
    /// Discovered via mDNS (LAN multicast).
    Mdns,
    /// Discovered via relay PeerPresent frame.
    PeerPresent,
}

// ── LivenessState ────────────────────────────────────────────────────────

/// Current liveness state of a tracked peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivenessState {
    /// Actively sending heartbeats.
    Alive,
    /// Missed heartbeats, might recover.
    Stale,
    /// Confirmed departed.
    Departed,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn sanitize_username_strips_control_and_truncates() {
        // Nominal : inchangé.
        assert_eq!(sanitize_username("alice"), "alice");
        // Sauts de ligne / retour chariot / tab → retirés (anti-injection JSON/log).
        assert_eq!(sanitize_username("ali\nce\r\t"), "alice");
        // Troncature à 32 octets sur frontière de caractère.
        let long = "a".repeat(100);
        assert_eq!(sanitize_username(&long).len(), 32);
        // UTF-8 multi-octets : jamais coupé au milieu d'un caractère.
        let emoji = "🦀".repeat(20); // 4 octets chacun → 8 tiennent (32 octets)
        let s = sanitize_username(&emoji);
        assert!(s.len() <= 32 && s.is_char_boundary(s.len()));
        assert_eq!(s.chars().count(), 8);
        // Username entièrement de contrôle → vide.
        assert_eq!(sanitize_username("\n\r\t"), "");
    }

    #[test]
    fn peer_announce_new() {
        let id = node_id(1);
        let announce = PeerAnnounce::new(id, "alice".into(), 0, vec![PeerRole::Peer]);

        assert_eq!(announce.node_id, id);
        assert_eq!(announce.username, "alice");
        assert_eq!(announce.app_build, 0);
        assert_eq!(announce.roles, vec![PeerRole::Peer]);
        assert!(announce.encryption_key.is_some());
        assert!(announce.timestamp > 0);
    }

    #[test]
    fn peer_announce_roundtrip() {
        let id = node_id(1);
        let announce = PeerAnnounce::new(id, "alice".into(), 42, vec![PeerRole::Relay]);

        let bytes = rmp_serde::to_vec(&announce).expect("serialize");
        let decoded: PeerAnnounce = rmp_serde::from_slice(&bytes).expect("deserialize");
        assert_eq!(announce, decoded);
    }

    #[test]
    fn timestamp_validation() {
        let id = node_id(1);
        let now = now_ms();

        let mut announce = PeerAnnounce::new(id, "alice".into(), 0, vec![]);
        announce.timestamp = now;
        assert!(announce.is_timestamp_valid(now));

        // Slightly in future — OK
        announce.timestamp = now + 1000;
        assert!(announce.is_timestamp_valid(now));

        // Too far in future — reject
        announce.timestamp = now + MAX_FUTURE_DRIFT_MS + 1;
        assert!(!announce.is_timestamp_valid(now));

        // Old but within 1 hour — OK
        announce.timestamp = now - 30 * 60 * 1000;
        assert!(announce.is_timestamp_valid(now));

        // Too old — reject
        announce.timestamp = now - 2 * 60 * 60 * 1000;
        assert!(!announce.is_timestamp_valid(now));
    }

    #[test]
    fn discovery_source_roundtrip() {
        for source in [DiscoverySource::Direct, DiscoverySource::Gossip, DiscoverySource::Announce, DiscoverySource::Dht, DiscoverySource::Mdns, DiscoverySource::PeerPresent] {
            let bytes = rmp_serde::to_vec(&source).expect("serialize");
            let decoded: DiscoverySource = rmp_serde::from_slice(&bytes).expect("deserialize");
            assert_eq!(source, decoded);
        }
    }
}
