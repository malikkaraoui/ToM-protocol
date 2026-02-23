use crate::backup::BackupCoordinator;
use crate::discovery::{EphemeralSubnetManager, HeartbeatTracker};
use crate::group::{GroupHub, GroupManager};
use crate::relay::{PeerRole, RelaySelector, Topology};
use crate::roles::RoleManager;
use crate::router::Router;
use crate::tracker::MessageTracker;
use crate::types::NodeId;

use super::effect::RuntimeEffect;
use super::RuntimeConfig;

/// Etat complet du protocole — logique pure, zero async, zero reseau.
///
/// Chaque methode handle_* / tick_* retourne Vec<RuntimeEffect>.
/// Aucune methode ne touche au reseau ni aux channels.
#[allow(dead_code)] // Fields used by handle_*/tick_* methods (Tasks 5-10)
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
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::RuntimeConfig;

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
}
