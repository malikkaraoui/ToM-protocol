use std::cmp::Reverse;

use crate::backup::{BackupAction, BackupCoordinator, BackupEvent};
use crate::discovery::{
    DiscoveryEvent, DiscoverySource, EphemeralSubnetManager, HeartbeatTracker, PeerAnnounce,
    SubnetEvent,
};
use crate::envelope::{Envelope, EnvelopeBuilder};
use crate::group::{
    GroupAction, GroupEvent, GroupHub, GroupId, GroupManager, GroupMessage, GroupPayload,
};
use crate::relay::{PeerInfo, PeerRole, PeerStatus, RelaySelector, Topology};
use crate::roles::{RoleAction, RoleManager};
use crate::router::{AckType, ReadReceiptPayload, Router, RoutingAction};
use crate::tracker::MessageTracker;
use crate::types::{now_ms, MessageStatus, MessageType, NodeId};

use super::effect::RuntimeEffect;
use super::{DeliveredMessage, ProtocolEvent, RuntimeCommand, RuntimeConfig};

// Phase R7.1: DHT discovery
use tom_dht::{DhtDiscovery, DhtNodeAddr};

/// Gossip event input for RuntimeState (avoids leaking gossip types).
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
        GroupPayload::ShadowAssigned { .. } => MessageType::GroupShadowAssigned,
        GroupPayload::HubUnreachable { .. } => MessageType::GroupHubUnreachable,
        GroupPayload::KickMember { .. } => MessageType::GroupKickMember,
        GroupPayload::UpdateMemberRole { .. } => MessageType::GroupUpdateMemberRole,
        GroupPayload::MemberRoleChanged { .. } => MessageType::GroupMemberRoleChanged,
        GroupPayload::InviteMember { .. } => MessageType::GroupInviteMember,
        GroupPayload::SyncRequest { .. } => MessageType::GroupSyncRequest,
        GroupPayload::SyncResponse { .. } => MessageType::GroupSyncResponse,
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

    // Phase R7.1: DHT-based peer discovery
    pub(crate) dht: Option<DhtDiscovery>,

    // Phase R8.2: State persistence
    pub(crate) store: Option<crate::storage::StateStore>,

    // Phase R9.2: Envelope cache for ACK-timeout retry
    pub(crate) pending_envelopes: std::collections::HashMap<String, crate::envelope::Envelope>,

    // Phase R11.1: Progressive anti-spam
    pub(crate) antispam: crate::roles::AntiSpam,

    // L1-001: Proof of Presence — ephemeral only, never persisted (LOCKED #2)
    pub(crate) presence: crate::presence::PresenceManager,

    // L1-003: witness-side presence observations — this node, as a relay on a
    // routing path, records first-hand which peers it saw alive via the signed
    // ACKs it forwarded. Ephemeral (30s TTL), never persisted (LOCKED #2).
    pub(crate) witness_log: crate::presence::WitnessLog,

    // L1-003: relay-side subscription table — weak devices that asked us to
    // publish presence views scoped to their contacts (D3). Ephemeral, TTL,
    // bounded. Never persisted.
    pub(crate) subscriptions: crate::presence::SubscriptionTable,

    // L1-003: consumer-side quorum aggregator — a peer is only promoted
    // Known → Online when ≥ required_witnesses DISTINCT witnesses concur
    // (kill-shot #3). `presence_view_activity` is a per-window view counter
    // feeding the dynamic quorum; reset each presence-cleanup tick.
    pub(crate) quorum: crate::presence::QuorumAggregator,
    pub(crate) presence_view_activity: usize,

    // Phase R16: Embedded relay logical state (tracked by pure state, no I/O)
    pub(crate) embedded_relay_state: super::LocalEmbeddedRelayState,
    pub(crate) embedded_relay_publication: super::EmbeddedRelayPublicationState,

    // Phase R16: Relay registry (passive consumption of RelayReadyAnnounce)
    pub(crate) relay_registry: crate::discovery::RelayRegistry,
}

impl RuntimeState {
    /// Creer un nouvel etat de protocole.
    pub fn new(local_id: NodeId, secret_seed: [u8; 32], config: RuntimeConfig) -> Self {
        // Phase R7.1: Initialize DHT if enabled
        let dht = if config.enable_dht {
            match DhtDiscovery::new() {
                Ok(d) => {
                    tracing::info!("DHT discovery enabled");
                    Some(d)
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize DHT: {e}");
                    None
                }
            }
        } else {
            tracing::info!("DHT discovery disabled");
            None
        };

        // Phase R8.2: Open state store and load persistent state
        let store = config.data_dir.as_ref().and_then(|dir| {
            let db_path = dir.join("state.db");
            match crate::storage::StateStore::open(&db_path) {
                Ok(s) => {
                    tracing::info!("State store opened: {}", db_path.display());
                    Some(s)
                }
                Err(e) => {
                    tracing::error!("Failed to open state store {}: {e}", db_path.display());
                    None
                }
            }
        });

        let mut group_manager = GroupManager::new(local_id, config.username.clone());
        let mut group_hub = GroupHub::new(local_id);
        let mut topology = Topology::new();
        let mut role_manager = RoleManager::new(local_id);
        let mut tracker = MessageTracker::new();

        if let Some(ref s) = store {
            match s.load() {
                Ok(snapshot) => {
                    if let Some(mgr_snap) = snapshot.manager {
                        let group_count = mgr_snap.groups.len();
                        let key_count = mgr_snap.local_sender_keys.len();
                        group_manager.restore(mgr_snap);
                        tracing::info!("Restored {group_count} groups, {key_count} sender keys");
                    }
                    if let Some(hub_snap) = snapshot.hub {
                        let hub_count = hub_snap.groups.len();
                        group_hub.restore(hub_snap);
                        tracing::info!("Restored {hub_count} hub groups");
                    }
                    for peer in snapshot.peers.values() {
                        let mut peer = peer.clone();
                        peer.status = PeerStatus::Offline; // QUIC connections lost on restart
                        topology.upsert(peer);
                    }
                    if !snapshot.peers.is_empty() {
                        tracing::info!("Restored {} peers (all marked Offline)", snapshot.peers.len());
                    }
                    if !snapshot.metrics.is_empty() {
                        let count = snapshot.metrics.len();
                        role_manager.restore_scores(snapshot.metrics);
                        tracing::info!("Restored {count} contribution metrics");
                    }
                    if !snapshot.tracked_messages.is_empty() {
                        let count = snapshot.tracked_messages.len();
                        tracker.restore(snapshot.tracked_messages);
                        tracing::info!("Restored {count} tracked messages");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load state: {e}");
                }
            }
        }

        Self {
            router: Router::new(local_id),
            relay_selector: RelaySelector::new(local_id),
            topology,
            tracker,
            heartbeat: HeartbeatTracker::new(),
            group_manager,
            group_hub,
            backup: BackupCoordinator::new(local_id),
            subnets: EphemeralSubnetManager::new(local_id),
            role_manager,
            local_roles: vec![PeerRole::Peer],
            role_announce_throttle: std::collections::HashMap::new(),
            dht,
            antispam: crate::roles::AntiSpam::new(config.antispam_config.clone()),
            presence: crate::presence::PresenceManager::new(),
            witness_log: crate::presence::WitnessLog::new(),
            subscriptions: crate::presence::SubscriptionTable::new(),
            quorum: crate::presence::QuorumAggregator::new(),
            presence_view_activity: 0,
            local_id,
            secret_seed,
            embedded_relay_state: super::LocalEmbeddedRelayState::Stopped,
            embedded_relay_publication: super::EmbeddedRelayPublicationState::NotPublished,
            relay_registry: crate::discovery::RelayRegistry::new(
                config.relay_registry_ttl.as_millis() as u64,
            ),
            config,
            store,
            pending_envelopes: std::collections::HashMap::new(),
        }
    }

    // ── Public accessors (for integration tests + consumers) ──────────

    /// This node's identity.
    pub fn local_id(&self) -> NodeId {
        self.local_id
    }

    /// Access the group manager (member-side state).
    pub fn group_manager(&self) -> &GroupManager {
        &self.group_manager
    }

    /// Access the group hub (hub-side state).
    pub fn group_hub(&self) -> &GroupHub {
        &self.group_hub
    }

    /// Access the topology.
    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    /// Access the role manager.
    pub fn role_manager(&self) -> &RoleManager {
        &self.role_manager
    }

    /// Access the message tracker.
    pub fn tracker(&self) -> &MessageTracker {
        &self.tracker
    }

    /// Publish this node's address to the DHT (BEP-0044).
    ///
    /// Called at startup and periodically (every 30 min) to keep our
    /// DHT record fresh. The loop passes real addresses from TomNode.
    pub(crate) async fn publish_to_dht(
        &self,
        signing_key: &[u8; 32],
        relay_urls: Vec<String>,
        direct_addrs: Vec<String>,
    ) {
        if let Some(ref dht) = self.dht {
            let our_addr = DhtNodeAddr {
                node_id: self.local_id.to_string(),
                relay_urls,
                direct_addrs,
                timestamp: now_ms(),
                // Per-node record under the node's OWN key → already BEP-0044
                // authenticated; no app-level signature needed here.
                ..Default::default()
            };

            if let Err(e) = dht.publish(signing_key, &our_addr).await {
                tracing::warn!("Failed to publish to DHT: {e}");
            } else {
                tracing::info!("Published to DHT: {}", self.local_id);
            }
        }
    }

    /// Check if DHT is enabled and return a reference for spawning lookups.
    pub(crate) fn dht(&self) -> Option<&DhtDiscovery> {
        self.dht.as_ref()
    }

    // ── State persistence ───────────────────────────────────────────────

    /// Save current state to SQLite (called periodically by runtime loop).
    pub fn save_state(&self) {
        let Some(ref store) = self.store else { return };

        let snapshot = crate::storage::StateSnapshot {
            manager: Some(self.group_manager.snapshot()),
            hub: Some(self.group_hub.snapshot()),
            peers: self.topology.peers_map().clone(),
            metrics: self.role_manager.scores().clone(),
            tracked_messages: self.tracker.snapshot(),
        };

        if let Err(e) = store.save(&snapshot) {
            tracing::error!("Failed to save state: {e}");
        } else {
            tracing::debug!("State saved to SQLite");
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
        // Clean orphaned envelope cache entries (R9.2)
        self.pending_envelopes
            .retain(|id, _| self.tracker.status(id).is_some());
        Vec::new()
    }

    /// Check for messages whose ACK deadline expired. Retry or mark failed.
    pub fn tick_delivery_deadlines(&mut self) -> Vec<RuntimeEffect> {
        let expired = self.tracker.expired_deadlines();
        let mut effects = Vec::new();

        for (message_id, to, retries_remaining) in expired {
            if retries_remaining > 0 {
                // Retry: resend cached envelope
                if let Some(envelope) = self.pending_envelopes.get(&message_id).cloned() {
                    self.tracker.reset_deadline(&message_id);
                    let attempt =
                        crate::tracker::DEFAULT_MAX_RETRIES - retries_remaining + 1;
                    effects.push(RuntimeEffect::SendEnvelope(envelope));
                    effects.push(RuntimeEffect::Emit(ProtocolEvent::DeliveryRetry {
                        message_id,
                        to,
                        attempt,
                    }));
                }
            } else {
                // All retries exhausted — mark failed
                let last_status = self
                    .tracker
                    .status(&message_id)
                    .unwrap_or(MessageStatus::Pending);
                if let Some(change) = self.tracker.mark_failed(&message_id) {
                    effects.push(RuntimeEffect::StatusChange(change));
                }
                self.pending_envelopes.remove(&message_id);
                effects.push(RuntimeEffect::Emit(ProtocolEvent::DeliveryTimeout {
                    message_id,
                    to,
                    last_status,
                }));
            }
        }
        effects
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

        let departed = self.heartbeat.cleanup_departed();
        for node_id in &departed {
            self.topology.remove(node_id);
        }

        // Prune expired relay registry entries
        let expired_relays = self.relay_registry.prune(now_ms());
        for entry in &expired_relays {
            effects.push(RuntimeEffect::Emit(ProtocolEvent::RelayRegistryExpired {
                node_id: entry.node_id,
                relay_url: entry.relay_url.clone(),
            }));
        }

        // Transport relay discovery: remove expired URLs from transport layer
        if self.config.enable_transport_relay_discovery {
            for entry in &expired_relays {
                // Only remove if no other active entry still references this URL
                if !self.relay_registry.has_active_url(&entry.relay_url) {
                    effects.push(RuntimeEffect::RemoveTransportRelay {
                        relay_url: entry.relay_url.clone(),
                    });
                }
            }
        }

        // Periodic republication of embedded relay announcement.
        // Piggybacks on heartbeat tick — effective cadence granularized by heartbeat_interval.
        if let super::EmbeddedRelayPublicationState::Published { ref url, published_at } =
            self.embedded_relay_publication
        {
            let interval_ms = self.config.relay_publish_interval.as_millis() as u64;
            if now_ms().saturating_sub(published_at) >= interval_ms {
                let url = url.clone();
                effects.extend(self.build_relay_publication(url));
            }
        }

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
        let actions = self.intercept_self_group_actions(actions);
        self.group_actions_to_effects(&actions)
    }

    // ── Tick: shadow ping watchdog ──────────────────────────────────────

    /// Shadow watchdog tick — send HubPing to primary for each group we shadow.
    ///
    /// Also checks for a prior ping that timed out with no pong: without this,
    /// a hub that goes silent is never counted as a failure and failover
    /// never triggers (`record_ping_failure` stayed dead at runtime).
    pub fn tick_shadow_ping(&mut self) -> Vec<RuntimeEffect> {
        let now = now_ms();

        let timeout_actions = self.group_manager.check_ping_timeouts(now);
        let timeout_actions = self.intercept_self_group_actions(timeout_actions);
        let mut effects = self.group_actions_to_effects(&timeout_actions);

        let shadow_groups: Vec<(crate::group::GroupId, NodeId)> = self
            .group_manager
            .shadow_groups()
            .into_iter()
            .map(|(gid, hub)| (gid.clone(), hub))
            .collect();

        for (group_id, hub_id) in shadow_groups {
            self.group_manager.note_ping_sent(&group_id, now);
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

    /// Purge expired hub messages (in-memory + SQLite). 24h TTL.
    pub fn tick_hub_cleanup(&mut self) -> Vec<RuntimeEffect> {
        const TTL_MS: u64 = 24 * 60 * 60 * 1000; // 24 hours
        let now = now_ms();

        // In-memory cleanup
        let mem_purged = self.group_hub.cleanup_expired_messages(now, TTL_MS);

        // SQLite cleanup — cutoff is an absolute timestamp, not a duration.
        let db_purged = if let Some(ref store) = self.store {
            store
                .cleanup_hub_messages(now.saturating_sub(TTL_MS))
                .unwrap_or(0)
        } else {
            0
        };

        // R14.3: purge expired sender keys (>7 days) on member + hub state.
        let mgr_keys_purged = self
            .group_manager
            .purge_expired_sender_keys(now, crate::group::SENDER_KEY_PURGE_MAX_AGE_MS);
        let hub_keys_purged = self
            .group_hub
            .purge_expired_sender_keys(now, crate::group::SENDER_KEY_PURGE_MAX_AGE_MS);

        if mem_purged + db_purged + mgr_keys_purged + hub_keys_purged > 0 {
            tracing::info!(
                "hub cleanup: purged {} in-memory + {} SQLite messages (>24h), {} manager keys + {} hub key entries (>7d)",
                mem_purged,
                db_purged,
                mgr_keys_purged,
                hub_keys_purged
            );
        }

        Vec::new() // no effects
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

    /// Build a relay-ready publication if the relay is healthy and publication is enabled.
    ///
    /// Returns a BroadcastRelayReady effect, or empty if conditions aren't met.
    fn build_relay_publication(&mut self, url: tom_connect::RelayUrl) -> Vec<RuntimeEffect> {
        // Guard: publication must be enabled
        if !self.config.enable_embedded_relay_publication {
            return Vec::new();
        }
        // Guard: relay must be healthy
        if !matches!(self.embedded_relay_state, super::LocalEmbeddedRelayState::Healthy { .. }) {
            return Vec::new();
        }
        // Universal reachability rule (no profile, env-driven): only advertise a
        // relay to the *global* gossip if its address is reachable from outside
        // the LAN. A relay bound to a private/loopback IP is usable locally but
        // would trap remote peers in an unreachable rendezvous, so it stays unpublished.
        if !relay_url_is_globally_reachable(&url) {
            tracing::info!(%url, "embedded relay bound to a non-global address — LAN-only, not published");
            return Vec::new();
        }

        let now = now_ms();
        let announce = crate::discovery::RelayReadyAnnounce::new(
            self.local_id,
            url.clone(),
            now,
            &self.secret_seed,
        );

        self.embedded_relay_publication = super::EmbeddedRelayPublicationState::Published {
            url,
            published_at: now,
        };

        tracing::info!("publishing embedded relay via gossip");
        vec![RuntimeEffect::BroadcastRelayReady(announce)]
    }

    /// Build effects to rejoin all restored groups (called once at startup).
    ///
    /// After a restart, groups are loaded from SQLite but the hub doesn't know
    /// we're back online. This sends a Join to each group's hub, which triggers
    /// a re-sync with current state and sender keys.
    pub fn build_rejoin_effects(&mut self) -> Vec<RuntimeEffect> {
        let actions = self.group_manager.rejoin_groups();
        if actions.is_empty() {
            return Vec::new();
        }
        tracing::info!("Rejoining {} groups after restart", actions.len());
        let actions = self.intercept_self_group_actions(actions);
        self.group_actions_to_effects(&actions)
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

    // ── Handle incoming relay-ready announcement ──────────────────────

    /// Handle a RelayReadyAnnounce from gossip.
    ///
    /// Validates signature, freshness, and emits an event for observability.
    /// Does NOT auto-select this relay — that's a future chantier.
    fn handle_relay_ready_announce(
        &mut self,
        announce: crate::discovery::RelayReadyAnnounce,
    ) -> Vec<RuntimeEffect> {
        // Ignore our own announcements
        if announce.node_id == self.local_id {
            return Vec::new();
        }

        // Verify signature
        if !announce.verify_signature() {
            tracing::warn!(
                node_id = %announce.node_id,
                "invalid signature on RelayReadyAnnounce"
            );
            return Vec::new();
        }

        // Check freshness
        if !announce.is_fresh(now_ms()) {
            tracing::debug!(
                node_id = %announce.node_id,
                "stale RelayReadyAnnounce (timestamp too old)"
            );
            return Vec::new();
        }

        tracing::info!(
            node_id = %announce.node_id,
            relay_url = %announce.relay_url,
            "received RelayReadyAnnounce"
        );

        // Store in registry
        let upsert_result = self.relay_registry.upsert(
            announce.node_id,
            announce.relay_url.clone(),
            announce.timestamp,
            now_ms(),
        );

        let mut effects = vec![RuntimeEffect::Emit(ProtocolEvent::RelayReadyReceived {
            node_id: announce.node_id,
            relay_url: announce.relay_url.clone(),
        })];

        // Transport relay discovery: inject/update relay URLs in transport layer
        if self.config.enable_transport_relay_discovery {
            use crate::discovery::UpsertResult;
            match upsert_result {
                UpsertResult::Inserted => {
                    effects.push(RuntimeEffect::InsertTransportRelay {
                        relay_url: announce.relay_url,
                    });
                }
                UpsertResult::UpdatedUrl { old_url } => {
                    effects.push(RuntimeEffect::InsertTransportRelay {
                        relay_url: announce.relay_url,
                    });
                    // Remove old URL only if no other active entry references it
                    if !self.relay_registry.has_active_url(&old_url) {
                        effects.push(RuntimeEffect::RemoveTransportRelay {
                            relay_url: old_url,
                        });
                    }
                }
                UpsertResult::RefreshedSameUrl => {
                    // No transport mutation needed on refresh
                }
            }
        }

        effects
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

            // Duplicate delivery (sender resent after a lost ACK): re-confirm by
            // re-sending the signed ACK, WITHOUT delivering to the app again.
            // Guarantees delivered ⟺ ACK (decision #1) survives ACK loss.
            RoutingAction::ReAck { response } => {
                let mut ack = response;
                ack.sign(&self.secret_seed);
                vec![RuntimeEffect::SendEnvelope(ack)]
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

                // L1-003 witness observation: when this relay forwards a SIGNED
                // ACK, the ACK's `from` cryptographically proved it was alive at
                // `now` (it signed over signing_bytes bound to `from`). Record it
                // as first-hand presence evidence, tied to the real acked message
                // id — never hearsay (ADR-011 §2). Gated on `signature_valid` so a
                // forged/unsigned ACK earns no observation. Only ACKs qualify: a
                // forwarded Chat isn't a presence proof by itself. Store the raw
                // envelope bytes so the consumer can cryptographically verify the proof.
                if signature_valid && envelope.msg_type == MessageType::Ack {
                    if let Ok(ack_payload) =
                        crate::router::AckPayload::from_bytes(&envelope.payload)
                    {
                        let envelope_bytes = envelope
                            .to_bytes()
                            .unwrap_or_default(); // serialize envelope for proof verification
                        self.witness_log.record(
                            envelope.from,
                            ack_payload.original_message_id,
                            ack_payload.ack_type,
                            now,
                            envelope_bytes,
                        );
                    }
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
                from,
            } => {
                // Verrou #1 (delivered ⟺ ACK): an unsigned or forged ACK must
                // never promote a message's status — otherwise anyone can
                // fabricate delivery/relay confirmation for a message they
                // never received.
                if !signature_valid {
                    return vec![RuntimeEffect::Emit(ProtocolEvent::MessageRejected {
                        reason: "forged or unsigned ACK rejected".into(),
                    })];
                }
                let change = match ack_type {
                    AckType::RelayForwarded => {
                        // L1-001: a signed RelayForwarded ACK is locally
                        // verified, cryptographic evidence that `from`
                        // relayed for us — this feeds the presence
                        // anti-Sybil gate (local score of the attester).
                        //
                        // Verrou anti-pumping (FINDING #7): credit relay
                        // evidence ONLY when the ACK matches a REAL message we
                        // originated AND `from` is not its final recipient (so
                        // `from` is plausibly a relay on the path). Without this,
                        // an attacker forges RelayForwarded ACKs with random
                        // message_ids — each escaping the anti-replay cache
                        // (distinct key) — to pump its local relay score without
                        // relaying anything, then forges its way past the
                        // presence gate / stranger cap. The anti-replay cache
                        // alone only stops REPLAY of the same ACK.
                        let is_real_relay = self
                            .tracker
                            .recipient_of(&original_message_id)
                            .is_some_and(|recipient| recipient != from);
                        if is_real_relay {
                            self.role_manager.record_relay(from, now_ms());
                        }
                        self.tracker.mark_relayed(&original_message_id)
                    }
                    AckType::RecipientReceived => {
                        // mark_delivered binds the ACK to the intended recipient
                        // (`from`): a signed ACK from a non-recipient (e.g. a
                        // relay on the path that dropped the message) is ignored.
                        // Only stop retrying once delivery is GENUINELY confirmed
                        // — a forged ACK must not silence the retry loop either.
                        let change = self.tracker.mark_delivered(&original_message_id, from);
                        if change.is_some() {
                            self.pending_envelopes.remove(&original_message_id);
                        }
                        change
                    }
                };
                change
                    .into_iter()
                    .map(RuntimeEffect::StatusChange)
                    .collect()
            }

            RoutingAction::ReadReceipt {
                original_message_id,
                from,
                ..
            } => {
                // Same guards as delivery ACK: reject forged/unsigned receipts,
                // and bind the receipt to the intended recipient (`from`).
                if !signature_valid {
                    return vec![RuntimeEffect::Emit(ProtocolEvent::MessageRejected {
                        reason: "forged or unsigned read receipt rejected".into(),
                    })];
                }
                self.tracker
                    .mark_read(&original_message_id, from)
                    .into_iter()
                    .map(RuntimeEffect::StatusChange)
                    .collect()
            }

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
        // Computed before decrypt_payload mutates the payload in place (which
        // would invalidate verify_signature — encrypt-then-sign covers the
        // ciphertext). Used to gate HubMigration against split-brain hijack.
        let signature_valid = envelope.is_signed() && envelope.verify_signature().is_ok();

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
            | GroupPayload::Leave { .. }
            | GroupPayload::KickMember { .. }
            | GroupPayload::UpdateMemberRole { .. }
            | GroupPayload::InviteMember { .. } => {
                // Extract group_id from known payloads before consuming; for Create we find it after.
                let known_group_id = match &group_payload {
                    GroupPayload::Join { group_id, .. }
                    | GroupPayload::Leave { group_id, .. }
                    | GroupPayload::KickMember { group_id, .. }
                    | GroupPayload::UpdateMemberRole { group_id, .. }
                    | GroupPayload::InviteMember { group_id, .. } => Some(group_id.clone()),
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
            GroupPayload::MemberRoleChanged {
                group_id,
                node_id,
                new_role,
            } => self.group_manager.handle_member_role_changed(
                &group_id, &node_id, new_role,
            ),
            GroupPayload::HubMigration {
                group_id,
                new_hub_id,
                ..
            } => {
                // Split-brain guard (part 2): an unsigned/forged envelope must
                // never redirect a group's hub, regardless of what `from`
                // claims (see GroupManager::handle_hub_migration for part 1).
                if !signature_valid {
                    vec![]
                } else {
                    self.group_manager
                        .handle_hub_migration(&group_id, new_hub_id, envelope.from)
                }
            }
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

            // Hub broadcasts the current shadow to all members. Must be
            // signed: an unsigned/forged ShadowAssigned would let an
            // attacker poison a victim's known-good shadow_id to their own
            // NodeId, defeating the handle_hub_migration check above (which
            // trusts group.shadow_id as ground truth).
            GroupPayload::ShadowAssigned { ref group_id, shadow_id } => {
                if !signature_valid {
                    vec![]
                } else {
                    self.group_manager.handle_shadow_assigned(group_id, shadow_id)
                }
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

            // ── R13: Offline delivery gap-fill ──────────────────────────
            GroupPayload::SyncRequest { ref group_id, since_seq } => {
                self.handle_sync_request(envelope.from, group_id, since_seq)
            }

            GroupPayload::SyncResponse { group_id, messages, latest_seq } => {
                self.handle_sync_response(&group_id, messages, latest_seq)
            }
        };

        // Intercept self-addressed group actions: when the hub sends to itself
        // (e.g. MemberJoined broadcast when hub is also a group member),
        // process locally via GroupManager instead of network round-trip.
        let actions = self.intercept_self_group_actions(actions);
        self.group_actions_to_effects(&actions)
    }

    // ── R13: Offline delivery gap-fill ──────────────────────────────────

    /// Handle SyncRequest from a member (hub-side).
    /// Loads missed messages from SQLite and sends SyncResponse.
    fn handle_sync_request(
        &self,
        from: NodeId,
        group_id: &GroupId,
        since_seq: u64,
    ) -> Vec<GroupAction> {
        // Only respond if we're the hub for this group
        if self.group_hub.get_group(group_id).is_none() {
            return vec![];
        }

        // Load missed messages from SQLite (if store is available)
        let mut messages = Vec::new();
        let mut latest_seq = since_seq;

        if let Some(ref store) = self.store {
            const MAX_SYNC_RESPONSE: usize = 500;
            if let Ok(rows) = store.load_hub_messages_since(group_id, since_seq, MAX_SYNC_RESPONSE) {
                for (seq, data) in rows {
                    if let Ok(msg) = rmp_serde::from_slice::<GroupMessage>(&data) {
                        if seq > latest_seq {
                            latest_seq = seq;
                        }
                        messages.push(msg);
                    }
                }
            }
        }

        // Also check in-memory history for any messages not yet in SQLite
        // (messages received since last persist cycle)
        if let Some(history) = self.group_hub.message_history(group_id) {
            for msg in history {
                if msg.seq > since_seq && !messages.iter().any(|m| m.message_id == msg.message_id) {
                    if msg.seq > latest_seq {
                        latest_seq = msg.seq;
                    }
                    messages.push(msg.clone());
                }
            }
            // Sort by seq to ensure ordering
            messages.sort_by_key(|m| m.seq);
        }

        if messages.is_empty() {
            return vec![];
        }

        vec![GroupAction::Send {
            to: from,
            payload: GroupPayload::SyncResponse {
                group_id: group_id.clone(),
                messages,
                latest_seq,
            },
        }]
    }

    /// Handle SyncResponse from hub (member-side).
    /// Delivers missed messages and updates last_seq.
    fn handle_sync_response(
        &mut self,
        group_id: &GroupId,
        messages: Vec<GroupMessage>,
        latest_seq: u64,
    ) -> Vec<GroupAction> {
        let mut actions = Vec::new();

        for msg in messages {
            // Deliver each missed message through the normal path
            let msg_actions = self.group_manager.handle_message(msg);
            actions.extend(msg_actions);
        }

        // Update last_seq to the latest from the response
        let current = self.group_manager.last_seq(group_id);
        if latest_seq > current {
            // Directly update via a method we'll add
            self.group_manager.set_last_seq(group_id, latest_seq);
        }

        actions
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
                // FINDING #9: a depositor we hold real contribution evidence for
                // is "known"; its backups survive eviction under a flood. Fresh
                // Sybil identities score 0 → stranger → evicted first / refused.
                let depositor_known = self.has_sustained_relay_evidence(&envelope.from, now);
                let actions = self.backup.handle_replication(
                    &payload,
                    envelope.from,
                    depositor_known,
                    now,
                );
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

    /// Is `node_id` KNOWN (relay-evidenced), i.e. exempt from stranger-flood
    /// gates? Requires BOTH the decaying contribution score AND a minimum
    /// count of successful relays (red-team PoP kill-shot #4): the score
    /// alone is trivially cleared by a single successful relay
    /// (`SUCCESS_RATE_WEIGHT` rewards 1/1 the same as 1000/1000), letting a
    /// Sybil farm buy ~36h of KNOWN status per identity for one cheap relay
    /// each. The relay-count floor forces sustained work per identity.
    fn has_sustained_relay_evidence(&self, node_id: &NodeId, now: u64) -> bool {
        let score_ok =
            self.role_manager.score(node_id, now) >= crate::presence::RESPONDER_KNOWN_MIN_SCORE;
        let relay_count_ok = self
            .role_manager
            .scores()
            .get(node_id)
            .is_some_and(|m| m.messages_relayed >= crate::presence::MIN_SUSTAINED_RELAYS_FOR_KNOWN);
        score_ok && relay_count_ok
    }

    /// Register a peer address learned from discovery (DHT/gossip/neighbor-up)
    /// without granting presence credit (ADR-011 PoP, ghost-peer fix).
    ///
    /// Discovery proves an address is reachable, not that the peer is alive
    /// — only real work (signed inbound, ACK, witnessed relay) should mark a
    /// peer `Online` and feed the heartbeat tracker. If the peer is already
    /// known with a stronger status (`Online`/`Stale`), this is a no-op: a
    /// re-announce must never downgrade a status earned by real work back to
    /// merely `Known`, nor refresh its `last_seen` for free.
    fn mark_known(&mut self, node_id: NodeId, role: PeerRole) {
        if self.topology.get(&node_id).is_some() {
            return;
        }
        self.topology.upsert(PeerInfo {
            node_id,
            role,
            status: PeerStatus::Known,
            last_seen: now_ms(),
        });
    }

    // ── Task 8: handle_incoming (unified dispatcher) ─────────────────────

    /// Unified entry point for all incoming raw data.
    ///
    /// Parses the envelope, verifies signature, auto-registers the peer,
    /// records heartbeat, then dispatches to the appropriate handler.
    pub fn handle_incoming(&mut self, raw_data: &[u8]) -> Vec<RuntimeEffect> {
        // Anti-spam: size check BEFORE parse (save CPU on oversized envelopes)
        if let Err(reason) = crate::roles::AntiSpam::validate_size(
            raw_data,
            self.config.antispam_config.max_envelope_size,
        ) {
            return vec![RuntimeEffect::Emit(ProtocolEvent::MessageRejected { reason })];
        }

        // Parse envelope
        let envelope = match Envelope::from_bytes(raw_data) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        // Anti-spam: rate check only for payload-carrying message types.
        // Protocol-internal messages (Ack, Heartbeat, ReadReceipt) are exempt — they
        // are generated by the protocol itself and throttling them breaks delivery
        // confirmation and peer liveness detection.
        let now = now_ms();
        let exempt = matches!(
            envelope.msg_type,
            MessageType::Ack | MessageType::Heartbeat | MessageType::ReadReceipt
        );
        if !exempt {
            let sender_score = self.role_manager.score(&envelope.from, now);
            if let Err(_reason) = self.antispam.check_rate(envelope.from, sender_score, now) {
                let current_rate = self.antispam.compute_rate(sender_score);
                return vec![RuntimeEffect::Emit(ProtocolEvent::SenderThrottled {
                    node_id: envelope.from,
                    score: sender_score,
                    current_rate,
                })];
            }
        }

        // Verify signature — gates presence/bandwidth credit below. `from` is bound
        // into signing_bytes(), so a spoofed `from` cannot produce a valid signature.
        // Without this gate (PoP red-team #1), the credit below was granted
        // unconditionally on the claimed `from`, letting an attacker mark any victim
        // Online and inflate its bytes_received/bandwidth_ratio with garbage traffic.
        let signature_valid = if envelope.is_signed() {
            envelope.verify_signature().is_ok()
        } else {
            false
        };

        if signature_valid {
            // Track bytes received (fixes bandwidth_ratio calculation)
            self.role_manager
                .record_bytes_received(envelope.from, raw_data.len() as u64, now);

            // Record heartbeat + auto-register / revive stale peers.
            // Always refresh last_seen and force Online — a message receipt is proof of liveness.
            // Without this, peers that went Stale (>20s) or Offline (>45s) between state_save
            // ticks would show online_count=0 even though messages keep flowing.
            self.heartbeat.record_heartbeat(envelope.from);
            let existing_role = self
                .topology
                .get(&envelope.from)
                .map(|p| p.role)
                .unwrap_or(PeerRole::Peer);
            self.topology.upsert(PeerInfo {
                node_id: envelope.from,
                role: existing_role,
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
            | MessageType::GroupShadowAssigned
            | MessageType::GroupHubUnreachable
            | MessageType::GroupKickMember
            | MessageType::GroupUpdateMemberRole
            | MessageType::GroupMemberRoleChanged
            | MessageType::GroupInviteMember
            | MessageType::GroupSyncRequest
            | MessageType::GroupSyncResponse => {
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

            MessageType::PresenceChallenge | MessageType::PresenceAttestation => {
                self.handle_incoming_presence(envelope, signature_valid)
            }

            MessageType::PresenceSubscribe => {
                self.handle_presence_subscribe(&envelope, signature_valid)
            }

            MessageType::RelayPresenceView => {
                self.handle_relay_presence_view(&envelope, signature_valid)
            }
        }
    }

    // ── L1-001: Proof of Presence ────────────────────────────────────────
    //
    // Hardened per docs/plans/L1-001-attestation-presence.md (V2):
    // signed both ways · one-shot challenges · freshness on OUR clock ·
    // anti-Sybil gate on OUR local score of the attester · silent drops
    // (answering an attacker with a reject is a free oracle).

    /// Issue a presence challenge toward `target` (we are A).
    ///
    /// Returns no effects when a memory cap refuses the challenge (§4.3).
    /// Presence time source. Normally `now_ms()`; a test/simulation harness
    /// can offset it via `config.presence_clock_offset_ms` to inject clock
    /// skew (anti-NTP hardening validation). Every presence handler reads
    /// time through THIS so a node's clock stays internally consistent — the
    /// whole point being that freshness works on one consistent clock.
    fn presence_now(&self) -> u64 {
        let base = now_ms() as i64;
        (base + self.config.presence_clock_offset_ms).max(0) as u64
    }

    pub fn initiate_presence_check(&mut self, target: NodeId) -> Vec<RuntimeEffect> {
        use chacha20poly1305::aead::rand_core::{OsRng, RngCore};

        if target == self.local_id {
            return Vec::new();
        }

        let now = self.presence_now();
        let mut nonce = vec![0u8; crate::presence::NONCE_LEN];
        OsRng.fill_bytes(&mut nonce);
        let challenge_id = uuid::Uuid::new_v4().to_string();

        let registered = self.presence.register_challenge(crate::presence::PendingChallenge {
            challenge_id: challenge_id.clone(),
            nonce: nonce.clone(),
            target,
            issued_at: now,
        });
        if !registered {
            tracing::debug!("presence: challenge refused by memory caps (target {target})");
            return Vec::new();
        }

        let payload = crate::presence::PresenceChallengePayload {
            challenge_id,
            nonce,
            timestamp: now,
            challenger_id: self.local_id,
        };
        let mut envelope = Envelope::new(
            self.local_id,
            target,
            MessageType::PresenceChallenge,
            payload.to_bytes(),
        );
        envelope.sign(&self.secret_seed);

        self.presence.record(crate::presence::PresenceOutcome::Issued);
        vec![RuntimeEffect::SendEnvelope(envelope)]
    }

    /// Challenge many peers at once (stress driving). Returns the send
    /// effects for all challenges that passed the memory caps.
    pub fn initiate_presence_check_many(&mut self, targets: &[NodeId]) -> Vec<RuntimeEffect> {
        let mut effects = Vec::new();
        for &target in targets {
            effects.extend(self.initiate_presence_check(target));
        }
        effects
    }

    /// Dispatch an incoming presence envelope (challenge or attestation).
    fn handle_incoming_presence(
        &mut self,
        envelope: Envelope,
        signature_valid: bool,
    ) -> Vec<RuntimeEffect> {
        // V1 is direct-only (spec §2.2: via = []). A presence envelope not
        // addressed to us is dropped, never forwarded.
        if envelope.to != self.local_id {
            return Vec::new();
        }
        match envelope.msg_type {
            MessageType::PresenceChallenge => {
                self.handle_presence_challenge(envelope, signature_valid)
            }
            MessageType::PresenceAttestation => {
                self.handle_presence_attestation(envelope, signature_valid)
            }
            _ => Vec::new(),
        }
    }

    /// We are B: someone challenges our presence — answer with a signed
    /// attestation, if the challenge is authentic and our budget allows.
    fn handle_presence_challenge(
        &mut self,
        envelope: Envelope,
        signature_valid: bool,
    ) -> Vec<RuntimeEffect> {
        use crate::presence::PresenceOutcome;
        self.presence.record(PresenceOutcome::ChallengeReceived);

        // A9: verify the challenger's signature BEFORE spending one of ours
        // (an unsigned challenge would let an attacker buy Ed25519 work for
        // free, and reflect attestations onto a spoofed `from`).
        if !signature_valid {
            self.presence.record(PresenceOutcome::RefusedBadSignature);
            return Vec::new();
        }

        let payload = match crate::presence::PresenceChallengePayload::from_bytes(&envelope.payload)
        {
            Ok(p) => p,
            Err(_) => {
                self.presence.record(PresenceOutcome::RefusedIncoherent);
                return Vec::new();
            }
        };

        // Identity coherence: the declared challenger IS the envelope signer.
        if payload.challenger_id != envelope.from {
            self.presence.record(PresenceOutcome::RefusedIncoherent);
            return Vec::new();
        }

        let now = self.presence_now();
        if payload.validate(now).is_err() {
            self.presence.record(PresenceOutcome::RefusedIncoherent);
            return Vec::new();
        }

        // A7: signing budget. KNOWN challengers (we hold relay evidence for them
        // — local role score ≥ threshold, un-forgeable by a fresh Sybil) get a
        // per-identity budget and BYPASS the stranger global cap so a swarm
        // can't starve them (FINDING #5). Strangers share the bounded global cap
        // (FINDING #4). Bounded map, self-purging either way.
        let has_evidence = self.has_sustained_relay_evidence(&envelope.from, now);
        if !self.presence.allow_response(envelope.from, has_evidence, now) {
            self.presence.record(PresenceOutcome::RefusedBudget);
            return Vec::new();
        }

        // Self-reported snapshot — ADVISORY ONLY on the challenger side.
        let own_score = self.role_manager.score(&self.local_id, now);
        let attestation = crate::presence::PresenceAttestationPayload {
            challenge_id: payload.challenge_id,
            nonce: payload.nonce,
            timestamp: now,
            attester_id: self.local_id,
            challenger_id: payload.challenger_id,
            relay_proof: crate::presence::RelayProof {
                proof_type: crate::presence::RelayProofType::SelfObserved,
                observer_id: self.local_id,
                observed_at: now,
                bytes_relayed: 0,
                observer_signature: Vec::new(),
                reliability_score: Some(own_score),
            },
        };

        let mut response = Envelope::new(
            self.local_id,
            envelope.from,
            MessageType::PresenceAttestation,
            attestation.to_bytes(),
        );
        response.sign(&self.secret_seed);

        self.presence.record(PresenceOutcome::Signed);
        vec![RuntimeEffect::SendEnvelope(response)]
    }

    /// We are A: an attestation came back. Checks ordered cheapest-first;
    /// every failure is a silent drop (no oracle for the attacker).
    fn handle_presence_attestation(
        &mut self,
        envelope: Envelope,
        signature_valid: bool,
    ) -> Vec<RuntimeEffect> {
        use crate::presence::PresenceOutcome;

        let payload =
            match crate::presence::PresenceAttestationPayload::from_bytes(&envelope.payload) {
                Ok(p) => p,
                Err(_) => {
                    self.presence.record(PresenceOutcome::DropIncoherent);
                    return Vec::new();
                }
            };

        // 1. Challenge issued by us, still pending (one-shot ⇒ a consumed
        //    challenge is gone: replay has nothing to match — A2).
        let (challenge_nonce, challenge_target, issued_at) =
            match self.presence.pending(&payload.challenge_id) {
                Some(c) => (c.nonce.clone(), c.target, c.issued_at),
                None => {
                    self.presence.record(PresenceOutcome::DropUnknownChallenge);
                    return Vec::new();
                }
            };

        // 2. Freshness on OUR clock only (no NTP on this network — A3).
        let now = self.presence_now();
        if now.saturating_sub(issued_at) > crate::presence::PRESENCE_TTL_MS {
            self.presence.record(PresenceOutcome::DropStale);
            return Vec::new();
        }

        // 3. The attestation comes from the node WE challenged (A8 — the
        //    payload travels in clear, any on-path relay knows the nonce).
        if envelope.from != challenge_target {
            self.presence.record(PresenceOutcome::DropWrongAttester);
            return Vec::new();
        }

        // 4. Ed25519 signature of the attester over the envelope.
        if !signature_valid {
            self.presence.record(PresenceOutcome::DropBadSignature);
            return Vec::new();
        }

        // 5. Payload ↔ envelope ↔ challenge coherence, including THE nonce.
        if payload.attester_id != envelope.from
            || payload.challenger_id != self.local_id
            || payload.nonce != challenge_nonce
        {
            self.presence.record(PresenceOutcome::DropIncoherent);
            return Vec::new();
        }

        // 6. Anti-Sybil gate: OUR locally observed relay score of the
        //    attester (A5). NEVER payload.relay_proof.reliability_score —
        //    that field is attacker-controlled. Threshold comes from config
        //    (default RELAY_CONTRIBUTION_MIN; 0.0 only for fleet plumbing
        //    tests, see L1-001 runbook).
        let local_score = self.role_manager.score(&envelope.from, now);
        if local_score < self.config.presence_contribution_min {
            tracing::debug!(
                "presence: attestation from {} dropped (local score {local_score:.2} < {})",
                envelope.from,
                self.config.presence_contribution_min
            );
            self.presence.record(PresenceOutcome::DropGate);
            return Vec::new();
        }

        // 7. Accept = consume the challenge (one-shot) + store for the
        //    30s aggregation window.
        let challenge_id = payload.challenge_id.clone();
        if !self
            .presence
            .consume_and_store(&challenge_id, payload, &envelope.signature, now)
        {
            self.presence.record(PresenceOutcome::DropStoreFull);
            return Vec::new();
        }

        let latency_ms = now.saturating_sub(issued_at);
        self.presence.record(PresenceOutcome::Accepted(latency_ms));
        vec![RuntimeEffect::Emit(ProtocolEvent::PresenceAttestationReceived {
            attester_id: envelope.from,
            challenge_id,
            latency_ms,
        })]
    }

    /// Periodic purge of every presence artifact past its 30s TTL.
    pub fn tick_presence_cleanup(&mut self) -> Vec<RuntimeEffect> {
        let now = self.presence_now();
        self.presence.cleanup(now);
        // L1-003: same 30s TTL discipline for witness observations; the
        // subscription table has its own (longer) TTL.
        self.witness_log.purge_expired(now);
        self.subscriptions.purge_expired(now);
        // Consumer-side quorum: drop stale attestations and reset the
        // per-window view-activity counter that feeds the dynamic quorum.
        self.quorum.purge_expired(now);
        self.presence_view_activity = 0;

        // Faille 1 (red-team hardening): degrade Online → Stale when a peer
        // has gone silent (no activity) for PEER_ONLINE_STALE_MS. This prevents
        // Sybil attacks where a peer is promoted via L1-003 quorum, then goes
        // silent — without this mechanism, the peer would remain Online
        // indefinitely even though no witness attests it anymore.
        //
        // Target `Stale`, NOT `Known`: `HeartbeatTracker` (discovery/heartbeat.rs)
        // keeps its OWN `last_heartbeat` map, separate from `PeerInfo.last_seen`,
        // and its `check_all` only restores `Online` from `Stale`/`Offline` — a
        // peer demoted straight to `Known` would never be picked back up by a
        // fresh heartbeat (its restore branch does not match `Known`), stranding
        // it. `Stale` also matches the existing semantic ("recently seen but may
        // be transitioning") and is emitted via the same event as the heartbeat
        // path for consistency with existing consumers.
        //
        // Iterate peers and degrade any Online peer whose last_seen is older
        // than the stale threshold (except self, never degrade self).
        let peers_to_degrade: Vec<NodeId> = self
            .topology
            .peers()
            .filter(|peer| {
                peer.status == PeerStatus::Online
                    && peer.node_id != self.local_id
                    && now.saturating_sub(peer.last_seen) >= crate::relay::PEER_ONLINE_STALE_MS
            })
            .map(|peer| peer.node_id)
            .collect();

        let mut effects = Vec::new();
        for node_id in peers_to_degrade {
            if let Some(peer) = self.topology.get(&node_id) {
                let last_seen_age_ms = now.saturating_sub(peer.last_seen);
                let mut updated = peer.clone();
                updated.status = PeerStatus::Stale;
                self.topology.upsert(updated);
                tracing::debug!(
                    node_id = %node_id,
                    last_seen_age_ms,
                    "peer demoted Online → Stale due to staleness"
                );
                effects.push(RuntimeEffect::Emit(ProtocolEvent::PeerStale { node_id }));
            }
        }

        effects
    }

    /// L1-003 (relay side): record a weak device's subscription to our presence
    /// views, scoped to the peers it declared (D3). Gated on a valid signature
    /// so a spoofed `from` cannot register a subscription (and cannot make us
    /// publish presence to a victim). Silent — a subscribe earns no reply
    /// envelope; the next publish tick serves it.
    ///
    /// Faille 2 (red-team hardening): validate that a `PresenceScope::Peers` does
    /// not exceed the maximum scope size to prevent asymmetric DoS where a
    /// malicious consumer declares a huge scope (millions of NodeIds) and forces
    /// the relay to process it at each presence tick. Scopes are bounded to
    /// MAX_VIEW_ENTRIES to match the max size of a published view.
    fn handle_presence_subscribe(
        &mut self,
        envelope: &Envelope,
        signature_valid: bool,
    ) -> Vec<RuntimeEffect> {
        if !signature_valid {
            return Vec::new();
        }
        let Ok(payload) =
            crate::presence::PresenceSubscribePayload::from_bytes(&envelope.payload)
        else {
            return Vec::new();
        };

        // Validate scope size: reject if PresenceScope::Peers exceeds MAX_VIEW_ENTRIES.
        // Group scopes are not bounded by size (already bounded by max group membership).
        if let crate::presence::PresenceScope::Peers(peers) = &payload.scope {
            if peers.len() > crate::presence::MAX_VIEW_ENTRIES {
                tracing::warn!(
                    subscriber = %envelope.from,
                    requested_size = peers.len(),
                    max_allowed = crate::presence::MAX_VIEW_ENTRIES,
                    "rejected presence subscription with scope too large"
                );
                return Vec::new();
            }
        }

        self.subscriptions
            .subscribe(envelope.from, payload.scope, now_ms());
        Vec::new()
    }

    /// L1-003 (consumer side, step 2c): receive a relay's signed presence view.
    ///
    /// Verify the view's OWN signature against its claimed `witness_id`
    /// (independently verifiable — NOT the transport envelope's signature) +
    /// structural validity, then DROP. Quorum aggregation and `Known → Online`
    /// promotion are step 3 — this node does NOT yet trust a single view
    /// (kill-shot #3). A signature proves WHO published, never that the content
    /// is TRUE.
    fn handle_relay_presence_view(
        &mut self,
        envelope: &Envelope,
        _signature_valid: bool,
    ) -> Vec<RuntimeEffect> {
        let Ok(view) = crate::presence::RelayPresenceView::from_bytes(&envelope.payload) else {
            return Vec::new();
        };
        // The view must be self-authenticated by its witness, and the witness
        // must be whoever sent the envelope (no relaying a third party's view
        // under your own `from`).
        if view.witness_id != envelope.from
            || view.validate().is_err()
            || !view.verify_signature()
        {
            return Vec::new();
        }

        // Feed the quorum aggregator: this witness attests each listed peer
        // alive. A witness contributes at most once per peer (the aggregator
        // dedups), so a single relay repeating itself cannot fake a quorum.
        // HARDENING: cryptographically verify each entry's ack_proof before
        // accepting it toward quorum. Entries with invalid/expired/forged proofs
        // are silently skipped, but valid entries still count.
        let now = now_ms();
        self.presence_view_activity = self.presence_view_activity.saturating_add(1);
        for entry in &view.present {
            // Verify the ack_proof is valid and fresh before accepting the attestation.
            if !self.verify_presence_entry_proof(entry, now) {
                continue; // Skip invalid/unverifiable entries.
            }
            self.quorum.record(view.witness_id, entry.peer_id, now);
        }

        // Promote to Online ONLY the peers now backed by a quorum of ≥ N
        // DISTINCT fresh witnesses (kill-shot #3: never trust a single view).
        // Promotion is the whole point — the weak device learns liveness from
        // the witness quorum without computing presence of N peers itself.
        let mut effects = Vec::new();
        for entry in &view.present {
            if !self
                .quorum
                .at_quorum(&entry.peer_id, self.presence_view_activity, now)
            {
                continue;
            }
            // Only lift a peer we already have an address for (discovery), and
            // only if it isn't already Online. Known/Stale/Offline → Online.
            if let Some(peer) = self.topology.get(&entry.peer_id) {
                if peer.status != PeerStatus::Online {
                    let mut updated = peer.clone();
                    updated.status = PeerStatus::Online;
                    updated.last_seen = now;
                    self.topology.upsert(updated);
                    effects.push(RuntimeEffect::Emit(ProtocolEvent::PeerOnline {
                        node_id: entry.peer_id,
                    }));
                }
            }
        }
        effects
    }

    /// Cryptographically verify a presence entry's ACK proof (hardening step 2).
    ///
    /// Checks (in order, fails fast if any check fails):
    /// 1. `ack_proof` bytes deserialize to an Envelope (an empty/garbage
    ///    `ack_proof` fails here — it is NEVER trusted, kill-shot: a witness
    ///    could otherwise omit the proof to skip every check below)
    /// 2. Envelope's msg_type is Ack
    /// 3. Envelope's `from` field matches the attested peer_id
    /// 4. Envelope's signature is valid (Ed25519)
    /// 5. Envelope's payload deserializes to AckPayload
    /// 6. AckPayload's fields match entry's proof_ref and proof_type
    /// 7. Envelope is fresh (within PRESENCE_TTL_MS)
    ///
    /// Returns false for any mismatch or crypto failure. Invalid entries are skipped
    /// but don't fail the whole view.
    fn verify_presence_entry_proof(
        &self,
        entry: &crate::presence::PresenceEntry,
        now: u64,
    ) -> bool {
        // Check 1: deserialize Envelope from ack_proof bytes. An empty or
        // garbage ack_proof fails here — never treated as implicitly valid.
        let Ok(ack_env) = Envelope::from_bytes(&entry.ack_proof) else {
            return false;
        };

        // Check 2: message type is Ack.
        if ack_env.msg_type != MessageType::Ack {
            return false;
        }

        // Check 3: Envelope.from must match the attested peer.
        if ack_env.from != entry.peer_id {
            return false;
        }

        // Check 4: cryptographic signature must be valid.
        if ack_env.verify_signature().is_err() {
            return false;
        }

        // Check 5: payload deserializes to AckPayload.
        let Ok(ack_payload) = crate::router::AckPayload::from_bytes(&ack_env.payload) else {
            return false;
        };

        // Check 6: AckPayload contents match entry's proof_ref and proof_type.
        if ack_payload.original_message_id != entry.proof_ref
            || ack_payload.ack_type != entry.proof_type
        {
            return false;
        }

        // Check 7: Envelope timestamp must be fresh (within TTL).
        let envelope_age = now.saturating_sub(ack_env.timestamp);
        if envelope_age >= crate::presence::PRESENCE_TTL_MS {
            return false;
        }

        true
    }

    /// L1-003 (relay side): publish a signed presence view to each live
    /// subscriber, scoped to what it asked for (D2, push). Skips subscribers
    /// for whom we currently hold no in-scope, non-expired observation (an
    /// empty view carries no signal and must not be emitted).
    ///
    /// The view carries the witness's OWN signature (independently verifiable);
    /// the transport envelope is signed too (for the generic ingress gate).
    pub fn tick_publish_presence_views(&mut self) -> Vec<RuntimeEffect> {
        let now = self.presence_now();
        let mut effects = Vec::new();
        for (subscriber, scope) in self.subscriptions.active(now) {
            let Some(mut view) = self.witness_log.build_view(self.local_id, scope, now) else {
                continue;
            };
            view.sign(&self.secret_seed);
            let mut envelope = Envelope::new(
                self.local_id,
                subscriber,
                MessageType::RelayPresenceView,
                view.to_bytes(),
            );
            envelope.sign(&self.secret_seed);
            effects.push(RuntimeEffect::SendEnvelope(envelope));
        }
        effects
    }

    /// Auto-probe (fleet observability): challenge up to 8 Online peers.
    /// Only runs when `config.presence_probe_interval` is set. Per-target
    /// and global caps of the PresenceManager still apply.
    pub fn tick_presence_probe(&mut self) -> Vec<RuntimeEffect> {
        if self.config.presence_probe_interval.is_none() {
            return Vec::new();
        }
        let targets: Vec<NodeId> = self
            .topology
            .peers()
            .filter(|p| p.status == PeerStatus::Online && p.node_id != self.local_id)
            .map(|p| p.node_id)
            .take(8)
            .collect();

        let mut effects = Vec::new();
        for target in targets {
            effects.extend(self.initiate_presence_check(target));
        }
        effects
    }

    /// Current entropy seed over the attestation window (input for L1-002).
    pub fn presence_seed(&self) -> [u8; 32] {
        self.presence.aggregate_seed()
    }

    /// Lifetime presence counters (observability / stress relevés).
    pub fn presence_metrics(&self) -> crate::presence::PresenceMetrics {
        self.presence.metrics()
    }

    /// Number of attestations in the current aggregation window.
    pub fn presence_attestation_count(&self) -> usize {
        self.presence.accepted_count()
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

        // Cache envelope for potential ACK-timeout retry (R9.2)
        self.pending_envelopes
            .insert(envelope_id, envelope.clone());

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
        let mut pre_effects = Vec::new();

        // R14.1 dual-trigger rotation before sending.
        let rotation_actions = self.group_manager.maybe_rotate_local_sender_key(&group_id);
        if !rotation_actions.is_empty() {
            let rotation_actions = self.intercept_self_group_actions(rotation_actions);
            pre_effects.extend(self.group_actions_to_effects(&rotation_actions));
        }

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
                group_id.clone(),
                self.local_id,
                self.config.username.clone(),
                text,
                &key,
                epoch,
            )
        } else {
            GroupMessage::new(
                group_id.clone(),
                self.local_id,
                self.config.username.clone(),
                text,
            )
        };

        msg.sign(&self.secret_seed);
        self.group_manager.note_local_message_sent(&group_id);
        let payload = GroupPayload::Message(msg);

        // If we ARE the hub, handle locally without network round-trip
        if hub_id == self.local_id {
            let actions = self.group_hub.handle_payload(payload, self.local_id);
            let actions = self.intercept_self_group_actions(actions);
            let mut effects = pre_effects;
            effects.extend(self.group_actions_to_effects(&actions));
            return effects;
        }

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

        pre_effects.push(RuntimeEffect::SendEnvelope(envelope));
        pre_effects
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

            RuntimeCommand::AddPeer { node_id, .. } => {
                // Discovery only (ADR-011 ghost-peer fix) — no heartbeat/Online
                // credit without real work. `source` no longer feeds the
                // heartbeat tracker; kept in the command for API compat.
                self.mark_known(node_id, PeerRole::Peer);
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
                invite_only,
            } => {
                // If we ARE the hub, handle creation locally without network round-trip
                if hub_relay_id == self.local_id {
                    let payload = GroupPayload::Create {
                        group_name: name.clone(),
                        creator_username: self.config.username.clone(),
                        initial_members: initial_members.clone(),
                        invite_only,
                    };
                    let actions = self.group_hub.handle_payload(payload, self.local_id);
                    // Intercept self-addressed actions (e.g. Created → self)
                    let actions = self.intercept_self_group_actions(actions);
                    self.group_actions_to_effects(&actions)
                } else {
                    // Remote hub — send GroupPayload::Create over network
                    let actions = self.group_manager.create_group_with_options(
                        name,
                        hub_relay_id,
                        initial_members,
                        invite_only,
                    );
                    self.group_actions_to_effects(&actions)
                }
            }

            RuntimeCommand::AcceptInvite { group_id } => {
                let actions =
                    self.group_manager.accept_invite(&group_id);
                let actions = self.intercept_self_group_actions(actions);
                self.group_actions_to_effects(&actions)
            }

            RuntimeCommand::DeclineInvite { group_id } => {
                self.group_manager.decline_invite(&group_id);
                Vec::new()
            }

            RuntimeCommand::LeaveGroup { group_id } => {
                let actions =
                    self.group_manager.leave_group(&group_id);
                let actions = self.intercept_self_group_actions(actions);
                self.group_actions_to_effects(&actions)
            }

            // ── Admin controls (R11.3) ──────────────────────────
            RuntimeCommand::KickMember { group_id, target_id } => {
                let hub_id = self
                    .group_manager
                    .get_group(&group_id)
                    .map(|g| g.hub_relay_id);
                let payload = GroupPayload::KickMember {
                    group_id,
                    target_id,
                };
                if hub_id == Some(self.local_id) {
                    let actions = self.group_hub.handle_payload(payload, self.local_id);
                    let actions = self.intercept_self_group_actions(actions);
                    self.group_actions_to_effects(&actions)
                } else if let Some(hub) = hub_id {
                    self.group_actions_to_effects(&[GroupAction::Send {
                        to: hub,
                        payload,
                    }])
                } else {
                    Vec::new()
                }
            }

            RuntimeCommand::UpdateMemberRole {
                group_id,
                target_id,
                new_role,
            } => {
                let hub_id = self
                    .group_manager
                    .get_group(&group_id)
                    .map(|g| g.hub_relay_id);
                let payload = GroupPayload::UpdateMemberRole {
                    group_id,
                    target_id,
                    new_role,
                };
                if hub_id == Some(self.local_id) {
                    let actions = self.group_hub.handle_payload(payload, self.local_id);
                    let actions = self.intercept_self_group_actions(actions);
                    self.group_actions_to_effects(&actions)
                } else if let Some(hub) = hub_id {
                    self.group_actions_to_effects(&[GroupAction::Send {
                        to: hub,
                        payload,
                    }])
                } else {
                    Vec::new()
                }
            }

            RuntimeCommand::InviteMember { group_id, target_id } => {
                let hub_id = self
                    .group_manager
                    .get_group(&group_id)
                    .map(|g| g.hub_relay_id);
                let payload = GroupPayload::InviteMember {
                    group_id,
                    target_id,
                };
                if hub_id == Some(self.local_id) {
                    let actions = self.group_hub.handle_payload(payload, self.local_id);
                    let actions = self.intercept_self_group_actions(actions);
                    self.group_actions_to_effects(&actions)
                } else if let Some(hub) = hub_id {
                    self.group_actions_to_effects(&[GroupAction::Send {
                        to: hub,
                        payload,
                    }])
                } else {
                    Vec::new()
                }
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

            RuntimeCommand::GetKnownRelays { reply } => {
                let mut relays: Vec<_> = self.relay_registry.all().cloned().collect();
                relays.sort_by_key(|a| Reverse(a.refreshed_at));
                let _ = reply.send(relays);
                Vec::new()
            }

            // DHT lookup completed — register the discovered peer.
            RuntimeCommand::CheckPresence { target } => self.initiate_presence_check(target),

            RuntimeCommand::CheckPresenceMany { targets } => {
                self.initiate_presence_check_many(&targets)
            }

            RuntimeCommand::CheckPresenceAllOnline => {
                let targets: Vec<NodeId> = self
                    .topology
                    .peers()
                    .filter(|p| p.status == PeerStatus::Online && p.node_id != self.local_id)
                    .map(|p| p.node_id)
                    .collect();
                self.initiate_presence_check_many(&targets)
            }

            RuntimeCommand::GetPresenceSeed { reply } => {
                let _ = reply.send((
                    self.presence.aggregate_seed(),
                    self.presence.accepted_count(),
                ));
                Vec::new()
            }

            RuntimeCommand::GetPresenceMetrics { reply } => {
                let _ = reply.send(self.presence.metrics());
                Vec::new()
            }

            RuntimeCommand::SetPresenceClockOffset { offset_ms } => {
                tracing::warn!("SIM: presence clock offset set to {offset_ms}ms");
                self.config.presence_clock_offset_ms = offset_ms;
                Vec::new()
            }

            RuntimeCommand::DhtLookupResult { addr } => {
                let Ok(node_id) = addr.node_id.parse::<NodeId>() else {
                    tracing::warn!("DHT lookup result: invalid node_id '{}'", addr.node_id);
                    return Vec::new();
                };
                // Discovery only (ADR-011 ghost-peer fix) — no heartbeat/Online
                // credit without real work.
                self.mark_known(node_id, PeerRole::Peer);
                tracing::info!(
                    node_id = %node_id,
                    relays = addr.relay_urls.len(),
                    addrs = addr.direct_addrs.len(),
                    "DHT lookup result applied"
                );
                Vec::new()
            }

            // Handled in the loop — needs transport access.
            RuntimeCommand::GetConnectedPeers { .. } => Vec::new(),
            RuntimeCommand::AddPeerAddr { .. } => Vec::new(),

            // Inject raw gossip bytes — test/debug bridge.
            RuntimeCommand::InjectGossipBytes { bytes } => {
                self.handle_gossip_event(GossipInput::PeerAnnounce(bytes))
            }

            // Handled in the loop — signals the loop to break.
            RuntimeCommand::Shutdown => Vec::new(),

            // Handled in the loop — flushes state then replies.
            RuntimeCommand::SaveState { .. } => Vec::new(),

            // Embedded relay — handled by the async loop, not state.
            RuntimeCommand::StartEmbeddedRelay { .. } => Vec::new(),
            RuntimeCommand::StopEmbeddedRelay => Vec::new(),

            // Embedded relay feedback — state records the result + conditionally publishes.
            RuntimeCommand::EmbeddedRelayStarted { ref url } => {
                tracing::info!(%url, "embedded relay is healthy");
                self.embedded_relay_state = super::LocalEmbeddedRelayState::Healthy {
                    bound_relay_url: url.clone(),
                };
                let mut effects = vec![RuntimeEffect::Emit(ProtocolEvent::EmbeddedRelayStarted {
                    url: url.clone(),
                })];
                // Conditionally publish if policy allows
                if self.config.enable_embedded_relay_publication {
                    effects.extend(self.build_relay_publication(url.clone()));
                }
                effects
            }
            RuntimeCommand::EmbeddedRelayFailed { ref error } => {
                tracing::warn!(%error, "embedded relay failed");
                self.embedded_relay_state = super::LocalEmbeddedRelayState::Failed {
                    error: error.clone(),
                    last_failure_at: now_ms(),
                };
                self.embedded_relay_publication = super::EmbeddedRelayPublicationState::NotPublished;
                vec![RuntimeEffect::Emit(ProtocolEvent::EmbeddedRelayFailed {
                    error: error.clone(),
                })]
            }
            RuntimeCommand::EmbeddedRelayStopped => {
                tracing::info!("embedded relay stopped");
                self.embedded_relay_state = super::LocalEmbeddedRelayState::Stopped;
                self.embedded_relay_publication = super::EmbeddedRelayPublicationState::NotPublished;
                vec![RuntimeEffect::Emit(ProtocolEvent::EmbeddedRelayStopped)]
            }
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
                        // Discovery only (ADR-011 ghost-peer fix) — no
                        // heartbeat/Online credit without real work.
                        self.mark_known(peer_id, role);
                        return vec![];
                    }
                }

                // Try RoleChangeAnnounce
                if let Ok(role_announce) =
                    rmp_serde::from_slice::<crate::discovery::RoleChangeAnnounce>(&bytes)
                {
                    return self.handle_role_announce(role_announce);
                }

                // Try RelayReadyAnnounce
                if let Ok(relay_announce) =
                    rmp_serde::from_slice::<crate::discovery::RelayReadyAnnounce>(&bytes)
                {
                    return self.handle_relay_ready_announce(relay_announce);
                }

                Vec::new()
            }

            GossipInput::NeighborUp(node_id) => {
                // Discovery only (ADR-011 ghost-peer fix) — no heartbeat/Online
                // credit without real work.
                self.mark_known(node_id, PeerRole::Peer);
                let mut effects = vec![RuntimeEffect::Emit(
                    ProtocolEvent::GossipNeighborUp { node_id },
                )];

                // Re-publish embedded relay announcement to new neighbor.
                // The initial publication at startup is missed by nodes that join
                // gossip after the first broadcast. This ensures late joiners
                // receive the relay announcement without waiting for the next
                // periodic republication interval.
                if let super::LocalEmbeddedRelayState::Healthy { ref bound_relay_url } =
                    self.embedded_relay_state
                {
                    effects.extend(self.build_relay_publication(bound_relay_url.clone()));
                }

                effects
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

    // ── Helper: self-addressed group action interception ─────────────────

    /// Intercept Send/Broadcast actions that target `local_id` and process
    /// them locally instead of sending over the network (QUIC self-sends).
    /// Recursive: handles chains where local processing generates more self-sends
    /// (e.g., MemberJoined → SenderKeyDistribution → hub fanout).
    fn intercept_self_group_actions(&mut self, actions: Vec<GroupAction>) -> Vec<GroupAction> {
        let mut result = Vec::new();
        for action in actions {
            match action {
                GroupAction::Send { to, payload } if to == self.local_id => {
                    let new_actions = self.handle_local_group_payload(payload);
                    // Recursively intercept any resulting self-sends
                    result.extend(self.intercept_self_group_actions(new_actions));
                }
                GroupAction::Broadcast { to, payload } if to.contains(&self.local_id) => {
                    // Process locally for self
                    let new_actions = self.handle_local_group_payload(payload.clone());
                    result.extend(self.intercept_self_group_actions(new_actions));
                    // Keep broadcast for remote targets
                    let remote: Vec<NodeId> =
                        to.into_iter().filter(|t| *t != self.local_id).collect();
                    if !remote.is_empty() {
                        result.push(GroupAction::Broadcast {
                            to: remote,
                            payload,
                        });
                    }
                }
                other => result.push(other),
            }
        }
        result
    }

    /// Dispatch a group payload locally. Routes hub-bound payloads to the
    /// GroupHub for processing, and member-bound payloads to the GroupManager.
    fn handle_local_group_payload(&mut self, payload: GroupPayload) -> Vec<GroupAction> {
        match payload {
            // ── Hub-bound payloads: route through GroupHub ─────────────
            GroupPayload::Create { .. }
            | GroupPayload::Join { .. }
            | GroupPayload::Leave { .. }
            | GroupPayload::KickMember { .. }
            | GroupPayload::UpdateMemberRole { .. }
            | GroupPayload::InviteMember { .. } => {
                self.group_hub.handle_payload(payload, self.local_id)
            }
            GroupPayload::SenderKeyDistribution {
                group_id, from, epoch, encrypted_keys,
            } => {
                let mut actions = Vec::new();
                // Always try to store keys for self via manager
                actions.extend(self.group_manager.handle_sender_key_distribution(
                    &group_id, from, epoch, &encrypted_keys, &self.secret_seed,
                ));
                // If there are keys for OTHER members and we're the hub, fan out
                let keys_for_others: Vec<_> = encrypted_keys
                    .into_iter()
                    .filter(|ek| ek.recipient_id != self.local_id)
                    .collect();
                if !keys_for_others.is_empty()
                    && self.group_hub.get_group(&group_id).is_some()
                {
                    let fanout_payload = GroupPayload::SenderKeyDistribution {
                        group_id,
                        from,
                        epoch,
                        encrypted_keys: keys_for_others,
                    };
                    actions.extend(
                        self.group_hub
                            .handle_payload(fanout_payload, self.local_id),
                    );
                }
                actions
            }
            GroupPayload::DeliveryAck { ref group_id, .. } => {
                if self.group_hub.get_group(group_id).is_some() {
                    self.group_hub.handle_payload(payload, self.local_id)
                } else {
                    vec![]
                }
            }

            // ── Member-bound payloads: route to GroupManager ──────────
            GroupPayload::MemberJoined { group_id, member } => {
                self.group_manager.handle_member_joined(&group_id, member)
            }
            GroupPayload::MemberLeft {
                group_id,
                node_id,
                username,
                reason,
            } => self
                .group_manager
                .handle_member_left(&group_id, &node_id, username, reason),
            GroupPayload::Message(msg) => self.group_manager.handle_message(msg),
            GroupPayload::Sync {
                group,
                recent_messages,
            } => self.group_manager.handle_group_sync(group, recent_messages),
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
                self.local_id,
            ),
            GroupPayload::MemberRoleChanged {
                group_id,
                node_id,
                new_role,
            } => self.group_manager.handle_member_role_changed(
                &group_id, &node_id, new_role,
            ),
            // Payloads that don't need local dispatch
            _ => vec![],
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
                    // R13: persist group messages to SQLite for offline gap-fill
                    if let GroupPayload::Message(ref msg) = payload {
                        if let Some(ref store) = self.store {
                            let data = rmp_serde::to_vec(msg).unwrap_or_default();
                            let _ = store.save_hub_message(
                                &msg.group_id, msg.seq, &data, crate::types::now_ms(),
                            );
                        }
                    }

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
            GroupEvent::MemberRoleChanged {
                group_id,
                node_id,
                new_role,
            } => ProtocolEvent::GroupMemberRoleChanged {
                group_id: group_id.clone(),
                node_id: *node_id,
                new_role: *new_role,
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
            GroupEvent::ShadowPromoted {
                group_id,
                new_hub_id,
            } => ProtocolEvent::GroupShadowPromoted {
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

// ── Relay reachability (universal, environment-driven) ─────────────────────

/// Whether a relay URL is reachable from outside the local network.
///
/// One rule for every node, decided purely from the address the embedded relay
/// bound to — no per-device profile, no configuration:
/// - DNS-name hosts → assumed globally resolvable (e.g. a public relay domain).
/// - Public IP literals → reachable.
/// - Private / loopback / link-local / CGNAT IP literals → LAN-only.
///
/// Used to gate *global* gossip publication: a LAN-only relay stays usable
/// locally but is never advertised to remote peers it could not serve.
fn relay_url_is_globally_reachable(url: &tom_connect::RelayUrl) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    // url::host_str() brackets IPv6 literals; strip them before parsing.
    let host = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);

    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => ipv4_is_global(ip),
        Ok(std::net::IpAddr::V6(ip)) => ipv6_is_global(ip),
        // Not an IP literal → a DNS name. Assume globally resolvable EXCEPT for
        // names that are non-routable by definition (loopback alias, mDNS/LAN
        // and private TLDs). Those would trap remote peers just like a private IP.
        Err(_) => !host_is_local_only(host),
    }
}

/// Hostnames that never resolve outside the local network (case-insensitive).
fn host_is_local_only(host: &str) -> bool {
    let h = host.trim_end_matches('.').to_ascii_lowercase();
    h == "localhost"
        || h.ends_with(".localhost")
        || h.ends_with(".local") // mDNS / Bonjour
        || h.ends_with(".internal")
        || h.ends_with(".lan")
        || h.ends_with(".home.arpa") // RFC 8375 home networks
        || h.ends_with(".intranet")
}

fn ipv4_is_global(ip: std::net::Ipv4Addr) -> bool {
    if ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_documentation()
    {
        return false;
    }
    // CGNAT shared address space 100.64.0.0/10 (Ipv4Addr::is_shared is unstable).
    let o = ip.octets();
    if o[0] == 100 && (64..=127).contains(&o[1]) {
        return false;
    }
    true
}

fn ipv6_is_global(ip: std::net::Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() {
        return false;
    }
    let seg = ip.segments();
    // Unique local fc00::/7.
    if (seg[0] & 0xfe00) == 0xfc00 {
        return false;
    }
    // Link-local fe80::/10.
    if (seg[0] & 0xffc0) == 0xfe80 {
        return false;
    }
    true
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::RuntimeConfig;
    use crate::relay::PeerStatus;
    use crate::runtime::{EmbeddedRelayPublicationState, LocalEmbeddedRelayState};

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    fn keypair(seed: u8) -> (NodeId, [u8; 32]) {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
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

    // ── ADR-009 : survie + livraison différée ────────────────────────────
    // Garde-fou de la garantie « destinataire hors-ligne → message conservé →
    // redélivré au retour ». Validé en endurance réelle (2026-07-04, campagne
    // multi-devices : 15/15 messages redélivrés) ; ce test verrouille la
    // mécanique de façon déterministe. Le trigger réel est `tick_heartbeat`
    // sur `DiscoveryEvent::PeerOnline` → `prepare_backup_delivery` (state.rs).

    #[test]
    fn backup_conserve_le_message_pour_un_destinataire_hors_ligne() {
        let mut state = default_state(1);
        let (bob, _) = keypair(2);

        // Un envoi vers B échoue → le message est stocké en backup (comme le
        // fait handle_send_message dans son on_failure).
        state.backup.store_message(
            "msg-offline-1".to_string(),
            b"salut B, tu etais parti".to_vec(),
            bob,
            state.local_id,
            now_ms(),
            None,
        );

        assert_eq!(
            state.backup.store().get_for_recipient(&bob).len(),
            1,
            "le message doit survivre en backup tant que B est hors-ligne"
        );
    }

    #[test]
    fn backup_redelivre_au_retour_du_destinataire() {
        let mut state = default_state(1);
        let (bob, _) = keypair(2);
        let (carol, _) = keypair(3);

        // 3 messages backupés pour B (hors-ligne) + 1 pour Carol (isolation).
        for i in 0..3 {
            state.backup.store_message(
                format!("pour-bob-{i}"),
                format!("message differe {i}").into_bytes(),
                bob,
                state.local_id,
                now_ms(),
                None,
            );
        }
        state.backup.store_message(
            "pour-carol".to_string(),
            b"pas pour bob".to_vec(),
            carol,
            state.local_id,
            now_ms(),
            None,
        );

        // B revient en ligne : c'est ce que déclenche PeerOnline via tick_heartbeat.
        let effects = state.prepare_backup_delivery(bob);

        // Les 3 messages de B (et EUX SEULS) sont ré-émis vers B.
        assert_eq!(effects.len(), 3, "les 3 messages backupés de B doivent être redélivrés");
        for effect in &effects {
            match effect {
                RuntimeEffect::SendWithBackupFallback { envelope, .. } => {
                    assert_eq!(envelope.to, bob, "redélivré au bon destinataire");
                }
                other => panic!("attendu SendWithBackupFallback, obtenu {other:?}"),
            }
        }

        // Le message de Carol ne part PAS quand B revient.
        assert_eq!(
            state.prepare_backup_delivery(carol).len(),
            1,
            "le backup est bien isolé par destinataire"
        );
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
    fn handle_incoming_never_panics_on_adversarial_bytes() {
        // Wire-format fuzz: the outermost parse+dispatch path must absorb any
        // byte string and any single-byte mutation of a valid envelope without
        // panicking (a panic here = remote DoS). Deterministic (seeded RNG).
        use rand::{RngCore, SeedableRng};
        let mut state = default_state(1);
        let mut rng = rand::rngs::StdRng::seed_from_u64(0xF0F0);

        // 1. Pure random byte strings of assorted lengths.
        for _ in 0..500 {
            let len = (rng.next_u32() % 512) as usize;
            let mut buf = vec![0u8; len];
            rng.fill_bytes(&mut buf);
            let _ = state.handle_incoming(&buf); // must not panic
        }

        // 2. A valid envelope, mutated one byte at a time (walks every offset).
        let (sender_id, sender_secret) = keypair(2);
        let base = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            state.local_id,
            MessageType::Chat,
            b"fuzz-seed".to_vec(),
        )
        .sign(&sender_secret)
        .to_bytes()
        .unwrap();
        for i in 0..base.len() {
            for flip in [0x01u8, 0x80, 0xFF] {
                let mut m = base.clone();
                m[i] ^= flip;
                let _ = state.handle_incoming(&m); // must not panic
            }
        }

        // 3. Truncations at every prefix length.
        for i in 0..base.len() {
            let _ = state.handle_incoming(&base[..i]); // must not panic
        }
    }

    #[test]
    fn forged_relay_ack_earns_no_score() {
        // FINDING #7: a signed RelayForwarded ACK with a message_id we never
        // sent must NOT credit the sender's local relay score. Otherwise an
        // attacker pumps its score with random ids (each escaping anti-replay)
        // and forges past the presence anti-Sybil gate.
        use crate::router::{AckPayload, AckType};
        let mut state = default_state(1);
        let (eve_id, eve_secret) = keypair(66);

        // 20 forged RelayForwarded ACKs, all with unknown (untracked) ids.
        for i in 0..20 {
            let payload = AckPayload {
                original_message_id: format!("ghost-{i}"),
                ack_type: AckType::RelayForwarded,
            }
            .to_bytes();
            let raw = crate::envelope::EnvelopeBuilder::new(
                eve_id,
                state.local_id,
                MessageType::Ack,
                payload,
            )
            .sign(&eve_secret)
            .to_bytes()
            .unwrap();
            state.handle_incoming(raw.as_slice());
        }

        assert_eq!(
            state.role_manager.score(&eve_id, crate::types::now_ms()),
            0.0,
            "forged RelayForwarded ACKs for untracked messages must earn no relay score"
        );
    }

    #[test]
    fn genuine_relay_ack_earns_score() {
        // Counterpart to forged_relay_ack_earns_no_score: a RelayForwarded ACK
        // for a message we REALLY sent, from a node that is not the final
        // recipient (i.e. a relay on the path), DOES credit relay evidence.
        use crate::router::{AckPayload, AckType};
        let mut state = default_state(1);
        let (bob_id, _) = keypair(2); // final recipient
        let (relay_id, relay_secret) = keypair(3); // relay on the path

        // Send a real message to bob → tracker records it (to = bob).
        let send = state.handle_send_message(bob_id, b"via relay".to_vec());
        let msg_id = send
            .iter()
            .find_map(|e| match e {
                RuntimeEffect::SendWithBackupFallback { envelope, .. } => Some(envelope.id.clone()),
                RuntimeEffect::SendEnvelope(env) => Some(env.id.clone()),
                _ => None,
            })
            .expect("send produced an envelope");

        let payload = AckPayload {
            original_message_id: msg_id,
            ack_type: AckType::RelayForwarded,
        }
        .to_bytes();
        let raw = crate::envelope::EnvelopeBuilder::new(
            relay_id,
            state.local_id,
            MessageType::Ack,
            payload,
        )
        .sign(&relay_secret)
        .to_bytes()
        .unwrap();
        state.handle_incoming(raw.as_slice());

        assert!(
            state.role_manager.score(&relay_id, crate::types::now_ms()) > 0.0,
            "a genuine relay's RelayForwarded ACK for a real message must earn score"
        );
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

    // ── L1-003 witness wiring ───────────────────────────────────────────

    /// Build a signed ACK from `acker` to `to`, routed back through our node
    /// (via chain), so `handle_incoming_chat` takes the Forward path.
    fn signed_ack_via_us(
        state: &RuntimeState,
        acker_seed: u8,
        to: NodeId,
        msg_id: &str,
        ack_type: crate::router::AckType,
    ) -> (Envelope, NodeId) {
        let (acker_id, acker_secret) = keypair(acker_seed);
        let payload = crate::router::AckPayload {
            original_message_id: msg_id.to_string(),
            ack_type,
        };
        let env = crate::envelope::EnvelopeBuilder::new(
            acker_id,
            to,
            MessageType::Ack,
            payload.to_bytes(),
        )
        .via(vec![state.local_id])
        .sign(&acker_secret);
        (env, acker_id)
    }

    #[test]
    fn witness_records_forwarded_signed_ack() {
        let mut state = default_state(1);
        let sender = node_id(3); // ACK's destination (original message sender)
        let (env, acker_id) =
            signed_ack_via_us(&state, 2, sender, "m-42", crate::router::AckType::RecipientReceived);
        let sig_valid = env.verify_signature().is_ok();
        assert!(sig_valid);
        assert_eq!(state.witness_log.len(), 0);

        let _ = state.handle_incoming_chat(env, sig_valid);

        assert_eq!(state.witness_log.len(), 1, "forwarded signed ACK must be observed");
        let now = crate::types::now_ms();
        let view = state
            .witness_log
            .build_view(
                state.local_id,
                crate::presence::PresenceScope::Peers(vec![acker_id]),
                now,
            )
            .expect("view for the acker");
        assert_eq!(view.present[0].peer_id, acker_id);
        assert_eq!(view.present[0].proof_ref, "m-42");
        assert_eq!(view.present[0].proof_type, crate::router::AckType::RecipientReceived);
    }

    #[test]
    fn witness_ignores_unsigned_ack() {
        let mut state = default_state(1);
        let sender = node_id(3);
        let (env, _) =
            signed_ack_via_us(&state, 2, sender, "m-42", crate::router::AckType::RecipientReceived);
        // Feed it as if the signature did NOT verify (forged/unsigned).
        let _ = state.handle_incoming_chat(env, false);
        assert_eq!(
            state.witness_log.len(),
            0,
            "an unsigned/forged forwarded ACK must earn no witness observation"
        );
    }

    #[test]
    fn witness_ignores_forwarded_chat() {
        let mut state = default_state(1);
        let (sender_id, sender_secret) = keypair(2);
        let recipient_id = node_id(3);
        let env = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            recipient_id,
            MessageType::Chat,
            b"relayed".to_vec(),
        )
        .via(vec![state.local_id])
        .sign(&sender_secret);
        let sig_valid = env.verify_signature().is_ok();
        let _ = state.handle_incoming_chat(env, sig_valid);
        assert_eq!(
            state.witness_log.len(),
            0,
            "a forwarded Chat is not a presence proof — only signed ACKs are"
        );
    }

    // ── L1-003 subscription + publication (2c) ──────────────────────────

    #[test]
    fn subscribe_then_publish_emits_signed_view() {
        let mut state = default_state(1);
        let (subscriber_id, _sub_secret) = keypair(7);
        // The relay has observed a peer alive (via a forwarded signed ACK).
        let (ack_env, acker_id) =
            signed_ack_via_us(&state, 2, node_id(3), "m-9", crate::router::AckType::RecipientReceived);
        let sig_ok = ack_env.verify_signature().is_ok();
        let _ = state.handle_incoming_chat(ack_env, sig_ok);

        // Subscriber asks for presence of `acker_id`.
        let payload = crate::presence::PresenceSubscribePayload {
            scope: crate::presence::PresenceScope::Peers(vec![acker_id]),
        };
        let (sub_id2, sub_secret2) = keypair(7);
        assert_eq!(sub_id2, subscriber_id);
        let sub_env = crate::envelope::EnvelopeBuilder::new(
            subscriber_id,
            state.local_id,
            MessageType::PresenceSubscribe,
            payload.to_bytes(),
        )
        .sign(&sub_secret2);
        let sub_sig = sub_env.verify_signature().is_ok();
        assert!(sub_sig);
        let _ = state.handle_incoming(&sub_env.to_bytes().unwrap());
        assert_eq!(state.subscriptions.len(), 1, "subscription recorded");

        // Publish tick → one signed RelayPresenceView to the subscriber.
        let effects = state.tick_publish_presence_views();
        let view_env = effects.iter().find_map(|e| match e {
            RuntimeEffect::SendEnvelope(env)
                if env.msg_type == MessageType::RelayPresenceView && env.to == subscriber_id =>
            {
                Some(env.clone())
            }
            _ => None,
        });
        let view_env = view_env.expect("a presence view published to the subscriber");
        let view =
            crate::presence::RelayPresenceView::from_bytes(&view_env.payload).expect("valid view");
        assert!(view.verify_signature(), "published view is witness-signed");
        assert_eq!(view.witness_id, state.local_id);
        assert!(view.present.iter().any(|p| p.peer_id == acker_id));
    }

    #[test]
    fn unsigned_subscribe_is_ignored() {
        let mut state = default_state(1);
        let (subscriber_id, _) = keypair(7);
        let payload = crate::presence::PresenceSubscribePayload {
            scope: crate::presence::PresenceScope::Peers(vec![node_id(3)]),
        };
        let env = crate::envelope::Envelope::new(
            subscriber_id,
            state.local_id,
            MessageType::PresenceSubscribe,
            payload.to_bytes(),
        ); // unsigned
        let _ = state.handle_incoming(&env.to_bytes().unwrap());
        assert_eq!(
            state.subscriptions.len(),
            0,
            "an unsigned subscribe must never register a subscriber"
        );
    }

    #[test]
    fn publish_skips_subscriber_with_no_in_scope_observation() {
        let mut state = default_state(1);
        // Subscriber asks about a peer we have NO observation for.
        let (subscriber_id, sub_secret) = keypair(7);
        let payload = crate::presence::PresenceSubscribePayload {
            scope: crate::presence::PresenceScope::Peers(vec![node_id(9)]),
        };
        let sub_env = crate::envelope::EnvelopeBuilder::new(
            subscriber_id,
            state.local_id,
            MessageType::PresenceSubscribe,
            payload.to_bytes(),
        )
        .sign(&sub_secret);
        let _ = state.handle_incoming(&sub_env.to_bytes().unwrap());
        assert_eq!(state.subscriptions.len(), 1);
        // No observations → empty view → nothing published.
        let effects = state.tick_publish_presence_views();
        assert!(
            !effects
                .iter()
                .any(|e| matches!(e, RuntimeEffect::SendEnvelope(env) if env.msg_type == MessageType::RelayPresenceView)),
            "no in-scope observation → no view emitted"
        );
    }

    // ── L1-003 consumer quorum promotion (step 3) ───────────────────────

    /// A signed presence view from `witness_seed`, sent to `to`, attesting each
    /// peer (given by SEED, not bare NodeId) alive at `now`. Takes seeds rather
    /// than NodeIds because each entry's `ack_proof` must be a REAL Ack
    /// envelope signed by that peer's own key (`verify_presence_entry_proof`
    /// now rejects anything else — no bypass for an absent/forged proof).
    fn signed_view_from(witness_seed: u8, to: NodeId, peer_seeds: &[u8], now: u64) -> Envelope {
        let (witness_id, witness_secret) = keypair(witness_seed);
        let present: Vec<crate::presence::PresenceEntry> = peer_seeds
            .iter()
            .map(|&seed| {
                let (peer_id, peer_secret) = keypair(seed);
                let ack_payload = crate::router::AckPayload {
                    original_message_id: "m".into(),
                    ack_type: crate::router::AckType::RelayForwarded,
                };
                let mut ack_env = crate::envelope::Envelope::new(
                    peer_id,
                    witness_id,
                    MessageType::Ack,
                    ack_payload.to_bytes(),
                );
                ack_env.timestamp = now;
                ack_env.sign(&peer_secret);
                crate::presence::PresenceEntry {
                    peer_id,
                    proof_ref: "m".into(),
                    proof_type: crate::router::AckType::RelayForwarded,
                    seen_at_ms: now,
                    ack_proof: ack_env.to_bytes().expect("ack envelope serializes"),
                }
            })
            .collect();
        let peers: Vec<NodeId> = present.iter().map(|e| e.peer_id).collect();
        let mut view = crate::presence::RelayPresenceView {
            witness_id,
            epoch_ms: now,
            scope: crate::presence::PresenceScope::Peers(peers),
            present,
            signature: Vec::new(),
        };
        view.sign(&witness_secret);
        let mut env = crate::envelope::Envelope::new(
            witness_id,
            to,
            MessageType::RelayPresenceView,
            view.to_bytes(),
        );
        env.sign(&witness_secret);
        env
    }

    #[test]
    fn quorum_of_two_witnesses_promotes_known_to_online() {
        let mut state = default_state(1);
        let target = node_id(50);
        // Discovered as an address only (Known) — not yet proven live.
        state.handle_command(RuntimeCommand::AddPeer {
            node_id: target,
            source: DiscoverySource::Direct,
        });
        assert_eq!(state.topology.get(&target).unwrap().status, PeerStatus::Known);
        let now = crate::types::now_ms();

        // Witness 1 alone → quorum not met (floor 2) → stays Known.
        let v1 = signed_view_from(2, state.local_id, &[50], now);
        let _ = state.handle_incoming(&v1.to_bytes().unwrap());
        assert_eq!(
            state.topology.get(&target).unwrap().status,
            PeerStatus::Known,
            "a single witness must never eclipse-promote (kill-shot #3)"
        );

        // Witness 2 (distinct) → quorum of 2 met → Online + event.
        let v2 = signed_view_from(3, state.local_id, &[50], now);
        let effects = state.handle_incoming(&v2.to_bytes().unwrap());
        assert_eq!(
            state.topology.get(&target).unwrap().status,
            PeerStatus::Online,
            "two DISTINCT witnesses concur → Known promoted to Online"
        );
        assert!(
            effects.iter().any(|e| matches!(
                e,
                RuntimeEffect::Emit(ProtocolEvent::PeerOnline { node_id }) if *node_id == target
            )),
            "promotion emits PeerOnline, got: {effects:?}"
        );
    }

    #[test]
    fn same_witness_twice_never_promotes() {
        let mut state = default_state(1);
        let target = node_id(50);
        state.handle_command(RuntimeCommand::AddPeer {
            node_id: target,
            source: DiscoverySource::Direct,
        });
        let now = crate::types::now_ms();
        // Same witness sends TWO views → still one distinct witness.
        let v1 = signed_view_from(2, state.local_id, &[50], now);
        let v1b = signed_view_from(2, state.local_id, &[50], now);
        let _ = state.handle_incoming(&v1.to_bytes().unwrap());
        let _ = state.handle_incoming(&v1b.to_bytes().unwrap());
        assert_eq!(
            state.topology.get(&target).unwrap().status,
            PeerStatus::Known,
            "one witness repeating itself cannot fake a quorum"
        );
    }

    #[test]
    fn quorum_does_not_promote_undiscovered_peer() {
        let mut state = default_state(1);
        let target = node_id(50); // never added to topology (no address known)
        let now = crate::types::now_ms();
        let v1 = signed_view_from(2, state.local_id, &[50], now);
        let v2 = signed_view_from(3, state.local_id, &[50], now);
        let _ = state.handle_incoming(&v1.to_bytes().unwrap());
        let _ = state.handle_incoming(&v2.to_bytes().unwrap());
        assert!(
            state.topology.get(&target).is_none(),
            "quorum attests liveness but we still need an address — no phantom Online"
        );
    }

    /// Regression: `verify_presence_entry_proof` must NEVER treat an
    /// empty/absent `ack_proof` as implicitly valid. A prior draft of the
    /// spot-check hardening special-cased empty proofs as "valid" for test
    /// convenience — that is an actual bypass in production (a witness could
    /// just omit the proof and skip every crypto check). Two DISTINCT
    /// witnesses both sending an empty `ack_proof` must NOT reach quorum.
    #[test]
    fn empty_ack_proof_never_promotes() {
        let mut state = default_state(1);
        let target = node_id(50);
        state.handle_command(RuntimeCommand::AddPeer {
            node_id: target,
            source: DiscoverySource::Direct,
        });
        let now = crate::types::now_ms();
        let local_id = state.local_id;

        let unproven_view = |witness_seed: u8| {
            let (witness_id, witness_secret) = keypair(witness_seed);
            let present = vec![crate::presence::PresenceEntry {
                peer_id: target,
                proof_ref: "m".into(),
                proof_type: crate::router::AckType::RelayForwarded,
                seen_at_ms: now,
                ack_proof: Vec::new(), // no proof at all — must be rejected
            }];
            let mut view = crate::presence::RelayPresenceView {
                witness_id,
                epoch_ms: now,
                scope: crate::presence::PresenceScope::Peers(vec![target]),
                present,
                signature: Vec::new(),
            };
            view.sign(&witness_secret);
            let mut env = crate::envelope::Envelope::new(
                witness_id,
                local_id,
                MessageType::RelayPresenceView,
                view.to_bytes(),
            );
            env.sign(&witness_secret);
            env
        };

        let _ = state.handle_incoming(&unproven_view(2).to_bytes().unwrap());
        let _ = state.handle_incoming(&unproven_view(3).to_bytes().unwrap());
        assert_eq!(
            state.topology.get(&target).unwrap().status,
            PeerStatus::Known,
            "empty ack_proof from ANY number of witnesses must never promote to Online"
        );
    }

    #[test]
    fn handle_incoming_chat_dedup_reacks() {
        let mut state = default_state(1);
        let (env, sig_valid) = make_signed_chat(2, state.local_id, b"once");
        let env2 = env.clone();

        let effects1 = state.handle_incoming_chat(env, sig_valid);
        assert!(
            effects1.iter().any(|e| matches!(e, RuntimeEffect::DeliverMessage(_))),
            "first delivery should deliver to the app"
        );

        // Duplicate (sender resent after lost ACK): re-send the ACK, do NOT
        // re-deliver to the app (decision #1 survives ACK loss).
        let effects2 = state.handle_incoming_chat(env2, sig_valid);
        assert!(
            !effects2.iter().any(|e| matches!(e, RuntimeEffect::DeliverMessage(_))),
            "duplicate must NOT re-deliver to the app, got: {effects2:?}"
        );
        assert!(
            effects2.iter().any(|e| matches!(e, RuntimeEffect::SendEnvelope(_))),
            "duplicate must re-send an ACK, got: {effects2:?}"
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
            state.handle_command(RuntimeCommand::AddPeer { node_id: peer, source: DiscoverySource::Direct });

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
        state.handle_command(RuntimeCommand::AddPeer { node_id: peer, source: DiscoverySource::Direct });
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
        // ADR-011 ghost-peer fix: discovery (gossip neighbor-up) registers an
        // address, not a liveness proof — status is Known until real work
        // (signed inbound/ACK/witnessed relay) promotes it to Online.
        assert_eq!(topo_peer.unwrap().status, PeerStatus::Known);
    }

    #[test]
    fn neighbor_up_republishes_relay_when_published() {
        let (local_id, local_secret) = keypair(1);
        let mut state = RuntimeState::new(
            local_id,
            local_secret,
            RuntimeConfig {
                enable_embedded_relay: true,
                enable_embedded_relay_publication: true,
                ..Default::default()
            },
        );

        // Simulate embedded relay becoming healthy (public IP → publishable)
        let relay_url: tom_connect::RelayUrl = "http://1.2.3.4:9999".parse().unwrap();
        state.embedded_relay_state = LocalEmbeddedRelayState::Healthy {
            bound_relay_url: relay_url.clone(),
        };

        // First publication (at startup)
        let pub_effects = state.build_relay_publication(relay_url);
        assert!(
            pub_effects.iter().any(|e| matches!(e, RuntimeEffect::BroadcastRelayReady(_))),
            "initial publication should produce BroadcastRelayReady"
        );

        // New gossip neighbor joins
        let peer = node_id(2);
        let effects = state.handle_gossip_event(super::GossipInput::NeighborUp(peer));

        // Must contain both GossipNeighborUp AND BroadcastRelayReady
        let has_neighbor_up = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::GossipNeighborUp { .. }))
        });
        let has_relay_broadcast = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::BroadcastRelayReady(_))
        });

        assert!(has_neighbor_up, "expected GossipNeighborUp event");
        assert!(
            has_relay_broadcast,
            "expected BroadcastRelayReady on NeighborUp when relay is published, got: {effects:?}"
        );
    }

    #[test]
    fn neighbor_up_no_republish_when_relay_not_active() {
        let mut state = default_state(1);
        let peer = node_id(2);

        let effects = state.handle_gossip_event(super::GossipInput::NeighborUp(peer));

        let has_relay_broadcast = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::BroadcastRelayReady(_))
        });
        assert!(
            !has_relay_broadcast,
            "should NOT broadcast relay when no embedded relay is active"
        );
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
        // ADR-011 ghost-peer fix: a gossip announce is an address, not a
        // liveness proof — Known, not Online.
        assert_eq!(topo_peer.unwrap().status, PeerStatus::Known);

        // A discovery-only announce no longer feeds the heartbeat tracker
        // (that's the fix: no PoP credit without real work), so no
        // PeerDiscovered fires from tick_heartbeat until the peer does real
        // work (signed inbound/ACK/witnessed relay).
        let tick_effects = state.tick_heartbeat();
        let has_discovered = tick_effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::PeerDiscovered { node_id, .. }) if *node_id == peer)
        });
        assert!(
            !has_discovered,
            "gossip announce alone must not emit PeerDiscovered (no liveness proof), got: {tick_effects:?}"
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
        // Relay ACKs are signed at emission (verrou #1) — the receiver gates
        // on signature_valid, so an unsigned ACK here would be rejected.
        let relay_secret = keypair(22).1;
        let relay_ack_env = EnvelopeBuilder::new(
            relay_id,
            alice_id,
            MessageType::Ack,
            relay_ack_payload.to_bytes(),
        )
        .sign(&relay_secret);
        let sig_valid = relay_ack_env.verify_signature().is_ok();
        let relay_effects = alice_state.handle_incoming_chat(relay_ack_env, sig_valid);
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
    fn forged_ack_rejected_no_status_change() {
        // Verrou #1 (delivered ⟺ ACK signé) — adversarial test: an attacker
        // sends an Ack envelope with no signature (or a bogus one). It must
        // NOT be able to fabricate delivery/relay confirmation.
        let (alice_id, alice_secret) = keypair(40);
        let (bob_id, _bob_secret) = keypair(41);

        let mut alice_state = RuntimeState::new(
            alice_id,
            alice_secret,
            RuntimeConfig {
                encryption: false,
                ..Default::default()
            },
        );

        let send_effects = alice_state.handle_send_message(bob_id, b"hi bob".to_vec());
        let envelope = match &send_effects[0] {
            RuntimeEffect::SendWithBackupFallback { envelope, .. } => envelope.clone(),
            other => panic!("expected SendWithBackupFallback, got: {other:?}"),
        };
        let msg_id = envelope.id.clone();

        use crate::router::{AckPayload, AckType};
        let forged_payload = AckPayload {
            original_message_id: msg_id,
            ack_type: AckType::RecipientReceived,
        };
        // Unsigned — an attacker impersonating Bob with no key at all.
        let forged_env = EnvelopeBuilder::new(
            bob_id,
            alice_id,
            MessageType::Ack,
            forged_payload.to_bytes(),
        )
        .build();
        assert!(!forged_env.is_signed());
        let effects = alice_state.handle_incoming_chat(forged_env, false);

        let has_status_change = effects
            .iter()
            .any(|e| matches!(e, RuntimeEffect::StatusChange(_)));
        assert!(
            !has_status_change,
            "forged/unsigned ACK must not produce a StatusChange, got: {effects:?}"
        );
        let rejected = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::MessageRejected { .. }))
        });
        assert!(rejected, "forged ACK should emit MessageRejected, got: {effects:?}");
    }

    #[test]
    fn tick_hub_cleanup_purges_expired_sqlite_rows() {
        // Verrou #2 (purge TTL 24h) — the SQLite cutoff passed to
        // cleanup_hub_messages must be an absolute timestamp (now - TTL),
        // not the raw TTL duration, or expired rows are never purged.
        let mut state = default_state(50);
        let store = crate::storage::StateStore::open_memory().unwrap();

        let group_id = GroupId::from("grp-purge-test".to_string());
        let now = now_ms();
        const TTL_MS: u64 = 24 * 60 * 60 * 1000;
        let expired_at = now.saturating_sub(TTL_MS + 60_000); // 25h old
        let fresh_at = now.saturating_sub(60_000); // 1 minute old

        store
            .save_hub_message(&group_id, 1, b"old message", expired_at)
            .unwrap();
        store
            .save_hub_message(&group_id, 2, b"recent message", fresh_at)
            .unwrap();

        state.store = Some(store);
        state.tick_hub_cleanup();

        let remaining = state
            .store
            .as_ref()
            .unwrap()
            .load_hub_messages_since(&group_id, 0, 100)
            .unwrap();
        assert_eq!(
            remaining.len(),
            1,
            "expected only the fresh message to survive purge, got: {remaining:?}"
        );
        assert_eq!(remaining[0].0, 2, "the surviving row should be seq=2 (fresh)");
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
        alice_state.tracker.mark_delivered(&msg_id, bob_id);

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
            invite_only: false,
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
        // Add a peer via AddPeer, verify topology. Remove via RemovePeer,
        // verify it's cleaned up.
        //
        // ADR-011 ghost-peer fix: AddPeer is discovery, not proof of work —
        // it no longer feeds the heartbeat tracker (liveness stays Departed
        // until the peer does real work: signed inbound/ACK/witnessed relay).
        let mut state = default_state(50);
        let peer = node_id(51);

        // Initially: not in topology or heartbeat
        assert!(state.topology.get(&peer).is_none());
        assert_eq!(state.heartbeat.liveness(&peer), crate::discovery::LivenessState::Departed);

        // Add peer
        state.handle_command(RuntimeCommand::AddPeer { node_id: peer, source: DiscoverySource::Direct });
        let topo_peer = state.topology.get(&peer);
        assert!(topo_peer.is_some(), "peer should be in topology after AddPeer");
        assert_eq!(topo_peer.unwrap().status, PeerStatus::Known);
        assert_eq!(
            state.heartbeat.liveness(&peer),
            crate::discovery::LivenessState::Departed,
            "AddPeer is discovery only — must not grant liveness without real work"
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
            "peer should stay untracked from heartbeat after RemovePeer"
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
            invite_only: false,
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

    // ── Hub-as-member self-send interception ───────────────────────────

    #[test]
    fn hub_as_member_join_emits_local_member_joined_event() {
        // When local node is BOTH hub and group creator, a remote Join should
        // produce a local GroupMemberJoined event (not a SendEnvelope to self).
        let (hub_id, hub_secret) = keypair(200);
        let (bob_id, bob_secret) = keypair(201);

        let mut state = RuntimeState::new(
            hub_id,
            hub_secret,
            RuntimeConfig {
                encryption: false,
                ..Default::default()
            },
        );

        // Create group locally (hub=self)
        let effects = state.handle_command(RuntimeCommand::CreateGroup {
            name: "SelfHubTest".to_string(),
            hub_relay_id: hub_id,
            initial_members: vec![bob_id],
            invite_only: false,
        });
        // Should emit GroupCreated event locally (no SendEnvelope to self)
        let has_created_event = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::GroupCreated { .. }))
        });
        assert!(
            has_created_event,
            "CreateGroup with hub=self should emit GroupCreated locally, got: {effects:?}"
        );
        // Should NOT have SendEnvelope to self
        let has_self_send = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::SendEnvelope(env) if env.to == hub_id)
        });
        assert!(
            !has_self_send,
            "CreateGroup with hub=self should NOT produce SendEnvelope to self"
        );

        // Get group_id
        let gid = state.group_hub.groups().next().unwrap().0.clone();

        // Bob joins via network
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

        let join_effects = state.handle_incoming_group(join_env);

        // Should emit GroupMemberJoined event LOCALLY (via interception)
        let has_member_joined = join_effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::GroupMemberJoined { member, .. })
                if member.node_id == bob_id)
        });
        assert!(
            has_member_joined,
            "Join when hub=self should emit local GroupMemberJoined, got: {join_effects:?}"
        );

        // Should NOT have SendEnvelope(GroupMemberJoined) to self
        let has_member_joined_self_send = join_effects.iter().any(|e| {
            matches!(e, RuntimeEffect::SendEnvelope(env)
                if env.to == hub_id && env.msg_type == MessageType::GroupMemberJoined)
        });
        assert!(
            !has_member_joined_self_send,
            "Join when hub=self should NOT SendEnvelope(MemberJoined) to self"
        );

        // Should still send Sync to bob (over network)
        let has_sync_to_bob = join_effects.iter().any(|e| {
            matches!(e, RuntimeEffect::SendEnvelope(env)
                if env.to == bob_id && env.msg_type == MessageType::GroupSync)
        });
        assert!(
            has_sync_to_bob,
            "Join should send GroupSync to joiner (bob), got: {join_effects:?}"
        );
    }

    #[test]
    fn hub_as_member_group_message_fans_out_not_self_send() {
        // When hub=self sends a group message, it should fan out to other
        // members via network (not self-send to hub).
        let (hub_id, hub_secret) = keypair(210);
        let (bob_id, bob_secret) = keypair(211);

        let mut state = RuntimeState::new(
            hub_id,
            hub_secret,
            RuntimeConfig {
                encryption: false,
                ..Default::default()
            },
        );

        // Create group locally (hub=self)
        state.handle_command(RuntimeCommand::CreateGroup {
            name: "MsgTest".to_string(),
            hub_relay_id: hub_id,
            initial_members: vec![bob_id],
            invite_only: false,
        });
        let gid = state.group_hub.groups().next().unwrap().0.clone();

        // Bob joins via network (full flow to populate both hub and manager)
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
        state.handle_incoming_group(join_env);

        // Send group message from hub (who is also a member)
        let effects = state.handle_command(RuntimeCommand::SendGroupMessage {
            group_id: gid.clone(),
            text: "hello from hub".to_string(),
        });

        // Should send to bob (network)
        let sends_to_bob: Vec<_> = effects.iter().filter(|e| {
            matches!(e, RuntimeEffect::SendEnvelope(env)
                if env.to == bob_id && env.msg_type == MessageType::GroupMessage)
        }).collect();
        assert!(
            !sends_to_bob.is_empty(),
            "Group message from hub should fan out to bob, got: {effects:?}"
        );

        // Should NOT self-send to hub
        let self_sends: Vec<_> = effects.iter().filter(|e| {
            matches!(e, RuntimeEffect::SendEnvelope(env)
                if env.to == hub_id)
        }).collect();
        assert!(
            self_sends.is_empty(),
            "Group message from hub should NOT produce SendEnvelope to self, got self-sends: {self_sends:?}"
        );
    }

    #[test]
    fn hub_heartbeat_does_not_self_send() {
        // When hub=self ticks heartbeat, it should broadcast to other members
        // but NOT produce a SendEnvelope to self.
        let (hub_id, hub_secret) = keypair(220);
        let (bob_id, bob_secret) = keypair(221);

        let mut state = RuntimeState::new(
            hub_id,
            hub_secret,
            RuntimeConfig {
                encryption: false,
                ..Default::default()
            },
        );

        // Create group locally (hub=self)
        state.handle_command(RuntimeCommand::CreateGroup {
            name: "HBTest".to_string(),
            hub_relay_id: hub_id,
            initial_members: vec![bob_id],
            invite_only: false,
        });
        let gid = state.group_hub.groups().next().unwrap().0.clone();

        // Bob joins
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
        state.handle_incoming_group(join_env);

        // Tick heartbeat
        let effects = state.tick_group_hub_heartbeat();

        // Should send heartbeat to bob (network)
        let sends_to_bob: Vec<_> = effects
            .iter()
            .filter(|e| {
                matches!(e, RuntimeEffect::SendEnvelope(env)
                    if env.to == bob_id && env.msg_type == MessageType::GroupHubHeartbeat)
            })
            .collect();
        assert!(
            !sends_to_bob.is_empty(),
            "Heartbeat should reach bob, got: {effects:?}"
        );

        // Should NOT self-send heartbeat to hub
        let self_sends: Vec<_> = effects
            .iter()
            .filter(|e| {
                matches!(e, RuntimeEffect::SendEnvelope(env)
                    if env.to == hub_id)
            })
            .collect();
        assert!(
            self_sends.is_empty(),
            "Heartbeat should NOT produce SendEnvelope to self, got: {self_sends:?}"
        );
    }

    // ── Anti-spam integration tests (R11.1) ─────────────────────────────

    #[test]
    fn antispam_handle_incoming_rejects_oversized() {
        let mut state = default_state(1);
        // Au-dessus du plafond anti-abus (64 Mo). En-dessous, les gros messages
        // sont désormais acceptés (segmentés au transport) — cf chunking.
        let huge = vec![0u8; crate::roles::antispam::MAX_ENVELOPE_SIZE + 1];
        let effects = state.handle_incoming(&huge);

        let rejected = effects.iter().any(|e| {
            matches!(e, RuntimeEffect::Emit(ProtocolEvent::MessageRejected { .. }))
        });
        assert!(rejected, "oversized envelope should be rejected: {effects:?}");
    }

    #[test]
    fn antispam_handle_incoming_throttles_spammer() {
        let (local_id, local_secret) = keypair(1);
        let mut antispam_config = crate::roles::AntiSpamConfig::default();
        antispam_config.min_rate = 1.0;
        antispam_config.max_rate = 1.0;

        let mut state = RuntimeState::new(
            local_id,
            local_secret,
            RuntimeConfig {
                antispam_config,
                ..Default::default()
            },
        );
        let (sender_id, sender_secret) = keypair(42);
        // Fixed low rate for deterministic CI behavior: rate=1 msg/s, burst=2.

        let env = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            state.local_id,
            MessageType::Chat,
            b"spam".to_vec(),
        )
        .sign(&sender_secret);
        let raw = env.to_bytes().expect("serialize");

        // Send a small rapid burst — first 2 may pass, the rest must throttle.
        let mut throttled = 0;
        for _ in 0..10 {
            let effects = state.handle_incoming(raw.as_slice());
            if effects.iter().any(|e| {
                matches!(e, RuntimeEffect::Emit(ProtocolEvent::SenderThrottled { .. }))
            }) {
                throttled += 1;
            }
        }

        assert!(throttled > 0, "spammer should be throttled after burst");
        // We only assert behavioral throttling, not an exact count.
    }

    #[test]
    fn antispam_handle_incoming_records_bytes_received() {
        let mut state = default_state(1);
        let (sender_id, sender_secret) = keypair(43);

        let env = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            state.local_id,
            MessageType::Chat,
            b"hello".to_vec(),
        )
        .sign(&sender_secret);
        let raw = env.to_bytes().expect("serialize");

        let before = state
            .role_manager
            .scores()
            .get(&sender_id)
            .map(|m| m.bytes_received)
            .unwrap_or(0);

        state.handle_incoming(raw.as_slice());

        let after = state
            .role_manager
            .scores()
            .get(&sender_id)
            .map(|m| m.bytes_received)
            .unwrap_or(0);

        assert!(
            after > before,
            "bytes_received should be tracked: before={before}, after={after}"
        );
        assert_eq!(after - before, raw.len() as u64);
    }

    #[test]
    fn antispam_exempts_ack_and_heartbeat() {
        let mut state = default_state(1);
        let (sender_id, sender_secret) = keypair(44);

        // Build ACK envelope
        let ack_env = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            state.local_id,
            MessageType::Ack,
            b"ack-payload".to_vec(),
        )
        .sign(&sender_secret);
        let ack_raw = ack_env.to_bytes().expect("serialize");

        // Build Heartbeat envelope
        let hb_env = crate::envelope::EnvelopeBuilder::new(
            sender_id,
            state.local_id,
            MessageType::Heartbeat,
            b"hb".to_vec(),
        )
        .sign(&sender_secret);
        let hb_raw = hb_env.to_bytes().expect("serialize");

        // Send 20 ACKs + 20 heartbeats rapidly — NONE should be throttled
        let mut throttled = 0;
        for _ in 0..20 {
            let effects = state.handle_incoming(ack_raw.as_slice());
            if effects.iter().any(|e| {
                matches!(e, RuntimeEffect::Emit(ProtocolEvent::SenderThrottled { .. }))
            }) {
                throttled += 1;
            }
        }
        for _ in 0..20 {
            let effects = state.handle_incoming(hb_raw.as_slice());
            if effects.iter().any(|e| {
                matches!(e, RuntimeEffect::Emit(ProtocolEvent::SenderThrottled { .. }))
            }) {
                throttled += 1;
            }
        }
        assert_eq!(
            throttled, 0,
            "ACK and Heartbeat should never be throttled, but {throttled} were"
        );
    }

    // ── Embedded relay publication tests ──────────────────────────────

    #[test]
    fn relay_publication_refused_when_not_healthy() {
        let mut state = default_state(50);
        state.config.enable_embedded_relay = true;
        state.config.enable_embedded_relay_publication = true;

        let url: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        let effects = state.build_relay_publication(url);
        assert!(effects.is_empty(), "should not publish when relay is Stopped");
    }

    #[test]
    fn relay_publication_refused_when_feature_disabled() {
        let mut state = default_state(51);
        state.config.enable_embedded_relay = true;
        state.config.enable_embedded_relay_publication = false;

        let url: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        state.embedded_relay_state = LocalEmbeddedRelayState::Healthy {
            bound_relay_url: url.clone(),
        };

        let effects = state.build_relay_publication(url);
        assert!(effects.is_empty(), "should not publish when feature is disabled");
    }

    #[test]
    fn relay_publication_ok_when_healthy_and_enabled() {
        let mut state = default_state(52);
        state.config.enable_embedded_relay = true;
        state.config.enable_embedded_relay_publication = true;

        let url: tom_connect::RelayUrl = "http://1.2.3.4:3340".parse().unwrap();
        state.embedded_relay_state = LocalEmbeddedRelayState::Healthy {
            bound_relay_url: url.clone(),
        };

        let effects = state.build_relay_publication(url.clone());
        assert_eq!(effects.len(), 1, "should produce one BroadcastRelayReady effect");
        assert!(
            matches!(&effects[0], RuntimeEffect::BroadcastRelayReady(announce)
                if announce.relay_url == url && announce.verify_signature()),
            "effect should be a valid signed BroadcastRelayReady"
        );

        // Publication state should be updated
        assert!(
            matches!(&state.embedded_relay_publication,
                EmbeddedRelayPublicationState::Published { url: pub_url, .. }
                if pub_url == &url),
            "publication state should be Published"
        );
    }

    #[test]
    fn relay_started_command_updates_state_and_publishes() {
        let mut state = default_state(53);
        state.config.enable_embedded_relay = true;
        state.config.enable_embedded_relay_publication = true;

        let url: tom_connect::RelayUrl = "http://1.2.3.4:3340".parse().unwrap();
        let effects = state.handle_command(RuntimeCommand::EmbeddedRelayStarted { url: url.clone() });

        // Should have: Emit(EmbeddedRelayStarted) + BroadcastRelayReady
        assert_eq!(effects.len(), 2);
        assert!(matches!(&effects[0], RuntimeEffect::Emit(ProtocolEvent::EmbeddedRelayStarted { .. })));
        assert!(matches!(&effects[1], RuntimeEffect::BroadcastRelayReady(_)));

        // State should be Healthy
        assert!(matches!(&state.embedded_relay_state,
            LocalEmbeddedRelayState::Healthy { bound_relay_url }
            if bound_relay_url == &url));
    }

    #[test]
    fn relay_with_private_ip_is_healthy_but_not_published() {
        // Universal rule: a relay bound to a LAN/private IP stays healthy and
        // usable locally, but is NEVER advertised to the global gossip mesh.
        let mut state = default_state(63);
        state.config.enable_embedded_relay = true;
        state.config.enable_embedded_relay_publication = true;

        let url: tom_connect::RelayUrl = "http://192.168.0.70:65127".parse().unwrap();
        let effects = state.handle_command(RuntimeCommand::EmbeddedRelayStarted { url: url.clone() });

        // Healthy event emitted, but NO BroadcastRelayReady.
        assert!(matches!(&effects[0], RuntimeEffect::Emit(ProtocolEvent::EmbeddedRelayStarted { .. })));
        assert!(
            !effects.iter().any(|e| matches!(e, RuntimeEffect::BroadcastRelayReady(_))),
            "private-IP relay must not be published to global gossip"
        );
        assert!(matches!(&state.embedded_relay_state,
            LocalEmbeddedRelayState::Healthy { .. }), "relay still healthy/usable on LAN");
        assert!(matches!(&state.embedded_relay_publication,
            EmbeddedRelayPublicationState::NotPublished), "must remain unpublished");
    }

    #[test]
    fn reachability_rule_covers_all_environments() {
        let pub_v4: tom_connect::RelayUrl = "http://1.2.3.4:3340".parse().unwrap();
        let dns: tom_connect::RelayUrl = "https://relay.example.com:443".parse().unwrap();
        let priv_v4: tom_connect::RelayUrl = "http://192.168.0.70:3340".parse().unwrap();
        let ten: tom_connect::RelayUrl = "http://10.0.0.1:3340".parse().unwrap();
        let cgnat: tom_connect::RelayUrl = "http://100.64.1.1:3340".parse().unwrap();
        let loop_v4: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        let link: tom_connect::RelayUrl = "http://169.254.1.1:3340".parse().unwrap();
        let loop_v6: tom_connect::RelayUrl = "http://[::1]:3340".parse().unwrap();
        // Non-routable DNS names must NOT be treated as global.
        let localhost: tom_connect::RelayUrl = "http://localhost:3340".parse().unwrap();
        let mdns: tom_connect::RelayUrl = "http://nas.local:3340".parse().unwrap();
        let intern: tom_connect::RelayUrl = "http://relay.internal:3340".parse().unwrap();

        assert!(relay_url_is_globally_reachable(&pub_v4));
        assert!(relay_url_is_globally_reachable(&dns));
        assert!(!relay_url_is_globally_reachable(&priv_v4));
        assert!(!relay_url_is_globally_reachable(&ten));
        assert!(!relay_url_is_globally_reachable(&cgnat));
        assert!(!relay_url_is_globally_reachable(&loop_v4));
        assert!(!relay_url_is_globally_reachable(&link));
        assert!(!relay_url_is_globally_reachable(&loop_v6));
        assert!(!relay_url_is_globally_reachable(&localhost), "localhost n'est pas global");
        assert!(!relay_url_is_globally_reachable(&mdns), "mDNS .local n'est pas global");
        assert!(!relay_url_is_globally_reachable(&intern), ".internal n'est pas global");
    }

    #[test]
    fn relay_failed_command_resets_publication() {
        let mut state = default_state(54);
        state.config.enable_embedded_relay_publication = true;

        let url: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        state.embedded_relay_state = LocalEmbeddedRelayState::Healthy {
            bound_relay_url: url.clone(),
        };
        state.embedded_relay_publication = EmbeddedRelayPublicationState::Published {
            url,
            published_at: 1000,
        };

        let effects = state.handle_command(RuntimeCommand::EmbeddedRelayFailed {
            error: "bind error".into(),
        });

        assert!(matches!(&state.embedded_relay_state,
            LocalEmbeddedRelayState::Failed { error, .. }
            if error == "bind error"));
        assert_eq!(state.embedded_relay_publication, EmbeddedRelayPublicationState::NotPublished);
        assert!(effects.iter().any(|e| matches!(e, RuntimeEffect::Emit(ProtocolEvent::EmbeddedRelayFailed { .. }))));
    }

    #[test]
    fn relay_stopped_command_resets_state() {
        let mut state = default_state(55);

        let url: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();
        state.embedded_relay_state = LocalEmbeddedRelayState::Healthy {
            bound_relay_url: url,
        };

        let effects = state.handle_command(RuntimeCommand::EmbeddedRelayStopped);

        assert_eq!(state.embedded_relay_state, LocalEmbeddedRelayState::Stopped);
        assert_eq!(state.embedded_relay_publication, EmbeddedRelayPublicationState::NotPublished);
        assert!(effects.iter().any(|e| matches!(e, RuntimeEffect::Emit(ProtocolEvent::EmbeddedRelayStopped))));
    }

    #[test]
    fn relay_ready_announce_received_emits_event() {
        let mut state = default_state(56);
        let (other_id, other_seed) = keypair(99);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:4444".parse().unwrap();

        let announce = crate::discovery::RelayReadyAnnounce::new(
            other_id,
            url.clone(),
            now_ms(),
            &other_seed,
        );

        let effects = state.handle_relay_ready_announce(announce);
        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0],
            RuntimeEffect::Emit(ProtocolEvent::RelayReadyReceived { node_id, relay_url })
            if *node_id == other_id && relay_url == &url));
    }

    #[test]
    fn relay_ready_announce_self_ignored() {
        let mut state = default_state(57);
        let url: tom_connect::RelayUrl = "http://127.0.0.1:3340".parse().unwrap();

        let announce = crate::discovery::RelayReadyAnnounce::new(
            state.local_id,
            url,
            now_ms(),
            &state.secret_seed,
        );

        let effects = state.handle_relay_ready_announce(announce);
        assert!(effects.is_empty(), "self-announce should be ignored");
    }

    #[test]
    fn relay_ready_announce_invalid_signature_rejected() {
        let mut state = default_state(58);
        let (other_id, _) = keypair(99);
        let (_, wrong_seed) = keypair(123);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:4444".parse().unwrap();

        let announce = crate::discovery::RelayReadyAnnounce::new(
            other_id,
            url,
            now_ms(),
            &wrong_seed,
        );

        let effects = state.handle_relay_ready_announce(announce);
        assert!(effects.is_empty(), "invalid signature should be rejected");
    }

    // ── Relay Registry integration tests ────────────────────────────────

    #[test]
    fn relay_ready_announce_stores_in_registry() {
        let mut state = default_state(60);
        let (other_id, other_seed) = keypair(100);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:3340".parse().unwrap();
        let now = now_ms();

        let announce = crate::discovery::RelayReadyAnnounce::new(
            other_id,
            url.clone(),
            now,
            &other_seed,
        );

        let effects = state.handle_relay_ready_announce(announce);

        // Should emit RelayReadyReceived
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            &effects[0],
            RuntimeEffect::Emit(ProtocolEvent::RelayReadyReceived { node_id, relay_url })
            if *node_id == other_id && relay_url == &url
        ));

        // Should be in registry
        let entry = state
            .relay_registry
            .get(&other_id)
            .expect("should be in registry");
        assert_eq!(entry.relay_url, url);
        assert_eq!(entry.announced_at, now);
    }

    #[test]
    fn relay_registry_prune_via_tick_heartbeat() {
        let mut state = default_state(61);
        let (other_id, _) = keypair(101);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:3340".parse().unwrap();

        // Insert directly with expires_at far in the past so prune catches it
        state.relay_registry = crate::discovery::RelayRegistry::new(1); // 1ms TTL
        state.relay_registry.upsert(other_id, url.clone(), 0, 0); // expires_at = 1
        assert_eq!(state.relay_registry.len(), 1);

        // Tick heartbeat should prune the expired entry (now_ms() >> 1)
        let effects = state.tick_heartbeat();
        assert!(state.relay_registry.is_empty());

        let has_expired = effects.iter().any(|e| {
            matches!(
                e,
                RuntimeEffect::Emit(ProtocolEvent::RelayRegistryExpired {
                    node_id,
                    relay_url
                }) if *node_id == other_id && relay_url == &url
            )
        });
        assert!(has_expired, "should emit RelayRegistryExpired");
    }

    // ── Transport Relay Discovery tests ────────────────────────────────

    fn state_with_transport_relay_discovery(seed: u8) -> RuntimeState {
        let (id, secret) = keypair(seed);
        let mut config = RuntimeConfig::default();
        config.enable_transport_relay_discovery = true;
        RuntimeState::new(id, secret, config)
    }

    #[test]
    fn announce_with_flag_off_no_transport_effect() {
        let mut state = default_state(70); // flag is false by default
        let (other_id, other_seed) = keypair(170);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:5555".parse().unwrap();

        let announce = crate::discovery::RelayReadyAnnounce::new(
            other_id, url, now_ms(), &other_seed,
        );
        let effects = state.handle_relay_ready_announce(announce);

        // Only RelayReadyReceived, no InsertTransportRelay
        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0], RuntimeEffect::Emit(ProtocolEvent::RelayReadyReceived { .. })));
        assert!(!effects.iter().any(|e| matches!(e, RuntimeEffect::InsertTransportRelay { .. })));
    }

    #[test]
    fn announce_with_flag_on_emits_insert() {
        let mut state = state_with_transport_relay_discovery(71);
        let (other_id, other_seed) = keypair(171);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:5555".parse().unwrap();

        let announce = crate::discovery::RelayReadyAnnounce::new(
            other_id, url.clone(), now_ms(), &other_seed,
        );
        let effects = state.handle_relay_ready_announce(announce);

        // Should have RelayReadyReceived + InsertTransportRelay
        assert_eq!(effects.len(), 2);
        assert!(matches!(&effects[0], RuntimeEffect::Emit(ProtocolEvent::RelayReadyReceived { .. })));
        assert!(matches!(&effects[1], RuntimeEffect::InsertTransportRelay { relay_url } if relay_url == &url));
    }

    #[test]
    fn announce_refresh_same_url_no_insert() {
        let mut state = state_with_transport_relay_discovery(72);
        let (other_id, other_seed) = keypair(172);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:5555".parse().unwrap();

        // First announce
        let announce1 = crate::discovery::RelayReadyAnnounce::new(
            other_id, url.clone(), now_ms(), &other_seed,
        );
        let _ = state.handle_relay_ready_announce(announce1);

        // Second announce (same URL = refresh)
        let announce2 = crate::discovery::RelayReadyAnnounce::new(
            other_id, url.clone(), now_ms(), &other_seed,
        );
        let effects = state.handle_relay_ready_announce(announce2);

        // Only RelayReadyReceived, no InsertTransportRelay (same URL refreshed)
        assert_eq!(effects.len(), 1);
        assert!(matches!(&effects[0], RuntimeEffect::Emit(ProtocolEvent::RelayReadyReceived { .. })));
        assert!(!effects.iter().any(|e| matches!(e, RuntimeEffect::InsertTransportRelay { .. })));
    }

    #[test]
    fn announce_url_change_emits_insert_and_remove() {
        let mut state = state_with_transport_relay_discovery(73);
        let (other_id, other_seed) = keypair(173);
        let url1: tom_connect::RelayUrl = "http://10.0.0.1:5555".parse().unwrap();
        let url2: tom_connect::RelayUrl = "http://10.0.0.2:6666".parse().unwrap();

        // First announce with url1
        let announce1 = crate::discovery::RelayReadyAnnounce::new(
            other_id, url1.clone(), now_ms(), &other_seed,
        );
        let _ = state.handle_relay_ready_announce(announce1);

        // Second announce with url2 (URL changed)
        let announce2 = crate::discovery::RelayReadyAnnounce::new(
            other_id, url2.clone(), now_ms(), &other_seed,
        );
        let effects = state.handle_relay_ready_announce(announce2);

        // Should emit: RelayReadyReceived + InsertTransportRelay(url2) + RemoveTransportRelay(url1)
        assert_eq!(effects.len(), 3);
        assert!(matches!(&effects[1], RuntimeEffect::InsertTransportRelay { relay_url } if relay_url == &url2));
        assert!(matches!(&effects[2], RuntimeEffect::RemoveTransportRelay { relay_url } if relay_url == &url1));
    }

    #[test]
    fn announce_url_change_shared_url_no_remove() {
        let mut state = state_with_transport_relay_discovery(74);
        let (id_a, seed_a) = keypair(174);
        let (id_b, seed_b) = keypair(175);
        let shared_url: tom_connect::RelayUrl = "http://10.0.0.1:5555".parse().unwrap();
        let new_url: tom_connect::RelayUrl = "http://10.0.0.2:6666".parse().unwrap();

        // Peer A announces shared_url
        let a1 = crate::discovery::RelayReadyAnnounce::new(
            id_a, shared_url.clone(), now_ms(), &seed_a,
        );
        let _ = state.handle_relay_ready_announce(a1);

        // Peer B also announces shared_url
        let b1 = crate::discovery::RelayReadyAnnounce::new(
            id_b, shared_url.clone(), now_ms(), &seed_b,
        );
        let _ = state.handle_relay_ready_announce(b1);

        // Peer A changes to new_url → shared_url still active via B
        let a2 = crate::discovery::RelayReadyAnnounce::new(
            id_a, new_url.clone(), now_ms(), &seed_a,
        );
        let effects = state.handle_relay_ready_announce(a2);

        // Should emit InsertTransportRelay(new_url) but NOT RemoveTransportRelay(shared_url)
        assert!(effects.iter().any(|e| matches!(e, RuntimeEffect::InsertTransportRelay { relay_url } if relay_url == &new_url)));
        assert!(!effects.iter().any(|e| matches!(e, RuntimeEffect::RemoveTransportRelay { relay_url } if relay_url == &shared_url)));
    }

    #[test]
    fn prune_with_flag_on_emits_remove() {
        let mut state = state_with_transport_relay_discovery(75);
        let (other_id, _) = keypair(176);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:5555".parse().unwrap();

        // Insert with very short TTL so it expires immediately
        state.relay_registry = crate::discovery::RelayRegistry::new(1); // 1ms TTL
        state.relay_registry.upsert(other_id, url.clone(), 0, 0); // expires_at = 1
        assert_eq!(state.relay_registry.len(), 1);

        let effects = state.tick_heartbeat();
        assert!(state.relay_registry.is_empty());

        // Should have RemoveTransportRelay for the expired URL
        assert!(
            effects.iter().any(|e| matches!(e, RuntimeEffect::RemoveTransportRelay { relay_url } if relay_url == &url)),
            "should emit RemoveTransportRelay on prune"
        );
    }

    #[test]
    fn prune_shared_url_no_remove() {
        let mut state = state_with_transport_relay_discovery(76);
        let (id_a, _) = keypair(177);
        let (id_b, _) = keypair(178);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:5555".parse().unwrap();

        // Use very short TTL
        state.relay_registry = crate::discovery::RelayRegistry::new(1); // 1ms TTL

        // Insert A with expired timestamp, B with fresh timestamp
        state.relay_registry.upsert(id_a, url.clone(), 0, 0); // expires_at = 1 (expired)

        // Re-create registry to set a long TTL for B
        // Actually, let's insert B with a future expires_at instead
        // The registry uses a single TTL, so we need another approach:
        // Insert B directly after prune — but prune happens atomically.
        // Simpler: use a normal TTL, insert A long ago, insert B recently.
        state.relay_registry = crate::discovery::RelayRegistry::new(100); // 100ms TTL
        state.relay_registry.upsert(id_a, url.clone(), 0, 0); // expires_at = 100 (expired, now >> 100)
        state.relay_registry.upsert(id_b, url.clone(), now_ms(), now_ms()); // fresh

        assert_eq!(state.relay_registry.len(), 2);

        let effects = state.tick_heartbeat();

        // A should be pruned, B should remain
        assert_eq!(state.relay_registry.len(), 1);
        assert!(state.relay_registry.get(&id_b).is_some());

        // URL is still active (via B), so NO RemoveTransportRelay
        assert!(
            !effects.iter().any(|e| matches!(e, RuntimeEffect::RemoveTransportRelay { .. })),
            "shared URL should NOT be removed"
        );
    }

    #[test]
    fn no_topology_mutation_on_announce() {
        let mut state = state_with_transport_relay_discovery(77);
        let (other_id, other_seed) = keypair(179);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:5555".parse().unwrap();

        let topo_before = state.topology.len();

        let announce = crate::discovery::RelayReadyAnnounce::new(
            other_id, url, now_ms(), &other_seed,
        );
        let _ = state.handle_relay_ready_announce(announce);

        // Topology must NOT be mutated by transport relay discovery
        assert_eq!(state.topology.len(), topo_before);
    }

    #[test]
    fn no_relay_selector_impact() {
        let mut state = state_with_transport_relay_discovery(78);
        let (other_id, other_seed) = keypair(180);
        let url: tom_connect::RelayUrl = "http://10.0.0.1:5555".parse().unwrap();

        let announce = crate::discovery::RelayReadyAnnounce::new(
            other_id, url, now_ms(), &other_seed,
        );
        let _ = state.handle_relay_ready_announce(announce);

        // RelaySelector should not have been modified
        // (select_best returns None relay for unknown target — no new relay added)
        let dummy_target = keypair(181).0;
        let selected = state.relay_selector.select_best(dummy_target, &state.topology);
        assert!(selected.relay_id.is_none(),
            "relay selector should not be affected by transport relay discovery");
    }

    #[test]
    fn get_known_relays_sorted_by_freshest() {
        let mut state = default_state(62);
        let (id1, _) = keypair(201);
        let (id2, _) = keypair(202);
        let url1: tom_connect::RelayUrl = "http://10.0.0.1:3340".parse().unwrap();
        let url2: tom_connect::RelayUrl = "http://10.0.0.2:3340".parse().unwrap();

        // Insert directly into registry with controlled timestamps
        state.relay_registry.upsert(id1, url1, 1000, 1000); // older
        state.relay_registry.upsert(id2, url2, 2000, 2000); // newer

        // Query via handle_command
        let (tx, mut rx) = tokio::sync::oneshot::channel();
        state.handle_command(super::RuntimeCommand::GetKnownRelays { reply: tx });
        let relays = rx.try_recv().expect("should receive");
        assert_eq!(relays.len(), 2);
        // id2 should be first (more recent refreshed_at)
        assert_eq!(relays[0].node_id, id2);
        assert_eq!(relays[1].node_id, id1);
    }

    // ── Relay republication tests ────────────────────────────────────────

    fn state_with_relay_publication(seed: u8) -> RuntimeState {
        let (id, secret) = keypair(seed);
        let mut config = RuntimeConfig::default();
        config.enable_embedded_relay = true;
        config.enable_embedded_relay_publication = true;
        config.relay_publish_interval = std::time::Duration::from_millis(100);
        config.heartbeat_interval = std::time::Duration::from_millis(50);
        let mut state = RuntimeState::new(id, secret, config);
        // Simulate healthy relay (public IP → publishable to global gossip)
        state.embedded_relay_state = LocalEmbeddedRelayState::Healthy {
            bound_relay_url: "http://1.2.3.4:3340".parse().unwrap(),
        };
        state.embedded_relay_publication = EmbeddedRelayPublicationState::Published {
            url: "http://1.2.3.4:3340".parse().unwrap(),
            published_at: now_ms(),
        };
        state
    }

    #[test]
    fn relay_republication_on_heartbeat_after_interval() {
        let mut state = state_with_relay_publication(90);
        // Set published_at to well before interval threshold
        state.embedded_relay_publication = EmbeddedRelayPublicationState::Published {
            url: "http://1.2.3.4:3340".parse().unwrap(),
            published_at: now_ms().saturating_sub(200), // 200ms ago, interval is 100ms
        };
        let effects = state.tick_heartbeat();
        let has_broadcast = effects.iter().any(|e|
            matches!(e, RuntimeEffect::BroadcastRelayReady(_))
        );
        assert!(has_broadcast, "should republish after interval elapsed");
    }

    #[test]
    fn relay_no_republication_before_interval() {
        let mut state = state_with_relay_publication(91);
        // published_at is now() — interval not elapsed
        let effects = state.tick_heartbeat();
        let has_broadcast = effects.iter().any(|e|
            matches!(e, RuntimeEffect::BroadcastRelayReady(_))
        );
        assert!(!has_broadcast, "should NOT republish before interval");
    }

    #[test]
    fn relay_no_republication_when_stopped() {
        let mut state = state_with_relay_publication(92);
        state.embedded_relay_state = LocalEmbeddedRelayState::Stopped;
        state.embedded_relay_publication = EmbeddedRelayPublicationState::Published {
            url: "http://127.0.0.1:3340".parse().unwrap(),
            published_at: now_ms().saturating_sub(200),
        };
        let effects = state.tick_heartbeat();
        let has_broadcast = effects.iter().any(|e|
            matches!(e, RuntimeEffect::BroadcastRelayReady(_))
        );
        assert!(!has_broadcast, "should NOT republish when relay stopped");
    }

    #[test]
    fn relay_no_republication_when_publication_disabled() {
        let (id, secret) = keypair(93);
        let mut config = RuntimeConfig::default();
        config.enable_embedded_relay = true;
        config.enable_embedded_relay_publication = false; // disabled
        config.relay_publish_interval = std::time::Duration::from_millis(100);
        let mut state = RuntimeState::new(id, secret, config);
        state.embedded_relay_state = LocalEmbeddedRelayState::Healthy {
            bound_relay_url: "http://127.0.0.1:3340".parse().unwrap(),
        };
        state.embedded_relay_publication = EmbeddedRelayPublicationState::Published {
            url: "http://127.0.0.1:3340".parse().unwrap(),
            published_at: now_ms().saturating_sub(200),
        };
        let effects = state.tick_heartbeat();
        let has_broadcast = effects.iter().any(|e|
            matches!(e, RuntimeEffect::BroadcastRelayReady(_))
        );
        assert!(!has_broadcast, "should NOT republish when publication disabled");
    }

    // ── Faille 1 tests: Online → Stale dégradation ──────────────────────────

    #[test]
    fn tick_presence_cleanup_degrades_online_peer_when_stale() {
        let mut state = default_state(1);
        let peer = node_id(2);

        // Register peer as Online with very old last_seen
        state.topology.upsert(crate::relay::PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 0, // very old
        });

        // Jump time forward by simulating presence clock offset (test harness)
        let time_jump_ms = crate::relay::PEER_ONLINE_STALE_MS + 1_000; // ensure > PEER_ONLINE_STALE_MS
        state.config.presence_clock_offset_ms = time_jump_ms as i64;

        // Cleanup should degrade the peer
        let effects = state.tick_presence_cleanup();

        // Check that peer is now Stale, not Online. NOT `Known`: HeartbeatTracker
        // only restores Online from Stale/Offline (never from Known), so landing
        // on Stale keeps the peer recoverable via a later fresh heartbeat too.
        let peer_info = state.topology.get(&peer).expect("peer should still exist");
        assert_eq!(peer_info.status, PeerStatus::Stale, "peer should be degraded to Stale");
        assert!(
            effects.iter().any(|e| matches!(
                e,
                RuntimeEffect::Emit(ProtocolEvent::PeerStale { node_id }) if *node_id == peer
            )),
            "demotion should emit PeerStale, got: {effects:?}"
        );
    }

    #[test]
    fn tick_presence_cleanup_does_not_degrade_fresh_online_peer() {
        let mut state = default_state(1);
        let peer = node_id(2);
        let now = state.presence_now();

        // Register peer as Online with fresh last_seen
        state.topology.upsert(crate::relay::PeerInfo {
            node_id: peer,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: now, // fresh
        });

        // Cleanup should NOT degrade the peer
        let _effects = state.tick_presence_cleanup();

        // Check that peer is still Online
        let peer_info = state.topology.get(&peer).expect("peer should still exist");
        assert_eq!(peer_info.status, PeerStatus::Online, "peer should remain Online");
    }

    #[test]
    fn tick_presence_cleanup_does_not_degrade_self() {
        let mut state = default_state(1);
        let self_id = state.local_id;

        // Register self as Online with very old last_seen
        state.topology.upsert(crate::relay::PeerInfo {
            node_id: self_id,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 0, // very old
        });

        // Jump time forward
        let time_jump_ms = crate::relay::PEER_ONLINE_STALE_MS + 1_000;
        state.config.presence_clock_offset_ms = time_jump_ms as i64;

        // Cleanup
        let _effects = state.tick_presence_cleanup();

        // Check that self was NOT degraded (special case: never degrade self)
        let self_info = state.topology.get(&self_id).expect("self should exist");
        assert_eq!(self_info.status, PeerStatus::Online, "self should never be degraded");
    }

    // ── Faille 2 tests: Scope size validation ────────────────────────────

    #[test]
    fn handle_presence_subscribe_rejects_oversized_peer_scope() {
        use crate::envelope::EnvelopeBuilder;

        let mut state = default_state(1);
        let subscriber = node_id(2);

        // Create a scope with more peers than allowed (MAX_VIEW_ENTRIES + 1)
        // Use u16 to avoid overflow when MAX_VIEW_ENTRIES = 256
        let mut all_peers = Vec::new();
        for i in 0..=(crate::presence::MAX_VIEW_ENTRIES as u16) {
            all_peers.push(node_id((i % 256) as u8));
        }
        let scope = crate::presence::PresenceScope::Peers(all_peers);

        // Build and sign the subscribe envelope
        let payload = crate::presence::PresenceSubscribePayload { scope };
        let payload_bytes = payload.to_bytes();
        let envelope = EnvelopeBuilder::new(
            subscriber,
            state.local_id,
            crate::types::MessageType::PresenceSubscribe,
            payload_bytes,
        )
        .sign(&state.secret_seed);

        let before_len = state.subscriptions.len();

        // Call handle_presence_subscribe with valid signature
        let _effects = state.handle_presence_subscribe(&envelope, true);

        // Check that subscription was NOT stored (rejected due to size)
        let after_len = state.subscriptions.len();
        assert_eq!(
            before_len, after_len,
            "oversized subscription should be rejected"
        );
    }

    #[test]
    fn handle_presence_subscribe_accepts_peer_scope_within_limit() {
        use crate::envelope::EnvelopeBuilder;

        let mut state = default_state(1);
        let subscriber = node_id(2);

        // Create a scope with peers within the limit
        let limited_peers: Vec<NodeId> = (0..10).map(|i| node_id(i as u8)).collect();
        let scope = crate::presence::PresenceScope::Peers(limited_peers);

        let payload = crate::presence::PresenceSubscribePayload { scope };
        let payload_bytes = payload.to_bytes();
        let envelope = EnvelopeBuilder::new(
            subscriber,
            state.local_id,
            crate::types::MessageType::PresenceSubscribe,
            payload_bytes,
        )
        .sign(&state.secret_seed);

        let before_len = state.subscriptions.len();

        // Call handle_presence_subscribe with valid signature
        let _effects = state.handle_presence_subscribe(&envelope, true);

        // Check that subscription WAS stored
        let after_len = state.subscriptions.len();
        assert_eq!(before_len + 1, after_len, "valid subscription should be accepted");
    }

    #[test]
    fn handle_presence_subscribe_accepts_group_scope_any_size() {
        use crate::envelope::EnvelopeBuilder;
        use crate::group::GroupId;

        let mut state = default_state(1);
        let subscriber = node_id(2);

        // Group scope is not size-limited (bounded by group membership elsewhere)
        let group_id = GroupId::new();
        let scope = crate::presence::PresenceScope::Group(group_id);

        let payload = crate::presence::PresenceSubscribePayload { scope };
        let payload_bytes = payload.to_bytes();
        let envelope = EnvelopeBuilder::new(
            subscriber,
            state.local_id,
            crate::types::MessageType::PresenceSubscribe,
            payload_bytes,
        )
        .sign(&state.secret_seed);

        let before_len = state.subscriptions.len();

        // Call handle_presence_subscribe
        let _effects = state.handle_presence_subscribe(&envelope, true);

        // Check that subscription WAS stored (not rejected for size)
        let after_len = state.subscriptions.len();
        assert_eq!(before_len + 1, after_len, "group subscription should always be accepted");
    }
}
