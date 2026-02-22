/// Message routing engine for ToM protocol.
///
/// Pure decision logic — receives an envelope, returns a `RoutingAction`
/// telling the caller what to do (deliver, forward, reject, drop).
/// No I/O, no transport dependency.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::envelope::Envelope;
use crate::error::TomProtocolError;
use crate::types::{now_ms, MessageType, NodeId};

/// Maximum relay chain depth (ToM design decision #2).
pub const MAX_RELAY_DEPTH: usize = 4;

/// TTL for message dedup cache entries (10 min).
const DEDUP_TTL: Duration = Duration::from_secs(600);

/// TTL for ACK anti-replay cache entries (5 min).
const ACK_TTL: Duration = Duration::from_secs(300);

/// Maximum cached entries per cache (DoS protection).
const MAX_CACHE_SIZE: usize = 10_000;

/// Maximum age for read receipt timestamps (7 days in ms).
const READ_RECEIPT_MAX_AGE_MS: u64 = 7 * 24 * 60 * 60 * 1000;

// ── Routing decisions ──────────────────────────────────────────────────

/// What to do with an incoming envelope.
#[derive(Debug)]
pub enum RoutingAction {
    /// Regular message for us — deliver to application.
    /// `response` is an unsigned delivery ACK to send back to the sender.
    Deliver {
        envelope: Envelope,
        response: Envelope,
    },
    /// ACK for us — update message status tracker.
    Ack {
        original_message_id: String,
        ack_type: AckType,
        from: NodeId,
    },
    /// Read receipt for us.
    ReadReceipt {
        original_message_id: String,
        read_at: u64,
        from: NodeId,
    },
    /// Forward to next hop. `relay_ack` is an unsigned ACK for the original sender.
    Forward {
        envelope: Envelope,
        next_hop: NodeId,
        relay_ack: Envelope,
    },
    /// Rejected (TTL exhausted, chain too deep, malformed, etc.)
    Reject { reason: String },
    /// Duplicate or expired — silently ignore.
    Drop,
}

// ── ACK types ──────────────────────────────────────────────────────────

/// ACK subtypes for message status pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AckType {
    /// Relay confirms it forwarded the message.
    RelayForwarded,
    /// Final recipient confirms delivery.
    RecipientReceived,
}

/// Serialized payload of an ACK envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckPayload {
    pub original_message_id: String,
    pub ack_type: AckType,
}

impl AckPayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        rmp_serde::to_vec(self).expect("AckPayload serialization cannot fail")
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, TomProtocolError> {
        rmp_serde::from_slice(data).map_err(Into::into)
    }
}

/// Serialized payload of a read-receipt envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadReceiptPayload {
    pub original_message_id: String,
    pub read_at: u64,
}

impl ReadReceiptPayload {
    pub fn to_bytes(&self) -> Vec<u8> {
        rmp_serde::to_vec(self).expect("ReadReceiptPayload serialization cannot fail")
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, TomProtocolError> {
        rmp_serde::from_slice(data).map_err(Into::into)
    }
}

// ── Router ─────────────────────────────────────────────────────────────

/// Pure routing engine — no I/O, no transport.
///
/// Call `route()` with an incoming envelope, act on the returned `RoutingAction`.
pub struct Router {
    local_id: NodeId,
    /// Dedup cache: "msg_id:from" → first seen. Prevents duplicate delivery.
    message_cache: HashMap<String, Instant>,
    /// ACK anti-replay cache: "msg_id:from:ack_type" → first seen.
    ack_cache: HashMap<String, Instant>,
}

impl Router {
    pub fn new(local_id: NodeId) -> Self {
        Self {
            local_id,
            message_cache: HashMap::new(),
            ack_cache: HashMap::new(),
        }
    }

    /// The local node's identity.
    pub fn local_id(&self) -> NodeId {
        self.local_id
    }

    /// Route an incoming envelope. Returns the action to take.
    ///
    /// All returned envelopes (ACKs) are **unsigned** — the caller must
    /// sign them before sending.
    pub fn route(&mut self, envelope: Envelope) -> RoutingAction {
        // Guard: relay chain too deep
        if envelope.via.len() > MAX_RELAY_DEPTH {
            return RoutingAction::Reject {
                reason: format!(
                    "relay chain depth {} exceeds max {}",
                    envelope.via.len(),
                    MAX_RELAY_DEPTH
                ),
            };
        }

        // Is this for us?
        if envelope.to == self.local_id {
            return self.handle_local(envelope);
        }

        // Are we in the relay chain?
        if let Some(pos) = envelope.via.iter().position(|id| *id == self.local_id) {
            return self.handle_forward_in_chain(envelope, pos);
        }

        // Not for us, not in chain — forward directly to recipient
        self.handle_direct_forward(envelope)
    }

    /// Evict expired entries from both caches.
    pub fn cleanup_caches(&mut self) {
        let now = Instant::now();
        self.message_cache
            .retain(|_, ts| now.duration_since(*ts) < DEDUP_TTL);
        self.ack_cache
            .retain(|_, ts| now.duration_since(*ts) < ACK_TTL);
    }

    /// Current sizes of (message_cache, ack_cache).
    pub fn cache_sizes(&self) -> (usize, usize) {
        (self.message_cache.len(), self.ack_cache.len())
    }

    // ── Internal ───────────────────────────────────────────────────────

    fn handle_local(&mut self, envelope: Envelope) -> RoutingAction {
        match envelope.msg_type {
            MessageType::Ack => self.handle_ack(envelope),
            MessageType::ReadReceipt => self.handle_read_receipt(envelope),
            _ => self.handle_deliver(envelope),
        }
    }

    fn handle_deliver(&mut self, envelope: Envelope) -> RoutingAction {
        // Dedup check
        let cache_key = format!("{}:{}", envelope.id, envelope.from);
        if self.message_cache.contains_key(&cache_key) {
            return RoutingAction::Drop;
        }

        // Evict if at capacity
        if self.message_cache.len() >= MAX_CACHE_SIZE {
            self.cleanup_caches();
        }
        self.message_cache.insert(cache_key, Instant::now());

        // Create delivery ACK (via reversed relay chain)
        let response = self.create_delivery_ack(&envelope);

        RoutingAction::Deliver { envelope, response }
    }

    fn handle_ack(&mut self, envelope: Envelope) -> RoutingAction {
        let ack = match AckPayload::from_bytes(&envelope.payload) {
            Ok(a) => a,
            Err(_) => {
                return RoutingAction::Reject {
                    reason: "malformed ACK payload".into(),
                }
            }
        };

        // Anti-replay
        let cache_key = format!(
            "{}:{}:{:?}",
            ack.original_message_id, envelope.from, ack.ack_type
        );
        if self.ack_cache.contains_key(&cache_key) {
            return RoutingAction::Drop;
        }
        if self.ack_cache.len() >= MAX_CACHE_SIZE {
            self.cleanup_caches();
        }
        self.ack_cache.insert(cache_key, Instant::now());

        RoutingAction::Ack {
            original_message_id: ack.original_message_id,
            ack_type: ack.ack_type,
            from: envelope.from,
        }
    }

    fn handle_read_receipt(&mut self, envelope: Envelope) -> RoutingAction {
        let rr = match ReadReceiptPayload::from_bytes(&envelope.payload) {
            Ok(r) => r,
            Err(_) => {
                return RoutingAction::Reject {
                    reason: "malformed read receipt payload".into(),
                }
            }
        };

        // Anti-replay
        let cache_key = format!("{}:{}:read", rr.original_message_id, envelope.from);
        if self.ack_cache.contains_key(&cache_key) {
            return RoutingAction::Drop;
        }
        if self.ack_cache.len() >= MAX_CACHE_SIZE {
            self.cleanup_caches();
        }
        self.ack_cache.insert(cache_key, Instant::now());

        // Clamp read_at: not future, not older than 7 days
        let now = now_ms();
        let read_at = rr.read_at.min(now).max(now.saturating_sub(READ_RECEIPT_MAX_AGE_MS));

        RoutingAction::ReadReceipt {
            original_message_id: rr.original_message_id,
            read_at,
            from: envelope.from,
        }
    }

    fn handle_forward_in_chain(&mut self, mut envelope: Envelope, position: usize) -> RoutingAction {
        // Determine next hop
        let next_hop = if position >= envelope.via.len() - 1 {
            // We're the last relay → send to final recipient
            envelope.to
        } else {
            envelope.via[position + 1]
        };

        // Decrement TTL
        if let Err(e) = envelope.decrement_ttl() {
            return RoutingAction::Reject {
                reason: e.to_string(),
            };
        }

        let relay_ack = self.create_relay_ack(&envelope);

        RoutingAction::Forward {
            envelope,
            next_hop,
            relay_ack,
        }
    }

    fn handle_direct_forward(&mut self, mut envelope: Envelope) -> RoutingAction {
        if let Err(e) = envelope.decrement_ttl() {
            return RoutingAction::Reject {
                reason: e.to_string(),
            };
        }

        let next_hop = envelope.to;
        let relay_ack = self.create_relay_ack(&envelope);

        RoutingAction::Forward {
            envelope,
            next_hop,
            relay_ack,
        }
    }

    /// Create a delivery ACK routed back through the reversed relay chain.
    fn create_delivery_ack(&self, original: &Envelope) -> Envelope {
        let payload = AckPayload {
            original_message_id: original.id.clone(),
            ack_type: AckType::RecipientReceived,
        }
        .to_bytes();

        let via: Vec<NodeId> = original.via.iter().rev().copied().collect();
        Envelope::new_via(self.local_id, original.from, via, MessageType::Ack, payload)
    }

    /// Create a relay ACK sent directly to the original sender (no relay chain).
    fn create_relay_ack(&self, original: &Envelope) -> Envelope {
        let payload = AckPayload {
            original_message_id: original.id.clone(),
            ack_type: AckType::RelayForwarded,
        }
        .to_bytes();

        Envelope::new(self.local_id, original.from, MessageType::Ack, payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DEFAULT_TTL;

    /// Generate a deterministic NodeId from a seed byte.
    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    /// Make a chat envelope from → to.
    fn chat(from: NodeId, to: NodeId, payload: &[u8]) -> Envelope {
        Envelope {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            via: Vec::new(),
            msg_type: MessageType::Chat,
            payload: payload.to_vec(),
            timestamp: 1708000000000,
            signature: Vec::new(),
            ttl: DEFAULT_TTL,
            encrypted: false,
        }
    }

    /// Make an ACK envelope.
    fn ack_envelope(from: NodeId, to: NodeId, original_id: &str, ack_type: AckType) -> Envelope {
        let payload = AckPayload {
            original_message_id: original_id.to_string(),
            ack_type,
        }
        .to_bytes();
        Envelope {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            via: Vec::new(),
            msg_type: MessageType::Ack,
            payload,
            timestamp: 1708000000000,
            signature: Vec::new(),
            ttl: DEFAULT_TTL,
            encrypted: false,
        }
    }

    // ── Deliver tests ──────────────────────────────────────────────────

    #[test]
    fn deliver_message_for_us() {
        let me = node_id(1);
        let sender = node_id(2);
        let mut router = Router::new(me);

        let env = chat(sender, me, b"hello");
        let msg_id = env.id.clone();

        match router.route(env) {
            RoutingAction::Deliver { envelope, response } => {
                assert_eq!(envelope.payload, b"hello");
                assert_eq!(response.to, sender);
                assert_eq!(response.from, me);
                assert_eq!(response.msg_type, MessageType::Ack);

                // ACK payload contains the original message ID
                let ack = AckPayload::from_bytes(&response.payload).unwrap();
                assert_eq!(ack.original_message_id, msg_id);
                assert_eq!(ack.ack_type, AckType::RecipientReceived);
            }
            other => panic!("expected Deliver, got {:?}", other),
        }
    }

    #[test]
    fn delivery_ack_reverses_via_chain() {
        let me = node_id(1);
        let sender = node_id(2);
        let relay1 = node_id(10);
        let relay2 = node_id(11);
        let mut router = Router::new(me);

        let mut env = chat(sender, me, b"routed");
        env.via = vec![relay1, relay2];

        match router.route(env) {
            RoutingAction::Deliver { response, .. } => {
                // ACK should travel via reversed chain
                assert_eq!(response.via, vec![relay2, relay1]);
            }
            other => panic!("expected Deliver, got {:?}", other),
        }
    }

    #[test]
    fn dedup_drops_duplicate() {
        let me = node_id(1);
        let sender = node_id(2);
        let mut router = Router::new(me);

        let env = chat(sender, me, b"once");
        let env2 = env.clone();

        assert!(matches!(router.route(env), RoutingAction::Deliver { .. }));
        assert!(matches!(router.route(env2), RoutingAction::Drop));
    }

    // ── Forward tests ──────────────────────────────────────────────────

    #[test]
    fn forward_in_chain() {
        let me = node_id(10); // I'm a relay
        let sender = node_id(1);
        let recipient = node_id(2);
        let mut router = Router::new(me);

        let mut env = chat(sender, recipient, b"relayed");
        env.via = vec![me]; // I'm the only relay
        let original_ttl = env.ttl;

        match router.route(env) {
            RoutingAction::Forward {
                envelope,
                next_hop,
                relay_ack,
            } => {
                assert_eq!(next_hop, recipient); // Last relay → forward to recipient
                assert_eq!(envelope.ttl, original_ttl - 1);

                // Relay ACK goes to original sender
                assert_eq!(relay_ack.to, sender);
                assert_eq!(relay_ack.from, me);
                let ack = AckPayload::from_bytes(&relay_ack.payload).unwrap();
                assert_eq!(ack.ack_type, AckType::RelayForwarded);
            }
            other => panic!("expected Forward, got {:?}", other),
        }
    }

    #[test]
    fn forward_multi_hop_chain() {
        let relay1 = node_id(10);
        let relay2 = node_id(11);
        let sender = node_id(1);
        let recipient = node_id(2);

        // I'm relay1, relay2 is after me
        let mut router = Router::new(relay1);
        let mut env = chat(sender, recipient, b"multi-hop");
        env.via = vec![relay1, relay2];

        match router.route(env) {
            RoutingAction::Forward { next_hop, .. } => {
                assert_eq!(next_hop, relay2); // Next relay, not recipient
            }
            other => panic!("expected Forward, got {:?}", other),
        }
    }

    #[test]
    fn direct_forward_not_in_chain() {
        let me = node_id(10);
        let sender = node_id(1);
        let recipient = node_id(2);
        let mut router = Router::new(me);

        // Message not for us and we're not in via chain
        let env = chat(sender, recipient, b"passing through");

        match router.route(env) {
            RoutingAction::Forward {
                next_hop,
                relay_ack,
                ..
            } => {
                assert_eq!(next_hop, recipient);
                assert_eq!(relay_ack.to, sender);
            }
            other => panic!("expected Forward, got {:?}", other),
        }
    }

    // ── Reject tests ───────────────────────────────────────────────────

    #[test]
    fn reject_deep_relay_chain() {
        let me = node_id(1);
        let sender = node_id(2);
        let mut router = Router::new(me);

        let mut env = chat(sender, me, b"too deep");
        env.via = (10..16).map(node_id).collect(); // 6 relays > MAX_RELAY_DEPTH (4)

        assert!(matches!(router.route(env), RoutingAction::Reject { .. }));
    }

    #[test]
    fn reject_ttl_exhausted() {
        let me = node_id(10);
        let sender = node_id(1);
        let recipient = node_id(2);
        let mut router = Router::new(me);

        let mut env = chat(sender, recipient, b"expired");
        env.via = vec![me];
        env.ttl = 0;

        assert!(matches!(router.route(env), RoutingAction::Reject { .. }));
    }

    // ── ACK tests ──────────────────────────────────────────────────────

    #[test]
    fn ack_received() {
        let me = node_id(1);
        let relay = node_id(10);
        let mut router = Router::new(me);

        let env = ack_envelope(relay, me, "msg-123", AckType::RelayForwarded);

        match router.route(env) {
            RoutingAction::Ack {
                original_message_id,
                ack_type,
                from,
            } => {
                assert_eq!(original_message_id, "msg-123");
                assert_eq!(ack_type, AckType::RelayForwarded);
                assert_eq!(from, relay);
            }
            other => panic!("expected Ack, got {:?}", other),
        }
    }

    #[test]
    fn ack_anti_replay() {
        let me = node_id(1);
        let relay = node_id(10);
        let mut router = Router::new(me);

        let env1 = ack_envelope(relay, me, "msg-123", AckType::RelayForwarded);
        let env2 = ack_envelope(relay, me, "msg-123", AckType::RelayForwarded);

        assert!(matches!(router.route(env1), RoutingAction::Ack { .. }));
        assert!(matches!(router.route(env2), RoutingAction::Drop));
    }

    #[test]
    fn different_ack_types_not_deduped() {
        let me = node_id(1);
        let peer = node_id(2);
        let mut router = Router::new(me);

        let env1 = ack_envelope(peer, me, "msg-123", AckType::RelayForwarded);
        let env2 = ack_envelope(peer, me, "msg-123", AckType::RecipientReceived);

        assert!(matches!(router.route(env1), RoutingAction::Ack { .. }));
        assert!(matches!(router.route(env2), RoutingAction::Ack { .. })); // Not deduped
    }

    #[test]
    fn malformed_ack_rejected() {
        let me = node_id(1);
        let peer = node_id(2);
        let mut router = Router::new(me);

        let mut env = chat(peer, me, b"not an ack payload");
        env.msg_type = MessageType::Ack;

        assert!(matches!(router.route(env), RoutingAction::Reject { .. }));
    }

    // ── Read receipt tests ─────────────────────────────────────────────

    #[test]
    fn read_receipt_received() {
        let me = node_id(1);
        let peer = node_id(2);
        let mut router = Router::new(me);

        let payload = ReadReceiptPayload {
            original_message_id: "msg-456".into(),
            read_at: 1708000000000,
        }
        .to_bytes();

        let mut env = chat(peer, me, &payload);
        env.msg_type = MessageType::ReadReceipt;

        match router.route(env) {
            RoutingAction::ReadReceipt {
                original_message_id,
                from,
                ..
            } => {
                assert_eq!(original_message_id, "msg-456");
                assert_eq!(from, peer);
            }
            other => panic!("expected ReadReceipt, got {:?}", other),
        }
    }

    #[test]
    fn read_receipt_anti_replay() {
        let me = node_id(1);
        let peer = node_id(2);
        let mut router = Router::new(me);

        let payload = ReadReceiptPayload {
            original_message_id: "msg-789".into(),
            read_at: 1708000000000,
        }
        .to_bytes();

        let mut env1 = chat(peer, me, &payload);
        env1.msg_type = MessageType::ReadReceipt;
        let mut env2 = env1.clone();
        env2.id = uuid::Uuid::new_v4().to_string();

        assert!(matches!(
            router.route(env1),
            RoutingAction::ReadReceipt { .. }
        ));
        assert!(matches!(router.route(env2), RoutingAction::Drop));
    }

    // ── Cache tests ────────────────────────────────────────────────────

    #[test]
    fn cleanup_caches() {
        let me = node_id(1);
        let sender = node_id(2);
        let mut router = Router::new(me);

        // Add some entries
        let env = chat(sender, me, b"cached");
        router.route(env);

        assert_eq!(router.cache_sizes(), (1, 0));

        router.cleanup_caches();
        // Entries are fresh — not evicted yet
        assert_eq!(router.cache_sizes(), (1, 0));
    }

    #[test]
    fn ack_payload_roundtrip() {
        let ack = AckPayload {
            original_message_id: "test-123".into(),
            ack_type: AckType::RecipientReceived,
        };
        let bytes = ack.to_bytes();
        let decoded = AckPayload::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.original_message_id, "test-123");
        assert_eq!(decoded.ack_type, AckType::RecipientReceived);
    }

    #[test]
    fn read_receipt_payload_roundtrip() {
        let rr = ReadReceiptPayload {
            original_message_id: "test-456".into(),
            read_at: 1708000000000,
        };
        let bytes = rr.to_bytes();
        let decoded = ReadReceiptPayload::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.original_message_id, "test-456");
        assert_eq!(decoded.read_at, 1708000000000);
    }

    #[test]
    fn exact_max_relay_depth_allowed() {
        let me = node_id(1);
        let sender = node_id(2);
        let mut router = Router::new(me);

        let mut env = chat(sender, me, b"at limit");
        env.via = (10..14).map(node_id).collect(); // Exactly 4 relays = MAX
        assert!(matches!(router.route(env), RoutingAction::Deliver { .. }));
    }

    #[test]
    fn forward_decrements_ttl() {
        let me = node_id(10);
        let sender = node_id(1);
        let recipient = node_id(2);
        let mut router = Router::new(me);

        let mut env = chat(sender, recipient, b"ttl test");
        env.via = vec![me];
        env.ttl = 3;

        match router.route(env) {
            RoutingAction::Forward { envelope, .. } => {
                assert_eq!(envelope.ttl, 2);
            }
            other => panic!("expected Forward, got {:?}", other),
        }
    }
}
