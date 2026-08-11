/// Integration test: log group spawn (rejoin-or-create orchestration).
///
/// Tests the S1 étape 1 log group orchestration pattern where:
/// 1. A single node creates the well-known "__tomlog__" group when no peer responds.
/// 2. Multiple nodes converge on a single "__tomlog__" group via election.
/// 3. All nodes become members of the log group.
///
/// Focus: testing the GroupManager and GroupHub behavior for the log group,
/// not the full runtime loop (that requires async orchestration).
use tom_protocol::{GroupAction, GroupHub, GroupId, GroupManager, GroupPayload, NodeId};

fn node_id(seed: u8) -> NodeId {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = tom_connect::SecretKey::generate(&mut rng);
    secret.public().to_string().parse().unwrap()
}

/// Test scenario: Node A is the hub, Node B joins the log group.
/// Verifies: single group, both members, hub is A.
#[test]
fn log_group_two_nodes_hub_and_joiner() {
    let hub_id = node_id(1);
    let joiner_id = node_id(2);
    let log_group_id = GroupId::from("__tomlog__".to_string());

    let mut hub_manager = GroupManager::new(hub_id, "hub".into());
    let mut joiner_manager = GroupManager::new(joiner_id, "joiner".into());
    let mut hub = GroupHub::new(hub_id);

    // ── Step 1: Hub creates the log group locally ────────────────────
    let create_effects = hub.handle_payload(
        GroupPayload::Create {
            group_name: "__tomlog__".into(),
            creator_username: "hub".into(),
            initial_members: vec![],
            invite_only: false,
        },
        hub_id,
    );

    // Hub responds with Created action
    assert_eq!(create_effects.len(), 1);
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &create_effects[0]
    else {
        panic!("expected Created from hub");
    };

    assert_eq!(group.group_id, log_group_id);
    assert_eq!(group.hub_relay_id, hub_id);
    assert_eq!(group.members.len(), 1); // Creator is auto-added

    // Deliver to hub manager to track locally
    hub_manager.handle_group_created(group.clone());
    assert!(hub_manager.is_in_group(&log_group_id));

    // ── Step 2: Joiner broadcasts a Join request ────────────────────
    let join_actions = vec![GroupAction::Broadcast {
        to: vec![hub_id],
        payload: GroupPayload::Join {
            group_id: log_group_id.clone(),
            username: "joiner".into(),
        },
    }];

    // For broadcast, we manually extract the payload and send to hub
    let GroupAction::Broadcast { to: _, payload } = &join_actions[0] else {
        panic!("expected Broadcast");
    };
    let GroupPayload::Join { group_id, username } = payload else {
        panic!("expected Join payload");
    };

    // Simulate network: deliver Join to hub
    let hub_join_actions = hub.handle_payload(
        GroupPayload::Join {
            group_id: group_id.clone(),
            username: username.clone(),
        },
        joiner_id,
    );

    // Hub responds: Sync (to joiner) + MemberJoined broadcast (to existing members)
    assert_eq!(hub_join_actions.len(), 2);

    // Verify Sync to joiner
    let sync_action = &hub_join_actions[0];
    let GroupAction::Send { to, payload: GroupPayload::Sync { group: sync_group, recent_messages: _ } } = sync_action
    else {
        panic!("expected Sync to joiner");
    };
    assert_eq!(*to, joiner_id);
    assert_eq!(sync_group.group_id, log_group_id);

    // Joiner manager receives Sync → becomes member
    joiner_manager.handle_group_sync(sync_group.clone(), vec![]);
    assert!(joiner_manager.is_in_group(&log_group_id));
    assert_eq!(sync_group.members.len(), 2); // hub + joiner

    // Verify no duplicate groups
    assert_eq!(hub_manager.group_count(), 1);
    assert_eq!(joiner_manager.group_count(), 1);

    // Verify both are members of the same group
    let hub_group = hub_manager.get_group(&log_group_id);
    let joiner_group = joiner_manager.get_group(&log_group_id);
    assert!(hub_group.is_some());
    assert!(joiner_group.is_some());
    assert_eq!(hub_group.unwrap().hub_relay_id, hub_id);
    assert_eq!(joiner_group.unwrap().hub_relay_id, hub_id);
}

/// Test scenario: Three nodes (A, B, C) all join the log group.
/// Only the smallest NodeId hub is active; others defer.
/// Verifies: single group, all members, single hub.
#[test]
fn log_group_three_nodes_convergence() {
    let node_a_id = node_id(1);
    let node_b_id = node_id(2);
    let node_c_id = node_id(3);
    let log_group_id = GroupId::from("__tomlog__".to_string());

    let mut node_a_mgr = GroupManager::new(node_a_id, "A".into());
    let mut node_b_mgr = GroupManager::new(node_b_id, "B".into());
    let mut node_c_mgr = GroupManager::new(node_c_id, "C".into());
    let mut hub_a = GroupHub::new(node_a_id);

    // ── A creates the log group (smallest NodeId) ────────────────────
    let a_create = hub_a.handle_payload(
        GroupPayload::Create {
            group_name: "__tomlog__".into(),
            creator_username: "A".into(),
            initial_members: vec![],
            invite_only: false,
        },
        node_a_id,
    );
    let GroupAction::Send { payload: GroupPayload::Created { group: group_a }, .. } = &a_create[0]
    else {
        panic!("A failed to create group");
    };
    node_a_mgr.handle_group_created(group_a.clone());

    // ── B joins A's group ────────────────────────────────────────────
    let b_join = hub_a.handle_payload(
        GroupPayload::Join {
            group_id: log_group_id.clone(),
            username: "B".into(),
        },
        node_b_id,
    );
    // Hub A responds with Sync
    let GroupAction::Send { payload: GroupPayload::Sync { group: group_b, .. }, .. } = &b_join[0]
    else {
        panic!("A failed to sync B");
    };
    node_b_mgr.handle_group_sync(group_b.clone(), vec![]);

    // ── C joins A's group ────────────────────────────────────────────
    let c_join = hub_a.handle_payload(
        GroupPayload::Join {
            group_id: log_group_id.clone(),
            username: "C".into(),
        },
        node_c_id,
    );
    let GroupAction::Send { payload: GroupPayload::Sync { group: group_c, .. }, .. } = &c_join[0]
    else {
        panic!("A failed to sync C");
    };
    node_c_mgr.handle_group_sync(group_c.clone(), vec![]);

    // ── Verify convergence ───────────────────────────────────────────
    // Chaque nœud est membre d'UN seul groupe, tous le même id "__tomlog__", même hub A.
    assert_eq!(node_a_mgr.group_count(), 1);
    assert_eq!(node_b_mgr.group_count(), 1);
    assert_eq!(node_c_mgr.group_count(), 1);

    let group_a = node_a_mgr.get_group(&log_group_id).unwrap();
    let group_b = node_b_mgr.get_group(&log_group_id).unwrap();
    let group_c = node_c_mgr.get_group(&log_group_id).unwrap();
    assert_eq!(group_a.group_id, log_group_id);
    assert_eq!(group_a.hub_relay_id, node_a_id);
    assert_eq!(group_b.hub_relay_id, node_a_id);
    assert_eq!(group_c.hub_relay_id, node_a_id);

    // Le HUB détient la vérité du membership : A + B + C = 3 membres distincts.
    // (Les vues des membres rattrapent via MemberJoined — propagation eventual,
    // non simulée ici ; ce qui compte pour S1 = la convergence sur un seul groupe.)
    let hub_members = &hub_a.get_group(&log_group_id).unwrap().members;
    assert_eq!(hub_members.len(), 3);
    let distinct: std::collections::HashSet<_> = hub_members.iter().map(|m| &m.node_id).collect();
    assert_eq!(distinct.len(), 3, "pas de membre dupliqué côté hub");
    for id in [&node_a_id, &node_b_id, &node_c_id] {
        assert!(distinct.contains(id), "membre manquant côté hub");
    }
}

/// Test scenario: log group is public (invite_only = false).
/// Verifies: any node can join without prior invitation.
#[test]
fn log_group_is_public() {
    let hub_id = node_id(1);
    let stranger_id = node_id(99);
    let log_group_id = GroupId::from("__tomlog__".to_string());

    let mut hub = GroupHub::new(hub_id);

    // Create public log group
    let create_effects = hub.handle_payload(
        GroupPayload::Create {
            group_name: "__tomlog__".into(),
            creator_username: "hub".into(),
            initial_members: vec![],
            invite_only: false,
        },
        hub_id,
    );
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &create_effects[0]
    else {
        panic!("failed to create");
    };

    assert!(!group.invite_only, "log group must be public");

    // Stranger (never invited) can join
    let stranger_join = hub.handle_payload(
        GroupPayload::Join {
            group_id: log_group_id.clone(),
            username: "stranger".into(),
        },
        stranger_id,
    );

    // Hub accepts and sends Sync (no invite needed)
    assert_eq!(stranger_join.len(), 2); // Sync + MemberJoined
    let GroupAction::Send { payload: GroupPayload::Sync { group: sync_group, .. }, .. } = &stranger_join[0]
    else {
        panic!("expected Sync to stranger");
    };

    assert_eq!(sync_group.members.len(), 2); // hub + stranger
    assert!(sync_group.is_member(&stranger_id));
}
