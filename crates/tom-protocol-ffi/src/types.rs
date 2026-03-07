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

    /// Directory for persistent state (SQLite)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,

    /// Gossip bootstrap peers (hex NodeId strings)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gossip_bootstrap_peers: Vec<String>,
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
