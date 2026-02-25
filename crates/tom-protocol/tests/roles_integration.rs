/// Integration tests for the dynamic roles module.
///
/// Simulates relay activity over time and verifies promotion/demotion transitions.
use tom_protocol::{
    ContributionMetrics, NodeId, PeerInfo, PeerRole, PeerStatus, RoleAction, RoleManager, Topology,
};

fn node_id(seed: u8) -> NodeId {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = iroh::SecretKey::generate(&mut rng);
    secret.public().to_string().parse().unwrap()
}

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

/// Simulate relay activity → promotion → idleness → demotion.
#[test]
fn full_promotion_demotion_lifecycle() {
    let local = node_id(1);
    let relay_candidate = node_id(2);
    let mut mgr = RoleManager::new(local);
    let mut topo = make_topology(&[(relay_candidate, PeerRole::Peer)]);

    // Phase 1: No relay activity → no promotion
    let actions = mgr.evaluate(&mut topo, 1000);
    assert!(actions.is_empty(), "no activity, no action");
    assert_eq!(topo.get(&relay_candidate).unwrap().role, PeerRole::Peer);

    // Phase 2: Build up relay count (20 relays over 20 seconds)
    for i in 0..20 {
        mgr.record_relay(relay_candidate, 2000 + i * 1000);
    }

    // Evaluate → should promote
    let actions = mgr.evaluate(&mut topo, 22_000);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, RoleAction::Promoted { node_id, .. } if *node_id == relay_candidate)),
        "should promote after 20 relays: {actions:?}"
    );
    assert_eq!(topo.get(&relay_candidate).unwrap().role, PeerRole::Relay);

    // Phase 3: Already promoted, re-evaluate should be no-op
    let actions = mgr.evaluate(&mut topo, 23_000);
    assert!(actions.is_empty(), "already Relay, no double-promote");

    // Phase 4: Long idleness (100 hours) → score decays below demotion threshold
    let now_after_idle = 22_000 + 100 * 3_600_000;
    let score = mgr.score(&relay_candidate, now_after_idle);
    assert!(score < 2.0, "score should be very low after 100h idle: {score}");

    let actions = mgr.evaluate(&mut topo, now_after_idle);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, RoleAction::Demoted { node_id, .. } if *node_id == relay_candidate)),
        "should demote after long idle: {actions:?}"
    );
    assert_eq!(topo.get(&relay_candidate).unwrap().role, PeerRole::Peer);
}

/// Multiple nodes: only the active relayer gets promoted.
#[test]
fn selective_promotion() {
    let local = node_id(1);
    let active = node_id(2);
    let idle = node_id(3);
    let mut mgr = RoleManager::new(local);
    let mut topo = make_topology(&[(active, PeerRole::Peer), (idle, PeerRole::Peer)]);

    // Only active node relays
    for i in 0..20 {
        mgr.record_relay(active, 1000 + i * 1000);
    }
    // Idle node does nothing
    mgr.record_relay(idle, 1000); // One relay to register

    let actions = mgr.evaluate(&mut topo, 21_000);

    let active_promoted = actions
        .iter()
        .any(|a| matches!(a, RoleAction::Promoted { node_id, .. } if *node_id == active));
    let idle_promoted = actions
        .iter()
        .any(|a| matches!(a, RoleAction::Promoted { node_id, .. } if *node_id == idle));

    assert!(active_promoted, "active node should be promoted");
    assert!(!idle_promoted, "idle node should not be promoted");
}

/// Score decay is progressive — never permanently bans (design decision #4).
#[test]
fn score_never_reaches_zero() {
    let m = {
        let mut m = ContributionMetrics::new(0);
        m.record_relay(1000);
        m
    };

    // Even after 100 hours of idleness
    let score = m.score(100 * 3_600_000);
    assert!(score > 0.0, "score should never be exactly zero: {score}");
}

/// Relay failures lower score compared to pure successes.
#[test]
fn failures_reduce_score() {
    let local = node_id(1);
    let good = node_id(2);
    let flaky = node_id(3);
    let mut mgr = RoleManager::new(local);

    // Good node: 10 successes, 0 failures
    for i in 0..10 {
        mgr.record_relay(good, 1000 + i * 1000);
    }
    // Flaky node: 10 successes, 10 failures (50% rate)
    for i in 0..10 {
        mgr.record_relay(flaky, 1000 + i * 1000);
    }
    for i in 10..20 {
        mgr.record_relay_failure(flaky, 1000 + i * 1000);
    }

    let now = 21_000;
    let good_score = mgr.score(&good, now);
    let flaky_score = mgr.score(&flaky, now);
    assert!(
        good_score > flaky_score,
        "100% success ({good_score}) should score higher than 50% success ({flaky_score})"
    );
}

/// Bandwidth contribution helps promotion even with few relays.
#[test]
fn bandwidth_affects_promotion() {
    let local = node_id(1);
    let candidate = node_id(2);
    let mut mgr = RoleManager::new(local);
    let mut topo = make_topology(&[(candidate, PeerRole::Peer)]);

    // Relay 5 messages (not enough for promotion alone: 5 < 10 threshold)
    for i in 0..5 {
        mgr.record_relay(candidate, 1000 + i * 1000);
    }

    // But relay 50 MB of data (should contribute 50×0.2 = 10 points)
    mgr.record_bytes_relayed(candidate, 50 * 1_048_576, 6000);

    let actions = mgr.evaluate(&mut topo, 6000);

    // Should promote: relay (5) + bandwidth (10) + success_rate (5) = 20 > threshold
    assert!(
        actions.iter().any(|a| matches!(a, RoleAction::Promoted { node_id, .. } if *node_id == candidate)),
        "Should promote with bandwidth contribution: {actions:?}"
    );
}

/// Removing a node clears its contribution history.
#[test]
fn remove_node_resets_scoring() {
    let local = node_id(1);
    let node = node_id(2);
    let mut mgr = RoleManager::new(local);

    for i in 0..20 {
        mgr.record_relay(node, 1000 + i * 1000);
    }
    assert!(mgr.score(&node, 21_000) > 10.0);

    mgr.remove_node(&node);
    assert_eq!(mgr.score(&node, 21_000), 0.0);
}
