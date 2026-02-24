/// Integration test: full group lifecycle.
///
/// Tests the GroupManager and GroupHub working together
/// without transport — pure in-memory message passing.
///
/// Scenario: Alice creates a group, invites Bob and Charlie.
/// Bob accepts, Charlie declines. Alice sends a message,
/// Bob receives it. Bob leaves. Hub election on failure.
use tom_protocol::{
    elect_hub, ElectionReason, GroupAction, GroupEvent, GroupHub, GroupId, GroupInfo, GroupManager,
    GroupMessage, GroupPayload, LeaveReason, NodeId, PeerInfo, PeerRole, PeerStatus, Topology,
};

fn node_id(seed: u8) -> NodeId {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = iroh::SecretKey::generate(&mut rng);
    secret.public().to_string().parse().unwrap()
}

/// Simulate the full lifecycle: create → invite → join → message → leave.
#[test]
fn full_group_lifecycle() {
    let alice_id = node_id(1);
    let bob_id = node_id(2);
    let charlie_id = node_id(3);
    let hub_id = node_id(10);

    let mut alice = GroupManager::new(alice_id, "alice".into());
    let mut bob = GroupManager::new(bob_id, "bob".into());
    let mut charlie = GroupManager::new(charlie_id, "charlie".into());
    let mut hub = GroupHub::new(hub_id);

    // ── Step 1: Alice creates a group ────────────────────────────────
    let create_actions = alice.create_group(
        "Test Group".into(),
        hub_id,
        vec![bob_id, charlie_id],
    );

    // Alice's action: Send Create to hub
    assert_eq!(create_actions.len(), 1);
    let GroupAction::Send { to, payload } = &create_actions[0] else {
        panic!("expected Send");
    };
    assert_eq!(*to, hub_id);

    // Deliver to hub
    let hub_actions = hub.handle_payload(payload.clone(), alice_id);

    // Hub responds: Created (to alice) + Invite (to bob) + Invite (to charlie)
    assert_eq!(hub_actions.len(), 3);

    // Deliver Created to Alice
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &hub_actions[0]
    else {
        panic!("expected Created");
    };
    let group_id = group.group_id.clone();
    alice.handle_group_created(group.clone());
    assert!(alice.is_in_group(&group_id));

    // Deliver Invite to Bob
    let GroupAction::Send { to: _, payload: GroupPayload::Invite { group_id: gid, group_name, inviter_id, inviter_username } } = &hub_actions[1]
    else {
        panic!("expected Invite");
    };
    bob.handle_invite(
        gid.clone(),
        group_name.clone(),
        *inviter_id,
        inviter_username.clone(),
        hub_id,
    );
    assert_eq!(bob.pending_invites().len(), 1);

    // Deliver Invite to Charlie
    let GroupAction::Send { payload: GroupPayload::Invite { group_id: gid2, group_name: gn2, inviter_id: inv2, inviter_username: iu2 }, .. } = &hub_actions[2]
    else {
        panic!("expected Invite to charlie");
    };
    charlie.handle_invite(gid2.clone(), gn2.clone(), *inv2, iu2.clone(), hub_id);

    // ── Step 2: Bob accepts, Charlie declines ────────────────────────
    let accept_actions = bob.accept_invite(&group_id);
    assert_eq!(accept_actions.len(), 1);

    // Bob sends Join to hub
    let GroupAction::Send { payload: join_payload, .. } = &accept_actions[0] else {
        panic!("expected Send Join");
    };
    let hub_join_actions = hub.handle_payload(join_payload.clone(), bob_id);

    // Hub responds: Sync (to bob) + MemberJoined broadcast (to alice)
    assert_eq!(hub_join_actions.len(), 2);

    // Deliver Sync to Bob
    let GroupAction::Send { payload: GroupPayload::Sync { group: sync_group, recent_messages }, .. } = &hub_join_actions[0]
    else {
        panic!("expected Sync");
    };
    bob.handle_group_sync(sync_group.clone(), recent_messages.clone());
    assert!(bob.is_in_group(&group_id));

    // Deliver MemberJoined to Alice
    let GroupAction::Broadcast { payload: GroupPayload::MemberJoined { member, .. }, .. } = &hub_join_actions[1]
    else {
        panic!("expected MemberJoined broadcast");
    };
    alice.handle_member_joined(&group_id, member.clone());
    assert_eq!(alice.get_group(&group_id).unwrap().member_count(), 2);

    // Charlie declines
    assert!(charlie.decline_invite(&group_id));
    assert_eq!(charlie.pending_invites().len(), 0);
    assert!(!charlie.is_in_group(&group_id));

    // ── Step 3: Alice sends a message ────────────────────────────────
    let msg = GroupMessage::new(
        group_id.clone(),
        alice_id,
        "alice".into(),
        "Hello group!".into(),
    );

    // Deliver message to hub
    let fanout = hub.handle_payload(GroupPayload::Message(msg.clone()), alice_id);
    assert_eq!(fanout.len(), 1);

    // Hub fans out to Bob (not Alice, since she's the sender)
    let GroupAction::Broadcast { to, payload: GroupPayload::Message(fanned_msg) } = &fanout[0]
    else {
        panic!("expected Broadcast Message");
    };
    assert_eq!(to.len(), 1);
    assert!(to.contains(&bob_id));
    assert!(!to.contains(&alice_id));

    // Bob receives the message
    let bob_msg_actions = bob.handle_message(fanned_msg.clone());
    assert_eq!(bob_msg_actions.len(), 1);
    assert_eq!(bob.message_history(&group_id).len(), 1);
    assert_eq!(bob.message_history(&group_id)[0].text, "Hello group!");

    // ── Step 4: Bob leaves ───────────────────────────────────────────
    let leave_actions = bob.leave_group(&group_id);
    assert_eq!(leave_actions.len(), 1);

    // Deliver Leave to hub
    let GroupAction::Send { payload: leave_payload, .. } = &leave_actions[0] else {
        panic!("expected Send Leave");
    };
    let hub_leave_actions = hub.handle_payload(leave_payload.clone(), bob_id);

    // Hub broadcasts MemberLeft to Alice
    assert_eq!(hub_leave_actions.len(), 1);
    let GroupAction::Broadcast { payload: GroupPayload::MemberLeft { node_id, reason, .. }, .. } = &hub_leave_actions[0]
    else {
        panic!("expected MemberLeft broadcast");
    };
    assert_eq!(*node_id, bob_id);
    assert_eq!(*reason, LeaveReason::Voluntary);

    // Alice processes the departure
    alice.handle_member_left(&group_id, &bob_id, "bob".into(), LeaveReason::Voluntary);
    assert_eq!(alice.get_group(&group_id).unwrap().member_count(), 1);

    // Bob is no longer in the group
    assert!(!bob.is_in_group(&group_id));
}

/// Test hub election when current hub fails.
#[test]
fn hub_election_on_failure() {
    let hub1 = node_id(10);
    let hub2 = node_id(11);
    let hub3 = node_id(12);

    let group = GroupInfo {
        group_id: GroupId::from("grp-election".to_string()),
        name: "Election Test".into(),
        hub_relay_id: hub1,
        backup_hub_id: Some(hub2),
        members: vec![],
        created_by: node_id(1),
        created_at: 1000,
        last_activity_at: 1000,
        max_members: 50,
        shadow_id: None,
        candidate_id: None,
    };

    let mut topology = Topology::new();
    // hub2 (backup) is online
    topology.upsert(PeerInfo {
        node_id: hub2,
        role: PeerRole::Relay,
        status: PeerStatus::Online,
        last_seen: 2000,
    });
    topology.upsert(PeerInfo {
        node_id: hub3,
        role: PeerRole::Relay,
        status: PeerStatus::Online,
        last_seen: 3000,
    });

    // Hub1 fails → election should pick hub2 (backup)
    let result = elect_hub(&group, &hub1, &topology);
    assert_eq!(result.new_hub_id, Some(hub2));
    assert_eq!(result.reason, ElectionReason::Backup);

    // If backup also fails → deterministic selection from remaining
    let result2 = elect_hub(&group, &hub2, &topology);
    assert_eq!(result2.new_hub_id, Some(hub3));
    assert_eq!(result2.reason, ElectionReason::Deterministic);
}

/// Test admin kick flow through hub.
#[test]
fn admin_kick_flow() {
    let alice_id = node_id(1);
    let bob_id = node_id(2);
    let hub_id = node_id(10);

    let mut alice = GroupManager::new(alice_id, "alice".into());
    let mut bob = GroupManager::new(bob_id, "bob".into());
    let mut hub = GroupHub::new(hub_id);

    // Create and populate group
    let create_actions = alice.create_group("Kick Test".into(), hub_id, vec![]);
    let GroupAction::Send { payload, .. } = &create_actions[0] else {
        panic!()
    };
    let hub_actions = hub.handle_payload(payload.clone(), alice_id);
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &hub_actions[0]
    else {
        panic!()
    };
    let gid = group.group_id.clone();
    alice.handle_group_created(group.clone());

    // Bob joins
    let join_actions = hub.handle_payload(
        GroupPayload::Join {
            group_id: gid.clone(),
            username: "bob".into(),
        },
        bob_id,
    );
    let GroupAction::Send { payload: GroupPayload::Sync { group: sg, recent_messages: rm }, .. } = &join_actions[0]
    else {
        panic!()
    };
    bob.handle_group_sync(sg.clone(), rm.clone());

    // Alice (admin) kicks Bob via hub
    let kick_actions = hub.kick_member(&gid, &alice_id, &bob_id);
    assert_eq!(kick_actions.len(), 1);

    let GroupAction::Broadcast { to, payload: GroupPayload::MemberLeft { reason, .. } } = &kick_actions[0]
    else {
        panic!("expected MemberLeft broadcast");
    };
    assert_eq!(*reason, LeaveReason::Kicked);
    // Both alice and bob notified
    assert!(to.contains(&alice_id));
    assert!(to.contains(&bob_id));

    // Hub should show only alice
    assert_eq!(hub.get_group(&gid).unwrap().member_count(), 1);
}

/// Test that hub rate-limits spam from a single sender.
#[test]
fn hub_rate_limits_spam() {
    let alice_id = node_id(1);
    let bob_id = node_id(2);
    let hub_id = node_id(10);

    let mut hub = GroupHub::new(hub_id);

    // Create group
    let hub_actions = hub.handle_payload(
        GroupPayload::Create {
            group_name: "Spam Test".into(),
            creator_username: "alice".into(),
            initial_members: vec![],
        },
        alice_id,
    );

    // Extract group_id from Created response
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &hub_actions[0]
    else {
        panic!("expected Created")
    };
    let gid = group.group_id.clone();

    // Bob joins
    hub.handle_payload(
        GroupPayload::Join {
            group_id: gid.clone(),
            username: "bob".into(),
        },
        bob_id,
    );

    // Send 10 messages — rate limit is 5/sec
    let mut delivered = 0;
    let mut blocked = 0;
    for i in 0..10 {
        let msg = GroupMessage::new(
            gid.clone(),
            alice_id,
            "alice".into(),
            format!("msg-{}", i),
        );
        let actions = hub.handle_payload(GroupPayload::Message(msg), alice_id);
        if actions.is_empty() {
            blocked += 1;
        } else {
            delivered += 1;
        }
    }

    assert_eq!(delivered, 5, "should deliver exactly 5 messages");
    assert_eq!(blocked, 5, "should block 5 messages");
}

fn secret_seed(seed: u8) -> [u8; 32] {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = iroh::SecretKey::generate(&mut rng);
    secret.to_bytes()
}

/// Non-member message is rejected with SecurityViolation.
#[test]
fn non_member_message_rejected() {
    let alice_id = node_id(1);
    let stranger_id = node_id(99);
    let hub_id = node_id(10);

    let mut hub = GroupHub::new(hub_id);

    // Create group (Alice only)
    let hub_actions = hub.handle_payload(
        GroupPayload::Create {
            group_name: "Secure Group".into(),
            creator_username: "alice".into(),
            initial_members: vec![],
        },
        alice_id,
    );
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &hub_actions[0]
    else {
        panic!("expected Created")
    };
    let gid = group.group_id.clone();

    // Stranger tries to send a message
    let msg = GroupMessage::new(gid.clone(), stranger_id, "stranger".into(), "I shouldn't be here".into());
    let actions = hub.handle_payload(GroupPayload::Message(msg), stranger_id);

    assert_eq!(actions.len(), 1);
    let GroupAction::Event(GroupEvent::SecurityViolation { group_id, node_id, reason }) = &actions[0]
    else {
        panic!("expected SecurityViolation event, got: {actions:?}");
    };
    assert_eq!(*group_id, gid);
    assert_eq!(*node_id, stranger_id);
    assert!(reason.contains("non-member"), "reason: {reason}");
}

/// Signed message round-trips through hub with valid signature.
#[test]
fn signed_message_passes_hub() {
    let alice_id = node_id(1);
    let bob_id = node_id(2);
    let hub_id = node_id(10);
    let alice_seed = secret_seed(1);

    let mut hub = GroupHub::new(hub_id);

    // Create group with Alice
    let hub_actions = hub.handle_payload(
        GroupPayload::Create {
            group_name: "Signed Group".into(),
            creator_username: "alice".into(),
            initial_members: vec![],
        },
        alice_id,
    );
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &hub_actions[0]
    else {
        panic!()
    };
    let gid = group.group_id.clone();

    // Bob joins
    hub.handle_payload(
        GroupPayload::Join { group_id: gid.clone(), username: "bob".into() },
        bob_id,
    );

    // Alice sends a signed message
    let mut msg = GroupMessage::new(gid.clone(), alice_id, "alice".into(), "Signed hello!".into());
    msg.sign(&alice_seed);
    assert!(msg.verify_signature());

    let actions = hub.handle_payload(GroupPayload::Message(msg), alice_id);
    assert_eq!(actions.len(), 1, "signed message should pass hub");

    let GroupAction::Broadcast { to, payload: GroupPayload::Message(fanned) } = &actions[0]
    else {
        panic!("expected Broadcast");
    };
    assert!(to.contains(&bob_id));
    assert!(fanned.verify_signature(), "signature should survive fan-out");
}

/// Tampered signature is detected by hub.
#[test]
fn tampered_signature_detected() {
    let alice_id = node_id(1);
    let bob_id = node_id(2);
    let hub_id = node_id(10);
    let alice_seed = secret_seed(1);

    let mut hub = GroupHub::new(hub_id);

    // Create group and add bob
    let hub_actions = hub.handle_payload(
        GroupPayload::Create {
            group_name: "Tamper Test".into(),
            creator_username: "alice".into(),
            initial_members: vec![],
        },
        alice_id,
    );
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &hub_actions[0]
    else {
        panic!()
    };
    let gid = group.group_id.clone();

    hub.handle_payload(
        GroupPayload::Join { group_id: gid.clone(), username: "bob".into() },
        bob_id,
    );

    // Alice signs, then tamper with the text
    let mut msg = GroupMessage::new(gid.clone(), alice_id, "alice".into(), "Original".into());
    msg.sign(&alice_seed);
    msg.text = "Tampered!".into(); // Tamper after signing

    let actions = hub.handle_payload(GroupPayload::Message(msg), alice_id);
    assert_eq!(actions.len(), 1);

    let GroupAction::Event(GroupEvent::SecurityViolation { reason, .. }) = &actions[0]
    else {
        panic!("expected SecurityViolation, got: {actions:?}");
    };
    assert!(reason.contains("invalid"), "reason: {reason}");
}

/// Test E2E encrypted group messaging with Sender Keys.
#[test]
fn encrypted_group_messaging_e2e() {
    let alice_id = node_id(1);
    let bob_id = node_id(2);
    let hub_id = node_id(10);
    let alice_seed = secret_seed(1);
    let bob_seed = secret_seed(2);

    let mut alice = GroupManager::new(alice_id, "alice".into());
    let mut bob = GroupManager::new(bob_id, "bob".into());
    let mut hub = GroupHub::new(hub_id);

    // ── Step 1: Alice creates group ──────────────────────
    let create_actions = alice.create_group("E2E Test".into(), hub_id, vec![bob_id]);
    let GroupAction::Send { payload, .. } = &create_actions[0] else {
        panic!()
    };
    let hub_actions = hub.handle_payload(payload.clone(), alice_id);

    let GroupAction::Send {
        payload: GroupPayload::Created { group },
        ..
    } = &hub_actions[0]
    else {
        panic!()
    };
    let gid = group.group_id.clone();
    alice.handle_group_created(group.clone());
    assert!(alice.local_sender_key(&gid).is_some());

    // ── Step 2: Bob joins ────────────────────────────────
    let GroupAction::Send {
        payload:
            GroupPayload::Invite {
                group_id,
                group_name,
                inviter_id,
                inviter_username,
            },
        ..
    } = &hub_actions[1]
    else {
        panic!()
    };
    bob.handle_invite(
        group_id.clone(),
        group_name.clone(),
        *inviter_id,
        inviter_username.clone(),
        hub_id,
    );
    let join_actions = bob.accept_invite(&gid);
    let GroupAction::Send {
        payload: join_payload,
        ..
    } = &join_actions[0]
    else {
        panic!()
    };
    let hub_join_actions = hub.handle_payload(join_payload.clone(), bob_id);

    // Deliver Sync to Bob (generates Bob's sender key + distribution)
    let GroupAction::Send {
        payload:
            GroupPayload::Sync {
                group: sync_group,
                recent_messages,
            },
        ..
    } = &hub_join_actions[0]
    else {
        panic!()
    };
    bob.handle_group_sync(sync_group.clone(), recent_messages.clone());
    assert!(bob.local_sender_key(&gid).is_some());

    // Deliver MemberJoined to Alice (triggers Alice distributing her key to Bob)
    let GroupAction::Broadcast {
        payload: GroupPayload::MemberJoined { member, .. },
        ..
    } = &hub_join_actions[1]
    else {
        panic!()
    };
    alice.handle_member_joined(&gid, member.clone());

    // ── Step 3: Key exchange ─────────────────────────────

    // Alice -> Bob: Alice's sender key (via hub)
    let alice_dist = alice.build_sender_key_distribution(&gid);
    if !alice_dist.is_empty() {
        let GroupAction::Send {
            payload: dist_payload,
            ..
        } = &alice_dist[0]
        else {
            panic!()
        };
        let hub_dist = hub.handle_payload(dist_payload.clone(), alice_id);
        for action in &hub_dist {
            if let GroupAction::Send {
                to,
                payload:
                    GroupPayload::SenderKeyDistribution {
                        from,
                        epoch,
                        encrypted_keys,
                        ..
                    },
            } = action
            {
                if *to == bob_id {
                    bob.handle_sender_key_distribution(
                        &gid,
                        *from,
                        *epoch,
                        encrypted_keys,
                        &bob_seed,
                    );
                }
            }
        }
    }

    // Bob -> Alice: Bob's sender key (via hub)
    let bob_dist = bob.build_sender_key_distribution(&gid);
    if !bob_dist.is_empty() {
        let GroupAction::Send {
            payload: dist_payload,
            ..
        } = &bob_dist[0]
        else {
            panic!()
        };
        let hub_dist2 = hub.handle_payload(dist_payload.clone(), bob_id);
        for action in &hub_dist2 {
            if let GroupAction::Send {
                to,
                payload:
                    GroupPayload::SenderKeyDistribution {
                        from,
                        epoch,
                        encrypted_keys,
                        ..
                    },
            } = action
            {
                if *to == alice_id {
                    alice.handle_sender_key_distribution(
                        &gid,
                        *from,
                        *epoch,
                        encrypted_keys,
                        &alice_seed,
                    );
                }
            }
        }
    }

    // Verify key exchange
    assert!(
        bob.get_sender_key(&gid, &alice_id).is_some(),
        "Bob should have Alice's key"
    );
    assert!(
        alice.get_sender_key(&gid, &bob_id).is_some(),
        "Alice should have Bob's key"
    );

    // ── Step 4: Alice sends encrypted message ────────────
    let alice_key = alice.local_sender_key(&gid).unwrap();
    let mut msg = GroupMessage::new_encrypted(
        gid.clone(),
        alice_id,
        "alice".into(),
        "Top secret message!".into(),
        &alice_key.key,
        alice_key.epoch,
    );
    msg.sign(&alice_seed);

    // Hub processes — should NOT see plaintext
    assert!(msg.text.is_empty());
    assert!(msg.encrypted);

    let fanout = hub.handle_payload(GroupPayload::Message(msg), alice_id);
    let GroupAction::Broadcast {
        to,
        payload: GroupPayload::Message(fanned_msg),
    } = &fanout[0]
    else {
        panic!()
    };
    assert!(to.contains(&bob_id));

    // Bob decrypts
    let bob_actions = bob.handle_message(fanned_msg.clone());
    assert_eq!(bob_actions.len(), 1);
    let GroupAction::Event(GroupEvent::MessageReceived(delivered)) = &bob_actions[0] else {
        panic!()
    };
    assert_eq!(delivered.text, "Top secret message!");
    assert_eq!(delivered.sender_username, "alice");
}

/// Test that shadow is assigned when a group has enough members.
#[test]
fn shadow_assigned_after_group_setup() {
    let hub_id = node_id(10);
    let alice_id = node_id(1);
    let bob_id = node_id(2);

    let mut hub = GroupHub::new(hub_id);

    hub.handle_payload(
        GroupPayload::Create {
            group_name: "Failover Test".into(),
            creator_username: "alice".into(),
            initial_members: vec![bob_id],
        },
        alice_id,
    );
    let gid = hub.groups().next().unwrap().0.clone();
    hub.handle_payload(
        GroupPayload::Join {
            group_id: gid.clone(),
            username: "alice".into(),
        },
        alice_id,
    );
    hub.handle_payload(
        GroupPayload::Join {
            group_id: gid.clone(),
            username: "bob".into(),
        },
        bob_id,
    );

    let actions = hub.assign_shadow(&gid);
    assert!(!actions.is_empty());

    // Verify the group now has a shadow
    let group = hub.get_group(&gid).unwrap();
    assert!(group.shadow_id.is_some());
}

/// Test complete hub failover: primary dies -> shadow promotes -> members re-route.
#[test]
fn hub_failover_shadow_promotes_on_primary_death() {
    let hub_id = node_id(10);
    let shadow_id = node_id(2);
    let candidate_id = node_id(3);
    let alice_id = node_id(1);

    // ── Setup: create group with hub, shadow, candidate ──
    let mut hub = GroupHub::new(hub_id);
    let mut shadow_mgr = GroupManager::new(shadow_id, "shadow".into());
    let mut alice_mgr = GroupManager::new(alice_id, "alice".into());

    // Create group on hub
    let hub_actions = hub.handle_payload(
        GroupPayload::Create {
            group_name: "Failover E2E".into(),
            creator_username: "alice".into(),
            initial_members: vec![],
        },
        alice_id,
    );
    let GroupAction::Send { payload: GroupPayload::Created { group }, .. } = &hub_actions[0]
    else { panic!() };
    let gid = group.group_id.clone();
    alice_mgr.handle_group_created(group.clone());

    // Join shadow and candidate via handle_payload (handle_join is private)
    hub.handle_payload(
        GroupPayload::Join {
            group_id: gid.clone(),
            username: "shadow".into(),
        },
        shadow_id,
    );
    hub.handle_payload(
        GroupPayload::Join {
            group_id: gid.clone(),
            username: "candidate".into(),
        },
        candidate_id,
    );

    // Shadow needs the group in its local state to accept shadow sync
    shadow_mgr.handle_group_created(GroupInfo {
        hub_relay_id: hub_id,
        ..group.clone()
    });

    // Hub assigns shadow
    let assign_actions = hub.assign_shadow(&gid);
    assert!(!assign_actions.is_empty());

    // Deliver HubShadowSync to shadow
    for action in &assign_actions {
        if let GroupAction::Send {
            to,
            payload: GroupPayload::HubShadowSync {
                group_id,
                members,
                candidate_id: cand,
                config_version,
            },
        } = action
        {
            if *to == shadow_id {
                shadow_mgr.handle_shadow_sync(group_id, members.clone(), *cand, *config_version);
            }
        }
    }
    assert!(shadow_mgr.is_shadow_for(&gid));

    // ── Simulate primary death: 2 ping failures ──
    let fail1 = shadow_mgr.record_ping_failure(&gid);
    assert!(fail1.is_empty());

    let fail2 = shadow_mgr.record_ping_failure(&gid);
    assert!(!fail2.is_empty(), "should promote after 2 failures");

    // Verify HubMigration is broadcast
    let has_migration = fail2.iter().any(|a| {
        matches!(
            a,
            GroupAction::Broadcast {
                payload: GroupPayload::HubMigration { new_hub_id, .. },
                ..
            } if *new_hub_id == shadow_id
        )
    });
    assert!(has_migration, "shadow should broadcast HubMigration with itself as new hub");

    // Shadow is no longer shadow (it's now hub)
    assert!(!shadow_mgr.is_shadow_for(&gid));

    // ── Alice receives migration and re-routes ──
    let alice_actions = alice_mgr.handle_hub_migration(&gid, shadow_id);
    assert_eq!(alice_actions.len(), 1);
    assert!(matches!(
        &alice_actions[0],
        GroupAction::Event(GroupEvent::HubMigrated { new_hub_id, .. }) if *new_hub_id == shadow_id
    ));

    // Verify Alice now routes to shadow as hub
    let alice_group = alice_mgr.get_group(&gid).unwrap();
    assert_eq!(alice_group.hub_relay_id, shadow_id);
}

/// Test that an ex-member cannot decrypt messages after key rotation.
#[test]
fn key_rotation_forward_secrecy() {
    let alice_id = node_id(1);
    let eve_id = node_id(3);
    let hub_id = node_id(10);

    let mut alice = GroupManager::new(alice_id, "alice".into());
    let mut hub = GroupHub::new(hub_id);

    // Create group
    let create_actions = alice.create_group("Rotation Test".into(), hub_id, vec![]);
    let GroupAction::Send { payload, .. } = &create_actions[0] else {
        panic!()
    };
    let hub_actions = hub.handle_payload(payload.clone(), alice_id);
    let GroupAction::Send {
        payload: GroupPayload::Created { group },
        ..
    } = &hub_actions[0]
    else {
        panic!()
    };
    let gid = group.group_id.clone();
    alice.handle_group_created(group.clone());

    // Eve joins via hub
    hub.handle_payload(
        GroupPayload::Join {
            group_id: gid.clone(),
            username: "eve".into(),
        },
        eve_id,
    );
    alice.handle_member_joined(
        &gid,
        tom_protocol::GroupMember {
            node_id: eve_id,
            username: "eve".into(),
            joined_at: 1000,
            role: tom_protocol::GroupMemberRole::Member,
        },
    );

    let old_key = alice.local_sender_key(&gid).unwrap().key;

    // Eve leaves via hub -> Alice rotates her key
    hub.handle_payload(
        GroupPayload::Leave {
            group_id: gid.clone(),
        },
        eve_id,
    );
    alice.handle_member_left(&gid, &eve_id, "eve".into(), LeaveReason::Voluntary);

    let new_sender_key = alice.local_sender_key(&gid).unwrap();
    let new_key = new_sender_key.key;
    let new_epoch = new_sender_key.epoch;
    assert_ne!(old_key, new_key, "key should have rotated");

    // New message with rotated key
    let msg = GroupMessage::new_encrypted(
        gid,
        alice_id,
        "alice".into(),
        "Post-rotation secret".into(),
        &new_key,
        new_epoch,
    );

    // Eve's old key fails to decrypt the new message
    assert!(
        msg.decrypt(&old_key).is_err(),
        "old key should not decrypt new message"
    );
}
