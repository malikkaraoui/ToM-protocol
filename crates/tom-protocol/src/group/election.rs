/// Deterministic hub election for group failover.
///
/// When a hub fails (missed heartbeats), all members independently
/// run the same election algorithm and arrive at the same winner.
/// No consensus protocol needed — determinism prevents split-brain.
use crate::group::types::*;
use crate::relay::{PeerRole, PeerStatus, Topology};
use crate::types::NodeId;

/// Result of a hub election.
#[derive(Debug, Clone)]
pub struct ElectionResult {
    /// The elected new hub (None if no candidates available).
    pub new_hub_id: Option<NodeId>,
    /// Why this hub was chosen.
    pub reason: ElectionReason,
    /// How many candidates were considered.
    pub candidate_count: usize,
}

/// Reason a hub was elected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectionReason {
    /// Pre-configured backup hub was available.
    Backup,
    /// Deterministic selection from candidates (lowest NodeId).
    Deterministic,
    /// No eligible candidates found.
    NoCandidates,
}

/// Run deterministic hub election for a group.
///
/// Algorithm:
/// 1. If the group has a `backup_hub_id` and it's online, elect it.
/// 2. Otherwise, filter candidates: must be relay, online, not the failed hub.
/// 3. Sort by NodeId lexicographically, elect the lowest.
///
/// All members run this independently and arrive at the same result.
pub fn elect_hub(
    group: &GroupInfo,
    failed_hub: &NodeId,
    topology: &Topology,
) -> ElectionResult {
    // 1. Check backup hub first
    if let Some(backup) = &group.backup_hub_id {
        if *backup != *failed_hub {
            if let Some(peer) = topology.get(backup) {
                if peer.status == PeerStatus::Online {
                    return ElectionResult {
                        new_hub_id: Some(*backup),
                        reason: ElectionReason::Backup,
                        candidate_count: 1,
                    };
                }
            }
        }
    }

    // 2. Filter eligible candidates from topology
    let mut candidates: Vec<NodeId> = topology
        .online_relays()
        .into_iter()
        .filter(|peer| {
            peer.node_id != *failed_hub
                && peer.role == PeerRole::Relay
                && peer.status == PeerStatus::Online
        })
        .map(|peer| peer.node_id)
        .collect();

    let candidate_count = candidates.len();

    if candidates.is_empty() {
        return ElectionResult {
            new_hub_id: None,
            reason: ElectionReason::NoCandidates,
            candidate_count: 0,
        };
    }

    // 3. Deterministic: sort by NodeId string representation, pick lowest
    candidates.sort_by_key(|a| a.to_string());

    ElectionResult {
        new_hub_id: Some(candidates[0]),
        reason: ElectionReason::Deterministic,
        candidate_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::PeerInfo;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    fn make_group(hub: NodeId, backup: Option<NodeId>) -> GroupInfo {
        GroupInfo {
            group_id: GroupId::from("grp-test".to_string()),
            name: "Test".into(),
            hub_relay_id: hub,
            backup_hub_id: backup,
            members: vec![],
            created_by: node_id(1),
            created_at: 1000,
            last_activity_at: 1000,
            max_members: MAX_GROUP_MEMBERS,
        }
    }

    fn add_relay(topology: &mut Topology, id: NodeId) {
        topology.upsert(PeerInfo {
            node_id: id,
            role: PeerRole::Relay,
            status: PeerStatus::Online,
            last_seen: 1000,
        });
    }

    #[test]
    fn elect_backup_hub() {
        let failed = node_id(10);
        let backup = node_id(11);
        let group = make_group(failed, Some(backup));

        let mut topology = Topology::new();
        add_relay(&mut topology, backup);

        let result = elect_hub(&group, &failed, &topology);
        assert_eq!(result.new_hub_id, Some(backup));
        assert_eq!(result.reason, ElectionReason::Backup);
    }

    #[test]
    fn elect_deterministic_lowest() {
        let failed = node_id(10);
        let group = make_group(failed, None);

        let mut topology = Topology::new();
        let r1 = node_id(20);
        let r2 = node_id(21);
        let r3 = node_id(22);
        add_relay(&mut topology, r1);
        add_relay(&mut topology, r2);
        add_relay(&mut topology, r3);

        let result = elect_hub(&group, &failed, &topology);
        assert!(result.new_hub_id.is_some());
        assert_eq!(result.reason, ElectionReason::Deterministic);
        assert_eq!(result.candidate_count, 3);

        // All nodes should independently arrive at the same winner
        let result2 = elect_hub(&group, &failed, &topology);
        assert_eq!(result.new_hub_id, result2.new_hub_id);
    }

    #[test]
    fn elect_excludes_failed_hub() {
        let failed = node_id(10);
        let group = make_group(failed, None);

        let mut topology = Topology::new();
        add_relay(&mut topology, failed); // Failed hub still in topology
        let other = node_id(20);
        add_relay(&mut topology, other);

        let result = elect_hub(&group, &failed, &topology);
        assert_eq!(result.new_hub_id, Some(other));
        assert_ne!(result.new_hub_id, Some(failed));
    }

    #[test]
    fn elect_no_candidates() {
        let failed = node_id(10);
        let group = make_group(failed, None);
        let topology = Topology::new(); // Empty

        let result = elect_hub(&group, &failed, &topology);
        assert!(result.new_hub_id.is_none());
        assert_eq!(result.reason, ElectionReason::NoCandidates);
    }

    #[test]
    fn elect_backup_offline_falls_through() {
        let failed = node_id(10);
        let backup = node_id(11);
        let group = make_group(failed, Some(backup));

        let mut topology = Topology::new();
        // Backup is offline
        topology.upsert(PeerInfo {
            node_id: backup,
            role: PeerRole::Relay,
            status: PeerStatus::Offline,
            last_seen: 1000,
        });

        let other = node_id(20);
        add_relay(&mut topology, other);

        let result = elect_hub(&group, &failed, &topology);
        // Should fall through to deterministic since backup is offline
        assert_eq!(result.new_hub_id, Some(other));
        assert_eq!(result.reason, ElectionReason::Deterministic);
    }

    #[test]
    fn elect_backup_is_failed_falls_through() {
        let failed = node_id(10);
        let group = make_group(failed, Some(failed)); // backup = failed hub

        let mut topology = Topology::new();
        let other = node_id(20);
        add_relay(&mut topology, other);

        let result = elect_hub(&group, &failed, &topology);
        assert_eq!(result.new_hub_id, Some(other));
        assert_eq!(result.reason, ElectionReason::Deterministic);
    }

    #[test]
    fn deterministic_across_calls() {
        let failed = node_id(10);
        let group = make_group(failed, None);

        let mut topology = Topology::new();
        for seed in 20..30 {
            add_relay(&mut topology, node_id(seed));
        }

        // Run election 10 times — must always produce same result
        let first = elect_hub(&group, &failed, &topology);
        for _ in 0..10 {
            let result = elect_hub(&group, &failed, &topology);
            assert_eq!(result.new_hub_id, first.new_hub_id);
        }
    }
}
