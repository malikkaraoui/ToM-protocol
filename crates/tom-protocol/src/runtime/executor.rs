//! Effect executor — the only place that touches I/O.
//!
//! Takes a list of RuntimeEffect and executes them concretely:
//! - SendEnvelope / SendEnvelopeTo -> transport.send_raw()
//! - DeliverMessage -> msg_tx.send()
//! - StatusChange -> status_tx.send()
//! - Emit -> event_tx.send()
//! - SendWithBackupFallback -> try send, execute on_success or on_failure

use std::time::Duration;

use tokio::sync::mpsc;

use crate::envelope::Envelope;
use crate::types::NodeId;

use super::effect::RuntimeEffect;
use super::metrics::ProtocolMetrics;
use super::transport::Transport;
use super::{DeliveredMessage, ProtocolEvent};
use crate::tracker::StatusChange;

/// Retry policy: attempt 1 immediate, attempt 2 after 500ms, attempt 3 after 1000ms.
const RETRY_DELAYS: [Duration; 2] = [
    Duration::from_millis(500),
    Duration::from_millis(1000),
];

/// Execute a list of effects using the given transport and channels.
pub(super) async fn execute_effects<T: Transport>(
    effects: Vec<RuntimeEffect>,
    transport: &T,
    msg_tx: &mpsc::Sender<DeliveredMessage>,
    status_tx: &mpsc::Sender<StatusChange>,
    event_tx: &mpsc::Sender<ProtocolEvent>,
    metrics: &ProtocolMetrics,
) {
    tracing::trace!("execute_effects: {} effects to process", effects.len());
    for (i, effect) in effects.into_iter().enumerate() {
        match effect {
            RuntimeEffect::SendEnvelope(ref envelope) => {
                let target = envelope.via.first().copied().unwrap_or(envelope.to);
                tracing::trace!("  effect[{}]: SendEnvelope to {}", i, target);
                send_envelope(transport, envelope, event_tx, metrics).await;
                tracing::trace!("  effect[{}]: SendEnvelope done", i);
            }
            RuntimeEffect::SendEnvelopeTo { target, ref envelope } => {
                tracing::trace!("  effect[{}]: SendEnvelopeTo {}", i, target);
                send_envelope_to(transport, target, envelope, event_tx, metrics).await;
                tracing::trace!("  effect[{}]: SendEnvelopeTo done", i);
            }
            RuntimeEffect::DeliverMessage(msg) => {
                // try_send: never block runtime, even with large buffer (4096)
                // Consumer is responsible for draining fast enough
                let _ = msg_tx.try_send(msg);
            }
            RuntimeEffect::StatusChange(change) => {
                let _ = status_tx.try_send(change);
            }
            RuntimeEffect::Emit(event) => {
                // try_send even for critical events: large buffer + fast consumer = reliable
                let _ = event_tx.try_send(event);
            }
            RuntimeEffect::BroadcastRoleChange(announce) => {
                // Handled in the runtime loop (needs gossip sender).
                // This arm is a fallback — log if reached.
                tracing::debug!(
                    "BroadcastRoleChange reached executor (should be intercepted by loop): {:?} -> {:?}",
                    announce.node_id,
                    announce.new_role,
                );
            }
            RuntimeEffect::SendWithBackupFallback {
                ref envelope,
                on_success,
                on_failure,
            } => {
                let target = envelope.via.first().copied().unwrap_or(envelope.to);
                tracing::trace!("  effect[{}]: SendWithBackupFallback to {}", i, target);
                let sent_ok = match envelope.to_bytes() {
                    Ok(bytes) => send_with_retry(transport, target, &bytes).await,
                    Err(_) => false,
                };
                if sent_ok {
                    metrics.inc_messages_sent();
                    Box::pin(execute_effects(
                        on_success, transport, msg_tx, status_tx, event_tx, metrics,
                    ))
                    .await;
                } else {
                    metrics.inc_messages_failed();
                    Box::pin(execute_effects(
                        on_failure, transport, msg_tx, status_tx, event_tx, metrics,
                    ))
                    .await;
                }
            }
        }
    }
}

/// Send an envelope to its first hop (relay or direct to envelope.to).
async fn send_envelope<T: Transport>(
    transport: &T,
    envelope: &Envelope,
    event_tx: &mpsc::Sender<ProtocolEvent>,
    metrics: &ProtocolMetrics,
) {
    let target = envelope.via.first().copied().unwrap_or(envelope.to);
    send_envelope_to(transport, target, envelope, event_tx, metrics).await;
}

/// Send an envelope to a specific node with retry + backoff.
///
/// Attempt 1: immediate. Attempt 2: +500ms. Attempt 3: +1000ms.
/// Serialization errors are NOT retried (permanent failures).
async fn send_envelope_to<T: Transport>(
    transport: &T,
    target: NodeId,
    envelope: &Envelope,
    event_tx: &mpsc::Sender<ProtocolEvent>,
    metrics: &ProtocolMetrics,
) {
    let bytes = match envelope.to_bytes() {
        Ok(b) => b,
        Err(e) => {
            metrics.inc_messages_failed();
            let _ = event_tx
                .send(ProtocolEvent::Error {
                    description: format!("serialize envelope failed: {e}"),
                })
                .await;
            return;
        }
    };

    // First attempt (immediate)
    let mut last_err = match transport.send_raw(target, &bytes).await {
        Ok(()) => {
            metrics.inc_messages_sent();
            tracing::trace!("send_envelope_to {}: OK (first attempt)", target);
            return;
        }
        Err(e) => {
            tracing::warn!("send_envelope_to {}: first attempt FAILED: {}", target, e);
            e
        }
    };

    // Retry attempts with backoff
    for (i, delay) in RETRY_DELAYS.iter().enumerate() {
        tracing::debug!("send to {target} retry {} after {}ms (prev: {last_err})", i + 1, delay.as_millis());
        tokio::time::sleep(*delay).await;

        match transport.send_raw(target, &bytes).await {
            Ok(()) => {
                metrics.inc_messages_sent();
                return;
            }
            Err(e) => last_err = e,
        }
    }

    // All retries exhausted
    metrics.inc_messages_failed();
    let _ = event_tx
        .send(ProtocolEvent::Error {
            description: format!("send to {target} failed after {} attempts: {last_err}", 1 + RETRY_DELAYS.len()),
        })
        .await;
}

/// Raw send with retry (for SendWithBackupFallback). Returns true on success.
async fn send_with_retry<T: Transport>(transport: &T, target: NodeId, bytes: &[u8]) -> bool {
    if transport.send_raw(target, bytes).await.is_ok() {
        return true;
    }

    for delay in &RETRY_DELAYS {
        tokio::time::sleep(*delay).await;
        if transport.send_raw(target, bytes).await.is_ok() {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::transport::mock::MockTransport;
    use crate::types::NodeId;

    fn test_node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[tokio::test]
    async fn send_with_retry_succeeds_immediately() {
        let transport = MockTransport::new();
        let target = test_node_id(1);

        let ok = send_with_retry(&transport, target, b"hello").await;
        assert!(ok);
        assert_eq!(*transport.send_attempts.lock().unwrap(), 1);
        assert_eq!(transport.sent().len(), 1);
    }

    #[tokio::test]
    async fn send_with_retry_recovers_after_failures() {
        let transport = MockTransport::new();
        transport.set_fail_count(2); // fail twice, then succeed
        let target = test_node_id(1);

        let ok = send_with_retry(&transport, target, b"hello").await;
        assert!(ok);
        assert_eq!(*transport.send_attempts.lock().unwrap(), 3); // 1 + 2 retries
        assert_eq!(transport.sent().len(), 1);
    }

    #[tokio::test]
    async fn send_with_retry_fails_after_all_attempts() {
        let transport = MockTransport::new();
        transport.set_fail_sends(true); // permanent failure
        let target = test_node_id(1);

        let ok = send_with_retry(&transport, target, b"hello").await;
        assert!(!ok);
        assert_eq!(*transport.send_attempts.lock().unwrap(), 3); // 1 + 2 retries
        assert!(transport.sent().is_empty());
    }

    #[tokio::test]
    async fn send_envelope_to_retries_on_transient_failure() {
        let transport = MockTransport::new();
        transport.set_fail_count(1); // fail once, then succeed
        let target = test_node_id(1);
        let (event_tx, mut event_rx) = mpsc::channel(16);
        let metrics = ProtocolMetrics::new();

        let envelope = crate::envelope::EnvelopeBuilder::new(
            test_node_id(10),
            target,
            crate::types::MessageType::Chat,
            b"test".to_vec(),
        )
        .build();

        send_envelope_to(&transport, target, &envelope, &event_tx, &metrics).await;

        // Should have retried and succeeded — no error event
        assert!(event_rx.try_recv().is_err());
        assert_eq!(*transport.send_attempts.lock().unwrap(), 2);
        assert_eq!(transport.sent().len(), 1);
        assert_eq!(metrics.snapshot().messages_sent, 1);
        assert_eq!(metrics.snapshot().messages_failed, 0);
    }

    #[tokio::test]
    async fn send_envelope_to_emits_error_after_all_retries() {
        let transport = MockTransport::new();
        transport.set_fail_sends(true);
        let target = test_node_id(1);
        let (event_tx, mut event_rx) = mpsc::channel(16);
        let metrics = ProtocolMetrics::new();

        let envelope = crate::envelope::EnvelopeBuilder::new(
            test_node_id(10),
            target,
            crate::types::MessageType::Chat,
            b"test".to_vec(),
        )
        .build();

        send_envelope_to(&transport, target, &envelope, &event_tx, &metrics).await;

        // Should have error event after 3 attempts
        let event = event_rx.try_recv().unwrap();
        match event {
            ProtocolEvent::Error { description } => {
                assert!(description.contains("failed after 3 attempts"), "got: {description}");
            }
            other => panic!("expected Error event, got: {other:?}"),
        }
        assert_eq!(*transport.send_attempts.lock().unwrap(), 3);
        assert_eq!(metrics.snapshot().messages_sent, 0);
        assert_eq!(metrics.snapshot().messages_failed, 1);
    }
}
