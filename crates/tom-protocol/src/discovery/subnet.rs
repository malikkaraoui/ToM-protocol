/// EphemeralSubnet — self-organizing clusters based on communication patterns.
///
/// Pure state machine: record communications, evaluate periodically.
/// No I/O — caller drives the timer and handles events.
///
/// Algorithm: BFS clustering over a weighted communication graph.
/// Edges decay linearly over time, subnets dissolve on inactivity.
use std::collections::{HashMap, HashSet, VecDeque};

use crate::types::NodeId;

// ── Constants ────────────────────────────────────────────────────────────

/// Minimum messages between two nodes to consider them connected.
pub const MIN_EDGE_WEIGHT: u32 = 3;

/// Minimum cluster size to form a subnet.
pub const MIN_SUBNET_SIZE: usize = 3;

/// Maximum cluster size (BFS stops here).
pub const MAX_SUBNET_SIZE: usize = 10;

/// Inactivity timeout — dissolve subnet after 5 minutes of silence.
pub const INACTIVITY_TIMEOUT_MS: u64 = 5 * 60 * 1000;

/// Edge decay starts after this age (10 minutes).
pub const EDGE_DECAY_MS: u64 = 10 * 60 * 1000;

/// How often to run evaluation (30 seconds). Caller uses this as interval.
pub const EVALUATION_INTERVAL_MS: u64 = 30_000;

// ── Types ────────────────────────────────────────────────────────────────

/// A communication edge between two nodes.
#[derive(Debug, Clone)]
pub struct CommunicationEdge {
    pub from: NodeId,
    pub to: NodeId,
    pub message_count: u32,
    pub last_seen: u64,
}

/// An ephemeral subnet — a cluster of nodes that communicate frequently.
#[derive(Debug, Clone)]
pub struct SubnetInfo {
    pub subnet_id: String,
    pub members: HashSet<NodeId>,
    pub formed_at: u64,
    pub last_activity: u64,
    pub density_score: f64,
}

impl SubnetInfo {
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

/// Events emitted by the subnet manager.
#[derive(Debug, Clone)]
pub enum SubnetEvent {
    SubnetFormed { subnet: SubnetInfo },
    SubnetDissolved { subnet_id: String, reason: DissolveReason },
    NodeJoinedSubnet { subnet_id: String, node_id: NodeId },
    NodeLeftSubnet { subnet_id: String, node_id: NodeId },
}

/// Why a subnet was dissolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DissolveReason {
    Inactive,
    InsufficientMembers,
}

// ── Manager ──────────────────────────────────────────────────────────────

/// Manages ephemeral subnets based on communication patterns.
///
/// Pure state machine — caller drives evaluation and handles events.
pub struct EphemeralSubnetManager {
    /// Local node identity (used by higher-level orchestration).
    #[allow(dead_code)]
    self_node_id: NodeId,
    /// Communication graph: edge_key → edge.
    edges: HashMap<String, CommunicationEdge>,
    /// Active subnets: subnet_id → info.
    subnets: HashMap<String, SubnetInfo>,
    /// Node → subnet mapping (one subnet per node).
    node_subnets: HashMap<NodeId, String>,
    /// Counter for deterministic subnet IDs.
    next_subnet_seq: u64,
}

impl EphemeralSubnetManager {
    /// Create a new subnet manager.
    pub fn new(self_node_id: NodeId) -> Self {
        Self {
            self_node_id,
            edges: HashMap::new(),
            subnets: HashMap::new(),
            node_subnets: HashMap::new(),
            next_subnet_seq: 0,
        }
    }

    /// Record a communication between two nodes.
    pub fn record_communication(&mut self, from: NodeId, to: NodeId, now: u64) {
        let key = edge_key(&from, &to);

        let edge = self.edges.entry(key).or_insert_with(|| CommunicationEdge {
            from: std::cmp::min_by(from, to, |a, b| a.to_string().cmp(&b.to_string())),
            to: std::cmp::max_by(from, to, |a, b| a.to_string().cmp(&b.to_string())),
            message_count: 0,
            last_seen: now,
        });

        edge.message_count = edge.message_count.saturating_add(1);
        edge.last_seen = now;

        // Update subnet activity if both nodes in same subnet
        if let Some(subnet_id) = self.node_subnets.get(&from) {
            if self.node_subnets.get(&to) == Some(subnet_id) {
                if let Some(subnet) = self.subnets.get_mut(subnet_id) {
                    subnet.last_activity = now;
                }
            }
        }
    }

    /// Run a full evaluation cycle. Returns events for the caller.
    pub fn evaluate(&mut self, now: u64) -> Vec<SubnetEvent> {
        let mut events = vec![];

        // 1. Decay old edges
        self.decay_edges(now);

        // 2. Dissolve inactive/undersize subnets — collect freed nodes
        let (dissolve_events, dissolved_nodes) = self.dissolve_inactive_subnets(now);
        events.extend(dissolve_events);

        // 3. Form new subnets via BFS (skip recently dissolved nodes)
        events.extend(self.form_new_subnets(now, &dissolved_nodes));

        events
    }

    /// Remove a node from tracking.
    pub fn remove_node(&mut self, node_id: &NodeId) -> Vec<SubnetEvent> {
        let mut events = vec![];

        // Remove from subnet
        if let Some(subnet_id) = self.node_subnets.remove(node_id) {
            events.push(SubnetEvent::NodeLeftSubnet {
                subnet_id: subnet_id.clone(),
                node_id: *node_id,
            });

            if let Some(subnet) = self.subnets.get_mut(&subnet_id) {
                subnet.members.remove(node_id);

                if subnet.members.len() < MIN_SUBNET_SIZE {
                    // Dissolve undersize subnet
                    let members: Vec<NodeId> = subnet.members.iter().copied().collect();
                    for member in &members {
                        self.node_subnets.remove(member);
                        events.push(SubnetEvent::NodeLeftSubnet {
                            subnet_id: subnet_id.clone(),
                            node_id: *member,
                        });
                    }
                    self.subnets.remove(&subnet_id);
                    events.push(SubnetEvent::SubnetDissolved {
                        subnet_id,
                        reason: DissolveReason::InsufficientMembers,
                    });
                }
            }
        }

        // Remove edges involving this node
        let node_str = node_id.to_string();
        self.edges.retain(|key, _| !key.contains(&node_str));

        events
    }

    /// Get the subnet a node belongs to.
    pub fn get_node_subnet(&self, node_id: &NodeId) -> Option<&SubnetInfo> {
        let subnet_id = self.node_subnets.get(node_id)?;
        self.subnets.get(subnet_id)
    }

    /// Check if two nodes are in the same subnet.
    pub fn are_in_same_subnet(&self, a: &NodeId, b: &NodeId) -> bool {
        match (self.node_subnets.get(a), self.node_subnets.get(b)) {
            (Some(sa), Some(sb)) => sa == sb,
            _ => false,
        }
    }

    /// Get all active subnets.
    pub fn all_subnets(&self) -> Vec<&SubnetInfo> {
        self.subnets.values().collect()
    }

    /// Number of active subnets.
    pub fn subnet_count(&self) -> usize {
        self.subnets.len()
    }

    /// Number of tracked edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Number of nodes currently in subnets.
    pub fn nodes_in_subnets(&self) -> usize {
        self.node_subnets.len()
    }

    // ── Internal ─────────────────────────────────────────────────────────

    /// Decay edges that are older than EDGE_DECAY_MS.
    fn decay_edges(&mut self, now: u64) {
        self.edges.retain(|_, edge| {
            let age = now.saturating_sub(edge.last_seen);
            if age <= EDGE_DECAY_MS {
                return true; // Not old enough to decay
            }

            // Linear decay: factor = max(0, 1 - (age/EDGE_DECAY_MS - 1))
            let ratio = age as f64 / EDGE_DECAY_MS as f64;
            let factor = (2.0 - ratio).max(0.0);
            edge.message_count = (edge.message_count as f64 * factor) as u32;

            edge.message_count > 0
        });
    }

    /// Dissolve subnets that are inactive or undersize.
    /// Returns events and the set of nodes freed (to suppress immediate re-clustering).
    fn dissolve_inactive_subnets(&mut self, now: u64) -> (Vec<SubnetEvent>, HashSet<NodeId>) {
        let mut events = vec![];
        let mut dissolved_nodes = HashSet::new();
        let mut to_dissolve = vec![];

        for (id, subnet) in &self.subnets {
            if subnet.members.len() < MIN_SUBNET_SIZE {
                to_dissolve.push((id.clone(), DissolveReason::InsufficientMembers));
            } else if now.saturating_sub(subnet.last_activity) > INACTIVITY_TIMEOUT_MS {
                to_dissolve.push((id.clone(), DissolveReason::Inactive));
            }
        }

        for (subnet_id, reason) in to_dissolve {
            if let Some(subnet) = self.subnets.remove(&subnet_id) {
                for member in &subnet.members {
                    self.node_subnets.remove(member);
                    dissolved_nodes.insert(*member);
                    events.push(SubnetEvent::NodeLeftSubnet {
                        subnet_id: subnet_id.clone(),
                        node_id: *member,
                    });
                }
                events.push(SubnetEvent::SubnetDissolved { subnet_id, reason });
            }
        }

        (events, dissolved_nodes)
    }

    /// Form new subnets via BFS clustering on the communication graph.
    /// `skip_nodes` — nodes recently dissolved, suppress immediate re-formation.
    fn form_new_subnets(&mut self, now: u64, skip_nodes: &HashSet<NodeId>) -> Vec<SubnetEvent> {
        let mut events = vec![];

        // Build adjacency list from strong edges
        let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for edge in self.edges.values() {
            if edge.message_count >= MIN_EDGE_WEIGHT {
                adjacency.entry(edge.from).or_default().push(edge.to);
                adjacency.entry(edge.to).or_default().push(edge.from);
            }
        }

        // BFS from each unvisited, unsubnetted node
        let mut visited: HashSet<NodeId> = HashSet::new();
        let all_nodes: Vec<NodeId> = adjacency.keys().copied().collect();

        for start in all_nodes {
            if visited.contains(&start)
                || self.node_subnets.contains_key(&start)
                || skip_nodes.contains(&start)
            {
                continue;
            }

            // BFS
            let mut cluster: Vec<NodeId> = vec![];
            let mut queue: VecDeque<NodeId> = VecDeque::new();
            queue.push_back(start);
            visited.insert(start);

            while let Some(node) = queue.pop_front() {
                if cluster.len() >= MAX_SUBNET_SIZE {
                    break;
                }
                cluster.push(node);

                if let Some(neighbors) = adjacency.get(&node) {
                    for &neighbor in neighbors {
                        if !visited.contains(&neighbor)
                            && !self.node_subnets.contains_key(&neighbor)
                            && !skip_nodes.contains(&neighbor)
                            && cluster.len() < MAX_SUBNET_SIZE
                        {
                            visited.insert(neighbor);
                            queue.push_back(neighbor);
                        }
                    }
                }
            }

            // Only form subnet if large enough
            if cluster.len() >= MIN_SUBNET_SIZE {
                let subnet_id = self.generate_subnet_id();
                let members: HashSet<NodeId> = cluster.iter().copied().collect();
                let density = self.calculate_density(&members);

                let subnet = SubnetInfo {
                    subnet_id: subnet_id.clone(),
                    members: members.clone(),
                    formed_at: now,
                    last_activity: now,
                    density_score: density,
                };

                for &member in &members {
                    self.node_subnets.insert(member, subnet_id.clone());
                    events.push(SubnetEvent::NodeJoinedSubnet {
                        subnet_id: subnet_id.clone(),
                        node_id: member,
                    });
                }

                events.push(SubnetEvent::SubnetFormed {
                    subnet: subnet.clone(),
                });
                self.subnets.insert(subnet_id, subnet);
            }
        }

        events
    }

    /// Calculate density score for a set of nodes.
    /// density = sum(edge_weights) / (n*(n-1)/2)
    fn calculate_density(&self, members: &HashSet<NodeId>) -> f64 {
        let n = members.len();
        if n < 2 {
            return 0.0;
        }

        let potential = n * (n - 1) / 2;
        let mut total_weight: u32 = 0;

        for edge in self.edges.values() {
            if members.contains(&edge.from) && members.contains(&edge.to) {
                total_weight = total_weight.saturating_add(edge.message_count);
            }
        }

        total_weight as f64 / potential as f64
    }

    fn generate_subnet_id(&mut self) -> String {
        self.next_subnet_seq += 1;
        format!("subnet-{}", self.next_subnet_seq)
    }
}

/// Create a canonical edge key (sorted by string representation).
fn edge_key(a: &NodeId, b: &NodeId) -> String {
    let a_str = a.to_string();
    let b_str = b.to_string();
    if a_str <= b_str {
        format!("{}:{}", a_str, b_str)
    } else {
        format!("{}:{}", b_str, a_str)
    }
}

fn _now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before epoch")
        .as_millis() as u64
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

    /// Helper: add N communications between two nodes.
    fn communicate(mgr: &mut EphemeralSubnetManager, a: NodeId, b: NodeId, count: u32, now: u64) {
        for _ in 0..count {
            mgr.record_communication(a, b, now);
        }
    }

    #[test]
    fn record_communication_creates_edge() {
        let alice = node_id(1);
        let bob = node_id(2);
        let mut mgr = EphemeralSubnetManager::new(alice);

        mgr.record_communication(alice, bob, 1000);
        assert_eq!(mgr.edge_count(), 1);

        mgr.record_communication(alice, bob, 1001);
        assert_eq!(mgr.edge_count(), 1); // Same edge, incremented
    }

    #[test]
    fn edge_key_is_canonical() {
        let a = node_id(1);
        let b = node_id(2);
        assert_eq!(edge_key(&a, &b), edge_key(&b, &a));
    }

    #[test]
    fn no_subnet_below_threshold() {
        let me = node_id(0);
        let a = node_id(1);
        let b = node_id(2);
        let c = node_id(3);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        // Only 2 messages each — below MIN_EDGE_WEIGHT (3)
        communicate(&mut mgr, a, b, 2, now);
        communicate(&mut mgr, b, c, 2, now);
        communicate(&mut mgr, a, c, 2, now);

        let events = mgr.evaluate(now);
        assert_eq!(mgr.subnet_count(), 0);
        assert!(events.is_empty());
    }

    #[test]
    fn form_subnet_with_strong_edges() {
        let me = node_id(0);
        let a = node_id(1);
        let b = node_id(2);
        let c = node_id(3);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        // Strong edges (>= MIN_EDGE_WEIGHT=3)
        communicate(&mut mgr, a, b, 5, now);
        communicate(&mut mgr, b, c, 5, now);
        communicate(&mut mgr, a, c, 5, now);

        let events = mgr.evaluate(now);
        assert_eq!(mgr.subnet_count(), 1);
        assert!(mgr.are_in_same_subnet(&a, &b));
        assert!(mgr.are_in_same_subnet(&b, &c));

        // Check events
        let formed = events.iter().filter(|e| matches!(e, SubnetEvent::SubnetFormed { .. })).count();
        let joined = events.iter().filter(|e| matches!(e, SubnetEvent::NodeJoinedSubnet { .. })).count();
        assert_eq!(formed, 1);
        assert_eq!(joined, 3); // a, b, c
    }

    #[test]
    fn subnet_respects_max_size() {
        let me = node_id(0);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        // Create a chain of 15 nodes: 1-2-3-...-15
        let nodes: Vec<NodeId> = (1..=15).map(node_id).collect();
        for i in 0..nodes.len() - 1 {
            communicate(&mut mgr, nodes[i], nodes[i + 1], 5, now);
        }

        mgr.evaluate(now);

        // No single subnet should exceed MAX_SUBNET_SIZE
        for subnet in mgr.all_subnets() {
            assert!(subnet.member_count() <= MAX_SUBNET_SIZE);
        }
    }

    #[test]
    fn dissolve_on_inactivity() {
        let me = node_id(0);
        let a = node_id(1);
        let b = node_id(2);
        let c = node_id(3);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        communicate(&mut mgr, a, b, 5, now);
        communicate(&mut mgr, b, c, 5, now);
        communicate(&mut mgr, a, c, 5, now);

        mgr.evaluate(now);
        assert_eq!(mgr.subnet_count(), 1);

        // Fast-forward past inactivity timeout
        let later = now + INACTIVITY_TIMEOUT_MS + 1;
        let events = mgr.evaluate(later);

        assert_eq!(mgr.subnet_count(), 0);
        let dissolved = events.iter().filter(|e| matches!(e, SubnetEvent::SubnetDissolved { .. })).count();
        assert_eq!(dissolved, 1);
    }

    #[test]
    fn activity_prevents_dissolution() {
        let me = node_id(0);
        let a = node_id(1);
        let b = node_id(2);
        let c = node_id(3);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        communicate(&mut mgr, a, b, 5, now);
        communicate(&mut mgr, b, c, 5, now);
        communicate(&mut mgr, a, c, 5, now);

        mgr.evaluate(now);
        assert_eq!(mgr.subnet_count(), 1);

        // Record activity before timeout
        let mid = now + INACTIVITY_TIMEOUT_MS / 2;
        mgr.record_communication(a, b, mid);

        // Evaluate at what would be timeout from original formation
        let events = mgr.evaluate(now + INACTIVITY_TIMEOUT_MS + 1);

        // Still alive because activity refreshed
        assert_eq!(mgr.subnet_count(), 1);
        let dissolved = events.iter().filter(|e| matches!(e, SubnetEvent::SubnetDissolved { .. })).count();
        assert_eq!(dissolved, 0);
    }

    #[test]
    fn edge_decay_reduces_weight() {
        let me = node_id(0);
        let a = node_id(1);
        let b = node_id(2);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        communicate(&mut mgr, a, b, 10, now);
        assert_eq!(mgr.edge_count(), 1);

        // Evaluate way past decay — edge should be removed
        let far_future = now + EDGE_DECAY_MS * 3;
        mgr.evaluate(far_future);

        assert_eq!(mgr.edge_count(), 0);
    }

    #[test]
    fn remove_node_dissolves_undersize_subnet() {
        let me = node_id(0);
        let a = node_id(1);
        let b = node_id(2);
        let c = node_id(3);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        communicate(&mut mgr, a, b, 5, now);
        communicate(&mut mgr, b, c, 5, now);
        communicate(&mut mgr, a, c, 5, now);

        mgr.evaluate(now);
        assert_eq!(mgr.subnet_count(), 1);

        // Remove one member → subnet drops below MIN_SUBNET_SIZE (3→2)
        let events = mgr.remove_node(&a);
        assert_eq!(mgr.subnet_count(), 0);

        let dissolved = events.iter().filter(|e| matches!(e, SubnetEvent::SubnetDissolved { .. })).count();
        assert_eq!(dissolved, 1);
    }

    #[test]
    fn get_node_subnet_returns_info() {
        let me = node_id(0);
        let a = node_id(1);
        let b = node_id(2);
        let c = node_id(3);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        assert!(mgr.get_node_subnet(&a).is_none());

        communicate(&mut mgr, a, b, 5, now);
        communicate(&mut mgr, b, c, 5, now);
        communicate(&mut mgr, a, c, 5, now);
        mgr.evaluate(now);

        let subnet = mgr.get_node_subnet(&a).expect("should be in subnet");
        assert!(subnet.members.contains(&a));
        assert!(subnet.members.contains(&b));
        assert!(subnet.members.contains(&c));
        assert!(subnet.density_score > 0.0);
    }

    #[test]
    fn density_score_calculation() {
        let me = node_id(0);
        let a = node_id(1);
        let b = node_id(2);
        let c = node_id(3);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        // 3 nodes, all pairs with 5 messages each → density = 15 / 3 = 5.0
        communicate(&mut mgr, a, b, 5, now);
        communicate(&mut mgr, b, c, 5, now);
        communicate(&mut mgr, a, c, 5, now);
        mgr.evaluate(now);

        let subnet = mgr.get_node_subnet(&a).unwrap();
        // 3 nodes → 3 potential edges, each has weight 5 → density = 15/3 = 5.0
        assert!((subnet.density_score - 5.0).abs() < 0.01);
    }

    #[test]
    fn two_separate_clusters() {
        let me = node_id(0);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        // Cluster 1: nodes 1,2,3
        communicate(&mut mgr, node_id(1), node_id(2), 5, now);
        communicate(&mut mgr, node_id(2), node_id(3), 5, now);
        communicate(&mut mgr, node_id(1), node_id(3), 5, now);

        // Cluster 2: nodes 10,11,12 (no connection to cluster 1)
        communicate(&mut mgr, node_id(10), node_id(11), 5, now);
        communicate(&mut mgr, node_id(11), node_id(12), 5, now);
        communicate(&mut mgr, node_id(10), node_id(12), 5, now);

        mgr.evaluate(now);

        assert_eq!(mgr.subnet_count(), 2);
        assert!(mgr.are_in_same_subnet(&node_id(1), &node_id(2)));
        assert!(mgr.are_in_same_subnet(&node_id(10), &node_id(11)));
        assert!(!mgr.are_in_same_subnet(&node_id(1), &node_id(10)));
    }

    #[test]
    fn already_subnetted_nodes_not_reclustered() {
        let me = node_id(0);
        let a = node_id(1);
        let b = node_id(2);
        let c = node_id(3);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        communicate(&mut mgr, a, b, 5, now);
        communicate(&mut mgr, b, c, 5, now);
        communicate(&mut mgr, a, c, 5, now);

        mgr.evaluate(now);
        let subnet_id_1 = mgr.get_node_subnet(&a).unwrap().subnet_id.clone();

        // Evaluate again — should not create duplicate subnet
        let events = mgr.evaluate(now);
        assert_eq!(mgr.subnet_count(), 1);
        assert_eq!(
            mgr.get_node_subnet(&a).unwrap().subnet_id,
            subnet_id_1
        );
        let formed = events.iter().filter(|e| matches!(e, SubnetEvent::SubnetFormed { .. })).count();
        assert_eq!(formed, 0);
    }

    #[test]
    fn stats() {
        let me = node_id(0);
        let mut mgr = EphemeralSubnetManager::new(me);
        let now = 10_000u64;

        assert_eq!(mgr.subnet_count(), 0);
        assert_eq!(mgr.edge_count(), 0);
        assert_eq!(mgr.nodes_in_subnets(), 0);

        communicate(&mut mgr, node_id(1), node_id(2), 5, now);
        communicate(&mut mgr, node_id(2), node_id(3), 5, now);
        communicate(&mut mgr, node_id(1), node_id(3), 5, now);

        assert_eq!(mgr.edge_count(), 3);

        mgr.evaluate(now);
        assert_eq!(mgr.subnet_count(), 1);
        assert_eq!(mgr.nodes_in_subnets(), 3);
    }
}
