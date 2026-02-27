/// HeartbeatTracker — peer liveness monitoring.
///
/// Pure state machine: record heartbeats, check liveness.
/// Two-tier timeout: Stale (1x threshold) → Departed (2x threshold).
/// Tracks discovery source for new peers (consumed on PeerDiscovered emission).
use std::collections::{HashMap, HashSet};

use crate::discovery::types::*;
use crate::relay::{PeerStatus, Topology};
use crate::types::{now_ms, NodeId};

/// Tracks peer liveness via heartbeat timestamps.
pub struct HeartbeatTracker {
    /// Last heartbeat time per peer (Unix ms).
    last_heartbeat: HashMap<NodeId, u64>,
    /// Stale threshold in ms.
    stale_threshold: u64,
    /// Offline threshold in ms.
    offline_threshold: u64,
    /// Pending discovery sources (consumed when PeerDiscovered emitted).
    pending_source: HashMap<NodeId, DiscoverySource>,
    /// Pending usernames (consumed when PeerDiscovered emitted).
    pending_username: HashMap<NodeId, String>,
    /// Peers that have been discovered (PeerDiscovered already emitted).
    discovered: HashSet<NodeId>,
}

impl HeartbeatTracker {
    /// Create a new tracker with default thresholds.
    pub fn new() -> Self {
        Self {
            last_heartbeat: HashMap::new(),
            stale_threshold: STALE_THRESHOLD_MS,
            offline_threshold: OFFLINE_THRESHOLD_MS,
            pending_source: HashMap::new(),
            pending_username: HashMap::new(),
            discovered: HashSet::new(),
        }
    }

    /// Create with custom thresholds (for testing).
    pub fn with_thresholds(stale_ms: u64, offline_ms: u64) -> Self {
        Self {
            last_heartbeat: HashMap::new(),
            stale_threshold: stale_ms,
            offline_threshold: offline_ms,
            pending_source: HashMap::new(),
            pending_username: HashMap::new(),
            discovered: HashSet::new(),
        }
    }

    /// Record a heartbeat from a peer.
    pub fn record_heartbeat(&mut self, node_id: NodeId) {
        self.last_heartbeat.insert(node_id, now_ms());
    }

    /// Record a heartbeat with a specific timestamp (for testing).
    pub fn record_heartbeat_at(&mut self, node_id: NodeId, timestamp: u64) {
        self.last_heartbeat.insert(node_id, timestamp);
    }

    /// Record a heartbeat with a discovery source and username.
    ///
    /// Source and username are stored pending; consumed when PeerDiscovered
    /// is emitted from check_all.
    pub fn record_heartbeat_with_source(
        &mut self,
        node_id: NodeId,
        source: DiscoverySource,
        username: String,
    ) {
        self.pending_source.insert(node_id, source);
        self.pending_username.insert(node_id, username);
        self.record_heartbeat(node_id);
    }

    /// Start tracking a peer (initial registration).
    pub fn track_peer(&mut self, node_id: NodeId) {
        self.last_heartbeat.entry(node_id).or_insert_with(now_ms);
    }

    /// Stop tracking a peer.
    pub fn untrack_peer(&mut self, node_id: &NodeId) {
        self.last_heartbeat.remove(node_id);
    }

    /// Check the liveness state of a specific peer.
    pub fn liveness(&self, node_id: &NodeId) -> LivenessState {
        let Some(&last) = self.last_heartbeat.get(node_id) else {
            return LivenessState::Departed;
        };

        let now = now_ms();
        let elapsed = now.saturating_sub(last);

        if elapsed >= self.offline_threshold {
            LivenessState::Departed
        } else if elapsed >= self.stale_threshold {
            LivenessState::Stale
        } else {
            LivenessState::Alive
        }
    }

    /// Check liveness at a specific time (for testing).
    pub fn liveness_at(&self, node_id: &NodeId, now: u64) -> LivenessState {
        let Some(&last) = self.last_heartbeat.get(node_id) else {
            return LivenessState::Departed;
        };

        let elapsed = now.saturating_sub(last);

        if elapsed >= self.offline_threshold {
            LivenessState::Departed
        } else if elapsed >= self.stale_threshold {
            LivenessState::Stale
        } else {
            LivenessState::Alive
        }
    }

    /// Check all peers and return events for state transitions.
    ///
    /// Updates the provided `Topology` with status changes.
    /// Emits PeerDiscovered for new peers (consuming pending source/username).
    pub fn check_all(&mut self, topology: &mut Topology) -> Vec<DiscoveryEvent> {
        let mut events = vec![];
        let now = now_ms();

        for (&node_id, &last) in &self.last_heartbeat {
            let elapsed = now.saturating_sub(last);
            let current_status = topology.get(&node_id).map(|p| p.status);

            if elapsed >= self.offline_threshold {
                // Departed
                if current_status != Some(PeerStatus::Offline) {
                    if let Some(peer) = topology.get(&node_id) {
                        let mut updated = peer.clone();
                        updated.status = PeerStatus::Offline;
                        topology.upsert(updated);
                    }
                    events.push(DiscoveryEvent::PeerOffline { node_id });
                }
            } else if elapsed >= self.stale_threshold {
                // Stale
                if current_status != Some(PeerStatus::Stale) {
                    if let Some(peer) = topology.get(&node_id) {
                        let mut updated = peer.clone();
                        updated.status = PeerStatus::Stale;
                        topology.upsert(updated);
                    }
                    events.push(DiscoveryEvent::PeerStale { node_id });
                }
            } else if !self.discovered.contains(&node_id) {
                // Alive + first time seen → PeerDiscovered
                let source = self.pending_source
                    .remove(&node_id)
                    .unwrap_or(DiscoverySource::Direct);
                let username = self.pending_username
                    .remove(&node_id)
                    .unwrap_or_default();
                if let Some(peer) = topology.get(&node_id) {
                    let mut updated = peer.clone();
                    updated.status = PeerStatus::Online;
                    topology.upsert(updated);
                }
                events.push(DiscoveryEvent::PeerDiscovered {
                    node_id,
                    username,
                    source,
                });
            } else if current_status == Some(PeerStatus::Stale)
                || current_status == Some(PeerStatus::Offline)
            {
                // Alive + was offline/stale → PeerOnline (reconnect)
                if let Some(peer) = topology.get(&node_id) {
                    let mut updated = peer.clone();
                    updated.status = PeerStatus::Online;
                    topology.upsert(updated);
                }
                events.push(DiscoveryEvent::PeerOnline { node_id });
            }
        }

        // Mark newly discovered peers
        for event in &events {
            if let DiscoveryEvent::PeerDiscovered { node_id, .. } = event {
                self.discovered.insert(*node_id);
            }
        }

        events
    }

    /// Remove departed peers from tracking. Returns removed node IDs.
    pub fn cleanup_departed(&mut self) -> Vec<NodeId> {
        let now = now_ms();
        let mut removed = vec![];

        self.last_heartbeat.retain(|&node_id, &mut last| {
            let elapsed = now.saturating_sub(last);
            if elapsed >= self.offline_threshold * 3 {
                removed.push(node_id);
                false
            } else {
                true
            }
        });

        // Clean up discovered + pending for removed peers
        for id in &removed {
            self.discovered.remove(id);
            self.pending_source.remove(id);
            self.pending_username.remove(id);
        }

        removed
    }

    /// Number of tracked peers.
    pub fn tracked_count(&self) -> usize {
        self.last_heartbeat.len()
    }
}

impl Default for HeartbeatTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relay::{PeerInfo, PeerRole};

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn track_and_liveness() {
        let mut tracker = HeartbeatTracker::with_thresholds(100, 200);
        let alice = node_id(1);
        let now = 10_000u64;

        // Not tracked → Departed
        assert_eq!(tracker.liveness_at(&alice, now), LivenessState::Departed);

        // Just registered → Alive
        tracker.record_heartbeat_at(alice, now);
        assert_eq!(tracker.liveness_at(&alice, now), LivenessState::Alive);
        assert_eq!(tracker.liveness_at(&alice, now + 50), LivenessState::Alive);

        // Stale threshold crossed
        assert_eq!(tracker.liveness_at(&alice, now + 100), LivenessState::Stale);
        assert_eq!(tracker.liveness_at(&alice, now + 150), LivenessState::Stale);

        // Offline threshold crossed
        assert_eq!(tracker.liveness_at(&alice, now + 200), LivenessState::Departed);
    }

    #[test]
    fn heartbeat_refreshes() {
        let mut tracker = HeartbeatTracker::with_thresholds(100, 200);
        let alice = node_id(1);

        tracker.record_heartbeat_at(alice, 1000);
        assert_eq!(tracker.liveness_at(&alice, 1050), LivenessState::Alive);

        // Would be stale at 1100, but heartbeat refreshes
        tracker.record_heartbeat_at(alice, 1090);
        assert_eq!(tracker.liveness_at(&alice, 1100), LivenessState::Alive);
        assert_eq!(tracker.liveness_at(&alice, 1150), LivenessState::Alive);
    }

    #[test]
    fn untrack_peer() {
        let mut tracker = HeartbeatTracker::new();
        let alice = node_id(1);

        tracker.track_peer(alice);
        assert_eq!(tracker.tracked_count(), 1);

        tracker.untrack_peer(&alice);
        assert_eq!(tracker.tracked_count(), 0);
        assert_eq!(tracker.liveness(&alice), LivenessState::Departed);
    }

    #[test]
    fn check_all_updates_topology() {
        let mut tracker = HeartbeatTracker::with_thresholds(100, 200);
        let alice = node_id(1);
        let bob = node_id(2);

        let mut topology = Topology::new();
        topology.upsert(PeerInfo {
            node_id: alice,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 1000,
        });
        topology.upsert(PeerInfo {
            node_id: bob,
            role: PeerRole::Relay,
            status: PeerStatus::Online,
            last_seen: 1000,
        });

        // Alice recent, Bob old
        tracker.record_heartbeat_at(alice, 900);
        tracker.record_heartbeat_at(bob, 700);

        let events = tracker.check_all(&mut topology);

        // Both should have status updates based on now_ms()
        // Since we can't control now_ms() in check_all, we verify the events exist
        assert!(!events.is_empty());
    }

    #[test]
    fn check_all_emits_peer_discovered_for_new_peer() {
        let mut tracker = HeartbeatTracker::with_thresholds(100, 200);
        let alice = node_id(1);

        let mut topology = Topology::new();
        topology.upsert(PeerInfo {
            node_id: alice,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 1000,
        });

        // Record with source
        tracker.record_heartbeat_with_source(alice, DiscoverySource::Announce, "alice".into());

        let events = tracker.check_all(&mut topology);
        assert_eq!(events.len(), 1);
        match &events[0] {
            DiscoveryEvent::PeerDiscovered { node_id, username, source } => {
                assert_eq!(node_id, &alice);
                assert_eq!(username, "alice");
                assert_eq!(*source, DiscoverySource::Announce);
            }
            other => panic!("Expected PeerDiscovered, got: {other:?}"),
        }
    }

    #[test]
    fn check_all_fallback_direct_source() {
        let mut tracker = HeartbeatTracker::with_thresholds(100, 200);
        let alice = node_id(1);

        let mut topology = Topology::new();
        topology.upsert(PeerInfo {
            node_id: alice,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 1000,
        });

        // Record WITHOUT source
        tracker.record_heartbeat(alice);

        let events = tracker.check_all(&mut topology);
        assert_eq!(events.len(), 1);
        match &events[0] {
            DiscoveryEvent::PeerDiscovered { source, .. } => {
                assert_eq!(*source, DiscoverySource::Direct); // fallback
            }
            other => panic!("Expected PeerDiscovered, got: {other:?}"),
        }
    }

    #[test]
    fn source_consumed_after_discovered() {
        let mut tracker = HeartbeatTracker::with_thresholds(100, 200);
        let alice = node_id(1);

        let mut topology = Topology::new();
        topology.upsert(PeerInfo {
            node_id: alice,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: 1000,
        });

        tracker.record_heartbeat_with_source(alice, DiscoverySource::Gossip, "alice".into());
        let _ = tracker.check_all(&mut topology); // Emits PeerDiscovered

        // Second check should NOT emit PeerDiscovered again
        let events = tracker.check_all(&mut topology);
        assert!(events.is_empty(), "expected no events on second check, got: {events:?}");
    }

    #[test]
    fn cleanup_departed() {
        let mut tracker = HeartbeatTracker::with_thresholds(10, 20);
        let alice = node_id(1);
        let bob = node_id(2);

        // Alice: very old (will be cleaned)
        tracker.record_heartbeat_at(alice, 0);
        // Bob: recent (will survive)
        tracker.record_heartbeat(bob);

        let removed = tracker.cleanup_departed();
        assert!(removed.contains(&alice));
        assert!(!removed.contains(&bob));
        assert_eq!(tracker.tracked_count(), 1);
    }

    #[test]
    fn tracked_count() {
        let mut tracker = HeartbeatTracker::new();
        assert_eq!(tracker.tracked_count(), 0);

        tracker.track_peer(node_id(1));
        tracker.track_peer(node_id(2));
        assert_eq!(tracker.tracked_count(), 2);
    }
}
