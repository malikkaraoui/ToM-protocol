/// Runtime-level integration tests (R12.1).
///
/// Two RuntimeState instances exchange envelopes in-memory.
/// No transport, no tokio — pure effect-based testing.
use tom_protocol::{
    MessageStatus, MessageType, ProtocolEvent, RuntimeCommand, RuntimeConfig, RuntimeEffect,
    RuntimeState, StateSnapshot, StateStore,
};

fn keypair(seed: u8) -> (tom_protocol::NodeId, [u8; 32]) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = tom_connect::SecretKey::generate(&mut rng);
    let node_id: tom_protocol::NodeId = secret.public().to_string().parse().unwrap();
    (node_id, secret.to_bytes())
}

fn state_with(seed: u8, encryption: bool) -> RuntimeState {
    let (id, secret) = keypair(seed);
    RuntimeState::new(
        id,
        secret,
        RuntimeConfig {
            encryption,
            username: format!("node-{seed}"),
            ..Default::default()
        },
    )
}

/// Helper: extract envelope bytes from a SendWithBackupFallback or SendEnvelope effect.
fn extract_envelope_bytes(effects: &[RuntimeEffect]) -> Vec<u8> {
    for e in effects {
        match e {
            RuntimeEffect::SendWithBackupFallback { envelope, .. }
            | RuntimeEffect::SendEnvelope(envelope)
            | RuntimeEffect::SendEnvelopeTo { envelope, .. } => {
                return envelope.to_bytes().unwrap();
            }
            _ => {}
        }
    }
    panic!("no envelope in effects: {effects:?}");
}

/// Helper: extract envelope bytes targeted at a specific node.
fn extract_bytes_for(effects: &[RuntimeEffect], target: tom_protocol::NodeId) -> Vec<Vec<u8>> {
    effects
        .iter()
        .filter_map(|e| match e {
            RuntimeEffect::SendEnvelopeTo {
                target: t,
                envelope,
                ..
            } if *t == target => Some(envelope.to_bytes().unwrap()),
            RuntimeEffect::SendEnvelope(env) if env.to == target => {
                Some(env.to_bytes().unwrap())
            }
            RuntimeEffect::SendWithBackupFallback { envelope, .. }
                if envelope.to == target =>
            {
                Some(envelope.to_bytes().unwrap())
            }
            _ => None,
        })
        .collect()
}

// ── Test 1: Two-node encrypted chat exchange ────────────────────────────

#[test]
fn two_node_encrypted_chat_roundtrip() {
    let mut alice = state_with(1, true);
    let mut bob = state_with(2, true);
    let alice_id = alice.local_id();
    let bob_id = bob.local_id();

    // Alice sends encrypted message to Bob
    let send_effects = alice.handle_send_message(bob_id, b"hello bob".to_vec());
    assert!(!send_effects.is_empty(), "should produce send effects");

    // Extract the envelope and deliver to Bob
    let raw = extract_envelope_bytes(&send_effects);
    let recv_effects = bob.handle_incoming(&raw);

    // Bob should get a DeliverMessage with decrypted payload
    let delivered = recv_effects.iter().find_map(|e| {
        if let RuntimeEffect::DeliverMessage(msg) = e {
            Some(msg)
        } else {
            None
        }
    });
    assert!(delivered.is_some(), "Bob should receive the message");
    let msg = delivered.unwrap();
    assert_eq!(msg.from, alice_id);
    assert_eq!(msg.payload, b"hello bob");
    assert!(msg.was_encrypted, "message should have been encrypted");
    assert!(msg.signature_valid, "signature should be valid");

    // Bob should also produce an ACK back to Alice
    let ack_bytes: Vec<Vec<u8>> = recv_effects
        .iter()
        .filter_map(|e| match e {
            RuntimeEffect::SendEnvelope(env) if env.msg_type == MessageType::Ack => {
                Some(env.to_bytes().unwrap())
            }
            _ => None,
        })
        .collect();
    assert!(!ack_bytes.is_empty(), "Bob should send ACK to Alice");

    // Deliver ACK to Alice
    let ack_effects = alice.handle_incoming(&ack_bytes[0]);

    // Alice should see status change to Delivered
    let status = ack_effects.iter().find_map(|e| {
        if let RuntimeEffect::StatusChange(sc) = e {
            Some(sc)
        } else {
            None
        }
    });
    assert!(status.is_some(), "Alice should get status update from ACK");
    assert_eq!(
        status.unwrap().current,
        MessageStatus::Delivered,
        "status should be Delivered after recipient ACK"
    );
}

#[test]
fn two_node_plaintext_chat_roundtrip() {
    let mut alice = state_with(1, false);
    let mut bob = state_with(2, false);
    let alice_id = alice.local_id();

    // Alice sends plaintext message to Bob
    let send_effects = alice.handle_send_message(bob.local_id(), b"plain text".to_vec());
    let raw = extract_envelope_bytes(&send_effects);
    let recv_effects = bob.handle_incoming(&raw);

    let delivered = recv_effects.iter().find_map(|e| {
        if let RuntimeEffect::DeliverMessage(msg) = e {
            Some(msg)
        } else {
            None
        }
    });
    assert!(delivered.is_some());
    let msg = delivered.unwrap();
    assert_eq!(msg.from, alice_id);
    assert_eq!(msg.payload, b"plain text");
    assert!(!msg.was_encrypted, "plaintext message should not be marked encrypted");
}

// ── Test 2: Group lifecycle via RuntimeState ─────────────────────────────

#[test]
fn group_lifecycle_via_runtime_state() {
    // Hub == Alice (self-send interception pattern)
    let mut alice = state_with(1, false);
    let mut bob = state_with(2, false);
    let alice_id = alice.local_id();
    let bob_id = bob.local_id();

    // ── Step 1: Alice creates group (she is the hub) ──
    let create_effects = alice.handle_command(RuntimeCommand::CreateGroup {
        name: "Runtime Test".to_string(),
        hub_relay_id: alice_id, // Alice IS the hub
        initial_members: vec![bob_id],
        invite_only: false,
    });

    // Should produce GroupCreated event (self-send intercepted)
    let has_created = create_effects.iter().any(|e| {
        matches!(e, RuntimeEffect::Emit(ProtocolEvent::GroupCreated { .. }))
    });
    assert!(has_created, "should emit GroupCreated event");

    // Should produce invite SendEnvelope for Bob
    let invite_envelope = extract_bytes_for(&create_effects, bob_id);
    assert!(
        !invite_envelope.is_empty(),
        "should send invite to Bob: {create_effects:?}"
    );

    // ── Step 2: Bob receives invite ──
    let invite_effects = bob.handle_incoming(&invite_envelope[0]);

    // Bob should get GroupInviteReceived event
    let has_invite = invite_effects.iter().any(|e| {
        matches!(e, RuntimeEffect::Emit(ProtocolEvent::GroupInviteReceived { .. }))
    });
    assert!(has_invite, "Bob should receive group invite");

    // Get group_id from Bob's pending invites
    let invites = bob.group_manager().pending_invites();
    assert_eq!(invites.len(), 1, "Bob should have 1 pending invite");
    let group_id = invites[0].group_id.clone();

    // ── Step 3: Bob accepts invite → sends Join to hub (Alice) ──
    let accept_effects = bob.handle_command(RuntimeCommand::AcceptInvite {
        group_id: group_id.clone(),
    });

    // Bob sends Join to Alice (the hub)
    let join_bytes = extract_envelope_bytes(&accept_effects);

    // ── Step 4: Alice (hub) processes Bob's Join ──
    let join_effects = alice.handle_incoming(&join_bytes);

    // Alice should send Sync to Bob
    let sync_bytes = extract_bytes_for(&join_effects, bob_id);
    assert!(
        !sync_bytes.is_empty(),
        "Alice should send Sync to Bob: {join_effects:?}"
    );

    // ── Step 5: Bob receives Sync ──
    let sync_effects = bob.handle_incoming(&sync_bytes[0]);

    // Bob should emit GroupJoined event
    let has_joined = sync_effects.iter().any(|e| {
        matches!(e, RuntimeEffect::Emit(ProtocolEvent::GroupJoined { .. }))
    });
    assert!(has_joined, "Bob should emit GroupJoined event");

    // ── Step 5b: Deliver Bob's SenderKeyDistribution to Alice (hub) ──
    // When Bob joins, he generates a sender key and distributes it to the hub.
    // The hub must receive this before it can decrypt Bob's group messages.
    let sk_bytes = extract_bytes_for(&sync_effects, alice_id);
    for sk in &sk_bytes {
        alice.handle_incoming(sk);
    }

    // Verify both are in the group
    assert!(
        alice.group_manager().is_in_group(&group_id),
        "Alice should be in group"
    );
    assert!(
        bob.group_manager().is_in_group(&group_id),
        "Bob should be in group"
    );

    // ── Step 6: Bob sends a group message ──
    let msg_effects = bob.handle_command(RuntimeCommand::SendGroupMessage {
        group_id: group_id.clone(),
        text: "hello group!".into(),
    });

    // Bob sends GroupMessage to hub (Alice)
    let msg_bytes = extract_envelope_bytes(&msg_effects);

    // Alice (hub) receives and fans out (back to Bob since hub intercepts local delivery)
    let fanout_effects = alice.handle_incoming(&msg_bytes);

    // Hub should get GroupMessageReceived (since she's also a member)
    let alice_got_msg = fanout_effects.iter().any(|e| {
        matches!(e, RuntimeEffect::Emit(ProtocolEvent::GroupMessageReceived { .. }))
    });
    assert!(
        alice_got_msg,
        "Alice (hub+member) should receive group message: {fanout_effects:?}"
    );

    // ── Step 7: Bob leaves ──
    let leave_effects = bob.handle_command(RuntimeCommand::LeaveGroup {
        group_id: group_id.clone(),
    });

    // Bob sends Leave to hub
    let leave_bytes = extract_envelope_bytes(&leave_effects);

    // Alice processes the leave
    let leave_hub_effects = alice.handle_incoming(&leave_bytes);

    // Alice should get MemberLeft event
    let has_left = leave_hub_effects.iter().any(|e| {
        matches!(e, RuntimeEffect::Emit(ProtocolEvent::GroupMemberLeft { .. }))
    });
    assert!(has_left, "Alice should see Bob left");

    // Bob is no longer in the group
    assert!(!bob.group_manager().is_in_group(&group_id));
}

// ── Test 3: Persistence save/restore with groups ─────────────────────────

#[test]
fn persistence_roundtrip_with_groups() {
    let mut alice = state_with(1, false);
    let alice_id = alice.local_id();
    let bob_id = keypair(2).0;

    // Create a group (alice = hub)
    alice.handle_command(RuntimeCommand::CreateGroup {
        name: "Persist Test".to_string(),
        hub_relay_id: alice_id,
        initial_members: vec![],
        invite_only: true,
    });

    // Verify group exists
    let groups: Vec<_> = alice.group_hub().groups().collect();
    assert_eq!(groups.len(), 1);
    let group_id = groups[0].0.clone();

    // Add a peer to topology
    alice.handle_command(RuntimeCommand::UpsertPeer {
        info: tom_protocol::PeerInfo {
            node_id: bob_id,
            role: tom_protocol::PeerRole::Peer,
            status: tom_protocol::PeerStatus::Online,
            last_seen: 42000,
        },
    });

    // Save state to temp SQLite
    let dir = tempfile::tempdir().unwrap();
    let store = StateStore::open(&dir.path().join("test.db")).unwrap();
    let snapshot = StateSnapshot {
        manager: Some(alice.group_manager().snapshot()),
        hub: Some(alice.group_hub().snapshot()),
        peers: alice.topology().peers_map().clone(),
        metrics: alice.role_manager().scores().clone(),
        tracked_messages: alice.tracker().snapshot(),
    };
    store.save(&snapshot).unwrap();

    // Load into a fresh state
    let loaded = store.load().unwrap();

    // Verify groups restored
    let mgr = loaded.manager.unwrap();
    assert!(mgr.groups.contains_key(&group_id));

    // Verify hub state restored
    let hub = loaded.hub.unwrap();
    assert!(hub.groups.contains_key(&group_id));
    assert!(hub.groups[&group_id].invite_only);

    // Verify peers restored
    assert!(loaded.peers.contains_key(&bob_id));
    assert_eq!(loaded.peers[&bob_id].last_seen, 42000);
}

// ── Test 4: Admin controls via RuntimeState ──────────────────────────────

#[test]
fn admin_kick_via_runtime_command() {
    let mut alice = state_with(1, false);
    let mut bob = state_with(2, false);
    let alice_id = alice.local_id();
    let bob_id = bob.local_id();

    // Alice creates group as hub
    let create_effects = alice.handle_command(RuntimeCommand::CreateGroup {
        name: "Kick Test".to_string(),
        hub_relay_id: alice_id,
        initial_members: vec![bob_id],
        invite_only: false,
    });

    // Deliver invite to Bob
    let invite_bytes = extract_bytes_for(&create_effects, bob_id);
    assert!(!invite_bytes.is_empty());
    bob.handle_incoming(&invite_bytes[0]);

    let group_id = bob.group_manager().pending_invites()[0].group_id.clone();

    // Bob joins
    let accept = bob.handle_command(RuntimeCommand::AcceptInvite {
        group_id: group_id.clone(),
    });
    let join_bytes = extract_envelope_bytes(&accept);
    let join_effects = alice.handle_incoming(&join_bytes);

    // Deliver Sync to Bob
    let sync_bytes = extract_bytes_for(&join_effects, bob_id);
    assert!(!sync_bytes.is_empty());
    bob.handle_incoming(&sync_bytes[0]);

    assert!(bob.group_manager().is_in_group(&group_id));

    // Alice kicks Bob
    let kick_effects = alice.handle_command(RuntimeCommand::KickMember {
        group_id: group_id.clone(),
        target_id: bob_id,
    });

    // Should produce a broadcast to Bob (MemberLeft with Kicked)
    let kick_bytes = extract_bytes_for(&kick_effects, bob_id);
    assert!(
        !kick_bytes.is_empty(),
        "should send kick notification to Bob: {kick_effects:?}"
    );

    // Bob processes kick
    let bob_kick_effects = bob.handle_incoming(&kick_bytes[0]);

    // Bob should emit MemberLeft event
    let has_kicked = bob_kick_effects.iter().any(|e| {
        matches!(e, RuntimeEffect::Emit(ProtocolEvent::GroupMemberLeft { .. }))
    });
    assert!(has_kicked, "Bob should receive kicked notification");

    // Bob is no longer in the group
    assert!(!bob.group_manager().is_in_group(&group_id));
}
