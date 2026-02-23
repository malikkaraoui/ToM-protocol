use crate::backup::BackupCoordinator;
use crate::discovery::{
    DiscoveryEvent, EphemeralSubnetManager, HeartbeatTracker, SubnetEvent,
};
use crate::envelope::EnvelopeBuilder;
use crate::group::{GroupHub, GroupManager};
use crate::relay::{PeerRole, RelaySelector, Topology};
use crate::roles::RoleManager;
use crate::router::Router;
use crate::tracker::MessageTracker;
use crate::types::{MessageType, NodeId};

use super::effect::RuntimeEffect;
use super::{ProtocolEvent, RuntimeConfig};

/// Etat complet du protocole — logique pure, zero async, zero reseau.
///
/// Chaque methode handle_* / tick_* retourne Vec<RuntimeEffect>.
/// Aucune methode ne touche au reseau ni aux channels.
#[allow(dead_code)] // Fields used by handle_*/tick_* methods (Tasks 6-10)
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
}
