use serde::{Deserialize, Serialize};

use crate::error::TomProtocolError;
use crate::types::{MessageType, NodeId, DEFAULT_TTL};

/// Protocol-level envelope — the unit of communication in ToM.
///
/// Serialized as MessagePack for compact binary wire format.
/// The `payload` is opaque bytes — the protocol routes and encrypts
/// without parsing the content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Envelope {
    /// Unique message identifier (UUID v4).
    pub id: String,
    /// Sender node identity.
    pub from: NodeId,
    /// Final recipient node identity.
    pub to: NodeId,
    /// Relay chain — intermediate nodes that forward this message.
    pub via: Vec<NodeId>,
    /// Message type — determines protocol handling.
    pub msg_type: MessageType,
    /// Opaque payload bytes (plaintext or ciphertext).
    pub payload: Vec<u8>,
    /// Creation timestamp (Unix milliseconds).
    pub timestamp: u64,
    /// Ed25519 signature over `signing_bytes()`. Empty if unsigned.
    pub signature: Vec<u8>,
    /// Remaining hop count. Decremented at each relay. Dropped at 0.
    pub ttl: u32,
    /// Whether `payload` is encrypted (E2E).
    pub encrypted: bool,
}

impl Envelope {
    /// Create a new unsigned envelope with default TTL.
    pub fn new(from: NodeId, to: NodeId, msg_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            via: Vec::new(),
            msg_type,
            payload,
            timestamp: now_ms(),
            signature: Vec::new(),
            ttl: DEFAULT_TTL,
            encrypted: false,
        }
    }

    /// Create a new envelope routed through specific relays.
    pub fn new_via(
        from: NodeId,
        to: NodeId,
        via: Vec<NodeId>,
        msg_type: MessageType,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            via,
            msg_type,
            payload,
            timestamp: now_ms(),
            signature: Vec::new(),
            ttl: DEFAULT_TTL,
            encrypted: false,
        }
    }

    /// Serialize to MessagePack bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, TomProtocolError> {
        rmp_serde::to_vec(self).map_err(Into::into)
    }

    /// Deserialize from MessagePack bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, TomProtocolError> {
        rmp_serde::from_slice(data).map_err(Into::into)
    }

    /// Produce the canonical bytes to sign/verify.
    ///
    /// Includes all fields except `signature` to avoid circular dependency.
    /// Deterministic: same envelope always produces the same signing bytes.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let signable = SignableEnvelope {
            id: &self.id,
            from: &self.from,
            to: &self.to,
            via: &self.via,
            msg_type: &self.msg_type,
            payload: &self.payload,
            timestamp: self.timestamp,
            ttl: self.ttl,
            encrypted: self.encrypted,
        };
        // Use MessagePack for deterministic serialization
        rmp_serde::to_vec(&signable).expect("signing_bytes serialization cannot fail")
    }

    /// Decrement TTL. Returns `Err` if TTL is already 0.
    pub fn decrement_ttl(&mut self) -> Result<(), TomProtocolError> {
        if self.ttl == 0 {
            return Err(TomProtocolError::RelayRejected {
                reason: "TTL exhausted".into(),
            });
        }
        self.ttl -= 1;
        Ok(())
    }

    /// Check if the envelope has a valid (non-empty) signature.
    pub fn is_signed(&self) -> bool {
        !self.signature.is_empty()
    }

    /// MessagePack size of this envelope in bytes.
    pub fn wire_size(&self) -> usize {
        self.to_bytes().map(|b| b.len()).unwrap_or(0)
    }
}

/// Internal struct for deterministic signing — all fields except signature.
#[derive(Serialize)]
struct SignableEnvelope<'a> {
    id: &'a str,
    from: &'a NodeId,
    to: &'a NodeId,
    via: &'a [NodeId],
    msg_type: &'a MessageType,
    payload: &'a [u8],
    timestamp: u64,
    ttl: u32,
    encrypted: bool,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a deterministic NodeId from a seed byte.
    /// Uses iroh's SecretKey to produce a valid Ed25519 public key.
    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        let id_str = secret.public().to_string();
        id_str.parse().unwrap()
    }

    /// Helper: create a test envelope with fixed fields.
    fn make_envelope(msg_type: MessageType, payload: Vec<u8>) -> Envelope {
        Envelope {
            id: "test-id-123".to_string(),
            from: node_id(1),
            to: node_id(2),
            via: Vec::new(),
            msg_type,
            payload,
            timestamp: 1708000000000,
            signature: Vec::new(),
            ttl: DEFAULT_TTL,
            encrypted: false,
        }
    }

    #[test]
    fn roundtrip_msgpack() {
        let env = make_envelope(MessageType::Chat, b"hello world".to_vec());

        let bytes = env.to_bytes().expect("serialize");
        let decoded = Envelope::from_bytes(&bytes).expect("deserialize");

        assert_eq!(env, decoded);
    }

    #[test]
    fn roundtrip_all_message_types() {
        let types = [
            MessageType::Chat,
            MessageType::Ack,
            MessageType::ReadReceipt,
            MessageType::Heartbeat,
            MessageType::GroupCreate,
            MessageType::GroupInvite,
            MessageType::GroupMessage,
            MessageType::GroupLeave,
            MessageType::PeerAnnounce,
            MessageType::BackupStore,
            MessageType::BackupDeliver,
        ];

        for msg_type in types {
            let env = make_envelope(msg_type, vec![42]);
            let bytes = env.to_bytes().expect("serialize");
            let decoded = Envelope::from_bytes(&bytes).expect("deserialize");
            assert_eq!(env.msg_type, decoded.msg_type);
        }
    }

    #[test]
    fn empty_payload() {
        let env = make_envelope(MessageType::Heartbeat, Vec::new());

        let bytes = env.to_bytes().expect("serialize");
        let decoded = Envelope::from_bytes(&bytes).expect("deserialize");

        assert!(decoded.payload.is_empty());
        assert_eq!(env, decoded);
    }

    #[test]
    fn large_payload_64kb() {
        let payload = vec![0xAB; 65536];
        let env = make_envelope(MessageType::Chat, payload.clone());

        let bytes = env.to_bytes().expect("serialize");
        let decoded = Envelope::from_bytes(&bytes).expect("deserialize");

        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn via_chain() {
        let relay1 = node_id(10);
        let relay2 = node_id(11);
        let relay3 = node_id(12);

        let env = Envelope {
            id: "routed-msg".to_string(),
            from: node_id(1),
            to: node_id(2),
            via: vec![relay1, relay2, relay3],
            msg_type: MessageType::Chat,
            payload: b"multi-hop".to_vec(),
            timestamp: 1708000000000,
            signature: Vec::new(),
            ttl: DEFAULT_TTL,
            encrypted: false,
        };

        let bytes = env.to_bytes().expect("serialize");
        let decoded = Envelope::from_bytes(&bytes).expect("deserialize");

        assert_eq!(decoded.via.len(), 3);
        assert_eq!(decoded.via[0], relay1);
        assert_eq!(decoded.via[1], relay2);
        assert_eq!(decoded.via[2], relay3);
    }

    #[test]
    fn signing_bytes_deterministic() {
        let env = make_envelope(MessageType::Chat, b"test".to_vec());

        let sb1 = env.signing_bytes();
        let sb2 = env.signing_bytes();

        assert_eq!(sb1, sb2, "signing_bytes must be deterministic");
    }

    #[test]
    fn signing_bytes_excludes_signature() {
        let mut env = make_envelope(MessageType::Chat, b"test".to_vec());

        let sb_unsigned = env.signing_bytes();
        env.signature = vec![1, 2, 3, 4, 5];
        let sb_signed = env.signing_bytes();

        assert_eq!(
            sb_unsigned, sb_signed,
            "signature must not affect signing_bytes"
        );
    }

    #[test]
    fn signing_bytes_changes_with_payload() {
        let env1 = make_envelope(MessageType::Chat, b"payload A".to_vec());
        let env2 = make_envelope(MessageType::Chat, b"payload B".to_vec());

        assert_ne!(
            env1.signing_bytes(),
            env2.signing_bytes(),
            "different payloads must produce different signing_bytes"
        );
    }

    #[test]
    fn decrement_ttl() {
        let mut env = make_envelope(MessageType::Chat, vec![]);
        assert_eq!(env.ttl, DEFAULT_TTL);

        env.decrement_ttl().expect("ttl > 0");
        assert_eq!(env.ttl, DEFAULT_TTL - 1);

        // Drain to 0
        for _ in 0..(DEFAULT_TTL - 1) {
            env.decrement_ttl().expect("ttl > 0");
        }
        assert_eq!(env.ttl, 0);

        // Next decrement should fail
        let result = env.decrement_ttl();
        assert!(result.is_err());
    }

    #[test]
    fn is_signed() {
        let mut env = make_envelope(MessageType::Chat, vec![]);
        assert!(!env.is_signed());

        env.signature = vec![0xFF; 64];
        assert!(env.is_signed());
    }

    #[test]
    fn wire_size_compact_vs_json() {
        let env = make_envelope(MessageType::Chat, b"hello".to_vec());
        let msgpack_size = env.wire_size();
        let json_size = serde_json::to_vec(&env).expect("json").len();

        assert!(
            msgpack_size < json_size,
            "MessagePack ({msgpack_size} bytes) should be smaller than JSON ({json_size} bytes)"
        );
    }

    #[test]
    fn invalid_bytes_rejected() {
        let result = Envelope::from_bytes(b"not valid msgpack");
        assert!(result.is_err());
    }

    #[test]
    fn new_generates_unique_ids() {
        let id1 = node_id(1);
        let id2 = node_id(2);
        let env1 = Envelope::new(id1, id2, MessageType::Chat, vec![]);
        let env2 = Envelope::new(id1, id2, MessageType::Chat, vec![]);
        assert_ne!(env1.id, env2.id, "new() should generate unique UUIDs");
    }

    #[test]
    fn encrypted_flag_roundtrip() {
        let mut env = make_envelope(MessageType::Chat, vec![1, 2, 3]);
        env.encrypted = true;

        let bytes = env.to_bytes().expect("serialize");
        let decoded = Envelope::from_bytes(&bytes).expect("deserialize");
        assert!(decoded.encrypted);
    }

    #[test]
    fn new_via_sets_relay_chain() {
        let from = node_id(1);
        let to = node_id(2);
        let relay = node_id(3);

        let env = Envelope::new_via(from, to, vec![relay], MessageType::Chat, b"hi".to_vec());

        assert_eq!(env.via.len(), 1);
        assert_eq!(env.via[0], relay);
        assert_eq!(env.from, from);
        assert_eq!(env.to, to);
    }
}
