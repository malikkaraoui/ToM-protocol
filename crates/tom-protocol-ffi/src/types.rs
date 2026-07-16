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
    /// App build number advertised by the peer (0 = unknown). Shown as « Nom Build ».
    pub app_build: u32,
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
    pub clock_skew_ms: Option<i64>,
    pub clock_skew_samples: u64,
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

/// Serializable version of ProtocolEvent for FFI
/// Flattened structure with type + optional fields per variant
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProtocolEventFFI {
    /// Event type name (e.g., "PeerDiscovered", "RolePromoted", "SenderThrottled")
    pub r#type: String,
    /// Timestamp when event was added to the queue (milliseconds)
    pub ts_ms: u64,

    // Peer/Node IDs (short hex, not full, for UI display)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>, // DiscoverySource

    // Scores and rates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_rate: Option<f64>,

    // Strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // Path/Role info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_kind: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtt_ms: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_role: Option<String>,

    // Presence
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,

    // Delivery/retry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,

    // Discovery timing (instrumentation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
}

impl ProtocolEventFFI {
    /// Convert ProtocolEvent to FFI-serializable format
    pub fn from_protocol_event(event: tom_protocol::ProtocolEvent) -> Self {
        let ts_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        match event {
            tom_protocol::ProtocolEvent::PeerDiscovered {
                node_id,
                username,
                source,
                app_build: _,
            } => {
                Self {
                    r#type: "PeerDiscovered".to_string(),
                    ts_ms,
                    node_id: Some(node_id.to_string()),
                    username: Some(username),
                    source: Some(format!("{:?}", source)),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::PeerStale { node_id } => {
                Self {
                    r#type: "PeerStale".to_string(),
                    ts_ms,
                    node_id: Some(node_id.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::PeerOffline { node_id } => {
                Self {
                    r#type: "PeerOffline".to_string(),
                    ts_ms,
                    node_id: Some(node_id.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::PeerOnline { node_id } => {
                Self {
                    r#type: "PeerOnline".to_string(),
                    ts_ms,
                    node_id: Some(node_id.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::MessageRejected { reason } => {
                Self {
                    r#type: "MessageRejected".to_string(),
                    ts_ms,
                    reason: Some(reason),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::Forwarded {
                envelope_id,
                next_hop,
            } => {
                Self {
                    r#type: "Forwarded".to_string(),
                    ts_ms,
                    message_id: Some(envelope_id),
                    node_id: Some(next_hop.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::PathChanged { event } => {
                Self {
                    r#type: "PathChanged".to_string(),
                    ts_ms,
                    path_kind: Some(format!("{}", event.kind)),
                    rtt_ms: Some(event.rtt.as_millis() as u64),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::Error { description } => {
                Self {
                    r#type: "Error".to_string(),
                    ts_ms,
                    description: Some(description),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::PresenceAttestationReceived {
                attester_id,
                challenge_id,
                latency_ms,
            } => {
                Self {
                    r#type: "PresenceAttestationReceived".to_string(),
                    ts_ms,
                    node_id: Some(attester_id.to_string()),
                    message_id: Some(challenge_id),
                    latency_ms: Some(latency_ms),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::RolePromoted { node_id, score } => {
                Self {
                    r#type: "RolePromoted".to_string(),
                    ts_ms,
                    node_id: Some(node_id.to_string()),
                    score: Some(score),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::RoleDemoted { node_id, score } => {
                Self {
                    r#type: "RoleDemoted".to_string(),
                    ts_ms,
                    node_id: Some(node_id.to_string()),
                    score: Some(score),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::LocalRoleChanged { new_role } => {
                Self {
                    r#type: "LocalRoleChanged".to_string(),
                    ts_ms,
                    new_role: Some(format!("{:?}", new_role)),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::BackupStored {
                message_id,
                recipient_id,
            } => {
                Self {
                    r#type: "BackupStored".to_string(),
                    ts_ms,
                    message_id: Some(message_id),
                    node_id: Some(recipient_id.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::BackupDelivered {
                message_id,
                recipient_id,
            } => {
                Self {
                    r#type: "BackupDelivered".to_string(),
                    ts_ms,
                    message_id: Some(message_id),
                    node_id: Some(recipient_id.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::BackupExpired {
                message_id,
                recipient_id,
            } => {
                Self {
                    r#type: "BackupExpired".to_string(),
                    ts_ms,
                    message_id: Some(message_id),
                    node_id: Some(recipient_id.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::SenderThrottled {
                node_id,
                score,
                current_rate,
            } => {
                Self {
                    r#type: "SenderThrottled".to_string(),
                    ts_ms,
                    node_id: Some(node_id.to_string()),
                    score: Some(score),
                    current_rate: Some(current_rate),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::DeliveryRetry {
                message_id,
                to,
                attempt,
            } => {
                Self {
                    r#type: "DeliveryRetry".to_string(),
                    ts_ms,
                    message_id: Some(message_id),
                    node_id: Some(to.to_string()),
                    attempt: Some(attempt),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::DeliveryTimeout {
                message_id,
                to,
                last_status,
            } => {
                Self {
                    r#type: "DeliveryTimeout".to_string(),
                    ts_ms,
                    message_id: Some(message_id),
                    node_id: Some(to.to_string()),
                    last_status: Some(format!("{:?}", last_status)),
                    ..Default::default()
                }
            }
            // Group events — simplified capture
            tom_protocol::ProtocolEvent::GroupCreated { group } => {
                Self {
                    r#type: "GroupCreated".to_string(),
                    ts_ms,
                    group_id: Some(group.group_id.to_string()),
                    username: Some(group.name.clone()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::GroupInviteReceived { invite } => {
                Self {
                    r#type: "GroupInviteReceived".to_string(),
                    ts_ms,
                    group_id: Some(invite.group_id.to_string()),
                    username: Some(invite.group_name.clone()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::GroupJoined {
                group_id,
                group_name,
            } => {
                Self {
                    r#type: "GroupJoined".to_string(),
                    ts_ms,
                    group_id: Some(group_id.to_string()),
                    username: Some(group_name),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::GroupMemberJoined { group_id, member } => {
                Self {
                    r#type: "GroupMemberJoined".to_string(),
                    ts_ms,
                    group_id: Some(group_id.to_string()),
                    node_id: Some(member.node_id.to_string()),
                    username: Some(member.username),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::GroupMemberLeft {
                group_id,
                node_id,
                username,
                reason,
            } => {
                Self {
                    r#type: "GroupMemberLeft".to_string(),
                    ts_ms,
                    group_id: Some(group_id.to_string()),
                    node_id: Some(node_id.to_string()),
                    username: Some(username),
                    reason: Some(format!("{:?}", reason)),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::GroupMessageReceived { .. } => {
                Self {
                    r#type: "GroupMessageReceived".to_string(),
                    ts_ms,
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::GroupHubMigrated {
                group_id,
                new_hub_id,
            } => {
                Self {
                    r#type: "GroupHubMigrated".to_string(),
                    ts_ms,
                    group_id: Some(group_id.to_string()),
                    node_id: Some(new_hub_id.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::GroupSecurityViolation {
                group_id,
                node_id,
                reason,
            } => {
                Self {
                    r#type: "GroupSecurityViolation".to_string(),
                    ts_ms,
                    group_id: Some(group_id.to_string()),
                    node_id: Some(node_id.to_string()),
                    reason: Some(reason),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::GossipNeighborUp { node_id } => {
                Self {
                    r#type: "GossipNeighborUp".to_string(),
                    ts_ms,
                    node_id: Some(node_id.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::GossipNeighborDown { node_id } => {
                Self {
                    r#type: "GossipNeighborDown".to_string(),
                    ts_ms,
                    node_id: Some(node_id.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::EmbeddedRelayStarted { url } => {
                Self {
                    r#type: "EmbeddedRelayStarted".to_string(),
                    ts_ms,
                    description: Some(url.to_string()),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::EmbeddedRelayFailed { error } => {
                Self {
                    r#type: "EmbeddedRelayFailed".to_string(),
                    ts_ms,
                    description: Some(error),
                    ..Default::default()
                }
            }
            tom_protocol::ProtocolEvent::DiscoveryTiming { elapsed_ms, detail } => {
                Self {
                    r#type: "DiscoveryTiming".to_string(),
                    ts_ms,
                    elapsed_ms: Some(elapsed_ms),
                    description: Some(detail),
                    ..Default::default()
                }
            }
            // Catch-all for remaining variants (expand as needed)
            _ => {
                Self {
                    r#type: format!("{:?}", event),
                    ts_ms,
                    ..Default::default()
                }
            }
        }
    }
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
            clock_skew_ms: Some(100),
            clock_skew_samples: 5,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(
            json,
            r#"{"node_id":"n","status":"Running","peers_count":3,"groups_count":1,"local_role":"Peer","path_kind":"DIRECT","path_rtt_ms":12,"relay_url_active":"http://relay.example:3340","clock_skew_ms":100,"clock_skew_samples":5}"#
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
            clock_skew_ms: None,
            clock_skew_samples: 0,
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
