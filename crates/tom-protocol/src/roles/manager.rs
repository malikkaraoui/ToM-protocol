/// Role manager — evaluates contribution scores and triggers role changes.
///
/// Periodically called by the runtime to check if any node should be
/// promoted (Peer → Relay) or demoted (Relay → Peer).
use std::collections::HashMap;

use crate::relay::{PeerRole, Topology};
use crate::types::NodeId;

use super::scoring::ContributionMetrics;

/// Score threshold for promotion to Relay.
const PROMOTION_THRESHOLD: f64 = 10.0;

/// Score threshold below which a Relay is demoted back to Peer.
const DEMOTION_THRESHOLD: f64 = 2.0;

/// Actions the runtime should execute after a role evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum RoleAction {
    /// A remote peer was promoted to Relay in topology.
    Promoted { node_id: NodeId, score: f64 },
    /// A remote peer was demoted to Peer in topology.
    Demoted { node_id: NodeId, score: f64 },
    /// Our local role changed — update gossip announces.
    LocalRoleChanged { new_role: PeerRole },
}

/// Manages contribution scores and role transitions.
pub struct RoleManager {
    local_id: NodeId,
    scores: HashMap<NodeId, ContributionMetrics>,
}

impl RoleManager {
    pub fn new(local_id: NodeId) -> Self {
        Self {
            local_id,
            scores: HashMap::new(),
        }
    }

    /// Record a successful relay by a node.
    pub fn record_relay(&mut self, node_id: NodeId, now: u64) {
        self.scores
            .entry(node_id)
            .or_insert_with(|| ContributionMetrics::new(now))
            .record_relay(now);
    }

    /// Record a relay failure for a node.
    pub fn record_relay_failure(&mut self, node_id: NodeId, now: u64) {
        self.scores
            .entry(node_id)
            .or_insert_with(|| ContributionMetrics::new(now))
            .record_relay_failure(now);
    }

    /// Get the current contribution score for a node.
    pub fn score(&self, node_id: &NodeId, now: u64) -> f64 {
        self.scores
            .get(node_id)
            .map(|m| m.score(now))
            .unwrap_or(0.0)
    }

    /// Record bytes relayed by a peer.
    pub fn record_bytes_relayed(&mut self, node_id: NodeId, bytes: u64, now: u64) {
        let metrics = self
            .scores
            .entry(node_id)
            .or_insert_with(|| ContributionMetrics::new(now));
        metrics.bytes_relayed += bytes;
        metrics.last_activity = now;
    }

    /// Record bytes received from network (for calculating give/take ratio).
    pub fn record_bytes_received(&mut self, node_id: NodeId, bytes: u64, now: u64) {
        let metrics = self
            .scores
            .entry(node_id)
            .or_insert_with(|| ContributionMetrics::new(now));
        metrics.bytes_received += bytes;
    }

    /// Remove all metrics for a departed node.
    pub fn remove_node(&mut self, node_id: &NodeId) {
        self.scores.remove(node_id);
    }

    /// Get complete metrics snapshot for a peer (debug/observability).
    pub fn get_metrics(
        &self,
        node_id: &NodeId,
        topology: &Topology,
        now: u64,
    ) -> Option<super::RoleMetrics> {
        let metrics = self.scores.get(node_id)?;
        let peer_info = topology.get(node_id)?;

        let total_attempts = metrics.messages_relayed + metrics.relay_failures;
        let success_rate = if total_attempts > 0 {
            metrics.messages_relayed as f64 / total_attempts as f64
        } else {
            1.0
        };

        let bandwidth_ratio = if metrics.bytes_received > 0 {
            metrics.bytes_relayed as f64 / metrics.bytes_received as f64
        } else if metrics.bytes_relayed > 0 {
            1.0
        } else {
            0.0
        };

        Some(super::RoleMetrics {
            node_id: *node_id,
            role: peer_info.role,
            score: self.score(node_id, now),
            relay_count: metrics.messages_relayed,
            relay_failures: metrics.relay_failures,
            success_rate,
            bytes_relayed: metrics.bytes_relayed,
            bytes_received: metrics.bytes_received,
            bandwidth_ratio,
            uptime_hours: metrics.total_uptime_ms as f64 / 3_600_000.0,
            first_seen: metrics.first_seen,
            last_activity: metrics.last_activity,
        })
    }

    /// Get all peers with their scores (debug/dashboard).
    pub fn get_all_scores(
        &self,
        topology: &Topology,
        now: u64,
    ) -> Vec<(NodeId, f64, PeerRole)> {
        topology
            .peers()
            .filter_map(|peer| {
                let score = self.score(&peer.node_id, now);
                Some((peer.node_id, score, peer.role))
            })
            .collect()
    }

    /// Evaluate all tracked nodes and update topology roles.
    ///
    /// Returns a list of actions (promotions, demotions, local role change).
    /// The runtime executes these actions and surfaces events to the application.
    pub fn evaluate(&self, topology: &mut Topology, now: u64) -> Vec<RoleAction> {
        let mut actions = Vec::new();

        for (node_id, metrics) in &self.scores {
            let score = metrics.score(now);
            let current_role = topology.get(node_id).map(|p| p.role);

            match current_role {
                Some(PeerRole::Peer) if score >= PROMOTION_THRESHOLD => {
                    // Promote: update topology role
                    if let Some(peer) = topology.get_mut(node_id) {
                        peer.role = PeerRole::Relay;
                    }
                    let action = if *node_id == self.local_id {
                        RoleAction::LocalRoleChanged {
                            new_role: PeerRole::Relay,
                        }
                    } else {
                        RoleAction::Promoted {
                            node_id: *node_id,
                            score,
                        }
                    };
                    actions.push(action);
                }
                Some(PeerRole::Relay) if score < DEMOTION_THRESHOLD => {
                    // Demote: update topology role
                    if let Some(peer) = topology.get_mut(node_id) {
                        peer.role = PeerRole::Peer;
                    }
                    let action = if *node_id == self.local_id {
                        RoleAction::LocalRoleChanged {
                            new_role: PeerRole::Peer,
                        }
                    } else {
                        RoleAction::Demoted {
                            node_id: *node_id,
                            score,
                        }
                    };
                    actions.push(action);
                }
                _ => {} // No change needed
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::{PeerInfo, PeerStatus};

    fn make_topology(nodes: &[(NodeId, PeerRole)]) -> Topology {
        let mut topo = Topology::new();
        for (id, role) in nodes {
            topo.upsert(PeerInfo {
                node_id: *id,
                role: *role,
                status: PeerStatus::Online,
                last_seen: 1000,
            });
        }
        topo
    }

    fn test_node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn new_manager_empty() {
        let id = test_node_id(1);
        let mgr = RoleManager::new(id);
        assert_eq!(mgr.score(&id, 1000), 0.0);
    }

    #[test]
    fn record_relay_builds_score() {
        let local = test_node_id(1);
        let relay_node = test_node_id(2);
        let mut mgr = RoleManager::new(local);

        for i in 0..20 {
            mgr.record_relay(relay_node, 1000 + i * 1000);
        }

        let score = mgr.score(&relay_node, 20_000);
        assert!(score > 10.0, "20 relays should exceed promotion threshold, got {score}");
    }

    #[test]
    fn promote_on_threshold() {
        let local = test_node_id(1);
        let relay_node = test_node_id(2);
        let mut mgr = RoleManager::new(local);
        let mut topo = make_topology(&[(relay_node, PeerRole::Peer)]);

        // Build up relay count past promotion threshold
        for i in 0..20 {
            mgr.record_relay(relay_node, 1000 + i * 1000);
        }

        let actions = mgr.evaluate(&mut topo, 20_000);
        assert!(
            actions.iter().any(|a| matches!(a, RoleAction::Promoted { node_id, .. } if *node_id == relay_node)),
            "should promote node with high score: {actions:?}"
        );
        assert_eq!(topo.get(&relay_node).unwrap().role, PeerRole::Relay);
    }

    #[test]
    fn demote_on_low_score() {
        let local = test_node_id(1);
        let relay_node = test_node_id(2);
        let mut mgr = RoleManager::new(local);
        let mut topo = make_topology(&[(relay_node, PeerRole::Relay)]);

        // Only 1 relay, then let it decay heavily (50 hours)
        mgr.record_relay(relay_node, 1000);
        let now = 1000 + 50 * 3_600_000;

        let actions = mgr.evaluate(&mut topo, now);
        assert!(
            actions.iter().any(|a| matches!(a, RoleAction::Demoted { node_id, .. } if *node_id == relay_node)),
            "should demote idle relay: score={}, actions={actions:?}",
            mgr.score(&relay_node, now)
        );
        assert_eq!(topo.get(&relay_node).unwrap().role, PeerRole::Peer);
    }

    #[test]
    fn no_action_in_between() {
        let local = test_node_id(1);
        let node = test_node_id(2);
        let mut mgr = RoleManager::new(local);
        let mut topo = make_topology(&[(node, PeerRole::Peer)]);

        // Score between demotion and promotion thresholds (~5-6 score)
        for i in 0..3 {
            mgr.record_relay(node, 1000 + i * 1000);
        }

        let score = mgr.score(&node, 4000);
        let actions = mgr.evaluate(&mut topo, 4000);
        assert!(actions.is_empty(), "mid-range score ({score}) should not trigger action: {actions:?}");
    }

    #[test]
    fn local_role_change_detected() {
        let local = test_node_id(1);
        let mut mgr = RoleManager::new(local);
        let mut topo = make_topology(&[(local, PeerRole::Peer)]);

        for i in 0..20 {
            mgr.record_relay(local, 1000 + i * 1000);
        }

        let actions = mgr.evaluate(&mut topo, 20_000);
        assert!(
            actions.iter().any(|a| matches!(a, RoleAction::LocalRoleChanged { new_role: PeerRole::Relay })),
            "should detect local promotion: {actions:?}"
        );
    }

    #[test]
    fn remove_node_clears_metrics() {
        let local = test_node_id(1);
        let node = test_node_id(2);
        let mut mgr = RoleManager::new(local);

        mgr.record_relay(node, 1000);
        assert!(mgr.score(&node, 1000) > 0.0);

        mgr.remove_node(&node);
        assert_eq!(mgr.score(&node, 1000), 0.0);
    }
}
