use crate::NodeId;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Message envelope — wire-compatible with TypeScript `MessageEnvelope`.
///
/// See `packages/core/src/types/envelope.ts` for the canonical definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Unique message identifier (UUID v4).
    pub id: String,

    /// Sender's NodeId.
    pub from: NodeId,

    /// Recipient's NodeId.
    pub to: NodeId,

    /// Relay chain (NodeIds this message transited through).
    pub via: Vec<NodeId>,

    /// Message type (e.g., "chat", "ack", "app").
    #[serde(rename = "type")]
    pub msg_type: String,

    /// Application payload (arbitrary JSON).
    pub payload: serde_json::Value,

    /// Unix timestamp in milliseconds.
    pub timestamp: u64,

    /// Ed25519 signature (empty string if unsigned).
    pub signature: String,

    // --- Optional fields (E2E encryption) ---
    /// Encrypted payload ciphertext.
    #[serde(skip_serializing_if = "Option::is_none", rename = "encryptedPayload")]
    pub encrypted_payload: Option<String>,

    /// Sender's ephemeral X25519 public key.
    #[serde(skip_serializing_if = "Option::is_none", rename = "ephemeralPublicKey")]
    pub ephemeral_public_key: Option<String>,

    /// Encryption nonce.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,

    // --- Optional fields (latency tracking) ---
    /// Timestamps at each hop.
    #[serde(skip_serializing_if = "Option::is_none", rename = "hopTimestamps")]
    pub hop_timestamps: Option<Vec<u64>>,

    /// Route type for path visualization.
    #[serde(skip_serializing_if = "Option::is_none", rename = "routeType")]
    pub route_type: Option<String>,
}

impl MessageEnvelope {
    /// Create a new envelope with a random UUID and current timestamp.
    pub fn new(
        from: NodeId,
        to: NodeId,
        msg_type: &str,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            via: Vec::new(),
            msg_type: msg_type.to_string(),
            payload,
            timestamp: now_ms(),
            signature: String::new(),
            encrypted_payload: None,
            ephemeral_public_key: None,
            nonce: None,
            hop_timestamps: None,
            route_type: None,
        }
    }

    /// Create with a relay chain.
    pub fn new_via(
        from: NodeId,
        to: NodeId,
        via: Vec<NodeId>,
        msg_type: &str,
        payload: serde_json::Value,
    ) -> Self {
        let mut envelope = Self::new(from, to, msg_type, payload);
        envelope.via = via;
        envelope
    }

    /// Serialize to JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// Current time in milliseconds since UNIX epoch.
#[inline]
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trip() {
        // Create a dummy NodeId by parsing a known hex string
        // In tests we can't easily create real ones, so test serialization format
        let json = r#"{
            "id": "test-123",
            "from": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "to": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "via": [],
            "type": "chat",
            "payload": {"text": "Hello!"},
            "timestamp": 1700000000000,
            "signature": ""
        }"#;

        let envelope: MessageEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.id, "test-123");
        assert_eq!(envelope.msg_type, "chat");
        assert_eq!(envelope.payload["text"], "Hello!");
        assert!(envelope.encrypted_payload.is_none());

        // Round-trip
        let bytes = envelope.to_bytes().unwrap();
        let decoded = MessageEnvelope::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.id, "test-123");
        assert_eq!(decoded.msg_type, "chat");
    }

    #[test]
    fn envelope_optional_fields() {
        let json = r#"{
            "id": "test-456",
            "from": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "to": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "via": [],
            "type": "chat",
            "payload": null,
            "timestamp": 0,
            "signature": "",
            "encryptedPayload": "cipher",
            "ephemeralPublicKey": "key123",
            "nonce": "nonce456",
            "hopTimestamps": [100, 200],
            "routeType": "direct"
        }"#;

        let envelope: MessageEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.encrypted_payload.as_deref(), Some("cipher"));
        assert_eq!(envelope.hop_timestamps, Some(vec![100, 200]));
        assert_eq!(envelope.route_type.as_deref(), Some("direct"));
    }

    #[test]
    fn envelope_skips_none_fields() {
        let json = r#"{
            "id": "test",
            "from": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "to": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "via": [],
            "type": "ping",
            "payload": {},
            "timestamp": 0,
            "signature": ""
        }"#;

        let envelope: MessageEnvelope = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&envelope).unwrap();

        // Optional fields with None should not appear in output
        assert!(!serialized.contains("encryptedPayload"));
        assert!(!serialized.contains("ephemeralPublicKey"));
        assert!(!serialized.contains("nonce"));
        assert!(!serialized.contains("hopTimestamps"));
        assert!(!serialized.contains("routeType"));
    }
}
