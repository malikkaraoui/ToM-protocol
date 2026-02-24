//! Effect executor — the only place that touches I/O.
//!
//! Takes a list of RuntimeEffect and executes them concretely:
//! - SendEnvelope / SendEnvelopeTo -> transport.send_raw()
//! - DeliverMessage -> msg_tx.send()
//! - StatusChange -> status_tx.send()
//! - Emit -> event_tx.send()
//! - SendWithBackupFallback -> try send, execute on_success or on_failure

use tokio::sync::mpsc;

use crate::envelope::Envelope;
use crate::types::NodeId;

use super::effect::RuntimeEffect;
use super::transport::Transport;
use super::{DeliveredMessage, ProtocolEvent};
use crate::tracker::StatusChange;

/// Execute a list of effects using the given transport and channels.
pub(super) async fn execute_effects<T: Transport>(
    effects: Vec<RuntimeEffect>,
    transport: &T,
    msg_tx: &mpsc::Sender<DeliveredMessage>,
    status_tx: &mpsc::Sender<StatusChange>,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    for effect in effects {
        match effect {
            RuntimeEffect::SendEnvelope(envelope) => {
                send_envelope(transport, &envelope, event_tx).await;
            }
            RuntimeEffect::SendEnvelopeTo { target, envelope } => {
                send_envelope_to(transport, target, &envelope, event_tx).await;
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
            RuntimeEffect::SendWithBackupFallback {
                envelope,
                on_success,
                on_failure,
            } => {
                let target = envelope.via.first().copied().unwrap_or(envelope.to);
                let sent_ok = match envelope.to_bytes() {
                    Ok(bytes) => transport.send_raw(target, &bytes).await.is_ok(),
                    Err(_) => false,
                };
                if sent_ok {
                    Box::pin(execute_effects(
                        on_success, transport, msg_tx, status_tx, event_tx,
                    ))
                    .await;
                } else {
                    Box::pin(execute_effects(
                        on_failure, transport, msg_tx, status_tx, event_tx,
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
) {
    let target = envelope.via.first().copied().unwrap_or(envelope.to);
    send_envelope_to(transport, target, envelope, event_tx).await;
}

/// Send an envelope to a specific node.
async fn send_envelope_to<T: Transport>(
    transport: &T,
    target: NodeId,
    envelope: &Envelope,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    match envelope.to_bytes() {
        Ok(bytes) => {
            if let Err(e) = transport.send_raw(target, &bytes).await {
                let _ = event_tx
                    .send(ProtocolEvent::Error {
                        description: format!("send to {target} failed: {e}"),
                    })
                    .await;
            }
        }
        Err(e) => {
            let _ = event_tx
                .send(ProtocolEvent::Error {
                    description: format!("serialize envelope failed: {e}"),
                })
                .await;
        }
    }
}
