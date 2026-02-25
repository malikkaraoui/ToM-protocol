//! Integration test for role change propagation across nodes.
//!
//! Simulates the full gossip flow: a node signs a RoleChangeAnnounce,
//! serializes it via MessagePack (the wire format), and a receiving node
//! deserializes + validates + updates topology via handle_gossip_event.

use tom_protocol::{
    GossipInput, NodeId, ProtocolEvent, RoleChangeAnnounce, RuntimeConfig, RuntimeEffect,
    RuntimeState,
};

fn keypair(seed: u8) -> (NodeId, [u8; 32]) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = iroh::SecretKey::generate(&mut rng);
    let node_id: NodeId = secret.public().to_string().parse().unwrap();
    let seed_bytes = secret.to_bytes();
    (node_id, seed_bytes)
}

fn make_state(seed: u8) -> RuntimeState {
    let (id, secret) = keypair(seed);
    RuntimeState::new(id, secret, RuntimeConfig::default())
}

/// Full propagation via handle_role_announce: sign → wire roundtrip → receive.
#[test]
fn role_change_full_propagation() {
    let (node_a_id, node_a_seed) = keypair(1);
    let mut state_b = make_state(2);

    // Node A creates a signed announce
    let announce = RoleChangeAnnounce::new(
        node_a_id,
        tom_protocol::PeerRole::Relay,
        15.0,
        tom_protocol::now_ms(),
        &node_a_seed,
    );

    // Serialize to wire format (MessagePack, same as gossip broadcast)
    let wire_bytes = rmp_serde::to_vec(&announce).expect("serialize");

    // Deserialize on receiving node (same as gossip handler)
    let received: RoleChangeAnnounce =
        rmp_serde::from_slice(&wire_bytes).expect("deserialize");

    // Verify signature survives wire roundtrip
    assert!(
        received.verify_signature(),
        "Signature must survive wire roundtrip"
    );

    // Node B processes the announce
    let effects = state_b.handle_role_announce(received);

    // Should emit RolePromoted event
    assert!(
        effects.iter().any(|e| matches!(
            e,
            RuntimeEffect::Emit(ProtocolEvent::RolePromoted { node_id, .. })
            if *node_id == node_a_id
        )),
        "Node B should emit RolePromoted for node A: {effects:?}"
    );
}

/// Demotion propagation: Relay → Peer.
#[test]
fn demotion_propagates() {
    let (node_a_id, node_a_seed) = keypair(1);
    let mut state_b = make_state(2);

    // Node A sends demotion announce
    let announce = RoleChangeAnnounce::new(
        node_a_id,
        tom_protocol::PeerRole::Peer,
        1.0,
        tom_protocol::now_ms(),
        &node_a_seed,
    );

    let wire_bytes = rmp_serde::to_vec(&announce).unwrap();
    let received: RoleChangeAnnounce = rmp_serde::from_slice(&wire_bytes).unwrap();

    let effects = state_b.handle_role_announce(received);

    assert!(
        effects.iter().any(|e| matches!(
            e,
            RuntimeEffect::Emit(ProtocolEvent::RoleDemoted { node_id, .. })
            if *node_id == node_a_id
        )),
        "Should emit RoleDemoted: {effects:?}"
    );
}

/// Forged announce (wrong signer) is rejected.
#[test]
fn forged_announce_rejected() {
    let (node_a_id, _) = keypair(1);
    let (_, attacker_seed) = keypair(99); // Attacker signs with their own key
    let mut state_b = make_state(2);

    // Attacker creates announce claiming to be node A but signs with their key
    let announce = RoleChangeAnnounce::new(
        node_a_id,
        tom_protocol::PeerRole::Relay,
        99.0,
        tom_protocol::now_ms(),
        &attacker_seed,
    );

    let effects = state_b.handle_role_announce(announce);

    // Should emit error
    assert!(
        effects.iter().any(|e| matches!(
            e,
            RuntimeEffect::Emit(ProtocolEvent::Error { .. })
        )),
        "Forged announce should be rejected: {effects:?}"
    );
}

/// Full gossip path: wire bytes → handle_gossip_event → RolePromoted.
#[test]
fn gossip_event_dispatches_role_announce() {
    let (node_a_id, node_a_seed) = keypair(1);
    let mut state_b = make_state(2);

    let announce = RoleChangeAnnounce::new(
        node_a_id,
        tom_protocol::PeerRole::Relay,
        15.0,
        tom_protocol::now_ms(),
        &node_a_seed,
    );

    // Serialize as wire bytes (this is what gossip broadcasts)
    let wire_bytes = rmp_serde::to_vec(&announce).unwrap();

    // Feed through gossip handler (same path as runtime loop)
    let effects = state_b.handle_gossip_event(GossipInput::PeerAnnounce(wire_bytes));

    // Should emit RolePromoted (gossip handler tries PeerAnnounce, fails, tries RoleChangeAnnounce)
    assert!(
        effects.iter().any(|e| matches!(
            e,
            RuntimeEffect::Emit(ProtocolEvent::RolePromoted { node_id, .. })
            if *node_id == node_a_id
        )),
        "Gossip handler should dispatch RoleChangeAnnounce: {effects:?}"
    );
}
