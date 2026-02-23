use crate::backup::{BackupAction, BackupCoordinator, BackupEvent};
use crate::discovery::{
    DiscoveryEvent, EphemeralSubnetManager, HeartbeatTracker, PeerAnnounce, SubnetEvent,
};
use crate::envelope::EnvelopeBuilder;
use crate::group::{GroupAction, GroupEvent, GroupHub, GroupManager, GroupPayload};
use crate::relay::{PeerRole, PeerStatus, RelaySelector, Topology};
use crate::roles::{RoleAction, RoleManager};
use crate::router::Router;
use crate::tracker::MessageTracker;
use crate::types::{now_ms, MessageType, NodeId};

use super::effect::RuntimeEffect;
use super::{ProtocolEvent, RuntimeConfig};

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
    }
}

/// Etat complet du protocole — logique pure, zero async, zero reseau.
///
/// Chaque methode handle_* / tick_* retourne Vec<RuntimeEffect>.
/// Aucune methode ne touche au reseau ni aux channels.
#[allow(dead_code)] // group_manager used by handle_* methods (Tasks 7-10)
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

    /// Check all peers for liveness, handle offline/reconnect events.
    ///
    /// - PeerOffline: remove from subnets + role_manager, emit events.
    /// - PeerOnline (reconnect): emit PeerDiscovered, prepare backup delivery.
    /// - PeerStale: ignored for MVP.
    pub fn tick_heartbeat(&mut self) -> Vec<RuntimeEffect> {
        let mut effects = Vec::new();

        let events = self.heartbeat.check_all(&mut self.topology);
        for disc_event in events {
            match disc_event {
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
                    effects.push(RuntimeEffect::Emit(ProtocolEvent::PeerDiscovered {
                        node_id,
                    }));
                    effects.extend(self.prepare_backup_delivery(node_id));
                }
                _ => {} // PeerStale, PeerDiscovered — log or ignore for MVP
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

    /// Convert a RoleAction into RuntimeEffects. Updates local_roles on LocalRoleChanged.
    fn surface_role_action(&mut self, action: &RoleAction) -> Vec<RuntimeEffect> {
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
                self.local_roles = vec![*new_role];
                ProtocolEvent::LocalRoleChanged {
                    new_role: *new_role,
                }
            }
        };
        vec![RuntimeEffect::Emit(proto_event)]
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
    fn tick_heartbeat_peer_reconnect_emits_discovered() {
        let mut state = default_state(1);
        let peer = node_id(2);

        // Put peer in Offline status in topology, then give it a recent heartbeat
        // so check_all sees it as alive (PeerOnline).
        state.topology.upsert(crate::relay::PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Offline,
            last_seen: 0,
        });
        // Record a fresh heartbeat so elapsed is near 0 → Alive
        state.heartbeat.record_heartbeat(peer);

        let effects = state.tick_heartbeat();

        let has_discovered = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::PeerDiscovered { node_id }) if *node_id == peer)
        });
        assert!(
            has_discovered,
            "expected PeerDiscovered event on reconnect, got: {effects:?}"
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
}
