use crate::backup::{BackupAction, BackupCoordinator, BackupEvent};
use crate::discovery::{
    DiscoveryEvent, DiscoverySource, EphemeralSubnetManager, HeartbeatTracker, PeerAnnounce,
    SubnetEvent,
};
use crate::envelope::{Envelope, EnvelopeBuilder};
use crate::group::{
    GroupAction, GroupEvent, GroupHub, GroupManager, GroupMessage, GroupPayload,
};
use crate::relay::{PeerInfo, PeerRole, PeerStatus, RelaySelector, Topology};
use crate::roles::{RoleAction, RoleManager};
use crate::router::{AckType, ReadReceiptPayload, Router, RoutingAction};
use crate::tracker::MessageTracker;
use crate::types::{now_ms, MessageType, NodeId};

use super::effect::RuntimeEffect;
use super::{DeliveredMessage, ProtocolEvent, RuntimeCommand, RuntimeConfig};

/// Gossip event input for RuntimeState (avoids leaking iroh_gossip types).
pub enum GossipInput {
    /// A peer announced itself via gossip.
    PeerAnnounce(Vec<u8>),
    /// A gossip neighbor connected.
    NeighborUp(NodeId),
    /// A gossip neighbor disconnected.
    NeighborDown(NodeId),
}

/// Map a GroupPayload variant to its corresponding MessageType.
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
        GroupPayload::SenderKeyDistribution { .. } => MessageType::GroupSenderKeyDistribution,
        GroupPayload::HubPing { .. } => MessageType::GroupHubPing,
        GroupPayload::HubPong { .. } => MessageType::GroupHubPong,
        GroupPayload::HubShadowSync { .. } => MessageType::GroupHubShadowSync,
        GroupPayload::CandidateAssigned { .. } => MessageType::GroupCandidateAssigned,
        GroupPayload::HubUnreachable { .. } => MessageType::GroupHubUnreachable,
    }
}

/// Etat complet du protocole — logique pure, zero async, zero reseau.
///
/// Chaque methode handle_* / tick_* retourne Vec<RuntimeEffect>.
/// Aucune methode ne touche au reseau ni aux channels.
pub struct RuntimeState {
    pub(crate) local_id: NodeId,
    pub(crate) secret_seed: [u8; 32],
    pub(crate) config: RuntimeConfig,

    // Protocol modules
    pub(crate) router: Router,
    pub(crate) relay_selector: RelaySelector,
    pub(crate) topology: Topology,
    pub(crate) tracker: MessageTracker,
    pub(crate) heartbeat: HeartbeatTracker,

    // Group
    pub(crate) group_manager: GroupManager,
    pub(crate) group_hub: GroupHub,

    // Backup
    pub(crate) backup: BackupCoordinator,

    // Discovery
    pub(crate) subnets: EphemeralSubnetManager,
    pub(crate) role_manager: RoleManager,
    pub(crate) local_roles: Vec<PeerRole>,

    /// Throttle role announcements (max 1 per peer per 30s).
    role_announce_throttle: std::collections::HashMap<NodeId, u64>,
}

impl RuntimeState {
    /// Creer un nouvel etat de protocole.
    pub fn new(local_id: NodeId, secret_seed: [u8; 32], config: RuntimeConfig) -> Self {
        Self {
            router: Router::new(local_id),
            relay_selector: RelaySelector::new(local_id),
            topology: Topology::new(),
            tracker: MessageTracker::new(),
            heartbeat: HeartbeatTracker::new(),
            group_manager: GroupManager::new(local_id, config.username.clone()),
            group_hub: GroupHub::new(local_id),
            backup: BackupCoordinator::new(local_id),
            subnets: EphemeralSubnetManager::new(local_id),
            role_manager: RoleManager::new(local_id),
            local_roles: vec![PeerRole::Peer],
            role_announce_throttle: std::collections::HashMap::new(),
            local_id,
            secret_seed,
            config,
        }
    }

    // ── Tick: cache cleanup ──────────────────────────────────────────────

    /// Purge expired entries from the router dedup / ACK caches.
    pub fn tick_cache_cleanup(&mut self) -> Vec<RuntimeEffect> {
        self.router.cleanup_caches();
        Vec::new()
    }

    // ── Tick: tracker cleanup ────────────────────────────────────────────

    /// Evict expired message status entries from the tracker.
    pub fn tick_tracker_cleanup(&mut self) -> Vec<RuntimeEffect> {
        self.tracker.evict_expired();
        Vec::new()
    }

    // ── Tick: heartbeat liveness check ───────────────────────────────────

    /// Check all peers for liveness, handle all 4 discovery events.
    ///
    /// - PeerDiscovered: new peer first seen (with source tracking).
    /// - PeerStale: missed heartbeats but might recover.
    /// - PeerOffline: remove from subnets + role_manager, emit event.
    /// - PeerOnline: reconnect after stale/offline, prepare backup delivery.
    pub fn tick_heartbeat(&mut self) -> Vec<RuntimeEffect> {
        let mut effects = Vec::new();

        let events = self.heartbeat.check_all(&mut self.topology);
        for disc_event in events {
            match disc_event {
                DiscoveryEvent::PeerDiscovered { node_id, username, source } => {
                    effects.push(RuntimeEffect::Emit(ProtocolEvent::PeerDiscovered {
                        node_id,
                        username,
                        source,
                    }));
                }
                DiscoveryEvent::PeerStale { node_id } => {
                    effects.push(RuntimeEffect::Emit(ProtocolEvent::PeerStale {
                        node_id,
                    }));
                }
                DiscoveryEvent::PeerOffline { node_id } => {
                    let subnet_events = self.subnets.remove_node(&node_id);
                    for se in &subnet_events {
                        effects.extend(self.surface_subnet_event(se));
                    }
                    self.role_manager.remove_node(&node_id);
                    effects.push(RuntimeEffect::Emit(ProtocolEvent::PeerOffline {
                        node_id,
                    }));
                }
                DiscoveryEvent::PeerOnline { node_id } => {
                    effects.push(RuntimeEffect::Emit(ProtocolEvent::PeerOnline {
                        node_id,
                    }));
                    effects.extend(self.prepare_backup_delivery(node_id));
                }
            }
        }

        self.heartbeat.cleanup_departed();
        effects
    }

    // ── Tick: subnet evaluation ──────────────────────────────────────────

    /// Evaluate communication patterns and form/dissolve ephemeral subnets.
    pub fn tick_subnets(&mut self) -> Vec<RuntimeEffect> {
        let events = self.subnets.evaluate(now_ms());
        let mut effects = Vec::new();
        for event in &events {
            effects.extend(self.surface_subnet_event(event));
        }
        effects
    }

    // ── Tick: role evaluation ────────────────────────────────────────────

    /// Evaluate contribution scores and promote/demote peers.
    pub fn tick_roles(&mut self) -> Vec<RuntimeEffect> {
        let actions = self.role_manager.evaluate(&mut self.topology, now_ms());
        let mut effects = Vec::new();
        for action in &actions {
            effects.extend(self.surface_role_action(action));
        }
        effects
    }

    // ── Tick: backup maintenance ─────────────────────────────────────────

    /// Run periodic backup maintenance (expire, viability, replication cleanup).
    pub fn tick_backup(&mut self) -> Vec<RuntimeEffect> {
        let actions = self.backup.tick(now_ms());
        self.backup_actions_to_effects(&actions)
    }

    // ── Tick: group hub heartbeat ────────────────────────────────────────

    /// Send heartbeat probes to all group members (hub-side).
    pub fn tick_group_hub_heartbeat(&mut self) -> Vec<RuntimeEffect> {
        let actions = self.group_hub.heartbeat_actions();
        self.group_actions_to_effects(&actions)
    }

    // ── Tick: shadow ping watchdog ──────────────────────────────────────

    /// Shadow watchdog tick — send HubPing to primary for each group we shadow.
    pub fn tick_shadow_ping(&mut self) -> Vec<RuntimeEffect> {
        let shadow_groups: Vec<(crate::group::GroupId, NodeId)> = self
            .group_manager
            .shadow_groups()
            .into_iter()
            .map(|(gid, hub)| (gid.clone(), hub))
            .collect();

        let mut effects = Vec::new();
        for (group_id, hub_id) in shadow_groups {
            let payload = GroupPayload::HubPing {
                group_id: group_id.clone(),
            };
            let payload_bytes =
                rmp_serde::to_vec(&payload).expect("group payload serialization");
            let via = self.relay_selector.select_path(hub_id, &self.topology);
            let envelope = EnvelopeBuilder::new(
                self.local_id,
                hub_id,
                MessageType::GroupHubPing,
                payload_bytes,
            )
            .via(via)
            .sign(&self.secret_seed);
            effects.push(RuntimeEffect::SendEnvelope(envelope));
        }
        effects
    }

    // ── Gossip announce builder ──────────────────────────────────────────

    /// Build a PeerAnnounce and serialize it to MessagePack bytes.
    ///
    /// Returns `None` if serialization fails (should never happen).
    pub fn build_gossip_announce(&self) -> Option<Vec<u8>> {
        let announce = PeerAnnounce::new(
            self.local_id,
            self.config.username.clone(),
            self.local_roles.clone(),
        );
        rmp_serde::to_vec(&announce).ok()
    }

    // ── Handle incoming role change announcement ──────────────────────

    /// Handle incoming role change announcement from gossip.
    ///
    /// Validates signature, throttles spam, updates topology.
    pub fn handle_role_announce(
        &mut self,
        announce: crate::discovery::RoleChangeAnnounce,
    ) -> Vec<RuntimeEffect> {
        let now = now_ms();

        // Throttle: max 1 announce per peer per 30s
        const THROTTLE_MS: u64 = 30_000;
        if let Some(&last_announce) = self.role_announce_throttle.get(&announce.node_id) {
            if now.saturating_sub(last_announce) < THROTTLE_MS {
                return Vec::new();
            }
        }

        // Verify signature
        if !announce.verify_signature() {
            return vec![RuntimeEffect::Emit(ProtocolEvent::Error {
                description: format!(
                    "Invalid signature on role announce from {}",
                    announce.node_id
                ),
            })];
        }

        // Update topology
        if let Some(peer) = self.topology.get(&announce.node_id) {
            let mut updated_peer = peer.clone();
            updated_peer.role = announce.new_role;
            updated_peer.last_seen = announce.timestamp;
            self.topology.upsert(updated_peer);
        } else {
            self.topology.upsert(PeerInfo {
                node_id: announce.node_id,
                role: announce.new_role,
                status: PeerStatus::Online,
                last_seen: announce.timestamp,
            });
        }

        // Update throttle
        self.role_announce_throttle.insert(announce.node_id, now);

        // Emit event for observability
        let event = match announce.new_role {
            PeerRole::Relay => ProtocolEvent::RolePromoted {
                node_id: announce.node_id,
                score: announce.score,
            },
            PeerRole::Peer => ProtocolEvent::RoleDemoted {
                node_id: announce.node_id,
                score: announce.score,
            },
        };

        vec![RuntimeEffect::Emit(event)]
    }

    // ── Task 7: handle_incoming_chat ───────────────────────────────────

    /// Handle an incoming Chat / Ack / ReadReceipt / Heartbeat envelope.
    ///
    /// Routes through the Router, then converts the RoutingAction into effects:
    /// - Deliver: decrypt if needed, produce DeliverMessage + ACK envelope
    /// - Forward: record relay score, forward to next_hop, send relay ACK
    /// - Ack: update tracker status
    /// - ReadReceipt: update tracker status
    /// - Reject: emit error event
    /// - Drop: nothing (dedup)
    pub fn handle_incoming_chat(
        &mut self,
        envelope: Envelope,
        signature_valid: bool,
    ) -> Vec<RuntimeEffect> {
        let action = self.router.route(envelope);

        match action {
            RoutingAction::Deliver {
                mut envelope,
                response,
            } => {
                let was_encrypted = envelope.encrypted;
                if envelope.encrypted {
                    if let Err(e) = envelope.decrypt_payload(&self.secret_seed) {
                        return vec![RuntimeEffect::Emit(ProtocolEvent::Error {
                            description: format!(
                                "decrypt failed from {}: {e}",
                                envelope.from
                            ),
                        })];
                    }
                }

                let mut effects = vec![RuntimeEffect::DeliverMessage(DeliveredMessage {
                    from: envelope.from,
                    payload: envelope.payload,
                    envelope_id: envelope.id,
                    timestamp: envelope.timestamp,
                    signature_valid,
                    was_encrypted,
                })];

                let mut ack = response;
                ack.sign(&self.secret_seed);
                effects.push(RuntimeEffect::SendEnvelope(ack));

                effects
            }

            RoutingAction::Forward {
                envelope,
                next_hop,
                relay_ack,
            } => {
                let envelope_id = envelope.id.clone();
                let sender = envelope.from;
                let now = now_ms();

                self.role_manager.record_relay(sender, now);

                // Track bandwidth: estimate size from serialized envelope
                let bytes = envelope
                    .to_bytes()
                    .map(|b| b.len() as u64)
                    .unwrap_or(0);
                if bytes > 0 {
                    self.role_manager.record_bytes_relayed(sender, bytes, now);
                }

                let mut ack = relay_ack;
                ack.sign(&self.secret_seed);

                vec![
                    RuntimeEffect::SendEnvelopeTo {
                        target: next_hop,
                        envelope,
                    },
                    RuntimeEffect::SendEnvelopeTo {
                        target: sender,
                        envelope: ack,
                    },
                    RuntimeEffect::Emit(ProtocolEvent::Forwarded {
                        envelope_id,
                        next_hop,
                    }),
                ]
            }

            RoutingAction::Ack {
                original_message_id,
                ack_type,
                ..
            } => {
                let change = match ack_type {
                    AckType::RelayForwarded => {
                        self.tracker.mark_relayed(&original_message_id)
                    }
                    AckType::RecipientReceived => {
                        self.tracker.mark_delivered(&original_message_id)
                    }
                };
                change
                    .into_iter()
                    .map(RuntimeEffect::StatusChange)
                    .collect()
            }

            RoutingAction::ReadReceipt {
                original_message_id,
                ..
            } => self
                .tracker
                .mark_read(&original_message_id)
                .into_iter()
                .map(RuntimeEffect::StatusChange)
                .collect(),

            RoutingAction::Reject { reason } => {
                vec![RuntimeEffect::Emit(ProtocolEvent::MessageRejected {
                    reason,
                })]
            }

            RoutingAction::Drop => Vec::new(),
        }
    }

    // ── Task 8: handle_incoming_group ────────────────────────────────────

    /// Handle an incoming group envelope (all Group* message types).
    ///
    /// Decrypts if needed, deserializes GroupPayload, dispatches to hub or
    /// member handler, then converts GroupActions to effects.
    pub fn handle_incoming_group(
        &mut self,
        mut envelope: Envelope,
    ) -> Vec<RuntimeEffect> {
        // Decrypt if needed
        if envelope.encrypted {
            if let Err(e) = envelope.decrypt_payload(&self.secret_seed) {
                return vec![RuntimeEffect::Emit(ProtocolEvent::Error {
                    description: format!("group decrypt failed: {e}"),
                })];
            }
        }

        // Deserialize GroupPayload
        let group_payload: GroupPayload = match rmp_serde::from_slice(&envelope.payload) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };

        // Dispatch: hub-bound messages go to GroupHub, member-bound go to GroupManager.
        let actions = match group_payload {
            // Always hub-bound — after handling, trigger shadow assignment
            GroupPayload::Create { .. }
            | GroupPayload::Join { .. }
            | GroupPayload::Leave { .. } => {
                // Extract group_id from Join/Leave before consuming; for Create we find it after.
                let known_group_id = match &group_payload {
                    GroupPayload::Join { group_id, .. }
                    | GroupPayload::Leave { group_id, .. } => Some(group_id.clone()),
                    _ => None,
                };

                let mut actions = self.group_hub.handle_payload(group_payload, envelope.from);

                // Determine the affected group_id (for Create, extract from the Created response)
                let group_id = known_group_id.or_else(|| {
                    actions.iter().find_map(|a| {
                        if let GroupAction::Send {
                            payload: GroupPayload::Created { group },
                            ..
                        } = a
                        {
                            Some(group.group_id.clone())
                        } else {
                            None
                        }
                    })
                });

                // Assign/update shadow for the affected group
                if let Some(gid) = group_id {
                    if self.group_hub.get_group(&gid).is_some() {
                        let shadow_actions = self.group_hub.assign_shadow(&gid);
                        actions.extend(shadow_actions);
                    }
                }

                actions
            }

            // Message: hub if we host the group, member otherwise
            GroupPayload::Message(ref msg) => {
                if self.group_hub.get_group(&msg.group_id).is_some() {
                    self.group_hub
                        .handle_payload(group_payload, envelope.from)
                } else {
                    let GroupPayload::Message(msg) = group_payload else {
                        unreachable!()
                    };
                    self.group_manager.handle_message(msg)
                }
            }

            // DeliveryAck: hub if we host the group, ignore otherwise
            GroupPayload::DeliveryAck { ref group_id, .. } => {
                if self.group_hub.get_group(group_id).is_some() {
                    self.group_hub
                        .handle_payload(group_payload, envelope.from)
                } else {
                    vec![]
                }
            }

            // Member-bound
            GroupPayload::Created { group } => {
                self.group_manager.handle_group_created(group)
            }
            GroupPayload::Invite {
                group_id,
                group_name,
                inviter_id,
                inviter_username,
            } => self.group_manager.handle_invite(
                group_id,
                group_name,
                inviter_id,
                inviter_username,
                envelope.from,
            ),
            GroupPayload::Sync {
                group,
                recent_messages,
            } => self
                .group_manager
                .handle_group_sync(group, recent_messages),
            GroupPayload::MemberJoined { group_id, member } => {
                self.group_manager
                    .handle_member_joined(&group_id, member)
            }
            GroupPayload::MemberLeft {
                group_id,
                node_id,
                username,
                reason,
            } => self.group_manager.handle_member_left(
                &group_id, &node_id, username, reason,
            ),
            GroupPayload::HubMigration {
                group_id,
                new_hub_id,
                ..
            } => self
                .group_manager
                .handle_hub_migration(&group_id, new_hub_id),
            GroupPayload::HubHeartbeat { .. } => vec![],

            // Shadow ping from shadow → primary responds with pong
            GroupPayload::HubPing { ref group_id } => {
                if self.group_hub.get_group(group_id).is_some() {
                    let actions = self.group_hub.handle_hub_ping(group_id, envelope.from);
                    return self.group_actions_to_effects(&actions);
                }
                vec![]
            }

            // Pong from primary → reset shadow ping failures
            GroupPayload::HubPong { ref group_id } => {
                self.group_manager.reset_ping_failures(group_id);
                vec![]
            }

            // Shadow sync from primary → store replicated state
            GroupPayload::HubShadowSync {
                ref group_id,
                ref members,
                candidate_id,
                config_version,
            } => {
                self.group_manager.handle_shadow_sync(
                    group_id,
                    members.clone(),
                    candidate_id,
                    config_version,
                )
            }

            // Candidate assignment
            GroupPayload::CandidateAssigned { ref group_id } => {
                return vec![RuntimeEffect::Emit(ProtocolEvent::GroupCandidateAssigned {
                    group_id: group_id.clone(),
                })];
            }

            // Member reports hub unreachable to shadow
            GroupPayload::HubUnreachable { ref group_id } => {
                self.group_manager.handle_hub_unreachable(group_id, envelope.from)
            }

            GroupPayload::SenderKeyDistribution {
                ref group_id,
                from,
                epoch,
                ref encrypted_keys,
            } => {
                if self.group_hub.get_group(group_id).is_some() {
                    // We're the hub — fan out to members
                    self.group_hub.handle_payload(group_payload, envelope.from)
                } else {
                    // We're a member — store the sender key
                    self.group_manager.handle_sender_key_distribution(
                        group_id,
                        from,
                        epoch,
                        encrypted_keys,
                        &self.secret_seed,
                    )
                }
            }
        };

        self.group_actions_to_effects(&actions)
    }

    // ── Task 8: handle_incoming_backup ───────────────────────────────────

    /// Handle an incoming backup envelope (all Backup* message types).
    pub fn handle_incoming_backup(
        &mut self,
        envelope: &Envelope,
    ) -> Vec<RuntimeEffect> {
        let now = now_ms();

        match envelope.msg_type {
            MessageType::BackupReplicate
            | MessageType::BackupStore
            | MessageType::BackupDeliver => {
                let payload: crate::backup::ReplicationPayload =
                    match rmp_serde::from_slice(&envelope.payload) {
                        Ok(p) => p,
                        Err(_) => return Vec::new(),
                    };
                let actions =
                    self.backup
                        .handle_replication(&payload, envelope.from, now);
                self.backup_actions_to_effects(&actions)
            }

            MessageType::BackupReplicateAck => {
                let message_id: String =
                    match rmp_serde::from_slice(&envelope.payload) {
                        Ok(p) => p,
                        Err(_) => return Vec::new(),
                    };
                let actions = self
                    .backup
                    .handle_replication_ack(&message_id, envelope.from);
                self.backup_actions_to_effects(&actions)
            }

            MessageType::BackupQuery => {
                let recipient_id: NodeId =
                    match rmp_serde::from_slice(&envelope.payload) {
                        Ok(p) => p,
                        Err(_) => return Vec::new(),
                    };
                let local_msgs =
                    self.backup.store().get_for_recipient(&recipient_id);
                if local_msgs.is_empty() {
                    return Vec::new();
                }
                let ids: Vec<String> =
                    local_msgs.iter().map(|m| m.message_id.clone()).collect();
                let response_bytes = rmp_serde::to_vec(&ids)
                    .expect("backup query response serialization");
                let response = EnvelopeBuilder::new(
                    self.local_id,
                    envelope.from,
                    MessageType::BackupQueryResponse,
                    response_bytes,
                )
                .sign(&self.secret_seed);
                vec![RuntimeEffect::SendEnvelope(response)]
            }

            MessageType::BackupQueryResponse => {
                let message_ids: Vec<String> =
                    match rmp_serde::from_slice(&envelope.payload) {
                        Ok(p) => p,
                        Err(_) => return Vec::new(),
                    };
                let _new_ids = self.backup.handle_query_response(
                    &envelope.from,
                    &message_ids,
                    now,
                );
                Vec::new()
            }

            MessageType::BackupConfirmDelivery => {
                let message_ids: Vec<String> =
                    match rmp_serde::from_slice(&envelope.payload) {
                        Ok(p) => p,
                        Err(_) => return Vec::new(),
                    };
                let actions =
                    self.backup.handle_delivery_confirmation(&message_ids);
                self.backup_actions_to_effects(&actions)
            }

            _ => Vec::new(),
        }
    }

    // ── Task 8: handle_peer_announce ─────────────────────────────────────

    /// Handle a direct QUIC PeerAnnounce envelope.
    ///
    /// Records heartbeat with Direct source so PeerDiscovered is emitted
    /// from the next tick_heartbeat call.
    pub fn handle_peer_announce(
        &mut self,
        envelope: &Envelope,
    ) -> Vec<RuntimeEffect> {
        if let Ok(announce) =
            rmp_serde::from_slice::<PeerAnnounce>(&envelope.payload)
        {
            if announce.is_timestamp_valid(now_ms()) {
                self.heartbeat.record_heartbeat_with_source(
                    announce.node_id,
                    DiscoverySource::Direct,
                    announce.username,
                );
                self.topology.upsert(PeerInfo {
                    node_id: announce.node_id,
                    role: PeerRole::Peer,
                    status: PeerStatus::Online,
                    last_seen: now_ms(),
                });
            }
        }
        Vec::new()
    }

    // ── Task 8: handle_incoming (unified dispatcher) ─────────────────────

    /// Unified entry point for all incoming raw data.
    ///
    /// Parses the envelope, verifies signature, auto-registers the peer,
    /// records heartbeat, then dispatches to the appropriate handler.
    pub fn handle_incoming(&mut self, raw_data: &[u8]) -> Vec<RuntimeEffect> {
        // Parse envelope
        let envelope = match Envelope::from_bytes(raw_data) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        // Verify signature
        let signature_valid = if envelope.is_signed() {
            envelope.verify_signature().is_ok()
        } else {
            false
        };

        // Record heartbeat + auto-register
        self.heartbeat.record_heartbeat(envelope.from);
        if self.topology.get(&envelope.from).is_none() {
            self.topology.upsert(PeerInfo {
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
                if envelope.msg_type == MessageType::Chat {
                    self.subnets.record_communication(
                        envelope.from,
                        self.local_id,
                        now_ms(),
                    );
                }
                self.handle_incoming_chat(envelope, signature_valid)
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
            | MessageType::GroupHubHeartbeat
            | MessageType::GroupSenderKeyDistribution
            | MessageType::GroupHubPing
            | MessageType::GroupHubPong
            | MessageType::GroupHubShadowSync
            | MessageType::GroupCandidateAssigned
            | MessageType::GroupHubUnreachable => {
                self.handle_incoming_group(envelope)
            }

            MessageType::BackupStore
            | MessageType::BackupDeliver
            | MessageType::BackupReplicate
            | MessageType::BackupReplicateAck
            | MessageType::BackupQuery
            | MessageType::BackupQueryResponse
            | MessageType::BackupConfirmDelivery => {
                self.handle_incoming_backup(&envelope)
            }

            MessageType::PeerAnnounce => self.handle_peer_announce(&envelope),
        }
    }

    // ── Task 9: handle_send_message ──────────────────────────────────────

    /// Build and send a chat message to a peer.
    ///
    /// Returns a SendWithBackupFallback effect: on success the tracker
    /// advances to Sent; on failure the message is stored as backup.
    pub fn handle_send_message(
        &mut self,
        to: NodeId,
        payload: Vec<u8>,
    ) -> Vec<RuntimeEffect> {
        let via = self.relay_selector.select_path(to, &self.topology);

        let builder = EnvelopeBuilder::new(
            self.local_id,
            to,
            MessageType::Chat,
            payload.clone(),
        )
        .via(via);

        let envelope = if self.config.encryption {
            let recipient_pk = to.as_bytes();
            match builder.encrypt_and_sign(&self.secret_seed, &recipient_pk) {
                Ok(env) => env,
                Err(e) => {
                    return vec![RuntimeEffect::Emit(ProtocolEvent::Error {
                        description: format!("encrypt failed for {to}: {e}"),
                    })];
                }
            }
        } else {
            builder.sign(&self.secret_seed)
        };

        let envelope_id = envelope.id.clone();

        // Track message in tracker
        let mut on_success = Vec::new();
        if let Some(change) = self.tracker.track(envelope_id.clone(), to) {
            on_success.push(RuntimeEffect::StatusChange(change));
        }

        // On success: mark as sent
        if let Some(change) = self.tracker.mark_sent(&envelope_id) {
            on_success.push(RuntimeEffect::StatusChange(change));
        }

        // On failure: store backup + emit error
        let backup_actions = self.backup.store_message(
            envelope_id.clone(),
            payload,
            to,
            self.local_id,
            now_ms(),
            None,
        );
        let mut on_failure = self.backup_actions_to_effects(&backup_actions);
        on_failure.push(RuntimeEffect::Emit(ProtocolEvent::Error {
            description: format!(
                "send to {} failed (backed up)",
                envelope.via.first().copied().unwrap_or(to)
            ),
        }));

        vec![RuntimeEffect::SendWithBackupFallback {
            envelope,
            on_success,
            on_failure,
        }]
    }

    // ── Task 9: handle_send_group_message ────────────────────────────────

    /// Build and send a text message to a group (via hub relay).
    pub fn handle_send_group_message(
        &mut self,
        group_id: crate::group::GroupId,
        text: String,
    ) -> Vec<RuntimeEffect> {
        let Some(group) = self.group_manager.get_group(&group_id) else {
            return vec![RuntimeEffect::Emit(ProtocolEvent::Error {
                description: format!("not a member of group {group_id}"),
            })];
        };

        let hub_id = group.hub_relay_id;

        // Build message — encrypted if we have a sender key, plaintext otherwise
        let mut msg = if let Some(sender_key) = self.group_manager.local_sender_key(&group_id) {
            let key = sender_key.key;
            let epoch = sender_key.epoch;
            GroupMessage::new_encrypted(
                group_id,
                self.local_id,
                self.config.username.clone(),
                text,
                &key,
                epoch,
            )
        } else {
            GroupMessage::new(
                group_id,
                self.local_id,
                self.config.username.clone(),
                text,
            )
        };

        msg.sign(&self.secret_seed);
        let payload = GroupPayload::Message(msg);
        let payload_bytes =
            rmp_serde::to_vec(&payload).expect("group msg serialization");

        let via = self.relay_selector.select_path(hub_id, &self.topology);
        let envelope = EnvelopeBuilder::new(
            self.local_id,
            hub_id,
            MessageType::GroupMessage,
            payload_bytes,
        )
        .via(via)
        .sign(&self.secret_seed);

        vec![RuntimeEffect::SendEnvelope(envelope)]
    }

    // ── Task 9: handle_send_read_receipt ─────────────────────────────────

    /// Build and send a read receipt for a previously received message.
    pub fn handle_send_read_receipt(
        &mut self,
        to: NodeId,
        original_message_id: String,
    ) -> Vec<RuntimeEffect> {
        let payload = ReadReceiptPayload {
            original_message_id,
            read_at: now_ms(),
        }
        .to_bytes();

        let via = self.relay_selector.select_path(to, &self.topology);
        let envelope = EnvelopeBuilder::new(
            self.local_id,
            to,
            MessageType::ReadReceipt,
            payload,
        )
        .via(via)
        .sign(&self.secret_seed);

        vec![RuntimeEffect::SendEnvelope(envelope)]
    }

    // ── Task 9: handle_command (unified dispatcher) ──────────────────────

    /// Unified command dispatcher — processes a RuntimeCommand and returns effects.
    ///
    /// Some commands (GetConnectedPeers, Shutdown) are handled in the loop
    /// because they need transport access; they return empty effects here.
    pub fn handle_command(
        &mut self,
        cmd: RuntimeCommand,
    ) -> Vec<RuntimeEffect> {
        match cmd {
            RuntimeCommand::SendMessage { to, payload } => {
                self.subnets
                    .record_communication(self.local_id, to, now_ms());
                self.handle_send_message(to, payload)
            }

            RuntimeCommand::SendGroupMessage { group_id, text } => {
                self.handle_send_group_message(group_id, text)
            }

            RuntimeCommand::SendReadReceipt {
                to,
                original_message_id,
            } => self.handle_send_read_receipt(to, original_message_id),

            RuntimeCommand::AddPeer { node_id } => {
                self.heartbeat.record_heartbeat_with_source(
                    node_id,
                    DiscoverySource::Direct,
                    String::new(),
                );
                self.topology.upsert(PeerInfo {
                    node_id,
                    role: PeerRole::Peer,
                    status: PeerStatus::Online,
                    last_seen: now_ms(),
                });
                Vec::new()
            }

            RuntimeCommand::UpsertPeer { info } => {
                self.heartbeat.record_heartbeat_with_source(
                    info.node_id,
                    DiscoverySource::Direct,
                    String::new(),
                );
                self.topology.upsert(info);
                Vec::new()
            }

            RuntimeCommand::RemovePeer { node_id } => {
                self.topology.remove(&node_id);
                self.heartbeat.untrack_peer(&node_id);
                Vec::new()
            }

            RuntimeCommand::CreateGroup {
                name,
                hub_relay_id,
                initial_members,
            } => {
                // If we ARE the hub, handle creation locally without network round-trip
                if hub_relay_id == self.local_id {
                    let payload = GroupPayload::Create {
                        group_name: name.clone(),
                        creator_username: self.config.username.clone(),
                        initial_members: initial_members.clone(),
                    };
                    let actions = self.group_hub.handle_payload(payload, self.local_id);
                    let mut effects = self.group_actions_to_effects(&actions);

                    // Also process the GroupCreated callback on the member side
                    // (since we're both hub and member)
                    if let Some((_, group_info)) = self.group_hub.groups().find(|(_, g)| g.name == name) {
                        let member_actions = self.group_manager.handle_group_created(group_info.clone());
                        effects.extend(self.group_actions_to_effects(&member_actions));
                    }

                    effects
                } else {
                    // Remote hub — send GroupPayload::Create over network
                    let actions = self.group_manager.create_group(
                        name,
                        hub_relay_id,
                        initial_members,
                    );
                    self.group_actions_to_effects(&actions)
                }
            }

            RuntimeCommand::AcceptInvite { group_id } => {
                let actions =
                    self.group_manager.accept_invite(&group_id);
                self.group_actions_to_effects(&actions)
            }

            RuntimeCommand::DeclineInvite { group_id } => {
                self.group_manager.decline_invite(&group_id);
                Vec::new()
            }

            RuntimeCommand::LeaveGroup { group_id } => {
                let actions =
                    self.group_manager.leave_group(&group_id);
                self.group_actions_to_effects(&actions)
            }

            RuntimeCommand::GetGroups { reply } => {
                let groups = self
                    .group_manager
                    .all_groups()
                    .into_iter()
                    .cloned()
                    .collect();
                let _ = reply.send(groups);
                Vec::new()
            }

            RuntimeCommand::GetPendingInvites { reply } => {
                let invites = self
                    .group_manager
                    .pending_invites()
                    .into_iter()
                    .cloned()
                    .collect();
                let _ = reply.send(invites);
                Vec::new()
            }

            RuntimeCommand::GetRoleMetrics { node_id, reply } => {
                let metrics =
                    self.role_manager
                        .get_metrics(&node_id, &self.topology, now_ms());
                let _ = reply.send(metrics);
                Vec::new()
            }

            RuntimeCommand::GetAllRoleScores { reply } => {
                let scores =
                    self.role_manager
                        .get_all_scores(&self.topology, now_ms());
                let _ = reply.send(scores);
                Vec::new()
            }

            // Handled in the loop — needs transport access.
            RuntimeCommand::GetConnectedPeers { .. } => Vec::new(),

            // Handled in the loop — signals the loop to break.
            RuntimeCommand::Shutdown => Vec::new(),
        }
    }

    // ── Task 10: handle_gossip_event ─────────────────────────────────────

    /// Handle a gossip event (peer announce, neighbor up/down).
    ///
    /// For NeighborUp, the state method returns effects but does NOT re-broadcast
    /// the gossip announce — that I/O is left to the loop.
    pub fn handle_gossip_event(
        &mut self,
        input: GossipInput,
    ) -> Vec<RuntimeEffect> {
        match input {
            GossipInput::PeerAnnounce(bytes) => {
                // Try PeerAnnounce first (most common)
                if let Ok(announce) =
                    rmp_serde::from_slice::<PeerAnnounce>(&bytes)
                {
                    if announce.is_timestamp_valid(now_ms()) {
                        let peer_id = announce.node_id;
                        let role =
                            if announce.roles.contains(&PeerRole::Relay) {
                                PeerRole::Relay
                            } else {
                                PeerRole::Peer
                            };
                        // Record with Announce source — PeerDiscovered emitted from tick_heartbeat
                        self.heartbeat.record_heartbeat_with_source(
                            peer_id,
                            DiscoverySource::Announce,
                            announce.username,
                        );
                        self.topology.upsert(PeerInfo {
                            node_id: peer_id,
                            role,
                            status: PeerStatus::Online,
                            last_seen: now_ms(),
                        });
                        return vec![];
                    }
                }

                // Try RoleChangeAnnounce
                if let Ok(role_announce) =
                    rmp_serde::from_slice::<crate::discovery::RoleChangeAnnounce>(&bytes)
                {
                    return self.handle_role_announce(role_announce);
                }

                Vec::new()
            }

            GossipInput::NeighborUp(node_id) => {
                self.heartbeat.record_heartbeat_with_source(
                    node_id,
                    DiscoverySource::Gossip,
                    String::new(),
                );
                self.topology.upsert(PeerInfo {
                    node_id,
                    role: PeerRole::Peer,
                    status: PeerStatus::Online,
                    last_seen: now_ms(),
                });
                vec![RuntimeEffect::Emit(
                    ProtocolEvent::GossipNeighborUp { node_id },
                )]
            }

            GossipInput::NeighborDown(node_id) => {
                vec![RuntimeEffect::Emit(
                    ProtocolEvent::GossipNeighborDown { node_id },
                )]
            }
        }
    }

    // ── Helper: surface subnet event ─────────────────────────────────────

    /// Convert a SubnetEvent into RuntimeEffects (only Formed/Dissolved surface).
    fn surface_subnet_event(&self, event: &SubnetEvent) -> Vec<RuntimeEffect> {
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
        proto_event
            .into_iter()
            .map(RuntimeEffect::Emit)
            .collect()
    }

    // ── Helper: prepare backup delivery for reconnected peer ─────────────

    /// Build SendWithBackupFallback effects for each backed-up message
    /// destined to the given peer.
    fn prepare_backup_delivery(&mut self, peer_id: NodeId) -> Vec<RuntimeEffect> {
        let entries: Vec<(String, Vec<u8>)> = self
            .backup
            .store()
            .get_for_recipient(&peer_id)
            .into_iter()
            .map(|e| (e.message_id.clone(), e.payload.clone()))
            .collect();

        if entries.is_empty() {
            return Vec::new();
        }

        let mut effects = Vec::new();

        for (message_id, payload) in entries {
            let via = self.relay_selector.select_path(peer_id, &self.topology);
            let builder = EnvelopeBuilder::new(
                self.local_id,
                peer_id,
                MessageType::Chat,
                payload,
            )
            .via(via);

            let envelope = if self.config.encryption {
                let recipient_pk = peer_id.as_bytes();
                match builder.encrypt_and_sign(&self.secret_seed, &recipient_pk) {
                    Ok(env) => env,
                    Err(_) => continue,
                }
            } else {
                builder.sign(&self.secret_seed)
            };

            // On success: emit BackupDelivered.
            // On failure: no action (message stays in backup store).
            let on_success = vec![RuntimeEffect::Emit(ProtocolEvent::BackupDelivered {
                message_id,
                recipient_id: peer_id,
            })];
            let on_failure = Vec::new();

            effects.push(RuntimeEffect::SendWithBackupFallback {
                envelope,
                on_success,
                on_failure,
            });
        }

        effects
    }

    // ── Helper: surface role action ──────────────────────────────────────

    /// Convert a RoleAction into RuntimeEffects.
    ///
    /// For local role changes, also broadcasts a signed `RoleChangeAnnounce` via gossip.
    fn surface_role_action(&mut self, action: &RoleAction) -> Vec<RuntimeEffect> {
        use crate::discovery::RoleChangeAnnounce;

        match action {
            RoleAction::Promoted { node_id, score } => {
                let mut effects = vec![RuntimeEffect::Emit(ProtocolEvent::RolePromoted {
                    node_id: *node_id,
                    score: *score,
                })];

                // Broadcast via gossip if it's our local promotion
                if *node_id == self.local_id {
                    let announce = RoleChangeAnnounce::new(
                        *node_id,
                        PeerRole::Relay,
                        *score,
                        now_ms(),
                        &self.secret_seed,
                    );
                    effects.push(RuntimeEffect::BroadcastRoleChange(announce));
                }

                effects
            }
            RoleAction::Demoted { node_id, score } => {
                let mut effects = vec![RuntimeEffect::Emit(ProtocolEvent::RoleDemoted {
                    node_id: *node_id,
                    score: *score,
                })];

                // Broadcast via gossip if it's our local demotion
                if *node_id == self.local_id {
                    let announce = RoleChangeAnnounce::new(
                        *node_id,
                        PeerRole::Peer,
                        *score,
                        now_ms(),
                        &self.secret_seed,
                    );
                    effects.push(RuntimeEffect::BroadcastRoleChange(announce));
                }

                effects
            }
            RoleAction::LocalRoleChanged { new_role } => {
                self.local_roles = vec![*new_role];
                let score = self.role_manager.score(&self.local_id, now_ms());

                let announce = RoleChangeAnnounce::new(
                    self.local_id,
                    *new_role,
                    score,
                    now_ms(),
                    &self.secret_seed,
                );

                vec![
                    RuntimeEffect::Emit(ProtocolEvent::LocalRoleChanged {
                        new_role: *new_role,
                    }),
                    RuntimeEffect::BroadcastRoleChange(announce),
                ]
            }
        }
    }

    // ── Helper: group actions → effects ──────────────────────────────────

    /// Convert GroupActions into RuntimeEffects (Send, Broadcast, Event).
    fn group_actions_to_effects(&self, actions: &[GroupAction]) -> Vec<RuntimeEffect> {
        let mut effects = Vec::new();
        for action in actions {
            match action {
                GroupAction::Send { to, payload } => {
                    let msg_type = group_payload_to_message_type(payload);
                    let payload_bytes =
                        rmp_serde::to_vec(payload).expect("group payload serialization");
                    let via = self.relay_selector.select_path(*to, &self.topology);
                    let envelope =
                        EnvelopeBuilder::new(self.local_id, *to, msg_type, payload_bytes)
                            .via(via)
                            .sign(&self.secret_seed);
                    effects.push(RuntimeEffect::SendEnvelope(envelope));
                }
                GroupAction::Broadcast { to, payload } => {
                    let msg_type = group_payload_to_message_type(payload);
                    let payload_bytes =
                        rmp_serde::to_vec(payload).expect("group payload serialization");
                    for target in to {
                        let via = self.relay_selector.select_path(*target, &self.topology);
                        let envelope = EnvelopeBuilder::new(
                            self.local_id,
                            *target,
                            msg_type,
                            payload_bytes.clone(),
                        )
                        .via(via)
                        .sign(&self.secret_seed);
                        effects.push(RuntimeEffect::SendEnvelope(envelope));
                    }
                }
                GroupAction::Event(event) => {
                    effects.extend(self.surface_group_event(event));
                }
                GroupAction::None => {}
            }
        }
        effects
    }

    // ── Helper: backup actions → effects ─────────────────────────────────

    /// Convert BackupActions into RuntimeEffects.
    fn backup_actions_to_effects(&self, actions: &[BackupAction]) -> Vec<RuntimeEffect> {
        let mut effects = Vec::new();
        for action in actions {
            match action {
                BackupAction::Replicate { target, payload } => {
                    let bytes =
                        rmp_serde::to_vec(payload).expect("backup replication serialization");
                    let via = self.relay_selector.select_path(*target, &self.topology);
                    let envelope = EnvelopeBuilder::new(
                        self.local_id,
                        *target,
                        MessageType::BackupReplicate,
                        bytes,
                    )
                    .via(via)
                    .sign(&self.secret_seed);
                    effects.push(RuntimeEffect::SendEnvelope(envelope));
                }
                BackupAction::ConfirmDelivery {
                    message_ids,
                    recipient_id: _,
                } => {
                    let bytes =
                        rmp_serde::to_vec(message_ids).expect("backup confirm serialization");
                    for peer in self.topology.peers() {
                        if peer.node_id != self.local_id && peer.status == PeerStatus::Online {
                            let envelope = EnvelopeBuilder::new(
                                self.local_id,
                                peer.node_id,
                                MessageType::BackupConfirmDelivery,
                                bytes.clone(),
                            )
                            .sign(&self.secret_seed);
                            effects.push(RuntimeEffect::SendEnvelope(envelope));
                        }
                    }
                }
                BackupAction::QueryPending { recipient_id } => {
                    let bytes =
                        rmp_serde::to_vec(recipient_id).expect("backup query serialization");
                    for peer in self.topology.peers() {
                        if peer.node_id != self.local_id && peer.status == PeerStatus::Online {
                            let envelope = EnvelopeBuilder::new(
                                self.local_id,
                                peer.node_id,
                                MessageType::BackupQuery,
                                bytes.clone(),
                            )
                            .sign(&self.secret_seed);
                            effects.push(RuntimeEffect::SendEnvelope(envelope));
                        }
                    }
                }
                BackupAction::Event(event) => {
                    effects.extend(self.surface_backup_event(event));
                }
            }
        }
        effects
    }

    // ── Helper: surface group event ──────────────────────────────────────

    /// Map a GroupEvent to a ProtocolEvent wrapped in RuntimeEffect::Emit.
    fn surface_group_event(&self, event: &GroupEvent) -> Vec<RuntimeEffect> {
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
        vec![RuntimeEffect::Emit(proto_event)]
    }

    // ── Helper: surface backup event ─────────────────────────────────────

    /// Map a BackupEvent to a ProtocolEvent (only first 3 variants surface to app).
    fn surface_backup_event(&self, event: &BackupEvent) -> Vec<RuntimeEffect> {
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
        proto_event
            .into_iter()
            .map(RuntimeEffect::Emit)
            .collect()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::RuntimeConfig;
    use crate::relay::PeerStatus;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    fn keypair(seed: u8) -> (NodeId, [u8; 32]) {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        let node_id: NodeId = secret.public().to_string().parse().unwrap();
        let seed_bytes = secret.to_bytes();
        (node_id, seed_bytes)
    }

    fn default_state(seed: u8) -> RuntimeState {
        let (id, secret) = keypair(seed);
        RuntimeState::new(id, secret, RuntimeConfig::default())
    }

    // ── Task 4 tests ─────────────────────────────────────────────────────

    #[test]
    fn tick_cache_cleanup_returns_no_effects() {
        let mut state = default_state(1);
        let effects = state.tick_cache_cleanup();
        assert!(effects.is_empty());
    }

    #[test]
    fn tick_tracker_cleanup_returns_no_effects() {
        let mut state = default_state(1);
        let effects = state.tick_tracker_cleanup();
        assert!(effects.is_empty());
    }

    // ── Task 5 tests ─────────────────────────────────────────────────────

    #[test]
    fn tick_heartbeat_empty_state_no_effects() {
        let mut state = default_state(1);
        let effects = state.tick_heartbeat();
        assert!(effects.is_empty());
    }

    #[test]
    fn tick_heartbeat_peer_offline_emits_event() {
        let mut state = default_state(1);
        let peer = node_id(2);

        // Register peer with a very old heartbeat so it goes offline
        state.heartbeat.record_heartbeat_at(peer, 0);
        state.topology.upsert(crate::relay::PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 0,
        });

        let effects = state.tick_heartbeat();

        // Should emit PeerOffline event
        let has_offline = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::PeerOffline { node_id }) if *node_id == peer)
        });
        assert!(has_offline, "expected PeerOffline event, got: {effects:?}");
    }

    #[test]
    fn tick_heartbeat_peer_reconnect_emits_online() {
        let mut state = default_state(1);
        let peer = node_id(2);

        // First, discover the peer so it's in the discovered set
        state.heartbeat.record_heartbeat(peer);
        state.topology.upsert(crate::relay::PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 0,
        });
        let _ = state.tick_heartbeat(); // emits PeerDiscovered (first time)

        // Now put peer in Offline status in topology, then give it a recent heartbeat
        // so check_all sees it as alive → PeerOnline (reconnect).
        state.topology.upsert(crate::relay::PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Offline,
            last_seen: 0,
        });
        state.heartbeat.record_heartbeat(peer);

        let effects = state.tick_heartbeat();

        let has_online = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::PeerOnline { node_id }) if *node_id == peer)
        });
        assert!(
            has_online,
            "expected PeerOnline event on reconnect, got: {effects:?}"
        );
    }

    // ── Task 6 tests ─────────────────────────────────────────────────────

    #[test]
    fn tick_subnets_empty_state_no_effects() {
        let mut state = default_state(1);
        let effects = state.tick_subnets();
        assert!(effects.is_empty());
    }

    #[test]
    fn tick_roles_empty_state_no_effects() {
        let mut state = default_state(1);
        let effects = state.tick_roles();
        assert!(effects.is_empty());
    }

    #[test]
    fn tick_backup_empty_state_no_effects() {
        let mut state = default_state(1);
        let effects = state.tick_backup();
        assert!(effects.is_empty());
    }

    #[test]
    fn tick_group_hub_heartbeat_empty_state_no_effects() {
        let mut state = default_state(1);
        let effects = state.tick_group_hub_heartbeat();
        assert!(effects.is_empty());
    }

    #[test]
    fn build_gossip_announce_returns_bytes() {
        let state = default_state(1);
        let bytes = state.build_gossip_announce();
        assert!(bytes.is_some(), "should produce announce bytes");

        // Verify it's valid MsgPack PeerAnnounce
        let bytes = bytes.unwrap();
        let announce: PeerAnnounce =
            rmp_serde::from_slice(&bytes).expect("should deserialize PeerAnnounce");
        assert_eq!(announce.node_id, state.local_id);
        assert_eq!(announce.username, "anonymous");
        assert_eq!(announce.roles, vec![PeerRole::Peer]);
    }

    // ── Task 7 tests ─────────────────────────────────────────────────────

    /// Build a signed chat envelope from sender (with known secret) to recipient.
    fn make_signed_chat(
        sender_seed: u8,
        recipient_id: NodeId,
        payload: &[u8],
    ) -> (crate::envelope::Envelope, bool) {
        let (sender_id, sender_secret) = keypair(sender_seed);
        let env = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            recipient_id,
            MessageType::Chat,
            payload.to_vec(),
        )
        .sign(&sender_secret);
        let sig_valid = env.verify_signature().is_ok();
        (env, sig_valid)
    }

    #[test]
    fn handle_incoming_chat_delivers_and_acks() {
        let mut state = default_state(1);
        let (sender_id, _) = keypair(2);

        let (env, sig_valid) = make_signed_chat(2, state.local_id, b"hello");

        let effects = state.handle_incoming_chat(env, sig_valid);

        // Should have DeliverMessage + SendEnvelope(ACK)
        let has_deliver = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::DeliverMessage(msg) if msg.from == sender_id && msg.payload == b"hello")
        });
        let has_ack = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::SendEnvelope(env) if env.msg_type == MessageType::Ack && env.to == sender_id)
        });
        assert!(has_deliver, "expected DeliverMessage, got: {effects:?}");
        assert!(has_ack, "expected ACK SendEnvelope, got: {effects:?}");
    }

    #[test]
    fn handle_incoming_chat_encrypted_decrypts() {
        // Create state with encryption
        let (local_id, local_secret) = keypair(1);
        let mut state = RuntimeState::new(
            local_id,
            local_secret,
            RuntimeConfig {
                encryption: true,
                ..Default::default()
            },
        );

        let (sender_id, sender_secret) = keypair(2);
        let plaintext = b"secret message";
        let recipient_pk = local_id.as_bytes();

        let env = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            local_id,
            MessageType::Chat,
            plaintext.to_vec(),
        )
        .encrypt_and_sign(&sender_secret, &recipient_pk)
        .expect("encrypt_and_sign");

        let sig_valid = env.verify_signature().is_ok();
        let effects = state.handle_incoming_chat(env, sig_valid);

        // Find the delivered message
        let delivered = effects.iter().find_map(|e| {
            if let RuntimeEffect::DeliverMessage(msg) = e {
                Some(msg)
            } else {
                None
            }
        });
        assert!(delivered.is_some(), "expected DeliverMessage");
        let msg = delivered.unwrap();
        assert_eq!(msg.payload, plaintext);
        assert!(msg.was_encrypted);
        assert!(msg.signature_valid);
    }

    #[test]
    fn handle_incoming_chat_forward_when_not_recipient() {
        let mut state = default_state(1);
        let (sender_id, sender_secret) = keypair(2);
        let recipient_id = node_id(3);

        // Build envelope from sender to recipient, routed via our node
        let env = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            recipient_id,
            MessageType::Chat,
            b"relayed".to_vec(),
        )
        .via(vec![state.local_id])
        .sign(&sender_secret);

        let sig_valid = env.verify_signature().is_ok();
        let effects = state.handle_incoming_chat(env, sig_valid);

        // Should have SendEnvelopeTo for next_hop + SendEnvelopeTo for ACK + Forwarded event
        let has_forward = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::SendEnvelopeTo { target, .. } if *target == recipient_id)
        });
        let has_relay_ack = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::SendEnvelopeTo { target, envelope } if *target == sender_id && envelope.msg_type == MessageType::Ack)
        });
        let has_forwarded_event = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::Forwarded { next_hop, .. }) if *next_hop == recipient_id)
        });
        assert!(has_forward, "expected forward to recipient, got: {effects:?}");
        assert!(has_relay_ack, "expected relay ACK to sender, got: {effects:?}");
        assert!(
            has_forwarded_event,
            "expected Forwarded event, got: {effects:?}"
        );
    }

    #[test]
    fn handle_incoming_chat_dedup_drops() {
        let mut state = default_state(1);
        let (env, sig_valid) = make_signed_chat(2, state.local_id, b"once");
        let env2 = env.clone();

        let effects1 = state.handle_incoming_chat(env, sig_valid);
        assert!(
            !effects1.is_empty(),
            "first delivery should produce effects"
        );

        let effects2 = state.handle_incoming_chat(env2, sig_valid);
        assert!(
            effects2.is_empty(),
            "duplicate should be dropped, got: {effects2:?}"
        );
    }

    // ── Task 8 tests ─────────────────────────────────────────────────────

    #[test]
    fn handle_incoming_parses_and_dispatches_chat() {
        let mut state = default_state(1);
        let (sender_id, sender_secret) = keypair(2);

        let env = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            state.local_id,
            MessageType::Chat,
            b"raw bytes test".to_vec(),
        )
        .sign(&sender_secret);
        let raw = env.to_bytes().expect("serialize");

        let effects = state.handle_incoming(raw.as_slice());

        let has_deliver = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::DeliverMessage(msg) if msg.from == sender_id)
        });
        assert!(
            has_deliver,
            "should dispatch chat and deliver, got: {effects:?}"
        );
    }

    #[test]
    fn handle_incoming_auto_registers_unknown_peer() {
        let mut state = default_state(1);
        let (sender_id, sender_secret) = keypair(2);

        // Verify peer is not in topology yet
        assert!(state.topology.get(&sender_id).is_none());

        let env = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            state.local_id,
            MessageType::Chat,
            b"auto-register".to_vec(),
        )
        .sign(&sender_secret);
        let raw = env.to_bytes().expect("serialize");

        state.handle_incoming(raw.as_slice());

        // Peer should now be in topology
        let peer = state.topology.get(&sender_id);
        assert!(peer.is_some(), "peer should be auto-registered in topology");
        assert_eq!(peer.unwrap().status, PeerStatus::Online);
    }

    // ── Task 9 tests ─────────────────────────────────────────────────────

    #[test]
    fn handle_send_message_produces_fallback_effect() {
        let mut state = default_state(1);
        let recipient = node_id(2);

        let effects = state.handle_send_message(recipient, b"hello".to_vec());

        assert_eq!(effects.len(), 1, "expected exactly one effect");
        assert!(
            matches!(&effects[0], RuntimeEffect::SendWithBackupFallback { .. }),
            "expected SendWithBackupFallback, got: {:?}",
            effects[0]
        );

        // Verify on_success has StatusChange effects
        if let RuntimeEffect::SendWithBackupFallback {
            on_success,
            on_failure,
            envelope,
            ..
        } = &effects[0]
        {
            assert!(
                !on_success.is_empty(),
                "on_success should have status changes"
            );
            assert!(
                !on_failure.is_empty(),
                "on_failure should have backup + error effects"
            );
            assert!(envelope.is_signed(), "envelope should be signed");
        }
    }

    #[test]
    fn handle_send_message_encrypted_when_config_enabled() {
        let (local_id, local_secret) = keypair(1);
        let mut state = RuntimeState::new(
            local_id,
            local_secret,
            RuntimeConfig {
                encryption: true,
                ..Default::default()
            },
        );
        let recipient = node_id(2);

        let effects = state.handle_send_message(recipient, b"encrypted".to_vec());

        if let RuntimeEffect::SendWithBackupFallback { envelope, .. } = &effects[0] {
            assert!(
                envelope.encrypted,
                "envelope should be encrypted when config.encryption is true"
            );
        } else {
            panic!("expected SendWithBackupFallback");
        }
    }

    #[test]
    fn handle_command_add_peer_updates_topology() {
        let mut state = default_state(1);
        let peer = node_id(2);

        assert!(state.topology.get(&peer).is_none());

        let effects =
            state.handle_command(RuntimeCommand::AddPeer { node_id: peer });

        assert!(effects.is_empty(), "AddPeer returns no effects");
        assert!(
            state.topology.get(&peer).is_some(),
            "peer should be in topology after AddPeer"
        );
    }

    #[test]
    fn handle_command_remove_peer_cleans_topology() {
        let mut state = default_state(1);
        let peer = node_id(2);

        // Add peer first
        state.handle_command(RuntimeCommand::AddPeer { node_id: peer });
        assert!(state.topology.get(&peer).is_some());

        // Remove peer
        let effects =
            state.handle_command(RuntimeCommand::RemovePeer { node_id: peer });

        assert!(effects.is_empty(), "RemovePeer returns no effects");
        assert!(
            state.topology.get(&peer).is_none(),
            "peer should be removed from topology"
        );
    }

    // ── Task 10 tests ────────────────────────────────────────────────────

    #[test]
    fn handle_gossip_neighbor_up_registers_peer() {
        let mut state = default_state(1);
        let peer = node_id(2);

        assert!(state.topology.get(&peer).is_none());

        let effects =
            state.handle_gossip_event(super::GossipInput::NeighborUp(peer));

        let has_event = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::GossipNeighborUp { node_id }) if *node_id == peer)
        });
        assert!(has_event, "expected GossipNeighborUp event, got: {effects:?}");

        let topo_peer = state.topology.get(&peer);
        assert!(
            topo_peer.is_some(),
            "peer should be registered in topology after NeighborUp"
        );
        assert_eq!(topo_peer.unwrap().status, PeerStatus::Online);
    }

    #[test]
    fn handle_gossip_announce_registers_peer() {
        let mut state = default_state(1);
        let peer = node_id(2);

        // Build a PeerAnnounce
        let announce = PeerAnnounce::new(peer, "bob".to_string(), vec![PeerRole::Peer]);
        let bytes = rmp_serde::to_vec(&announce).expect("serialize announce");

        assert!(state.topology.get(&peer).is_none());

        let effects =
            state.handle_gossip_event(super::GossipInput::PeerAnnounce(bytes));

        // PeerAnnounce no longer emits immediately — PeerDiscovered comes from tick_heartbeat
        assert!(
            effects.is_empty(),
            "PeerAnnounce should not emit directly, got: {effects:?}"
        );

        let topo_peer = state.topology.get(&peer);
        assert!(
            topo_peer.is_some(),
            "peer should be registered in topology after gossip announce"
        );

        // Verify PeerDiscovered emitted on next heartbeat tick
        let tick_effects = state.tick_heartbeat();
        let has_discovered = tick_effects.iter().any(|e| {
            matches!(
                e,
                RuntimeEffect::Emit(ProtocolEvent::PeerDiscovered { node_id, username, source })
                    if *node_id == peer && username == "bob" && *source == DiscoverySource::Announce
            )
        });
        assert!(
            has_discovered,
            "expected PeerDiscovered from tick_heartbeat, got: {tick_effects:?}"
        );
    }

    // ── Task 13: Integration tests ──────────────────────────────────────

    #[test]
    fn message_e2e_encrypt_decrypt_roundtrip() {
        // Alice encrypts a chat message for Bob. Bob's RuntimeState handles
        // it via handle_incoming(). Verify: plaintext recovered, was_encrypted,
        // signature_valid.
        let (alice_id, alice_secret) = keypair(10);
        let (bob_id, bob_secret) = keypair(11);

        let mut bob_state = RuntimeState::new(
            bob_id,
            bob_secret,
            RuntimeConfig {
                encryption: true,
                ..Default::default()
            },
        );

        let plaintext = b"Hello Bob, this is Alice!";
        let bob_pk = bob_id.as_bytes();
        let env = EnvelopeBuilder::new(
            alice_id,
            bob_id,
            MessageType::Chat,
            plaintext.to_vec(),
        )
        .encrypt_and_sign(&alice_secret, &bob_pk)
        .expect("encrypt_and_sign should succeed");

        // Sanity: envelope is encrypted and signed
        assert!(env.encrypted);
        assert!(env.is_signed());

        let raw = env.to_bytes().expect("serialize");
        let effects = bob_state.handle_incoming(&raw);

        let delivered = effects.iter().find_map(|e| {
            if let RuntimeEffect::DeliverMessage(msg) = e {
                Some(msg)
            } else {
                None
            }
        });
        assert!(delivered.is_some(), "expected DeliverMessage, got: {effects:?}");
        let msg = delivered.unwrap();
        assert_eq!(msg.payload, plaintext, "plaintext should match after decryption");
        assert!(msg.was_encrypted, "message should report was_encrypted=true");
        assert!(msg.signature_valid, "signature should be valid");
        assert_eq!(msg.from, alice_id, "sender should be Alice");
    }

    #[test]
    fn ack_updates_tracker_status() {
        // Send a message, then simulate relay ACK and recipient ACK.
        // Verify StatusChange effects progress through expected states.
        let (alice_id, alice_secret) = keypair(20);
        let (bob_id, bob_secret) = keypair(21);

        let mut alice_state = RuntimeState::new(
            alice_id,
            alice_secret,
            RuntimeConfig {
                encryption: false,
                ..Default::default()
            },
        );

        // Send message from Alice to Bob
        let send_effects = alice_state.handle_send_message(bob_id, b"hi bob".to_vec());
        assert_eq!(send_effects.len(), 1);
        let envelope = match &send_effects[0] {
            RuntimeEffect::SendWithBackupFallback { envelope, on_success, .. } => {
                // on_success should contain Pending and Sent status changes
                let has_status = on_success.iter().any(|e| matches!(e, RuntimeEffect::StatusChange(_)));
                assert!(has_status, "on_success should have StatusChange effects");
                envelope.clone()
            }
            other => panic!("expected SendWithBackupFallback, got: {other:?}"),
        };
        let msg_id = envelope.id.clone();

        // Simulate relay ACK (RelayForwarded)
        use crate::router::{AckPayload, AckType};
        let relay_id = node_id(22);
        let relay_ack_payload = AckPayload {
            original_message_id: msg_id.clone(),
            ack_type: AckType::RelayForwarded,
        };
        let relay_ack_env = EnvelopeBuilder::new(
            relay_id,
            alice_id,
            MessageType::Ack,
            relay_ack_payload.to_bytes(),
        )
        .build();
        // We use the relay_ack_env unsigned — that's fine, sig_valid=false
        let relay_effects = alice_state.handle_incoming_chat(relay_ack_env, false);
        let relay_status = relay_effects.iter().find_map(|e| {
            if let RuntimeEffect::StatusChange(sc) = e { Some(sc) } else { None }
        });
        assert!(relay_status.is_some(), "relay ACK should produce StatusChange, got: {relay_effects:?}");
        let sc = relay_status.unwrap();
        assert_eq!(sc.current, crate::types::MessageStatus::Relayed);

        // Simulate recipient ACK (RecipientReceived)
        let recipient_ack_payload = AckPayload {
            original_message_id: msg_id.clone(),
            ack_type: AckType::RecipientReceived,
        };
        let recipient_ack_env = EnvelopeBuilder::new(
            bob_id,
            alice_id,
            MessageType::Ack,
            recipient_ack_payload.to_bytes(),
        )
        .sign(&bob_secret);
        let sig_valid = recipient_ack_env.verify_signature().is_ok();
        let recv_effects = alice_state.handle_incoming_chat(recipient_ack_env, sig_valid);
        let recv_status = recv_effects.iter().find_map(|e| {
            if let RuntimeEffect::StatusChange(sc) = e { Some(sc) } else { None }
        });
        assert!(recv_status.is_some(), "recipient ACK should produce StatusChange, got: {recv_effects:?}");
        let sc = recv_status.unwrap();
        assert_eq!(sc.current, crate::types::MessageStatus::Delivered);
    }

    #[test]
    fn read_receipt_produces_status_read() {
        // Track a message, then handle an incoming ReadReceipt envelope.
        // Verify the StatusChange marks it as Read.
        let (alice_id, alice_secret) = keypair(30);
        let (bob_id, bob_secret) = keypair(31);

        let mut alice_state = RuntimeState::new(
            alice_id,
            alice_secret,
            RuntimeConfig {
                encryption: false,
                ..Default::default()
            },
        );

        // Send a message to get an envelope_id and track it
        let send_effects = alice_state.handle_send_message(bob_id, b"read me".to_vec());
        let envelope = match &send_effects[0] {
            RuntimeEffect::SendWithBackupFallback { envelope, .. } => envelope.clone(),
            other => panic!("expected SendWithBackupFallback, got: {other:?}"),
        };
        let msg_id = envelope.id.clone();

        // Advance to Delivered first (Read requires Delivered or earlier)
        alice_state.tracker.mark_delivered(&msg_id);

        // Build a ReadReceipt envelope from Bob
        use crate::router::ReadReceiptPayload;
        let rr_payload = ReadReceiptPayload {
            original_message_id: msg_id.clone(),
            read_at: crate::types::now_ms(),
        };
        let rr_env = EnvelopeBuilder::new(
            bob_id,
            alice_id,
            MessageType::ReadReceipt,
            rr_payload.to_bytes(),
        )
        .sign(&bob_secret);
        let sig_valid = rr_env.verify_signature().is_ok();
        let effects = alice_state.handle_incoming_chat(rr_env, sig_valid);

        let read_status = effects.iter().find_map(|e| {
            if let RuntimeEffect::StatusChange(sc) = e { Some(sc) } else { None }
        });
        assert!(read_status.is_some(), "ReadReceipt should produce StatusChange, got: {effects:?}");
        let sc = read_status.unwrap();
        assert_eq!(sc.current, crate::types::MessageStatus::Read);
        assert_eq!(sc.message_id, msg_id);
    }

    #[test]
    fn group_create_produces_send_effects() {
        // Call handle_command with CreateGroup. Verify it produces
        // SendEnvelope effects (the GroupCreate payload to the hub).
        let mut state = default_state(40);
        let hub_id = node_id(41);
        let member1 = node_id(42);
        let member2 = node_id(43);

        let effects = state.handle_command(RuntimeCommand::CreateGroup {
            name: "Test Group".to_string(),
            hub_relay_id: hub_id,
            initial_members: vec![member1, member2],
        });

        // Should produce at least one SendEnvelope (the GroupCreate to hub)
        let send_envelopes: Vec<_> = effects.iter().filter(|e| {
            matches!(e, RuntimeEffect::SendEnvelope(env) if env.to == hub_id && env.msg_type == MessageType::GroupCreate)
        }).collect();
        assert!(
            !send_envelopes.is_empty(),
            "CreateGroup should produce SendEnvelope to hub, got: {effects:?}"
        );

        // Verify the envelope is signed and directed to the hub
        if let RuntimeEffect::SendEnvelope(env) = &send_envelopes[0] {
            assert!(env.is_signed(), "group create envelope should be signed");
            assert_eq!(env.to, hub_id);
            assert_eq!(env.from, state.local_id);
        }
    }

    #[test]
    fn peer_add_then_remove_cleans_state() {
        // Add a peer via AddPeer, verify topology and heartbeat.
        // Remove via RemovePeer, verify both are cleaned.
        let mut state = default_state(50);
        let peer = node_id(51);

        // Initially: not in topology or heartbeat
        assert!(state.topology.get(&peer).is_none());
        assert_eq!(state.heartbeat.liveness(&peer), crate::discovery::LivenessState::Departed);

        // Add peer
        state.handle_command(RuntimeCommand::AddPeer { node_id: peer });
        assert!(state.topology.get(&peer).is_some(), "peer should be in topology after AddPeer");
        assert_ne!(
            state.heartbeat.liveness(&peer),
            crate::discovery::LivenessState::Departed,
            "peer should be tracked in heartbeat after AddPeer"
        );

        // Remove peer
        state.handle_command(RuntimeCommand::RemovePeer { node_id: peer });
        assert!(
            state.topology.get(&peer).is_none(),
            "peer should be removed from topology after RemovePeer"
        );
        assert_eq!(
            state.heartbeat.liveness(&peer),
            crate::discovery::LivenessState::Departed,
            "peer should be untracked from heartbeat after RemovePeer"
        );
    }

    #[test]
    fn build_gossip_announce_roundtrip() {
        // Build gossip announce bytes, deserialize them back,
        // verify fields match (node_id, username, roles).
        let (local_id, local_secret) = keypair(60);
        let state = RuntimeState::new(
            local_id,
            local_secret,
            RuntimeConfig {
                username: "alice_test".to_string(),
                ..Default::default()
            },
        );

        let bytes = state.build_gossip_announce();
        assert!(bytes.is_some(), "should produce announce bytes");
        let bytes = bytes.unwrap();

        let announce: PeerAnnounce =
            rmp_serde::from_slice(&bytes).expect("should deserialize PeerAnnounce");
        assert_eq!(announce.node_id, local_id);
        assert_eq!(announce.username, "alice_test");
        assert_eq!(announce.roles, vec![PeerRole::Peer]);
        assert!(
            announce.is_timestamp_valid(crate::types::now_ms()),
            "announce timestamp should be valid at current time"
        );
    }

    #[test]
    fn handle_incoming_rejects_garbage_bytes() {
        // Pass garbage bytes to handle_incoming(). Verify it returns
        // empty effects (graceful handling, no panic).
        let mut state = default_state(70);

        let garbage = b"this is not valid msgpack at all!!! \x00\xff\xfe";
        let effects = state.handle_incoming(garbage);
        assert!(
            effects.is_empty(),
            "garbage input should produce no effects (graceful drop), got: {effects:?}"
        );

        // Also test empty bytes
        let effects = state.handle_incoming(&[]);
        assert!(
            effects.is_empty(),
            "empty input should produce no effects, got: {effects:?}"
        );

        // Also test partially valid but corrupted msgpack
        let effects = state.handle_incoming(&[0x93, 0x01, 0x02]);
        assert!(
            effects.is_empty(),
            "corrupted msgpack should produce no effects, got: {effects:?}"
        );
    }

    #[test]
    fn send_message_unencrypted_when_config_disabled() {
        // Create state with config.encryption = false. Call handle_send_message.
        // Verify the envelope is NOT encrypted.
        let (local_id, local_secret) = keypair(80);
        let mut state = RuntimeState::new(
            local_id,
            local_secret,
            RuntimeConfig {
                encryption: false,
                ..Default::default()
            },
        );
        let recipient = node_id(81);

        let effects = state.handle_send_message(recipient, b"plaintext msg".to_vec());
        assert_eq!(effects.len(), 1);

        match &effects[0] {
            RuntimeEffect::SendWithBackupFallback { envelope, .. } => {
                assert!(
                    !envelope.encrypted,
                    "envelope should NOT be encrypted when config.encryption=false"
                );
                assert!(envelope.is_signed(), "envelope should still be signed");
                assert_eq!(envelope.to, recipient);
                assert_eq!(envelope.from, local_id);
                // Payload should be the original plaintext (not ciphertext)
                assert_eq!(envelope.payload, b"plaintext msg");
            }
            other => panic!("expected SendWithBackupFallback, got: {other:?}"),
        }
    }

    // ── Task 5 (failover): shadow auto-assignment ────────────────────

    #[test]
    fn handle_incoming_group_create_triggers_shadow_assignment() {
        // When the hub processes a GroupCreate, it should automatically
        // call assign_shadow after the group is created.
        let (hub_id, hub_secret) = keypair(90);
        let (alice_id, alice_secret) = keypair(91);
        let (bob_id, bob_secret) = keypair(92);

        let mut hub_state = RuntimeState::new(
            hub_id,
            hub_secret,
            RuntimeConfig {
                encryption: false,
                ..Default::default()
            },
        );

        // Build a GroupCreate envelope from Alice to the hub
        let create_payload = crate::group::GroupPayload::Create {
            group_name: "Shadow Auto Test".into(),
            creator_username: "alice".into(),
            initial_members: vec![bob_id],
        };
        let payload_bytes = rmp_serde::to_vec(&create_payload).unwrap();
        let create_env = EnvelopeBuilder::new(
            alice_id,
            hub_id,
            MessageType::GroupCreate,
            payload_bytes,
        )
        .sign(&alice_secret);

        let effects = hub_state.handle_incoming_group(create_env);

        // After Create, the only member is Alice (the creator).
        // assign_shadow should have been called — it picks the lowest non-hub member.
        // Since Alice is the only member and she is not the hub, she becomes shadow.
        let has_shadow_sync = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::SendEnvelope(env) if env.msg_type == MessageType::GroupHubShadowSync)
        });
        assert!(
            has_shadow_sync,
            "GroupCreate should trigger shadow assignment and send HubShadowSync, got: {effects:?}"
        );

        // Now Bob joins — shadow should be reassigned/updated
        let gid = hub_state.group_hub.groups().next().unwrap().0.clone();
        let join_payload = crate::group::GroupPayload::Join {
            group_id: gid.clone(),
            username: "bob".into(),
        };
        let join_bytes = rmp_serde::to_vec(&join_payload).unwrap();
        let join_env = EnvelopeBuilder::new(
            bob_id,
            hub_id,
            MessageType::GroupJoin,
            join_bytes,
        )
        .sign(&bob_secret);

        let join_effects = hub_state.handle_incoming_group(join_env);

        // After join, assign_shadow should run again
        let has_shadow_sync_after_join = join_effects.iter().any(|e| {
            matches!(e, RuntimeEffect::SendEnvelope(env) if env.msg_type == MessageType::GroupHubShadowSync)
        });
        assert!(
            has_shadow_sync_after_join,
            "GroupJoin should trigger shadow reassignment, got: {join_effects:?}"
        );

        // Verify the group now has a shadow_id set
        let group = hub_state.group_hub.get_group(&gid).unwrap();
        assert!(
            group.shadow_id.is_some(),
            "group should have a shadow after join"
        );
    }

    // ── Bandwidth tracking tests ────────────────────────────────────────

    #[test]
    fn role_manager_bandwidth_tracking_via_runtime_state() {
        let mut state = default_state(1);
        let peer = node_id(2);

        // Directly record bandwidth through the role_manager
        state.role_manager.record_relay(peer, 1000);
        state
            .role_manager
            .record_bytes_relayed(peer, 50 * 1_048_576, 1000); // 50 MB

        let score = state.role_manager.score(&peer, 1000);
        // Score should include bandwidth: relay(1) + success(5) + bandwidth_mb(50*0.2=10) = 16+
        assert!(
            score > 15.0,
            "Score should reflect bandwidth contribution, got {score}"
        );
    }

    #[test]
    fn local_role_change_broadcasts_announce() {
        let mut state = default_state(1);

        // Simulate local promotion
        let action = RoleAction::LocalRoleChanged {
            new_role: PeerRole::Relay,
        };

        let effects = state.surface_role_action(&action);

        // Should emit event + broadcast announce
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, RuntimeEffect::Emit(ProtocolEvent::LocalRoleChanged { .. }))),
            "Should emit LocalRoleChanged event"
        );
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, RuntimeEffect::BroadcastRoleChange(_))),
            "Should broadcast role change announce"
        );
    }

    #[test]
    fn handle_role_announce_updates_topology() {
        use crate::discovery::RoleChangeAnnounce;

        let mut state = default_state(1);
        let (remote, remote_seed) = keypair(2);

        let announce = RoleChangeAnnounce::new(
            remote,
            PeerRole::Relay,
            15.0,
            now_ms(),
            &remote_seed,
        );

        let effects = state.handle_role_announce(announce);

        // Should emit RolePromoted
        assert!(
            effects.iter().any(|e| matches!(
                e,
                RuntimeEffect::Emit(ProtocolEvent::RolePromoted { node_id, .. })
                if *node_id == remote
            )),
            "Should emit RolePromoted: {effects:?}"
        );

        // Topology should be updated
        let peer = state.topology.get(&remote).expect("peer in topology");
        assert_eq!(peer.role, PeerRole::Relay);
    }

    #[test]
    fn handle_role_announce_throttle() {
        use crate::discovery::RoleChangeAnnounce;

        let mut state = default_state(1);
        let (remote, remote_seed) = keypair(2);

        let announce1 = RoleChangeAnnounce::new(
            remote, PeerRole::Relay, 15.0, now_ms(), &remote_seed,
        );
        let announce2 = RoleChangeAnnounce::new(
            remote, PeerRole::Peer, 1.0, now_ms(), &remote_seed,
        );

        // First announce accepted
        let effects1 = state.handle_role_announce(announce1);
        assert!(!effects1.is_empty(), "First announce should be accepted");

        // Second announce within 30s throttled
        let effects2 = state.handle_role_announce(announce2);
        assert!(effects2.is_empty(), "Second announce should be throttled");
    }

    #[test]
    fn handle_role_announce_rejects_invalid_signature() {
        use crate::discovery::RoleChangeAnnounce;

        let mut state = default_state(1);
        let remote = node_id(2);

        // Sign with WRONG key (node 3's key, not node 2's)
        let (_, wrong_seed) = keypair(3);

        let announce = RoleChangeAnnounce::new(
            remote, PeerRole::Relay, 15.0, now_ms(), &wrong_seed,
        );

        let effects = state.handle_role_announce(announce);

        // Should emit Error
        assert!(
            effects.iter().any(|e| matches!(
                e,
                RuntimeEffect::Emit(ProtocolEvent::Error { description })
                if description.contains("Invalid signature")
            )),
            "Should reject invalid signature: {effects:?}"
        );

        // Topology should NOT be updated
        assert!(state.topology.get(&remote).is_none());
    }

    // ── r4: Role validation integration tests ───────────────────────────

    #[test]
    fn tick_roles_promotes_active_peer() {
        let mut state = default_state(1);
        let peer = node_id(2);
        let now = now_ms();

        // Register peer in topology
        state.topology.upsert(PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: now,
        });

        // Simulate 20 relays (enough to exceed PROMOTION_THRESHOLD=10.0)
        for i in 0..20 {
            state.role_manager.record_relay(peer, now + i * 1000);
        }

        let effects = state.tick_roles();

        let has_promoted = effects.iter().any(|e| {
            matches!(
                e,
                RuntimeEffect::Emit(ProtocolEvent::RolePromoted { node_id, .. })
                if *node_id == peer
            )
        });
        assert!(
            has_promoted,
            "expected RolePromoted after 20 relays, got: {effects:?}"
        );

        // Topology should now show Relay
        assert_eq!(state.topology.get(&peer).unwrap().role, PeerRole::Relay);
    }

    #[test]
    fn tick_roles_demotes_idle_relay() {
        let mut state = default_state(1);
        let peer = node_id(2);
        let now = now_ms();

        // Register and promote peer
        state.topology.upsert(PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: now,
        });
        for i in 0..20 {
            state.role_manager.record_relay(peer, now + i * 1000);
        }
        let _ = state.tick_roles(); // Promotes
        assert_eq!(state.topology.get(&peer).unwrap().role, PeerRole::Relay);

        // 100 hours of idleness — score should decay below DEMOTION_THRESHOLD=2.0
        let future = now + 100 * 3_600_000;
        let score = state.role_manager.score(&peer, future);
        assert!(
            score < 2.0,
            "score should be below demotion threshold after 100h idle: {score}"
        );

        // tick_roles uses now_ms() (can't fake time), so test via evaluate directly
        let actions = state.role_manager.evaluate(&mut state.topology, future);
        assert!(
            actions.iter().any(|a| matches!(
                a,
                crate::roles::RoleAction::Demoted { node_id, .. }
                if *node_id == peer
            )),
            "expected demotion after 100h idle: {actions:?}"
        );
        assert_eq!(state.topology.get(&peer).unwrap().role, PeerRole::Peer);
    }

    #[test]
    fn get_role_metrics_via_command() {
        let mut state = default_state(1);
        let peer = node_id(2);
        let now = now_ms();

        state.topology.upsert(PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: now,
        });

        // Record some activity
        for i in 0..5 {
            state.role_manager.record_relay(peer, now + i * 1000);
        }
        state.role_manager.record_bytes_relayed(peer, 10 * 1_048_576, now + 5000);

        // Query via command handler
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        let effects = state.handle_command(RuntimeCommand::GetRoleMetrics {
            node_id: peer,
            reply: tx,
        });
        assert!(effects.is_empty(), "GetRoleMetrics should not emit effects");

        let metrics = rx.try_recv().expect("should receive response");
        let metrics = metrics.expect("metrics should exist for tracked peer");

        assert_eq!(metrics.node_id, peer);
        assert_eq!(metrics.role, PeerRole::Peer);
        assert_eq!(metrics.relay_count, 5);
        assert_eq!(metrics.relay_failures, 0);
        assert!(metrics.score > 0.0, "score should be positive");
        assert_eq!(metrics.bytes_relayed, 10 * 1_048_576);
        assert!(
            (metrics.success_rate - 1.0).abs() < f64::EPSILON,
            "100% success rate"
        );
    }

    #[test]
    fn get_all_role_scores_via_command() {
        let mut state = default_state(1);
        let peer_a = node_id(2);
        let peer_b = node_id(3);
        let now = now_ms();

        for &peer in &[peer_a, peer_b] {
            state.topology.upsert(PeerInfo {
                node_id: peer,
                role: PeerRole::Peer,
                status: PeerStatus::Online,
                last_seen: now,
            });
        }

        // Only peer_a has relay activity
        state.role_manager.record_relay(peer_a, now);

        let (tx, mut rx) = tokio::sync::oneshot::channel();
        let effects = state.handle_command(RuntimeCommand::GetAllRoleScores {
            reply: tx,
        });
        assert!(effects.is_empty());

        let scores = rx.try_recv().expect("should receive response");
        assert_eq!(scores.len(), 2, "should list both peers");

        let a_entry = scores.iter().find(|(id, _, _)| *id == peer_a);
        let b_entry = scores.iter().find(|(id, _, _)| *id == peer_b);

        assert!(a_entry.is_some(), "peer_a should be in scores");
        assert!(b_entry.is_some(), "peer_b should be in scores");

        let (_, a_score, _) = a_entry.unwrap();
        let (_, b_score, _) = b_entry.unwrap();
        assert!(a_score > b_score, "active peer should score higher");
    }

    #[test]
    fn bandwidth_tracking_via_role_metrics_command() {
        let mut state = default_state(1);
        let relay = node_id(2);
        let now = now_ms();

        state.topology.upsert(PeerInfo {
            node_id: relay,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: now,
        });

        state.role_manager.record_bytes_relayed(relay, 100 * 1_048_576, now);
        state.role_manager.record_bytes_received(relay, 50 * 1_048_576, now);

        let (tx, mut rx) = tokio::sync::oneshot::channel();
        state.handle_command(RuntimeCommand::GetRoleMetrics {
            node_id: relay,
            reply: tx,
        });

        let metrics = rx.try_recv().unwrap().unwrap();
        assert_eq!(metrics.bytes_relayed, 100 * 1_048_576);
        assert_eq!(metrics.bytes_received, 50 * 1_048_576);
        assert!(
            (metrics.bandwidth_ratio - 2.0).abs() < f64::EPSILON,
            "bandwidth ratio should be 2.0 (100/50): {}",
            metrics.bandwidth_ratio
        );
    }
}
