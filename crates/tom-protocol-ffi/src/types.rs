//! FFI-safe types for JSON serialization/deserialization

use serde::{Deserialize, Serialize};
use tom_protocol::types::NodeId;

/// Serializable version of DeliveredMessage for FFI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveredMessageFFI {
    pub from: String,
    pub payload: Vec<u8>,
    pub envelope_id: String,
    pub timestamp: u64,
    pub signature_valid: bool,
    pub was_encrypted: bool,
}

impl From<tom_protocol::DeliveredMessage> for DeliveredMessageFFI {
    fn from(msg: tom_protocol::DeliveredMessage) -> Self {
        Self {
            from: msg.from.to_string(),
            payload: msg.payload,
            envelope_id: msg.envelope_id,
            timestamp: msg.timestamp,
            signature_valid: msg.signature_valid,
            was_encrypted: msg.was_encrypted,
        }
    }
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
