/// Integration tests: discovery subsystem.
///
/// Tests HeartbeatTracker + EphemeralSubnetManager + Topology
/// working together without transport — pure in-memory simulation.
///
/// Scenario: A small network of peers communicating, forming subnets,
/// with heartbeats tracking liveness and triggering topology changes.
use tom_protocol::{
    DissolveReason, EphemeralSubnetManager, HeartbeatTracker, LivenessState, PeerAnnounce,
    PeerInfo, PeerRole, PeerStatus, RoleChangeAnnounce, SubnetEvent, Topology,
};
use tom_protocol::discovery::RelayReadyAnnounce;

type NodeId = tom_protocol::NodeId;

fn node_id(seed: u8) -> NodeId {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = tom_connect::SecretKey::generate(&mut rng);
    secret.public().to_string().parse().unwrap()
}

/// Full discovery lifecycle: announce → heartbeat → subnet formation → dissolution.
#[test]
fn discovery_lifecycle() {
    let alice = node_id(1);
    let bob = node_id(2);
    let charlie = node_id(3);

    // ── Step 1: PeerAnnounce validation ─────────────────────────────────
    // Use real-ish epoch ms so PeerAnnounce::new() timestamps are valid
    let real_now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let alice_announce = PeerAnnounce::new(alice, "alice".into(), 0, vec![PeerRole::Peer]);
    assert!(alice_announce.is_timestamp_valid(real_now));

    let bob_announce = PeerAnnounce::new(bob, "bob".into(), 0, vec![PeerRole::Relay]);
    assert!(bob_announce.is_timestamp_valid(real_now));

    // For heartbeat tests, use controlled timestamps
    let now = 100_000u64;

    // ── Step 2: Register peers in topology + heartbeat tracker ──────────
    let mut topology = Topology::new();
    let mut heartbeats = HeartbeatTracker::with_thresholds(100, 200);

    for &id in &[alice, bob, charlie] {
        topology.upsert(PeerInfo {
            node_id: id,
            role: PeerRole::Peer,
            status: PeerStatus::Online,
            last_seen: now,
        });
        heartbeats.record_heartbeat_at(id, now);
    }

    assert_eq!(heartbeats.tracked_count(), 3);
    assert_eq!(heartbeats.liveness_at(&alice, now), LivenessState::Alive);
    assert_eq!(heartbeats.liveness_at(&bob, now), LivenessState::Alive);

    // ── Step 3: Time passes, Alice goes stale ───────────────────────────
    // Bob and Charlie refresh, Alice doesn't
    heartbeats.record_heartbeat_at(bob, now + 80);
    heartbeats.record_heartbeat_at(charlie, now + 80);

    assert_eq!(heartbeats.liveness_at(&alice, now + 100), LivenessState::Stale);
    assert_eq!(heartbeats.liveness_at(&bob, now + 100), LivenessState::Alive);
    assert_eq!(heartbeats.liveness_at(&charlie, now + 100), LivenessState::Alive);

    // ── Step 4: Alice goes offline, topology updates ────────────────────
    assert_eq!(heartbeats.liveness_at(&alice, now + 200), LivenessState::Departed);

    let events = heartbeats.check_all(&mut topology);
    // At least alice should have a status change event
    assert!(!events.is_empty());

    // ── Step 5: Alice comes back ────────────────────────────────────────
    heartbeats.record_heartbeat_at(alice, now + 250);
    assert_eq!(heartbeats.liveness_at(&alice, now + 260), LivenessState::Alive);
}

/// Subnet formation from communication patterns.
#[test]
fn subnet_formation_from_communication() {
    let alice = node_id(1);
    let bob = node_id(2);
    let charlie = node_id(3);
    let dave = node_id(4);

    let mut subnets = EphemeralSubnetManager::new(alice);
    let now = 100_000u64;

    // ── Step 1: No subnet initially ─────────────────────────────────────
    assert_eq!(subnets.subnet_count(), 0);
    assert!(!subnets.are_in_same_subnet(&alice, &bob));

    // ── Step 2: Build communication pattern ─────────────────────────────
    // Alice, Bob, Charlie form a triangle (heavy communication)
    for _ in 0..5 {
        subnets.record_communication(alice, bob, now);
        subnets.record_communication(bob, charlie, now);
        subnets.record_communication(alice, charlie, now);
    }

    // Dave only talks to Alice once (weak edge)
    subnets.record_communication(alice, dave, now);

    // ── Step 3: Evaluate → subnet formed ────────────────────────────────
    let events = subnets.evaluate(now);

    assert_eq!(subnets.subnet_count(), 1);
    assert!(subnets.are_in_same_subnet(&alice, &bob));
    assert!(subnets.are_in_same_subnet(&bob, &charlie));
    assert!(!subnets.are_in_same_subnet(&alice, &dave)); // Dave excluded (weak edge)

    // Check events
    let formed_count = events
        .iter()
        .filter(|e| matches!(e, SubnetEvent::SubnetFormed { .. }))
        .count();
    assert_eq!(formed_count, 1);

    // ── Step 4: Dave strengthens connections → new evaluation ────────────
    for _ in 0..5 {
        subnets.record_communication(alice, dave, now + 1000);
        subnets.record_communication(bob, dave, now + 1000);
        subnets.record_communication(charlie, dave, now + 1000);
    }

    // Dave should join existing subnet members on next eval
    // (but since they're already subnetted, Dave might form a new cluster
    //  or get absorbed — depends on BFS traversal)
    subnets.evaluate(now + 1000);

    // Either way, the communication is tracked
    assert!(subnets.edge_count() >= 4); // At least 4 edges in the graph
}

/// HeartbeatTracker + EphemeralSubnetManager coordination.
#[test]
fn heartbeat_drives_subnet_removal() {
    let alice = node_id(1);
    let bob = node_id(2);
    let charlie = node_id(3);

    let mut heartbeats = HeartbeatTracker::with_thresholds(100, 200);
    let mut subnets = EphemeralSubnetManager::new(alice);
    let now = 100_000u64;

    // Setup: all three active and communicating
    for &id in &[alice, bob, charlie] {
        heartbeats.record_heartbeat_at(id, now);
    }

    for _ in 0..5 {
        subnets.record_communication(alice, bob, now);
        subnets.record_communication(bob, charlie, now);
        subnets.record_communication(alice, charlie, now);
    }

    subnets.evaluate(now);
    assert_eq!(subnets.subnet_count(), 1);
    assert_eq!(subnets.nodes_in_subnets(), 3);

    // Charlie goes offline (detected by heartbeat)
    assert_eq!(
        heartbeats.liveness_at(&charlie, now + 200),
        LivenessState::Departed
    );

    // Application removes charlie from subnet tracking
    let events = subnets.remove_node(&charlie);

    // Subnet dissolved (dropped below MIN_SUBNET_SIZE)
    assert_eq!(subnets.subnet_count(), 0);
    let dissolved = events
        .iter()
        .filter(|e| matches!(e, SubnetEvent::SubnetDissolved { reason: DissolveReason::InsufficientMembers, .. }))
        .count();
    assert_eq!(dissolved, 1);
}

/// PeerAnnounce timestamp validation protects against replay.
#[test]
fn announce_timestamp_guards() {
    let alice = node_id(1);
    let now = 10_000_000_000u64; // ~2286, realistic epoch ms

    // Valid: recent announcement
    let mut announce = PeerAnnounce::new(alice, "alice".into(), 0, vec![PeerRole::Peer]);
    announce.timestamp = now;
    assert!(announce.is_timestamp_valid(now));

    // Valid: slightly in the future (clock drift)
    announce.timestamp = now + 60_000; // 1 minute ahead
    assert!(announce.is_timestamp_valid(now));

    // Invalid: too far in the future (>5 minutes)
    announce.timestamp = now + 6 * 60 * 1000;
    assert!(!announce.is_timestamp_valid(now));

    // Invalid: too old (>1 hour)
    announce.timestamp = now - 2 * 60 * 60 * 1000;
    assert!(!announce.is_timestamp_valid(now));

    // Valid: 30 minutes old
    announce.timestamp = now - 30 * 60 * 1000;
    assert!(announce.is_timestamp_valid(now));
}

/// Edge decay prevents stale communication patterns from persisting.
#[test]
fn edge_decay_lifecycle() {
    let alice = node_id(1);
    let bob = node_id(2);
    let charlie = node_id(3);

    let mut subnets = EphemeralSubnetManager::new(alice);
    let now = 100_000u64;

    // Build strong triangle
    for _ in 0..10 {
        subnets.record_communication(alice, bob, now);
        subnets.record_communication(bob, charlie, now);
        subnets.record_communication(alice, charlie, now);
    }

    subnets.evaluate(now);
    assert_eq!(subnets.subnet_count(), 1);

    // Fast-forward: subnet goes inactive, then edges decay
    let inactivity_expired = now + 5 * 60 * 1000 + 1; // Past INACTIVITY_TIMEOUT_MS
    subnets.evaluate(inactivity_expired);
    assert_eq!(subnets.subnet_count(), 0); // Dissolved

    // Edges still exist but are decaying
    assert!(subnets.edge_count() > 0);

    // Way past edge decay (edges formed at now=100000, decay at 10 min)
    let edges_expired = now + 20 * 60 * 1000 + 1; // 2x EDGE_DECAY_MS
    subnets.evaluate(edges_expired);
    assert_eq!(subnets.edge_count(), 0); // All edges decayed to zero
}

/// Multiple subnets can coexist independently.
#[test]
fn multiple_independent_subnets() {
    let me = node_id(0);
    let mut subnets = EphemeralSubnetManager::new(me);
    let now = 100_000u64;

    // Group A: nodes 1,2,3
    let a1 = node_id(1);
    let a2 = node_id(2);
    let a3 = node_id(3);
    for _ in 0..5 {
        subnets.record_communication(a1, a2, now);
        subnets.record_communication(a2, a3, now);
        subnets.record_communication(a1, a3, now);
    }

    // Group B: nodes 10,11,12
    let b1 = node_id(10);
    let b2 = node_id(11);
    let b3 = node_id(12);
    for _ in 0..5 {
        subnets.record_communication(b1, b2, now);
        subnets.record_communication(b2, b3, now);
        subnets.record_communication(b1, b3, now);
    }

    subnets.evaluate(now);

    assert_eq!(subnets.subnet_count(), 2);
    assert!(subnets.are_in_same_subnet(&a1, &a2));
    assert!(subnets.are_in_same_subnet(&b1, &b2));
    assert!(!subnets.are_in_same_subnet(&a1, &b1));

    // Group A goes silent, Group B keeps talking
    subnets.record_communication(b1, b2, now + 4 * 60 * 1000);

    let far = now + 5 * 60 * 1000 + 1;
    subnets.evaluate(far);

    // Group A dissolved (inactive), Group B survives
    assert_eq!(subnets.subnet_count(), 1);
    assert!(subnets.are_in_same_subnet(&b1, &b2));
    assert!(!subnets.are_in_same_subnet(&a1, &a2));
}

// ─── Gossip adversarial tests ────────────────────────────────────────────────

fn relay_url() -> tom_connect::RelayUrl {
    "http://127.0.0.1:3340".parse().unwrap()
}

fn keypair_relay(seed: u8) -> (NodeId, [u8; 32]) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = tom_connect::SecretKey::generate(&mut rng);
    let seed_bytes = secret.to_bytes();
    let id = secret.public().to_string().parse().unwrap();
    (id, seed_bytes)
}

/// A RelayReadyAnnounce with an all-zeros forged signature is rejected.
#[test]
fn gossip_relay_announce_forged_sig_rejected() {
    let (id, seed) = keypair_relay(1);
    let mut announce = RelayReadyAnnounce::new(id, relay_url(), 1_000_000, &seed);
    announce.signature = vec![0u8; 64];
    assert!(
        !announce.verify_signature(),
        "forged all-zeros signature must be rejected"
    );
}

/// Tampering the relay URL after signing breaks the signature.
#[test]
fn gossip_relay_announce_tampered_url_breaks_sig() {
    let (id, seed) = keypair_relay(2);
    let mut announce = RelayReadyAnnounce::new(id, relay_url(), 1_000_000, &seed);
    assert!(announce.verify_signature(), "original must be valid");
    announce.relay_url = "http://evil.attacker.com:9999".parse().unwrap();
    assert!(
        !announce.verify_signature(),
        "tampered relay URL must break the signature"
    );
}

/// Tampering the node_id after signing breaks the RelayReadyAnnounce signature.
#[test]
fn gossip_relay_announce_wrong_node_id_breaks_sig() {
    let (id, seed) = keypair_relay(3);
    let (other_id, _) = keypair_relay(4);
    let mut announce = RelayReadyAnnounce::new(id, relay_url(), 1_000_000, &seed);
    assert!(announce.verify_signature());
    announce.node_id = other_id;
    assert!(
        !announce.verify_signature(),
        "swapped node_id must break signature (wrong verifying key)"
    );
}

/// A RelayReadyAnnounce signed by a different key is rejected.
#[test]
fn gossip_relay_announce_wrong_signer_rejected() {
    let (id, _) = keypair_relay(5);
    let (_, wrong_seed) = keypair_relay(6);
    // Sign with wrong_seed but advertise `id` as the source
    let announce = RelayReadyAnnounce::new(id, relay_url(), 1_000_000, &wrong_seed);
    assert!(
        !announce.verify_signature(),
        "relay announce signed by wrong key must be rejected"
    );
}

/// Tampering the score in a RoleChangeAnnounce breaks the signature.
#[test]
fn gossip_role_announce_tampered_score_rejected() {
    let (id, seed) = keypair_relay(7);
    let mut announce = RoleChangeAnnounce::new(id, PeerRole::Relay, 0.85, 1_000_000, &seed);
    assert!(announce.verify_signature(), "original must be valid");
    announce.score = 0.0; // attacker tries to degrade reputation
    assert!(
        !announce.verify_signature(),
        "tampered score must break the signature"
    );
}

/// RoleChangeAnnounce with a zero-length signature is rejected without panic.
#[test]
fn gossip_role_announce_empty_sig_rejected() {
    let (id, seed) = keypair_relay(8);
    let mut announce = RoleChangeAnnounce::new(id, PeerRole::Peer, 0.5, 1_000_000, &seed);
    announce.signature = vec![];
    assert!(
        !announce.verify_signature(),
        "empty signature must be rejected (not panic)"
    );
}

/// PeerAnnounce timestamp far in the future is rejected by the timestamp guard.
#[test]
fn gossip_peer_announce_far_future_timestamp_rejected() {
    let now = 10_000_000_000u64;
    let mut announce = PeerAnnounce::new(node_id(9), "mallory".into(), 0, vec![PeerRole::Relay]);
    // 10 minutes in the future (> 5-minute MAX_FUTURE_DRIFT_MS)
    announce.timestamp = now + 10 * 60 * 1000;
    assert!(
        !announce.is_timestamp_valid(now),
        "far-future PeerAnnounce must be rejected (clock spoofing)"
    );
}

/// PeerAnnounce with very old timestamp is rejected.
#[test]
fn gossip_peer_announce_stale_timestamp_rejected() {
    let now = 10_000_000_000u64;
    let mut announce = PeerAnnounce::new(node_id(10), "zombie".into(), 0, vec![PeerRole::Peer]);
    announce.timestamp = now - 2 * 60 * 60 * 1000; // 2 hours old
    assert!(
        !announce.is_timestamp_valid(now),
        "2-hour-old PeerAnnounce must be rejected"
    );
}
