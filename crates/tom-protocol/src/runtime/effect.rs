use crate::discovery::RoleChangeAnnounce;
use crate::envelope::Envelope;
use crate::tracker::StatusChange;
use crate::types::NodeId;
use tom_connect::RelayUrl;

use super::{DeliveredMessage, ProtocolEvent};

/// Intention produite par la logique pure de RuntimeState.
///
/// Chaque methode handle_* / tick_* retourne Vec<RuntimeEffect>.
/// La boucle principale execute ensuite ces effets via Transport + channels.
#[derive(Debug)]
pub enum RuntimeEffect {
    /// Envoyer une enveloppe au premier hop (relay ou direct).
    SendEnvelope(Envelope),

    /// Envoyer une enveloppe a un noeud precis (hop explicite).
    SendEnvelopeTo {
        target: NodeId,
        envelope: Envelope,
    },

    /// Livrer un message decrypte a l'application (TUI, bot...).
    DeliverMessage(DeliveredMessage),

    /// Notifier un changement de statut (pending -> sent -> relayed -> delivered -> read).
    StatusChange(StatusChange),

    /// Emettre un evenement protocole (peer offline, group created, etc.).
    Emit(ProtocolEvent),

    /// Essayer d'envoyer — si le transport echoue, executer le plan B.
    /// Utilise pour le backup automatique quand un peer est offline.
    SendWithBackupFallback {
        envelope: Envelope,
        on_success: Vec<RuntimeEffect>,
        on_failure: Vec<RuntimeEffect>,
    },

    /// Broadcast a role change via gossip to all neighbors.
    BroadcastRoleChange(RoleChangeAnnounce),

    /// Start the embedded relay server with the given config.
    StartEmbeddedRelay {
        config: super::EmbeddedRelayConfig,
    },

    /// Stop the embedded relay server.
    StopEmbeddedRelay,

    /// Broadcast a relay-ready announcement via gossip.
    BroadcastRelayReady(crate::discovery::RelayReadyAnnounce),

    /// Insert a discovered relay into the transport layer.
    /// Handled by loop interceptor (needs node access).
    InsertTransportRelay {
        relay_url: RelayUrl,
    },

    /// Remove a discovered relay from the transport layer.
    /// Only emitted when the URL is no longer referenced by any active registry entry.
    /// Handled by loop interceptor (needs node access).
    RemoveTransportRelay {
        relay_url: RelayUrl,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::EnvelopeBuilder;
    use crate::types::{MessageStatus, MessageType, NodeId};
    use crate::tracker::StatusChange;

    // ── Helpers ──────────────────────────────────────────────────────────

    fn test_node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    fn test_envelope(from_seed: u8, to_seed: u8) -> Envelope {
        EnvelopeBuilder::new(
            test_node_id(from_seed),
            test_node_id(to_seed),
            MessageType::Chat,
            b"hello".to_vec(),
        )
        .build()
    }

    fn test_delivered_message(from_seed: u8) -> DeliveredMessage {
        DeliveredMessage {
            from: test_node_id(from_seed),
            payload: b"payload".to_vec(),
            envelope_id: "test-id".to_string(),
            timestamp: 1_000_000,
            signature_valid: true,
            was_encrypted: true,
        }
    }

    fn test_status_change() -> StatusChange {
        StatusChange {
            message_id: "msg-1".to_string(),
            previous: MessageStatus::Pending,
            current: MessageStatus::Sent,
        }
    }

    fn test_relay_url() -> RelayUrl {
        "http://127.0.0.1:3340".parse().unwrap()
    }

    // ── SendEnvelope ─────────────────────────────────────────────────────

    #[test]
    fn send_envelope_constructs() {
        let env = test_envelope(1, 2);
        let effect = RuntimeEffect::SendEnvelope(env);
        assert!(matches!(effect, RuntimeEffect::SendEnvelope(_)));
    }

    #[test]
    fn send_envelope_pattern_match_extracts_envelope() {
        let env = test_envelope(1, 2);
        let to = test_node_id(2);
        let effect = RuntimeEffect::SendEnvelope(env);
        match effect {
            RuntimeEffect::SendEnvelope(ref e) => assert_eq!(e.to, to),
            _ => panic!("wrong variant"),
        }
    }

    // ── SendEnvelopeTo ───────────────────────────────────────────────────

    #[test]
    fn send_envelope_to_constructs() {
        let target = test_node_id(5);
        let env = test_envelope(1, 2);
        let effect = RuntimeEffect::SendEnvelopeTo { target, envelope: env };
        assert!(matches!(effect, RuntimeEffect::SendEnvelopeTo { .. }));
    }

    #[test]
    fn send_envelope_to_extracts_target() {
        let target = test_node_id(7);
        let env = test_envelope(1, 2);
        let effect = RuntimeEffect::SendEnvelopeTo { target, envelope: env };
        match effect {
            RuntimeEffect::SendEnvelopeTo { target: t, .. } => assert_eq!(t, target),
            _ => panic!("wrong variant"),
        }
    }

    // ── DeliverMessage ───────────────────────────────────────────────────

    #[test]
    fn deliver_message_constructs() {
        let msg = test_delivered_message(3);
        let effect = RuntimeEffect::DeliverMessage(msg);
        assert!(matches!(effect, RuntimeEffect::DeliverMessage(_)));
    }

    #[test]
    fn deliver_message_preserves_payload() {
        let msg = test_delivered_message(3);
        let effect = RuntimeEffect::DeliverMessage(msg);
        match effect {
            RuntimeEffect::DeliverMessage(m) => {
                assert_eq!(m.payload, b"payload");
                assert!(m.signature_valid);
                assert!(m.was_encrypted);
            }
            _ => panic!("wrong variant"),
        }
    }

    // ── StatusChange ─────────────────────────────────────────────────────

    #[test]
    fn status_change_constructs() {
        let change = test_status_change();
        let effect = RuntimeEffect::StatusChange(change);
        assert!(matches!(effect, RuntimeEffect::StatusChange(_)));
    }

    #[test]
    fn status_change_extracts_transition() {
        let change = StatusChange {
            message_id: "abc".to_string(),
            previous: MessageStatus::Sent,
            current: MessageStatus::Delivered,
        };
        let effect = RuntimeEffect::StatusChange(change);
        match effect {
            RuntimeEffect::StatusChange(c) => {
                assert_eq!(c.message_id, "abc");
                assert_eq!(c.previous, MessageStatus::Sent);
                assert_eq!(c.current, MessageStatus::Delivered);
            }
            _ => panic!("wrong variant"),
        }
    }

    // ── Emit ─────────────────────────────────────────────────────────────

    #[test]
    fn emit_error_constructs() {
        let effect = RuntimeEffect::Emit(ProtocolEvent::Error {
            description: "test error".to_string(),
        });
        assert!(matches!(effect, RuntimeEffect::Emit(_)));
    }

    #[test]
    fn emit_peer_offline_constructs() {
        let id = test_node_id(9);
        let effect = RuntimeEffect::Emit(ProtocolEvent::PeerOffline { node_id: id });
        match effect {
            RuntimeEffect::Emit(ProtocolEvent::PeerOffline { node_id }) => {
                assert_eq!(node_id, id);
            }
            _ => panic!("wrong variant"),
        }
    }

    // ── SendWithBackupFallback ───────────────────────────────────────────

    #[test]
    fn backup_fallback_constructs_with_empty_branches() {
        let env = test_envelope(1, 2);
        let effect = RuntimeEffect::SendWithBackupFallback {
            envelope: env,
            on_success: vec![],
            on_failure: vec![],
        };
        assert!(matches!(effect, RuntimeEffect::SendWithBackupFallback { .. }));
    }

    #[test]
    fn backup_fallback_with_nested_effects() {
        let env = test_envelope(1, 2);
        let on_success = vec![
            RuntimeEffect::Emit(ProtocolEvent::Error { description: "ok".to_string() }),
        ];
        let on_failure = vec![
            RuntimeEffect::StatusChange(test_status_change()),
        ];
        let effect = RuntimeEffect::SendWithBackupFallback {
            envelope: env,
            on_success,
            on_failure,
        };
        match effect {
            RuntimeEffect::SendWithBackupFallback { on_success, on_failure, .. } => {
                assert_eq!(on_success.len(), 1);
                assert_eq!(on_failure.len(), 1);
                assert!(matches!(on_success[0], RuntimeEffect::Emit(_)));
                assert!(matches!(on_failure[0], RuntimeEffect::StatusChange(_)));
            }
            _ => panic!("wrong variant"),
        }
    }

    // ── Loop-intercepted variants ────────────────────────────────────────

    #[test]
    fn stop_embedded_relay_constructs() {
        let effect = RuntimeEffect::StopEmbeddedRelay;
        assert!(matches!(effect, RuntimeEffect::StopEmbeddedRelay));
    }

    #[test]
    fn insert_transport_relay_constructs() {
        let url = test_relay_url();
        let effect = RuntimeEffect::InsertTransportRelay { relay_url: url.clone() };
        match effect {
            RuntimeEffect::InsertTransportRelay { relay_url } => {
                assert_eq!(relay_url, url);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn remove_transport_relay_constructs() {
        let url = test_relay_url();
        let effect = RuntimeEffect::RemoveTransportRelay { relay_url: url.clone() };
        match effect {
            RuntimeEffect::RemoveTransportRelay { relay_url } => {
                assert_eq!(relay_url, url);
            }
            _ => panic!("wrong variant"),
        }
    }

    // ── Vec<RuntimeEffect> batch ─────────────────────────────────────────

    #[test]
    fn empty_effects_vec_is_valid() {
        let effects: Vec<RuntimeEffect> = vec![];
        assert!(effects.is_empty());
    }

    #[test]
    fn mixed_effects_vec() {
        let effects = vec![
            RuntimeEffect::SendEnvelope(test_envelope(1, 2)),
            RuntimeEffect::DeliverMessage(test_delivered_message(3)),
            RuntimeEffect::StatusChange(test_status_change()),
            RuntimeEffect::StopEmbeddedRelay,
        ];
        assert_eq!(effects.len(), 4);
        assert!(matches!(effects[0], RuntimeEffect::SendEnvelope(_)));
        assert!(matches!(effects[1], RuntimeEffect::DeliverMessage(_)));
        assert!(matches!(effects[2], RuntimeEffect::StatusChange(_)));
        assert!(matches!(effects[3], RuntimeEffect::StopEmbeddedRelay));
    }

    #[test]
    fn debug_format_does_not_panic() {
        let effect = RuntimeEffect::SendEnvelope(test_envelope(1, 2));
        let _ = format!("{:?}", effect);
        let effect2 = RuntimeEffect::StopEmbeddedRelay;
        let _ = format!("{:?}", effect2);
    }
}
