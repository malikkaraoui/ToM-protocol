/// The protocol runtime event loop.
///
/// A single async task that owns all mutable protocol state and
/// multiplexes over transport events, application commands, and timers.
use tokio::sync::{broadcast, mpsc};
use tom_transport::{PathEvent, TomNode};

use crate::discovery::{DiscoveryEvent, HeartbeatTracker};
use crate::envelope::{Envelope, EnvelopeBuilder};
use crate::relay::{PeerInfo, PeerRole, PeerStatus, RelaySelector, Topology};
use crate::router::{AckType, ReadReceiptPayload, Router, RoutingAction};
use crate::tracker::{MessageTracker, StatusChange};
use crate::types::{MessageType, NodeId};

use super::{DeliveredMessage, ProtocolEvent, RuntimeCommand, RuntimeConfig};

/// Main event loop — owns all protocol state.
#[allow(clippy::too_many_arguments)]
pub(super) async fn runtime_loop(
    mut node: TomNode,
    local_id: NodeId,
    secret_seed: [u8; 32],
    config: RuntimeConfig,
    mut cmd_rx: mpsc::Receiver<RuntimeCommand>,
    msg_tx: mpsc::Sender<DeliveredMessage>,
    status_tx: mpsc::Sender<StatusChange>,
    event_tx: mpsc::Sender<ProtocolEvent>,
    mut path_rx: broadcast::Receiver<PathEvent>,
) {
    // ── Protocol state ──────────────────────────────────────────────
    let mut router = Router::new(local_id);
    let relay_selector = RelaySelector::new(local_id);
    let mut topology = Topology::new();
    let mut tracker = MessageTracker::new();
    let mut heartbeat = HeartbeatTracker::new();

    // ── Timers ──────────────────────────────────────────────────────
    let mut cache_cleanup = tokio::time::interval(config.cache_cleanup_interval);
    let mut tracker_cleanup = tokio::time::interval(config.tracker_cleanup_interval);
    let mut heartbeat_check = tokio::time::interval(config.heartbeat_interval);

    // Skip the immediate first tick on all intervals
    cache_cleanup.tick().await;
    tracker_cleanup.tick().await;
    heartbeat_check.tick().await;

    loop {
        tokio::select! {
            // ── 1. Incoming data from transport ─────────────────
            result = node.recv_raw() => {
                match result {
                    Ok((_transport_from, data)) => {
                        handle_incoming(
                            &data,
                            local_id,
                            &secret_seed,
                            &config,
                            &mut router,
                            &mut tracker,
                            &mut heartbeat,
                            &mut topology,
                            &node,
                            &msg_tx,
                            &status_tx,
                            &event_tx,
                        ).await;
                    }
                    Err(e) => {
                        let _ = event_tx.send(ProtocolEvent::Error {
                            description: format!("recv error: {e}"),
                        }).await;
                    }
                }
            }

            // ── 2. Commands from application ────────────────────
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    RuntimeCommand::SendMessage { to, payload } => {
                        handle_send_message(
                            local_id,
                            &secret_seed,
                            &config,
                            to,
                            payload,
                            &relay_selector,
                            &topology,
                            &mut tracker,
                            &node,
                            &status_tx,
                            &event_tx,
                        ).await;
                    }
                    RuntimeCommand::SendReadReceipt { to, original_message_id } => {
                        handle_send_read_receipt(
                            local_id,
                            &secret_seed,
                            to,
                            original_message_id,
                            &relay_selector,
                            &topology,
                            &node,
                        ).await;
                    }
                    RuntimeCommand::AddPeer { node_id } => {
                        // Register peer in topology + iroh discovery handles the rest
                        heartbeat.record_heartbeat(node_id);
                        topology.upsert(PeerInfo {
                            node_id,
                            role: PeerRole::Peer,
                            status: PeerStatus::Online,
                            last_seen: now_ms(),
                        });
                    }
                    RuntimeCommand::UpsertPeer { info } => {
                        heartbeat.record_heartbeat(info.node_id);
                        topology.upsert(info);
                    }
                    RuntimeCommand::RemovePeer { node_id } => {
                        topology.remove(&node_id);
                        heartbeat.untrack_peer(&node_id);
                    }
                    RuntimeCommand::GetConnectedPeers { reply } => {
                        let peers = node.connected_peers().await;
                        let _ = reply.send(peers);
                    }
                    RuntimeCommand::Shutdown => {
                        break;
                    }
                }
            }

            // ── 3. Path events from transport ───────────────────
            Ok(event) = path_rx.recv() => {
                let _ = event_tx.send(ProtocolEvent::PathChanged {
                    event,
                }).await;
            }

            // ── 4. Timer: router cache cleanup ──────────────────
            _ = cache_cleanup.tick() => {
                router.cleanup_caches();
            }

            // ── 5. Timer: tracker eviction ──────────────────────
            _ = tracker_cleanup.tick() => {
                tracker.evict_expired();
            }

            // ── 6. Timer: heartbeat liveness check ──────────────
            _ = heartbeat_check.tick() => {
                let events = heartbeat.check_all(&mut topology);
                for disc_event in events {
                    match disc_event {
                        DiscoveryEvent::PeerOffline { node_id } => {
                            let _ = event_tx.send(ProtocolEvent::PeerOffline {
                                node_id,
                            }).await;
                        }
                        DiscoveryEvent::PeerDiscovered { node_id, .. } => {
                            let _ = event_tx.send(ProtocolEvent::PeerDiscovered {
                                node_id,
                            }).await;
                        }
                        _ => {} // PeerStale — log or ignore for MVP
                    }
                }
                heartbeat.cleanup_departed();
            }

            else => break,
        }
    }

    // Graceful shutdown
    if let Err(e) = node.shutdown().await {
        tracing::warn!("runtime shutdown error: {e}");
    }
}

// ── Inbound handler ─────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn handle_incoming(
    data: &[u8],
    _local_id: NodeId,
    secret_seed: &[u8; 32],
    _config: &RuntimeConfig,
    router: &mut Router,
    tracker: &mut MessageTracker,
    heartbeat: &mut HeartbeatTracker,
    topology: &mut Topology,
    node: &TomNode,
    msg_tx: &mpsc::Sender<DeliveredMessage>,
    status_tx: &mpsc::Sender<StatusChange>,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    // 1. Deserialize
    let envelope = match Envelope::from_bytes(data) {
        Ok(e) => e,
        Err(e) => {
            tracing::debug!("bad envelope: {e}");
            return;
        }
    };

    // 2. Verify signature
    let signature_valid = if envelope.is_signed() {
        envelope.verify_signature().is_ok()
    } else {
        false
    };

    // 3. Record heartbeat (sender is alive) + auto-register in topology
    heartbeat.record_heartbeat(envelope.from);
    if topology.get(&envelope.from).is_none() {
        topology.upsert(PeerInfo {
            node_id: envelope.from,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: now_ms(),
        });
    }

    // 4. Route
    let action = router.route(envelope);

    match action {
        RoutingAction::Deliver { mut envelope, response } => {
            // Decrypt if encrypted
            let was_encrypted = envelope.encrypted;
            if envelope.encrypted {
                if let Err(e) = envelope.decrypt_payload(secret_seed) {
                    tracing::warn!("decrypt failed from {}: {e}", envelope.from);
                    let _ = event_tx
                        .send(ProtocolEvent::Error {
                            description: format!("decrypt failed from {}: {e}", envelope.from),
                        })
                        .await;
                    return;
                }
            }

            // Deliver to application
            let _ = msg_tx
                .send(DeliveredMessage {
                    from: envelope.from,
                    payload: envelope.payload,
                    envelope_id: envelope.id,
                    timestamp: envelope.timestamp,
                    signature_valid,
                    was_encrypted,
                })
                .await;

            // Send delivery ACK back (sign it first)
            let mut ack = response;
            ack.sign(secret_seed);
            send_envelope(node, &ack, event_tx).await;
        }

        RoutingAction::Forward {
            envelope,
            next_hop,
            relay_ack,
        } => {
            let envelope_id = envelope.id.clone();
            let sender = envelope.from;

            // Forward the envelope to next_hop
            send_envelope_to(node, next_hop, &envelope, event_tx).await;

            // Send relay ACK back to original sender
            let mut ack = relay_ack;
            ack.sign(secret_seed);
            send_envelope_to(node, sender, &ack, event_tx).await;

            let _ = event_tx
                .send(ProtocolEvent::Forwarded {
                    envelope_id,
                    next_hop,
                })
                .await;
        }

        RoutingAction::Ack {
            original_message_id,
            ack_type,
            ..
        } => {
            let change = match ack_type {
                AckType::RelayForwarded => tracker.mark_relayed(&original_message_id),
                AckType::RecipientReceived => tracker.mark_delivered(&original_message_id),
            };
            if let Some(change) = change {
                let _ = status_tx.send(change).await;
            }
        }

        RoutingAction::ReadReceipt {
            original_message_id,
            ..
        } => {
            if let Some(change) = tracker.mark_read(&original_message_id) {
                let _ = status_tx.send(change).await;
            }
        }

        RoutingAction::Reject { reason } => {
            let _ = event_tx
                .send(ProtocolEvent::MessageRejected { reason })
                .await;
        }

        RoutingAction::Drop => {
            // Silently ignore (duplicate or expired)
        }
    }
}

// ── Outbound handlers ───────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn handle_send_message(
    local_id: NodeId,
    secret_seed: &[u8; 32],
    config: &RuntimeConfig,
    to: NodeId,
    payload: Vec<u8>,
    relay_selector: &RelaySelector,
    topology: &Topology,
    tracker: &mut MessageTracker,
    node: &TomNode,
    status_tx: &mpsc::Sender<StatusChange>,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    // 1. Select relay path
    let via = relay_selector.select_path(to, topology);

    // 2. Build envelope
    let builder = EnvelopeBuilder::new(local_id, to, MessageType::Chat, payload).via(via.clone());

    let envelope = if config.encryption {
        let recipient_pk = to.as_bytes();
        match builder.encrypt_and_sign(secret_seed, &recipient_pk) {
            Ok(env) => env,
            Err(e) => {
                let _ = event_tx
                    .send(ProtocolEvent::Error {
                        description: format!("encrypt failed for {to}: {e}"),
                    })
                    .await;
                return;
            }
        }
    } else {
        builder.sign(secret_seed)
    };

    // 3. Track the message
    let envelope_id = envelope.id.clone();
    if let Some(change) = tracker.track(envelope_id.clone(), to) {
        let _ = status_tx.send(change).await;
    }

    // 4. Determine first hop
    let first_hop = via.first().copied().unwrap_or(to);

    // 5. Send via transport
    match envelope.to_bytes() {
        Ok(bytes) => match node.send_raw(first_hop, &bytes).await {
            Ok(()) => {
                if let Some(change) = tracker.mark_sent(&envelope_id) {
                    let _ = status_tx.send(change).await;
                }
            }
            Err(e) => {
                let _ = event_tx
                    .send(ProtocolEvent::Error {
                        description: format!("send to {first_hop} failed: {e}"),
                    })
                    .await;
            }
        },
        Err(e) => {
            let _ = event_tx
                .send(ProtocolEvent::Error {
                    description: format!("serialize failed: {e}"),
                })
                .await;
        }
    }
}

async fn handle_send_read_receipt(
    local_id: NodeId,
    secret_seed: &[u8; 32],
    to: NodeId,
    original_message_id: String,
    relay_selector: &RelaySelector,
    topology: &Topology,
    node: &TomNode,
) {
    let payload = ReadReceiptPayload {
        original_message_id,
        read_at: now_ms(),
    }
    .to_bytes();

    let via = relay_selector.select_path(to, topology);
    let envelope =
        EnvelopeBuilder::new(local_id, to, MessageType::ReadReceipt, payload)
            .via(via)
            .sign(secret_seed);

    // Best-effort send, no tracking for read receipts
    if let Ok(bytes) = envelope.to_bytes() {
        let first_hop = envelope.via.first().copied().unwrap_or(to);
        let _ = node.send_raw(first_hop, &bytes).await;
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Send an envelope to its first hop (relay or direct).
async fn send_envelope(
    node: &TomNode,
    envelope: &Envelope,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    let first_hop = envelope.via.first().copied().unwrap_or(envelope.to);
    send_envelope_to(node, first_hop, envelope, event_tx).await;
}

/// Send an envelope to a specific node.
async fn send_envelope_to(
    node: &TomNode,
    target: NodeId,
    envelope: &Envelope,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    match envelope.to_bytes() {
        Ok(bytes) => {
            if let Err(e) = node.send_raw(target, &bytes).await {
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

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
