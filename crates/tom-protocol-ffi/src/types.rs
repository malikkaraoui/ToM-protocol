//! FFI-safe types for JSON serialization/deserialization

use base64::Engine;
use serde::{Deserialize, Serialize};
use tom_protocol::types::NodeId;

/// Serializable version of DeliveredMessage for FFI
/// Note: payload is base64-encoded for Swift Data compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveredMessageFFI {
    pub from: String,
    pub payload: String,
    pub envelope_id: String,
    pub timestamp: u64,
    pub signature_valid: bool,
    pub was_encrypted: bool,
}

impl From<tom_protocol::DeliveredMessage> for DeliveredMessageFFI {
    fn from(msg: tom_protocol::DeliveredMessage) -> Self {
        Self {
            from: msg.from.to_string(),
            payload: base64::engine::general_purpose::STANDARD.encode(&msg.payload),
            envelope_id: msg.envelope_id,
            timestamp: msg.timestamp,
            signature_valid: msg.signature_valid,
            was_encrypted: msg.was_encrypted,
        }
    }
}

/// Peer address for add_peer_addr FFI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAddrFFI {
    /// Node ID (hex string)
    pub node_id: String,

    /// Relay URL (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay_url: Option<String>,

    /// Direct socket addresses (optional, e.g. ["192.168.0.83:3340"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_addrs: Option<Vec<String>>,
}

/// Node configuration (transport layer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfigFFI {
    /// Custom relay URL (optional, overrides TOM_RELAY_URL env var)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay_url: Option<String>,

    /// Enable n0-computer address discovery (Pkarr/DNS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n0_discovery: Option<bool>,

    /// Path to persistent identity file (32-byte Ed25519 secret key)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_path: Option<String>,
}

/// Runtime configuration (protocol layer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfigFFI {
    /// Local username for group membership
    pub username: String,

    /// Enable E2E encryption for outbound messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<bool>,

    /// Enable DHT-based peer discovery
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_dht: Option<bool>,

    /// Custom relay URL (duplicated here for convenience)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relay_url: Option<String>,

    /// Path to persistent identity file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_path: Option<String>,

    /// Enable n0 discovery
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n0_discovery: Option<bool>,

    /// Enable local LAN discovery via mDNS bootstrap hints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_discovery: Option<bool>,

    /// Directory for persistent state (SQLite)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,

    /// Gossip bootstrap peers (hex NodeId strings)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gossip_bootstrap_peers: Vec<String>,

    /// Start an embedded relay server inside this node process (full-node mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_embedded_relay: Option<bool>,

    /// Publish this node's relay URL via gossip so peers can discover it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_embedded_relay_publication: Option<bool>,

    /// Inject gossip-discovered relay URLs into the QUIC transport layer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_transport_relay_discovery: Option<bool>,

    /// L1-001 presence: anti-Sybil gate threshold (local score required of an
    /// attester). None = protocol default (2.0). 0.0 = fleet plumbing mode
    /// (phase 1 of the L1-001 runbook) — accepts any well-formed signed
    /// attestation; structural defenses stay armed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_contribution_min: Option<f64>,

    /// L1-001 presence: auto-probe interval in seconds (challenges up to 8
    /// Online peers each tick, results land in the Live Log). None or 0 = off.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_probe_interval_secs: Option<u32>,
}

/// Group creation config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfigFFI {
    /// Group name
    pub name: String,

    /// Hub relay node ID (hex string)
    #[serde(deserialize_with = "deserialize_node_id")]
    pub hub_relay_id: NodeId,

    /// Initial members (hex strings)
    #[serde(deserialize_with = "deserialize_node_ids")]
    pub initial_members: Vec<NodeId>,

    /// Invite-only group
    pub invite_only: bool,
}

/// A peer discovered via gossip/DHT/direct announce
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeerFFI {
    pub node_id: String,
    pub username: String,
    pub source: String,
    pub discovered_at: u64,
}

/// Node status snapshot exposed over FFI.
///
/// The serialized field names are the wire contract decoded by the Swift
/// `TomNodeStatus` struct (`apps/tom-node-tvos/TomNode/Models/TomModels.swift`).
/// Renaming a field here silently breaks the tvOS status panel: the Swift
/// `JSONDecoder` returns `nil` and the UI stops updating. Going through serde
/// (instead of a hand-rolled `format!`) also guarantees valid JSON escaping for
/// every string field. Keep in sync and covered by the contract tests below.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeStatusFFI {
    pub node_id: String,
    pub status: String,
    pub peers_count: u64,
    pub groups_count: u64,
    pub local_role: String,
    pub path_kind: String,
    pub path_rtt_ms: u64,
    pub relay_url_active: String,
}

/// L1-001 presence stats snapshot polled by Swift (key contract with the
/// `TomPresenceStats` decoder — keep field names stable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceStatsFFI {
    /// Attestations accepted since node start (monotonic).
    pub accepted_total: u64,
    /// Short id of the most recent accepted attester ("" if none yet).
    pub last_attester: String,
    /// Round-trip of the most recent accepted attestation (challenger clock).
    pub last_latency_ms: u64,
    /// Attestations currently in the 30s aggregation window.
    pub window_count: u64,
    /// First 8 hex chars of the current entropy seed.
    pub seed_prefix: String,
}

/// Build 20 — full per-outcome presence counters (stress relevés). Mirrors
/// `tom_protocol::PresenceMetrics`; keep field names in sync with the Swift
/// `TomPresenceMetrics` decoder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceMetricsFFI {
    pub issued: u64,
    pub accepted: u64,
    pub drop_unknown_challenge: u64,
    pub drop_stale: u64,
    pub drop_wrong_attester: u64,
    pub drop_bad_signature: u64,
    pub drop_incoherent: u64,
    pub drop_gate: u64,
    pub drop_store_full: u64,
    pub challenges_received: u64,
    pub signed: u64,
    pub refused_bad_signature: u64,
    pub refused_incoherent: u64,
    pub refused_budget: u64,
    pub latency_min_ms: u64,
    pub latency_max_ms: u64,
    pub latency_mean_ms: u64,
}

impl From<tom_protocol::PresenceMetrics> for PresenceMetricsFFI {
    fn from(m: tom_protocol::PresenceMetrics) -> Self {
        Self {
            issued: m.issued,
            accepted: m.accepted,
            drop_unknown_challenge: m.drop_unknown_challenge,
            drop_stale: m.drop_stale,
            drop_wrong_attester: m.drop_wrong_attester,
            drop_bad_signature: m.drop_bad_signature,
            drop_incoherent: m.drop_incoherent,
            drop_gate: m.drop_gate,
            drop_store_full: m.drop_store_full,
            challenges_received: m.challenges_received,
            signed: m.signed,
            refused_bad_signature: m.refused_bad_signature,
            refused_incoherent: m.refused_incoherent,
            refused_budget: m.refused_budget,
            latency_min_ms: m.latency_min_ms,
            latency_max_ms: m.latency_max_ms,
            latency_mean_ms: m.mean_latency_ms(),
        }
    }
}

fn deserialize_node_id<'de, D>(deserializer: D) -> Result<NodeId, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

fn deserialize_node_ids<'de, D>(deserializer: D) -> Result<Vec<NodeId>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let strings: Vec<String> = Deserialize::deserialize(deserializer)?;
    strings
        .into_iter()
        .map(|s| s.parse().map_err(serde::de::Error::custom))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The serialized JSON below is the wire contract decoded by the Swift
    // `Codable` structs in `apps/tom-node-tvos/TomNode/Models/TomModels.swift`.
    // These tests lock the exact key set + order so a Rust-side rename can no
    // longer silently break the tvOS UI (decode failure → panel freezes).

    #[test]
    fn node_status_json_keys_match_swift_decoder() {
        let status = NodeStatusFFI {
            node_id: "n".into(),
            status: "Running".into(),
            peers_count: 3,
            groups_count: 1,
            local_role: "Peer".into(),
            path_kind: "DIRECT".into(),
            path_rtt_ms: 12,
            relay_url_active: "http://relay.example:3340".into(),
        };
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(
            json,
            r#"{"node_id":"n","status":"Running","peers_count":3,"groups_count":1,"local_role":"Peer","path_kind":"DIRECT","path_rtt_ms":12,"relay_url_active":"http://relay.example:3340"}"#
        );
    }

    #[test]
    fn node_status_json_escapes_special_chars() {
        // A value containing a double quote corrupted the previous hand-rolled
        // `format!` JSON, making the Swift decode fail. serde must escape it so
        // the payload still round-trips.
        let status = NodeStatusFFI {
            node_id: "n".into(),
            status: "Running".into(),
            peers_count: 0,
            groups_count: 0,
            local_role: "we\"ird".into(),
            path_kind: "DIRECT".into(),
            path_rtt_ms: 0,
            relay_url_active: "".into(),
        };
        let json = serde_json::to_string(&status).unwrap();
        let round_trip: NodeStatusFFI = serde_json::from_str(&json).unwrap();
        assert_eq!(round_trip, status);
    }

    #[test]
    fn discovered_peer_json_keys_match_swift_decoder() {
        let peer = DiscoveredPeerFFI {
            node_id: "abc".into(),
            username: "alice".into(),
            source: "Announce".into(),
            discovered_at: 42,
        };
        let json = serde_json::to_string(&peer).unwrap();
        assert_eq!(
            json,
            r#"{"node_id":"abc","username":"alice","source":"Announce","discovered_at":42}"#
        );
    }

    #[test]
    fn delivered_message_json_keys_match_swift_decoder() {
        let msg = DeliveredMessageFFI {
            from: "sender".into(),
            payload: "QUJD".into(), // base64("ABC")
            envelope_id: "env-1".into(),
            timestamp: 100,
            signature_valid: true,
            was_encrypted: true,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(
            json,
            r#"{"from":"sender","payload":"QUJD","envelope_id":"env-1","timestamp":100,"signature_valid":true,"was_encrypted":true}"#
        );
    }
}
