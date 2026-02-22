/// Relay selection for ToM protocol.
///
/// Chooses the best relay node based on network topology: role,
/// online status, and last-seen timestamp.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::NodeId;

/// Maximum relay depth for path selection.
pub const MAX_RELAY_DEPTH: usize = 4;

// ── Peer topology info ─────────────────────────────────────────────────

/// Role a node plays in the network (assigned dynamically).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerRole {
    /// Regular participant — sends/receives messages.
    Peer,
    /// Relay-capable — can forward messages for others.
    Relay,
}

/// Current status of a known peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    Online,
    Offline,
    /// Recently seen but may be transitioning.
    Stale,
}

/// Information about a known peer in the network topology.
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub node_id: NodeId,
    pub role: PeerRole,
    pub status: PeerStatus,
    /// Unix ms timestamp of last observed activity.
    pub last_seen: u64,
}

// ── Network topology ───────────────────────────────────────────────────

/// Snapshot of known network topology — peers and their roles/status.
///
/// Updated by the discovery layer (gossip). The RelaySelector reads
/// this to make routing decisions.
#[derive(Debug, Default)]
pub struct Topology {
    peers: HashMap<NodeId, PeerInfo>,
}

impl Topology {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update a peer.
    pub fn upsert(&mut self, info: PeerInfo) {
        self.peers.insert(info.node_id, info);
    }

    /// Remove a peer.
    pub fn remove(&mut self, node_id: &NodeId) {
        self.peers.remove(node_id);
    }

    /// Get info for a specific peer.
    pub fn get(&self, node_id: &NodeId) -> Option<&PeerInfo> {
        self.peers.get(node_id)
    }

    /// Get mutable info for a specific peer (used by RoleManager to update roles).
    pub fn get_mut(&mut self, node_id: &NodeId) -> Option<&mut PeerInfo> {
        self.peers.get_mut(node_id)
    }

    /// All known peers.
    pub fn peers(&self) -> impl Iterator<Item = &PeerInfo> {
        self.peers.values()
    }

    /// Number of known peers.
    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// All online relay-capable peers, sorted by most recently seen.
    pub fn online_relays(&self) -> Vec<&PeerInfo> {
        let mut relays: Vec<&PeerInfo> = self
            .peers
            .values()
            .filter(|p| p.role == PeerRole::Relay && p.status == PeerStatus::Online)
            .collect();
        relays.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
        relays
    }
}

// ── Relay selection ────────────────────────────────────────────────────

/// Why a particular relay was selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionReason {
    /// Most recently seen online relay.
    MostRecent,
    /// Only available relay.
    OnlyOption,
    /// Alternate after primary failed.
    Alternate,
    /// No relay available.
    NoRelayAvailable,
}

/// Result of relay selection.
#[derive(Debug)]
pub struct RelaySelection {
    pub relay_id: Option<NodeId>,
    pub reason: SelectionReason,
}

/// Selects the best relay for message routing.
///
/// Pure logic — reads topology, returns a selection. No I/O.
pub struct RelaySelector {
    self_id: NodeId,
}

impl RelaySelector {
    pub fn new(self_id: NodeId) -> Self {
        Self { self_id }
    }

    /// Select the best relay to reach `target`.
    ///
    /// Filters: must be a relay, must be online, must not be self or target.
    /// Prefers the most recently seen relay.
    pub fn select_best(
        &self,
        target: NodeId,
        topology: &Topology,
    ) -> RelaySelection {
        self.select_best_excluding(target, topology, &[])
    }

    /// Select the best relay, excluding specific nodes (e.g., failed relays).
    pub fn select_best_excluding(
        &self,
        target: NodeId,
        topology: &Topology,
        exclude: &[NodeId],
    ) -> RelaySelection {
        let candidates: Vec<&PeerInfo> = topology
            .online_relays()
            .into_iter()
            .filter(|p| {
                p.node_id != self.self_id
                    && p.node_id != target
                    && !exclude.contains(&p.node_id)
            })
            .collect();

        match candidates.len() {
            0 => RelaySelection {
                relay_id: None,
                reason: SelectionReason::NoRelayAvailable,
            },
            1 => RelaySelection {
                relay_id: Some(candidates[0].node_id),
                reason: SelectionReason::OnlyOption,
            },
            _ => RelaySelection {
                relay_id: Some(candidates[0].node_id), // Already sorted by last_seen desc
                reason: SelectionReason::MostRecent,
            },
        }
    }

    /// Select an alternate relay after a failure.
    pub fn select_alternate(
        &self,
        target: NodeId,
        topology: &Topology,
        failed: &[NodeId],
    ) -> RelaySelection {
        let selection = self.select_best_excluding(target, topology, failed);
        if selection.relay_id.is_some() {
            RelaySelection {
                relay_id: selection.relay_id,
                reason: SelectionReason::Alternate,
            }
        } else {
            selection
        }
    }

    /// Build a multi-hop relay path to reach `target` (BFS through relay nodes).
    ///
    /// Returns a `via` chain of relay NodeIds, capped at `MAX_RELAY_DEPTH`.
    /// For the PoC, returns a single-hop path (one relay).
    pub fn select_path(
        &self,
        target: NodeId,
        topology: &Topology,
    ) -> Vec<NodeId> {
        // PoC: single-hop relay selection
        match self.select_best(target, topology).relay_id {
            Some(relay) => vec![relay],
            None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    fn make_relay(seed: u8, last_seen: u64) -> PeerInfo {
        PeerInfo {
            node_id: node_id(seed),
            role: PeerRole::Relay,
            status: PeerStatus::Online,
            last_seen,
        }
    }

    fn make_peer(seed: u8) -> PeerInfo {
        PeerInfo {
            node_id: node_id(seed),
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 1708000000000,
        }
    }

    // ── Topology tests ─────────────────────────────────────────────────

    #[test]
    fn topology_upsert_and_get() {
        let mut topo = Topology::new();
        let info = make_relay(1, 1000);
        let id = info.node_id;

        topo.upsert(info);
        assert_eq!(topo.len(), 1);
        assert!(topo.get(&id).is_some());
    }

    #[test]
    fn topology_remove() {
        let mut topo = Topology::new();
        let info = make_relay(1, 1000);
        let id = info.node_id;

        topo.upsert(info);
        topo.remove(&id);
        assert!(topo.is_empty());
    }

    #[test]
    fn online_relays_sorted_by_last_seen() {
        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 1000)); // oldest
        topo.upsert(make_relay(2, 3000)); // newest
        topo.upsert(make_relay(3, 2000)); // middle

        let relays = topo.online_relays();
        assert_eq!(relays.len(), 3);
        assert_eq!(relays[0].last_seen, 3000);
        assert_eq!(relays[1].last_seen, 2000);
        assert_eq!(relays[2].last_seen, 1000);
    }

    #[test]
    fn online_relays_excludes_peers_and_offline() {
        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 1000)); // online relay
        topo.upsert(make_peer(2));         // peer, not relay
        topo.upsert(PeerInfo {
            node_id: node_id(3),
            role: PeerRole::Relay,
            status: PeerStatus::Offline,
            last_seen: 5000,
        }); // offline relay

        let relays = topo.online_relays();
        assert_eq!(relays.len(), 1);
        assert_eq!(relays[0].node_id, node_id(1));
    }

    // ── Selection tests ────────────────────────────────────────────────

    #[test]
    fn select_best_picks_most_recent() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 1000));
        topo.upsert(make_relay(2, 3000)); // most recent
        topo.upsert(make_relay(3, 2000));

        let result = selector.select_best(target, &topo);
        assert_eq!(result.relay_id, Some(node_id(2)));
        assert_eq!(result.reason, SelectionReason::MostRecent);
    }

    #[test]
    fn select_best_excludes_self_and_target() {
        let me = node_id(1);
        let target = node_id(2);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 5000)); // self — excluded
        topo.upsert(make_relay(2, 4000)); // target — excluded
        topo.upsert(make_relay(3, 3000)); // valid

        let result = selector.select_best(target, &topo);
        assert_eq!(result.relay_id, Some(node_id(3)));
    }

    #[test]
    fn select_best_no_relays_available() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);
        let topo = Topology::new();

        let result = selector.select_best(target, &topo);
        assert_eq!(result.relay_id, None);
        assert_eq!(result.reason, SelectionReason::NoRelayAvailable);
    }

    #[test]
    fn select_best_only_option() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 1000));

        let result = selector.select_best(target, &topo);
        assert_eq!(result.relay_id, Some(node_id(1)));
        assert_eq!(result.reason, SelectionReason::OnlyOption);
    }

    #[test]
    fn select_best_excluding_failed() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 3000)); // best but failed
        topo.upsert(make_relay(2, 2000)); // fallback

        let failed = vec![node_id(1)];
        let result = selector.select_best_excluding(target, &topo, &failed);
        assert_eq!(result.relay_id, Some(node_id(2)));
    }

    #[test]
    fn select_alternate() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 3000));
        topo.upsert(make_relay(2, 2000));

        let result = selector.select_alternate(target, &topo, &[node_id(1)]);
        assert_eq!(result.relay_id, Some(node_id(2)));
        assert_eq!(result.reason, SelectionReason::Alternate);
    }

    #[test]
    fn select_path_single_hop() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);

        let mut topo = Topology::new();
        topo.upsert(make_relay(1, 3000));

        let path = selector.select_path(target, &topo);
        assert_eq!(path, vec![node_id(1)]);
    }

    #[test]
    fn select_path_no_relay() {
        let me = node_id(100);
        let target = node_id(200);
        let selector = RelaySelector::new(me);
        let topo = Topology::new();

        let path = selector.select_path(target, &topo);
        assert!(path.is_empty());
    }

    #[test]
    fn topology_upsert_updates_existing() {
        let mut topo = Topology::new();
        let id = node_id(1);

        topo.upsert(PeerInfo {
            node_id: id,
            role: PeerRole::Relay,
            status: PeerStatus::Online,
            last_seen: 1000,
        });

        topo.upsert(PeerInfo {
            node_id: id,
            role: PeerRole::Relay,
            status: PeerStatus::Offline,
            last_seen: 2000,
        });

        assert_eq!(topo.len(), 1);
        assert_eq!(topo.get(&id).unwrap().status, PeerStatus::Offline);
        assert_eq!(topo.get(&id).unwrap().last_seen, 2000);
    }
}
