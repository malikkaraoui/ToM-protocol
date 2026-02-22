/// The protocol runtime event loop.
///
/// A single async task that owns all mutable protocol state and
/// multiplexes over transport events, application commands, and timers.
use tokio::sync::{broadcast, mpsc};
use tom_transport::{PathEvent, TomNode};

use crate::backup::{BackupAction, BackupCoordinator, BackupEvent};
use crate::discovery::{EphemeralSubnetManager, HeartbeatTracker, DiscoveryEvent, PeerAnnounce, SubnetEvent};
use crate::roles::RoleManager;
use crate::envelope::{Envelope, EnvelopeBuilder};
use crate::group::{
    GroupAction, GroupEvent, GroupHub, GroupManager, GroupMessage, GroupPayload,
};
use crate::relay::{PeerInfo, PeerRole, PeerStatus, RelaySelector, Topology};
use crate::router::{AckType, ReadReceiptPayload, Router, RoutingAction};
use crate::tracker::{MessageTracker, StatusChange};
use crate::types::{MessageType, NodeId};

use iroh_gossip::Gossip;
use iroh_gossip::api::Event as GossipEvent;
use n0_future::StreamExt;

use super::{DeliveredMessage, ProtocolEvent, RuntimeCommand, RuntimeConfig};

/// Fixed gossip topic for ToM peer discovery (all nodes share this).
const TOM_GOSSIP_TOPIC: [u8; 32] = *b"tom-protocol-gossip-discovery-v1";

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
    gossip: Gossip,
    gossip_bootstrap_peers: Vec<NodeId>,
) {
    // ── Protocol state ──────────────────────────────────────────────
    let mut router = Router::new(local_id);
    let relay_selector = RelaySelector::new(local_id);
    let mut topology = Topology::new();
    let mut tracker = MessageTracker::new();
    let mut heartbeat = HeartbeatTracker::new();

    // ── Group state ─────────────────────────────────────────────────
    let mut group_manager = GroupManager::new(local_id, config.username.clone());
    let mut group_hub = GroupHub::new(local_id);

    // ── Backup state ────────────────────────────────────────────────
    let mut backup = BackupCoordinator::new(local_id);

    // ── Discovery state ───────────────────────────────────────────────
    let mut subnets = EphemeralSubnetManager::new(local_id);
    let mut role_manager = RoleManager::new(local_id);
    let mut local_roles = vec![PeerRole::Peer];

    // ── Timers ──────────────────────────────────────────────────────
    let mut cache_cleanup = tokio::time::interval(config.cache_cleanup_interval);
    let mut tracker_cleanup = tokio::time::interval(config.tracker_cleanup_interval);
    let mut heartbeat_check = tokio::time::interval(config.heartbeat_interval);
    let mut group_hub_heartbeat = tokio::time::interval(config.group_hub_heartbeat_interval);
    let mut backup_tick = tokio::time::interval(config.backup_tick_interval);

    let mut gossip_announce = tokio::time::interval(config.gossip_announce_interval);
    let mut subnet_eval = tokio::time::interval(std::time::Duration::from_secs(30));
    let mut role_eval = tokio::time::interval(std::time::Duration::from_secs(60));

    // Skip the immediate first tick on all intervals
    cache_cleanup.tick().await;
    tracker_cleanup.tick().await;
    heartbeat_check.tick().await;
    group_hub_heartbeat.tick().await;
    backup_tick.tick().await;
    gossip_announce.tick().await;
    subnet_eval.tick().await;
    role_eval.tick().await;

    // ── Gossip subscription ──────────────────────────────────────────
    let topic_id = iroh_gossip::TopicId::from_bytes(TOM_GOSSIP_TOPIC);
    let bootstrap: Vec<iroh::EndpointId> = gossip_bootstrap_peers
        .iter()
        .map(|n| *n.as_endpoint_id())
        .collect();

    let (gossip_sender, mut gossip_receiver) = match gossip.subscribe(topic_id, bootstrap).await {
        Ok(topic) => {
            let (s, r) = topic.split();
            tracing::info!("gossip: subscribed to discovery topic");
            (Some(s), Some(r))
        }
        Err(e) => {
            tracing::warn!("gossip: subscription failed: {e}");
            (None, None)
        }
    };

    loop {
        tokio::select! {
            // ── 1. Incoming data from transport ─────────────────
            result = node.recv_raw() => {
                match result {
                    Ok((_transport_from, data)) => {
                        // Parse envelope
                        let envelope = match Envelope::from_bytes(&data) {
                            Ok(e) => e,
                            Err(e) => {
                                tracing::debug!("bad envelope: {e}");
                                continue;
                            }
                        };

                        // Verify signature (common to all types)
                        let signature_valid = if envelope.is_signed() {
                            envelope.verify_signature().is_ok()
                        } else {
                            false
                        };

                        // Record heartbeat + auto-register (common to all types)
                        heartbeat.record_heartbeat(envelope.from);
                        if topology.get(&envelope.from).is_none() {
                            topology.upsert(PeerInfo {
                                node_id: envelope.from,
                                role: PeerRole::Peer,
                                status: PeerStatus::Online,
                                last_seen: now_ms(),
                            });
                        }

                        // Dispatch by message type
                        match envelope.msg_type {
                            MessageType::Chat
                            | MessageType::Ack
                            | MessageType::ReadReceipt
                            | MessageType::Heartbeat => {
                                // Record communication for subnet formation
                                if envelope.msg_type == MessageType::Chat {
                                    subnets.record_communication(envelope.from, local_id, now_ms());
                                }
                                handle_incoming_chat(
                                    envelope,
                                    signature_valid,
                                    &secret_seed,
                                    &mut router,
                                    &mut tracker,
                                    &mut role_manager,
                                    &node,
                                    &msg_tx,
                                    &status_tx,
                                    &event_tx,
                                ).await;
                            }

                            MessageType::GroupCreate
                            | MessageType::GroupCreated
                            | MessageType::GroupInvite
                            | MessageType::GroupJoin
                            | MessageType::GroupSync
                            | MessageType::GroupMessage
                            | MessageType::GroupLeave
                            | MessageType::GroupMemberJoined
                            | MessageType::GroupMemberLeft
                            | MessageType::GroupHubMigration
                            | MessageType::GroupDeliveryAck
                            | MessageType::GroupHubHeartbeat => {
                                handle_incoming_group(
                                    envelope,
                                    local_id,
                                    &secret_seed,
                                    &mut group_manager,
                                    &mut group_hub,
                                    &relay_selector,
                                    &topology,
                                    &node,
                                    &event_tx,
                                ).await;
                            }

                            MessageType::BackupStore
                            | MessageType::BackupDeliver
                            | MessageType::BackupReplicate
                            | MessageType::BackupReplicateAck
                            | MessageType::BackupQuery
                            | MessageType::BackupQueryResponse
                            | MessageType::BackupConfirmDelivery => {
                                handle_incoming_backup(
                                    &envelope,
                                    local_id,
                                    &secret_seed,
                                    &mut backup,
                                    &relay_selector,
                                    &topology,
                                    &node,
                                    &event_tx,
                                ).await;
                            }

                            MessageType::PeerAnnounce => {
                                // Direct QUIC peer announce (heartbeat + topology already handled above)
                                if let Ok(announce) = rmp_serde::from_slice::<PeerAnnounce>(&envelope.payload) {
                                    if announce.is_timestamp_valid(now_ms()) {
                                        let _ = event_tx.send(ProtocolEvent::PeerAnnounceReceived {
                                            node_id: announce.node_id,
                                            username: announce.username,
                                        }).await;
                                    }
                                }
                            }
                        }
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
                        subnets.record_communication(local_id, to, now_ms());
                        handle_send_message(
                            local_id,
                            &secret_seed,
                            &config,
                            to,
                            payload,
                            &relay_selector,
                            &topology,
                            &mut tracker,
                            &mut backup,
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

                    // ── Group commands ───────────────────────────
                    RuntimeCommand::CreateGroup { name, hub_relay_id, initial_members } => {
                        let actions = group_manager.create_group(name, hub_relay_id, initial_members);
                        execute_group_actions(
                            &actions, local_id, &secret_seed,
                            &relay_selector, &topology, &node, &event_tx,
                        ).await;
                    }
                    RuntimeCommand::AcceptInvite { group_id } => {
                        let actions = group_manager.accept_invite(&group_id);
                        execute_group_actions(
                            &actions, local_id, &secret_seed,
                            &relay_selector, &topology, &node, &event_tx,
                        ).await;
                    }
                    RuntimeCommand::DeclineInvite { group_id } => {
                        group_manager.decline_invite(&group_id);
                    }
                    RuntimeCommand::LeaveGroup { group_id } => {
                        let actions = group_manager.leave_group(&group_id);
                        execute_group_actions(
                            &actions, local_id, &secret_seed,
                            &relay_selector, &topology, &node, &event_tx,
                        ).await;
                    }
                    RuntimeCommand::SendGroupMessage { group_id, text } => {
                        handle_send_group_message(
                            local_id,
                            &secret_seed,
                            &config,
                            &group_manager,
                            group_id,
                            text,
                            &relay_selector,
                            &topology,
                            &node,
                            &event_tx,
                        ).await;
                    }
                    RuntimeCommand::GetGroups { reply } => {
                        let groups = group_manager.all_groups().into_iter().cloned().collect();
                        let _ = reply.send(groups);
                    }
                    RuntimeCommand::GetPendingInvites { reply } => {
                        let invites = group_manager.pending_invites().into_iter().cloned().collect();
                        let _ = reply.send(invites);
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
                            let subnet_events = subnets.remove_node(&node_id);
                            for se in &subnet_events {
                                surface_subnet_event(se, &event_tx).await;
                            }
                            role_manager.remove_node(&node_id);
                            let _ = event_tx.send(ProtocolEvent::PeerOffline {
                                node_id,
                            }).await;
                        }
                        DiscoveryEvent::PeerDiscovered { node_id, .. } => {
                            let _ = event_tx.send(ProtocolEvent::PeerDiscovered {
                                node_id,
                            }).await;

                            // Deliver backed-up messages for this peer
                            deliver_backups_for_peer(
                                node_id,
                                local_id,
                                &secret_seed,
                                &config,
                                &mut backup,
                                &relay_selector,
                                &topology,
                                &mut tracker,
                                &node,
                                &status_tx,
                                &event_tx,
                            ).await;
                        }
                        _ => {} // PeerStale — log or ignore for MVP
                    }
                }
                heartbeat.cleanup_departed();
            }

            // ── 7. Timer: group hub heartbeat ───────────────────
            _ = group_hub_heartbeat.tick() => {
                let actions = group_hub.heartbeat_actions();
                execute_group_actions(
                    &actions, local_id, &secret_seed,
                    &relay_selector, &topology, &node, &event_tx,
                ).await;
            }

            // ── 8. Timer: backup maintenance ────────────────────
            _ = backup_tick.tick() => {
                let actions = backup.tick(now_ms());
                execute_backup_actions(
                    &actions, local_id, &secret_seed,
                    &relay_selector, &topology, &node, &event_tx,
                ).await;
            }

            // ── 9. Gossip events ─────────────────────────────────
            event = async {
                match gossip_receiver.as_mut() {
                    Some(rx) => rx.next().await,
                    None => std::future::pending::<Option<_>>().await,
                }
            } => {
                if let Some(Ok(event)) = event {
                    match event {
                        GossipEvent::Received(msg) => {
                            if let Ok(announce) = rmp_serde::from_slice::<PeerAnnounce>(&msg.content) {
                                if announce.is_timestamp_valid(now_ms()) {
                                    let peer_id = announce.node_id;
                                    let role = if announce.roles.contains(&PeerRole::Relay) {
                                        PeerRole::Relay
                                    } else {
                                        PeerRole::Peer
                                    };
                                    heartbeat.record_heartbeat(peer_id);
                                    topology.upsert(PeerInfo {
                                        node_id: peer_id,
                                        role,
                                        status: PeerStatus::Online,
                                        last_seen: now_ms(),
                                    });
                                    let _ = event_tx.send(ProtocolEvent::PeerAnnounceReceived {
                                        node_id: peer_id,
                                        username: announce.username,
                                    }).await;
                                }
                            }
                        }
                        GossipEvent::NeighborUp(endpoint_id) => {
                            let node_id = NodeId::from_endpoint_id(endpoint_id);
                            heartbeat.record_heartbeat(node_id);
                            topology.upsert(PeerInfo {
                                node_id,
                                role: PeerRole::Peer,
                                status: PeerStatus::Online,
                                last_seen: now_ms(),
                            });
                            let _ = event_tx.send(ProtocolEvent::GossipNeighborUp {
                                node_id,
                            }).await;

                            // Re-broadcast our announce on NeighborUp
                            // (key learning from PoC-3: initial broadcast has no neighbors)
                            if let Some(ref sender) = gossip_sender {
                                let announce = PeerAnnounce::new(
                                    local_id,
                                    config.username.clone(),
                                    local_roles.clone(),
                                );
                                if let Ok(bytes) = rmp_serde::to_vec(&announce) {
                                    let _ = sender.broadcast(bytes::Bytes::from(bytes)).await;
                                }
                            }
                        }
                        GossipEvent::NeighborDown(endpoint_id) => {
                            let node_id = NodeId::from_endpoint_id(endpoint_id);
                            let _ = event_tx.send(ProtocolEvent::GossipNeighborDown {
                                node_id,
                            }).await;
                        }
                        GossipEvent::Lagged => {
                            tracing::warn!("gossip: receiver lagged, missed events");
                        }
                    }
                }
            }

            // ── 10. Timer: subnet evaluation ──────────────────────
            _ = subnet_eval.tick() => {
                let events = subnets.evaluate(now_ms());
                for event in &events {
                    surface_subnet_event(event, &event_tx).await;
                }
            }

            // ── 12. Timer: role evaluation ──────────────────────
            _ = role_eval.tick() => {
                let actions = role_manager.evaluate(&mut topology, now_ms());
                for action in &actions {
                    surface_role_action(action, &mut local_roles, &event_tx).await;
                }
            }

            // ── 11. Timer: gossip announce ────────────────────────
            _ = gossip_announce.tick() => {
                if let Some(ref sender) = gossip_sender {
                    let announce = PeerAnnounce::new(
                        local_id,
                        config.username.clone(),
                        local_roles.clone(),
                    );
                    if let Ok(bytes) = rmp_serde::to_vec(&announce) {
                        if let Err(e) = sender.broadcast(bytes::Bytes::from(bytes)).await {
                            tracing::debug!("gossip: announce broadcast failed: {e}");
                        }
                    }
                }
            }

            else => break,
        }
    }

    // Graceful shutdown
    if let Err(e) = node.shutdown().await {
        tracing::warn!("runtime shutdown error: {e}");
    }
}

// ── Chat inbound handler ────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn handle_incoming_chat(
    envelope: Envelope,
    signature_valid: bool,
    secret_seed: &[u8; 32],
    router: &mut Router,
    tracker: &mut MessageTracker,
    role_manager: &mut RoleManager,
    node: &TomNode,
    msg_tx: &mpsc::Sender<DeliveredMessage>,
    status_tx: &mpsc::Sender<StatusChange>,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    let action = router.route(envelope);

    match action {
        RoutingAction::Deliver { mut envelope, response } => {
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

            // We are relaying this message — count toward our local score.
            // Use envelope.from as key so each unique sender adds to relay count.
            role_manager.record_relay(sender, now_ms());

            send_envelope_to(node, next_hop, &envelope, event_tx).await;

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

        RoutingAction::Drop => {}
    }
}

// ── Group inbound handler ───────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn handle_incoming_group(
    mut envelope: Envelope,
    local_id: NodeId,
    secret_seed: &[u8; 32],
    group_manager: &mut GroupManager,
    group_hub: &mut GroupHub,
    relay_selector: &RelaySelector,
    topology: &Topology,
    node: &TomNode,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    // Decrypt if needed
    if envelope.encrypted {
        if let Err(e) = envelope.decrypt_payload(secret_seed) {
            tracing::debug!("group decrypt failed: {e}");
            return;
        }
    }

    // Deserialize GroupPayload
    let group_payload: GroupPayload = match rmp_serde::from_slice(&envelope.payload) {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!("bad group payload: {e}");
            return;
        }
    };

    // Dispatch: hub-bound messages go to GroupHub, member-bound go to GroupManager.
    // For Message and DeliveryAck, we check if we actually host the group —
    // if not, route to GroupManager (we're a member receiving fan-out from the hub).
    let actions = match group_payload {
        // Always hub-bound
        GroupPayload::Create { .. }
        | GroupPayload::Join { .. }
        | GroupPayload::Leave { .. } => {
            group_hub.handle_payload(group_payload, envelope.from)
        }

        // Message: hub if we host the group, member otherwise
        GroupPayload::Message(ref msg) => {
            if group_hub.get_group(&msg.group_id).is_some() {
                group_hub.handle_payload(group_payload, envelope.from)
            } else {
                let GroupPayload::Message(msg) = group_payload else { unreachable!() };
                group_manager.handle_message(msg)
            }
        }

        // DeliveryAck: hub if we host the group, ignore otherwise
        GroupPayload::DeliveryAck { ref group_id, .. } => {
            if group_hub.get_group(group_id).is_some() {
                group_hub.handle_payload(group_payload, envelope.from)
            } else {
                vec![]
            }
        }

        // Member-bound (we are a group member)
        GroupPayload::Created { group } => {
            group_manager.handle_group_created(group)
        }
        GroupPayload::Invite {
            group_id,
            group_name,
            inviter_id,
            inviter_username,
        } => group_manager.handle_invite(
            group_id,
            group_name,
            inviter_id,
            inviter_username,
            envelope.from,
        ),
        GroupPayload::Sync {
            group,
            recent_messages,
        } => group_manager.handle_group_sync(group, recent_messages),
        GroupPayload::MemberJoined { group_id, member } => {
            group_manager.handle_member_joined(&group_id, member)
        }
        GroupPayload::MemberLeft {
            group_id,
            node_id,
            username,
            reason,
        } => group_manager.handle_member_left(&group_id, &node_id, username, reason),
        GroupPayload::HubMigration {
            group_id,
            new_hub_id,
            ..
        } => group_manager.handle_hub_migration(&group_id, new_hub_id),
        GroupPayload::HubHeartbeat { .. } => vec![],
    };

    execute_group_actions(
        &actions, local_id, secret_seed, relay_selector, topology, node, event_tx,
    )
    .await;
}

// ── Backup inbound handler ──────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn handle_incoming_backup(
    envelope: &Envelope,
    local_id: NodeId,
    secret_seed: &[u8; 32],
    backup: &mut BackupCoordinator,
    relay_selector: &RelaySelector,
    topology: &Topology,
    node: &TomNode,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    let now = now_ms();

    match envelope.msg_type {
        MessageType::BackupReplicate | MessageType::BackupStore | MessageType::BackupDeliver => {
            let payload: crate::backup::ReplicationPayload =
                match rmp_serde::from_slice(&envelope.payload) {
                    Ok(p) => p,
                    Err(_) => return,
                };
            let actions = backup.handle_replication(&payload, envelope.from, now);
            execute_backup_actions(
                &actions, local_id, secret_seed, relay_selector, topology, node, event_tx,
            )
            .await;
        }

        MessageType::BackupReplicateAck => {
            let message_id: String = match rmp_serde::from_slice(&envelope.payload) {
                Ok(p) => p,
                Err(_) => return,
            };
            let actions = backup.handle_replication_ack(&message_id, envelope.from);
            execute_backup_actions(
                &actions, local_id, secret_seed, relay_selector, topology, node, event_tx,
            )
            .await;
        }

        MessageType::BackupQuery => {
            let recipient_id: NodeId = match rmp_serde::from_slice(&envelope.payload) {
                Ok(p) => p,
                Err(_) => return,
            };
            let local_msgs = backup.store().get_for_recipient(&recipient_id);
            if !local_msgs.is_empty() {
                let ids: Vec<String> = local_msgs.iter().map(|m| m.message_id.clone()).collect();
                let response_bytes =
                    rmp_serde::to_vec(&ids).expect("backup query response serialization");
                let response = EnvelopeBuilder::new(
                    local_id,
                    envelope.from,
                    MessageType::BackupQueryResponse,
                    response_bytes,
                )
                .sign(secret_seed);
                send_envelope(node, &response, event_tx).await;
            }
        }

        MessageType::BackupQueryResponse => {
            let message_ids: Vec<String> = match rmp_serde::from_slice(&envelope.payload) {
                Ok(p) => p,
                Err(_) => return,
            };
            let _new_ids = backup.handle_query_response(&envelope.from, &message_ids, now);
        }

        MessageType::BackupConfirmDelivery => {
            let message_ids: Vec<String> = match rmp_serde::from_slice(&envelope.payload) {
                Ok(p) => p,
                Err(_) => return,
            };
            let actions = backup.handle_delivery_confirmation(&message_ids);
            execute_backup_actions(
                &actions, local_id, secret_seed, relay_selector, topology, node, event_tx,
            )
            .await;
        }

        _ => {}
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
    backup: &mut BackupCoordinator,
    node: &TomNode,
    status_tx: &mpsc::Sender<StatusChange>,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    let via = relay_selector.select_path(to, topology);

    let builder =
        EnvelopeBuilder::new(local_id, to, MessageType::Chat, payload.clone()).via(via.clone());

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

    let envelope_id = envelope.id.clone();
    if let Some(change) = tracker.track(envelope_id.clone(), to) {
        let _ = status_tx.send(change).await;
    }

    let first_hop = via.first().copied().unwrap_or(to);

    match envelope.to_bytes() {
        Ok(bytes) => match node.send_raw(first_hop, &bytes).await {
            Ok(()) => {
                if let Some(change) = tracker.mark_sent(&envelope_id) {
                    let _ = status_tx.send(change).await;
                }
            }
            Err(e) => {
                // Send failed — store backup for offline recipient
                let backup_actions = backup.store_message(
                    envelope_id.clone(),
                    payload,
                    to,
                    local_id,
                    now_ms(),
                    None,
                );
                execute_backup_actions(
                    &backup_actions,
                    local_id,
                    secret_seed,
                    relay_selector,
                    topology,
                    node,
                    event_tx,
                )
                .await;

                let _ = event_tx
                    .send(ProtocolEvent::Error {
                        description: format!("send to {first_hop} failed (backed up): {e}"),
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

#[allow(clippy::too_many_arguments)]
async fn handle_send_group_message(
    local_id: NodeId,
    secret_seed: &[u8; 32],
    config: &RuntimeConfig,
    group_manager: &GroupManager,
    group_id: crate::group::GroupId,
    text: String,
    relay_selector: &RelaySelector,
    topology: &Topology,
    node: &TomNode,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    let Some(group) = group_manager.get_group(&group_id) else {
        let _ = event_tx
            .send(ProtocolEvent::Error {
                description: format!("not a member of group {group_id}"),
            })
            .await;
        return;
    };

    let hub_id = group.hub_relay_id;
    let mut msg = GroupMessage::new(group_id, local_id, config.username.clone(), text);
    msg.sign(secret_seed);
    let payload = GroupPayload::Message(msg);
    let payload_bytes = rmp_serde::to_vec(&payload).expect("group msg serialization");

    let via = relay_selector.select_path(hub_id, topology);
    let envelope = EnvelopeBuilder::new(local_id, hub_id, MessageType::GroupMessage, payload_bytes)
        .via(via)
        .sign(secret_seed);
    send_envelope(node, &envelope, event_tx).await;
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
    let envelope = EnvelopeBuilder::new(local_id, to, MessageType::ReadReceipt, payload)
        .via(via)
        .sign(secret_seed);

    if let Ok(bytes) = envelope.to_bytes() {
        let first_hop = envelope.via.first().copied().unwrap_or(to);
        let _ = node.send_raw(first_hop, &bytes).await;
    }
}

// ── Backup delivery on peer reconnect ───────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn deliver_backups_for_peer(
    peer_id: NodeId,
    local_id: NodeId,
    secret_seed: &[u8; 32],
    config: &RuntimeConfig,
    backup: &mut BackupCoordinator,
    relay_selector: &RelaySelector,
    topology: &Topology,
    tracker: &mut MessageTracker,
    node: &TomNode,
    status_tx: &mpsc::Sender<StatusChange>,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    let entries: Vec<(String, Vec<u8>)> = backup
        .store()
        .get_for_recipient(&peer_id)
        .into_iter()
        .map(|e| (e.message_id.clone(), e.payload.clone()))
        .collect();

    if entries.is_empty() {
        return;
    }

    let mut delivered_ids = Vec::new();

    for (message_id, payload) in &entries {
        let via = relay_selector.select_path(peer_id, topology);
        let builder =
            EnvelopeBuilder::new(local_id, peer_id, MessageType::Chat, payload.clone())
                .via(via.clone());

        let envelope = if config.encryption {
            let recipient_pk = peer_id.as_bytes();
            match builder.encrypt_and_sign(secret_seed, &recipient_pk) {
                Ok(env) => env,
                Err(_) => continue,
            }
        } else {
            builder.sign(secret_seed)
        };

        let envelope_id = envelope.id.clone();
        if let Some(change) = tracker.track(envelope_id.clone(), peer_id) {
            let _ = status_tx.send(change).await;
        }

        let first_hop = via.first().copied().unwrap_or(peer_id);
        if let Ok(bytes) = envelope.to_bytes() {
            if node.send_raw(first_hop, &bytes).await.is_ok() {
                if let Some(change) = tracker.mark_sent(&envelope_id) {
                    let _ = status_tx.send(change).await;
                }
                delivered_ids.push(message_id.clone());

                let _ = event_tx
                    .send(ProtocolEvent::BackupDelivered {
                        message_id: message_id.clone(),
                        recipient_id: peer_id,
                    })
                    .await;
            }
        }
    }

    if !delivered_ids.is_empty() {
        let actions = backup.confirm_delivery(&delivered_ids, peer_id);
        execute_backup_actions(
            &actions, local_id, secret_seed, relay_selector, topology, node, event_tx,
        )
        .await;
    }
}

// ── Group action executor ───────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn execute_group_actions(
    actions: &[GroupAction],
    local_id: NodeId,
    secret_seed: &[u8; 32],
    relay_selector: &RelaySelector,
    topology: &Topology,
    node: &TomNode,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    for action in actions {
        match action {
            GroupAction::Send { to, payload } => {
                let msg_type = group_payload_to_message_type(payload);
                let payload_bytes =
                    rmp_serde::to_vec(payload).expect("group payload serialization");
                let via = relay_selector.select_path(*to, topology);
                let envelope =
                    EnvelopeBuilder::new(local_id, *to, msg_type, payload_bytes)
                        .via(via)
                        .sign(secret_seed);
                send_envelope(node, &envelope, event_tx).await;
            }
            GroupAction::Broadcast { to, payload } => {
                let msg_type = group_payload_to_message_type(payload);
                let payload_bytes =
                    rmp_serde::to_vec(payload).expect("group payload serialization");
                for target in to {
                    let via = relay_selector.select_path(*target, topology);
                    let envelope = EnvelopeBuilder::new(
                        local_id,
                        *target,
                        msg_type,
                        payload_bytes.clone(),
                    )
                    .via(via)
                    .sign(secret_seed);
                    send_envelope(node, &envelope, event_tx).await;
                }
            }
            GroupAction::Event(event) => {
                surface_group_event(event, event_tx).await;
            }
            GroupAction::None => {}
        }
    }
}

// ── Backup action executor ──────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn execute_backup_actions(
    actions: &[BackupAction],
    local_id: NodeId,
    secret_seed: &[u8; 32],
    relay_selector: &RelaySelector,
    topology: &Topology,
    node: &TomNode,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    for action in actions {
        match action {
            BackupAction::Replicate { target, payload } => {
                let bytes =
                    rmp_serde::to_vec(payload).expect("backup replication serialization");
                let via = relay_selector.select_path(*target, topology);
                let envelope = EnvelopeBuilder::new(
                    local_id,
                    *target,
                    MessageType::BackupReplicate,
                    bytes,
                )
                .via(via)
                .sign(secret_seed);
                send_envelope(node, &envelope, event_tx).await;
            }
            BackupAction::ConfirmDelivery {
                message_ids,
                recipient_id: _,
            } => {
                let bytes =
                    rmp_serde::to_vec(message_ids).expect("backup confirm serialization");
                for peer in topology.peers() {
                    if peer.node_id != local_id && peer.status == PeerStatus::Online {
                        let envelope = EnvelopeBuilder::new(
                            local_id,
                            peer.node_id,
                            MessageType::BackupConfirmDelivery,
                            bytes.clone(),
                        )
                        .sign(secret_seed);
                        send_envelope(node, &envelope, event_tx).await;
                    }
                }
            }
            BackupAction::QueryPending { recipient_id } => {
                let bytes =
                    rmp_serde::to_vec(recipient_id).expect("backup query serialization");
                for peer in topology.peers() {
                    if peer.node_id != local_id && peer.status == PeerStatus::Online {
                        let envelope = EnvelopeBuilder::new(
                            local_id,
                            peer.node_id,
                            MessageType::BackupQuery,
                            bytes.clone(),
                        )
                        .sign(secret_seed);
                        send_envelope(node, &envelope, event_tx).await;
                    }
                }
            }
            BackupAction::Event(event) => {
                surface_backup_event(event, event_tx).await;
            }
        }
    }
}

// ── Event surfacing ─────────────────────────────────────────────────

async fn surface_group_event(
    event: &GroupEvent,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    let proto_event = match event {
        GroupEvent::GroupCreated(info) => ProtocolEvent::GroupCreated {
            group: info.clone(),
        },
        GroupEvent::InviteReceived(invite) => ProtocolEvent::GroupInviteReceived {
            invite: invite.clone(),
        },
        GroupEvent::Joined {
            group_id,
            group_name,
        } => ProtocolEvent::GroupJoined {
            group_id: group_id.clone(),
            group_name: group_name.clone(),
        },
        GroupEvent::MemberJoined { group_id, member } => ProtocolEvent::GroupMemberJoined {
            group_id: group_id.clone(),
            member: member.clone(),
        },
        GroupEvent::MemberLeft {
            group_id,
            node_id,
            username,
            reason,
        } => ProtocolEvent::GroupMemberLeft {
            group_id: group_id.clone(),
            node_id: *node_id,
            username: username.clone(),
            reason: *reason,
        },
        GroupEvent::MessageReceived(msg) => ProtocolEvent::GroupMessageReceived {
            message: msg.clone(),
        },
        GroupEvent::HubMigrated {
            group_id,
            new_hub_id,
        } => ProtocolEvent::GroupHubMigrated {
            group_id: group_id.clone(),
            new_hub_id: *new_hub_id,
        },
        GroupEvent::SecurityViolation {
            group_id,
            node_id,
            reason,
        } => ProtocolEvent::GroupSecurityViolation {
            group_id: group_id.clone(),
            node_id: *node_id,
            reason: reason.clone(),
        },
    };
    let _ = event_tx.send(proto_event).await;
}

async fn surface_backup_event(
    event: &BackupEvent,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    let proto_event = match event {
        BackupEvent::MessageStored {
            message_id,
            recipient_id,
        } => Some(ProtocolEvent::BackupStored {
            message_id: message_id.clone(),
            recipient_id: *recipient_id,
        }),
        BackupEvent::MessageDelivered {
            message_id,
            recipient_id,
        } => Some(ProtocolEvent::BackupDelivered {
            message_id: message_id.clone(),
            recipient_id: *recipient_id,
        }),
        BackupEvent::MessageExpired {
            message_id,
            recipient_id,
        } => Some(ProtocolEvent::BackupExpired {
            message_id: message_id.clone(),
            recipient_id: *recipient_id,
        }),
        // Internal events — don't surface to application
        BackupEvent::ReplicationNeeded { .. }
        | BackupEvent::SelfDeleteRecommended { .. }
        | BackupEvent::MessageReplicated { .. } => None,
    };

    if let Some(event) = proto_event {
        let _ = event_tx.send(event).await;
    }
}

async fn surface_subnet_event(
    event: &SubnetEvent,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    let proto_event = match event {
        SubnetEvent::SubnetFormed { subnet } => Some(ProtocolEvent::SubnetFormed {
            subnet_id: subnet.subnet_id.clone(),
            members: subnet.members.iter().copied().collect(),
        }),
        SubnetEvent::SubnetDissolved { subnet_id, reason } => {
            Some(ProtocolEvent::SubnetDissolved {
                subnet_id: subnet_id.clone(),
                reason: format!("{reason:?}"),
            })
        }
        // NodeJoined/Left are internal bookkeeping
        _ => None,
    };

    if let Some(event) = proto_event {
        let _ = event_tx.send(event).await;
    }
}

async fn surface_role_action(
    action: &crate::roles::RoleAction,
    local_roles: &mut Vec<PeerRole>,
    event_tx: &mpsc::Sender<ProtocolEvent>,
) {
    use crate::roles::RoleAction;
    let proto_event = match action {
        RoleAction::Promoted { node_id, score } => ProtocolEvent::RolePromoted {
            node_id: *node_id,
            score: *score,
        },
        RoleAction::Demoted { node_id, score } => ProtocolEvent::RoleDemoted {
            node_id: *node_id,
            score: *score,
        },
        RoleAction::LocalRoleChanged { new_role } => {
            // Update local_roles for gossip announces
            *local_roles = vec![*new_role];
            ProtocolEvent::LocalRoleChanged {
                new_role: *new_role,
            }
        }
    };
    let _ = event_tx.send(proto_event).await;
}

// ── Helpers ─────────────────────────────────────────────────────────

fn group_payload_to_message_type(payload: &GroupPayload) -> MessageType {
    match payload {
        GroupPayload::Create { .. } => MessageType::GroupCreate,
        GroupPayload::Created { .. } => MessageType::GroupCreated,
        GroupPayload::Invite { .. } => MessageType::GroupInvite,
        GroupPayload::Join { .. } => MessageType::GroupJoin,
        GroupPayload::Sync { .. } => MessageType::GroupSync,
        GroupPayload::Message(_) => MessageType::GroupMessage,
        GroupPayload::Leave { .. } => MessageType::GroupLeave,
        GroupPayload::MemberJoined { .. } => MessageType::GroupMemberJoined,
        GroupPayload::MemberLeft { .. } => MessageType::GroupMemberLeft,
        GroupPayload::DeliveryAck { .. } => MessageType::GroupDeliveryAck,
        GroupPayload::HubMigration { .. } => MessageType::GroupHubMigration,
        GroupPayload::HubHeartbeat { .. } => MessageType::GroupHubHeartbeat,
    }
}

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
