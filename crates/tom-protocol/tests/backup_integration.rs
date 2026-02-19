/// Integration tests: backup subsystem (virus metaphor).
///
/// Tests BackupStore + BackupCoordinator working together
/// without transport — pure in-memory simulation.
///
/// Scenario: Messages stored for offline recipient, replicated
/// across nodes, delivered when recipient comes online, cleaned up.
use tom_protocol::{
    BackupAction, BackupCoordinator, BackupEvent, HostFactors, NodeId, ReplicationPayload,
};

fn node_id(seed: u8) -> NodeId {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = iroh::SecretKey::generate(&mut rng);
    secret.public().to_string().parse().unwrap()
}

/// Full lifecycle: store → replicate → deliver → cleanup.
#[test]
fn full_backup_lifecycle() {
    let relay1 = node_id(10);
    let relay2 = node_id(11);
    let alice = node_id(1); // offline recipient
    let bob = node_id(2); // sender
    let now = 100_000u64;

    let mut coord1 = BackupCoordinator::new(relay1);
    let mut coord2 = BackupCoordinator::new(relay2);

    // ── Step 1: Bob sends message, relay1 stores backup ─────────────────
    let actions = coord1.store_message("msg-1".into(), vec![42], alice, bob, now, None);
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], BackupAction::Event(BackupEvent::MessageStored { .. })));
    assert_eq!(coord1.store().message_count(), 1);

    // ── Step 2: Relay1 replicates to relay2 ─────────────────────────────
    let actions = coord1.replicate_to("msg-1", relay2, now);
    assert_eq!(actions.len(), 1);
    let BackupAction::Replicate { target, payload } = &actions[0] else {
        panic!("expected Replicate");
    };
    assert_eq!(*target, relay2);

    // Relay2 receives replication
    let actions = coord2.handle_replication(payload, relay1, now);
    assert!(coord2.store().has("msg-1"));
    assert!(!actions.is_empty());

    // Relay1 receives ACK
    let actions = coord1.handle_replication_ack("msg-1", relay2);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        actions[0],
        BackupAction::Event(BackupEvent::MessageReplicated { .. })
    ));

    // Both have the message
    assert!(coord1.store().has("msg-1"));
    assert!(coord2.store().has("msg-1"));

    // ── Step 3: Alice comes online, relay1 queries ──────────────────────
    let actions = coord1.query_pending(alice, now + 1000);
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], BackupAction::QueryPending { .. }));

    // ── Step 4: Alice confirms delivery ─────────────────────────────────
    let actions = coord1.confirm_delivery(&["msg-1".into()], alice);

    // Should have delivered event + confirm broadcast
    let has_delivered = actions.iter().any(|a| {
        matches!(a, BackupAction::Event(BackupEvent::MessageDelivered { .. }))
    });
    let has_confirm = actions.iter().any(|a| {
        matches!(a, BackupAction::ConfirmDelivery { .. })
    });
    assert!(has_delivered);
    assert!(has_confirm);

    // Relay1 cleaned up
    assert!(!coord1.store().has("msg-1"));

    // ── Step 5: Relay2 receives delivery confirmation ───────────────────
    let actions = coord2.handle_delivery_confirmation(&["msg-1".into()]);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        actions[0],
        BackupAction::Event(BackupEvent::MessageDelivered { .. })
    ));

    // Both stores empty
    assert_eq!(coord1.store().message_count(), 0);
    assert_eq!(coord2.store().message_count(), 0);
}

/// TTL expiry: messages self-delete after TTL.
#[test]
fn ttl_expiry() {
    let relay = node_id(10);
    let alice = node_id(1);
    let bob = node_id(2);
    let now = 100_000u64;

    let mut coord = BackupCoordinator::new(relay);

    // Store with short TTL (5 seconds)
    coord.store_message("msg-1".into(), vec![], alice, bob, now, Some(5000));
    coord.store_message("msg-2".into(), vec![], alice, bob, now, Some(10_000));

    // Before expiry — both alive
    let actions = coord.tick(now + 3000);
    let expired = actions
        .iter()
        .filter(|a| matches!(a, BackupAction::Event(BackupEvent::MessageExpired { .. })))
        .count();
    assert_eq!(expired, 0);
    assert_eq!(coord.store().message_count(), 2);

    // msg-1 expires at now+5000
    let actions = coord.tick(now + 6000);
    let expired = actions
        .iter()
        .filter(|a| matches!(a, BackupAction::Event(BackupEvent::MessageExpired { .. })))
        .count();
    assert_eq!(expired, 1);
    assert_eq!(coord.store().message_count(), 1);
    assert!(!coord.store().has("msg-1"));
    assert!(coord.store().has("msg-2"));

    // msg-2 expires at now+10000
    let actions = coord.tick(now + 11_000);
    let expired = actions
        .iter()
        .filter(|a| matches!(a, BackupAction::Event(BackupEvent::MessageExpired { .. })))
        .count();
    assert_eq!(expired, 1);
    assert_eq!(coord.store().message_count(), 0);
}

/// Viability monitoring: low viability triggers replication needed.
#[test]
fn viability_triggers_replication() {
    let relay = node_id(10);
    let alice = node_id(1);
    let bob = node_id(2);
    let now = 100_000u64;

    let mut coord = BackupCoordinator::new(relay);
    coord.store_message("msg-1".into(), vec![], alice, bob, now, None);

    // Healthy host — no replication needed
    let actions = coord.tick(now + 100);
    let repl_needed = actions
        .iter()
        .filter(|a| matches!(a, BackupAction::Event(BackupEvent::ReplicationNeeded { .. })))
        .count();
    assert_eq!(repl_needed, 0);

    // Degraded host — replication needed
    coord.store_mut().update_host_factors(HostFactors {
        stability: 0,
        bandwidth: 0,
        contribution: 0,
    });

    let actions = coord.tick(now + 200);
    let repl_needed = actions
        .iter()
        .filter(|a| matches!(a, BackupAction::Event(BackupEvent::ReplicationNeeded { .. })))
        .count();
    assert!(repl_needed >= 1);
}

/// Critically low viability recommends self-deletion.
#[test]
fn critical_viability_self_delete() {
    let relay = node_id(10);
    let alice = node_id(1);
    let bob = node_id(2);
    let now = 100_000u64;

    let mut coord = BackupCoordinator::new(relay);
    coord.store_message("msg-1".into(), vec![], alice, bob, now, None);

    // Critically bad host + low message viability
    coord.store_mut().update_host_factors(HostFactors {
        stability: 0,
        bandwidth: 0,
        contribution: 0,
    });
    coord.store_mut().update_viability("msg-1", 5);

    let actions = coord.tick(now + 100);
    let self_delete = actions
        .iter()
        .filter(|a| matches!(a, BackupAction::Event(BackupEvent::SelfDeleteRecommended { .. })))
        .count();
    assert!(self_delete >= 1);
}

/// Replication prevents expiry-induced data loss.
#[test]
fn replication_survives_node_failure() {
    let relay1 = node_id(10);
    let relay2 = node_id(11);
    let alice = node_id(1);
    let bob = node_id(2);
    let now = 100_000u64;

    let mut coord1 = BackupCoordinator::new(relay1);
    let mut coord2 = BackupCoordinator::new(relay2);

    // Store on relay1
    coord1.store_message("msg-1".into(), vec![42], alice, bob, now, Some(60_000));

    // Replicate to relay2
    let actions = coord1.replicate_to("msg-1", relay2, now);
    let BackupAction::Replicate { payload, .. } = &actions[0] else {
        panic!("expected Replicate");
    };
    coord2.handle_replication(payload, relay1, now);

    // Relay1 "fails" (we drop coord1)
    drop(coord1);

    // Relay2 still has the message
    assert!(coord2.store().has("msg-1"));
    let entry = coord2.store().get("msg-1").unwrap();
    assert_eq!(entry.payload, vec![42]);
    assert!(!entry.is_expired(now + 30_000));

    // Alice comes online, relay2 delivers
    let actions = coord2.confirm_delivery(&["msg-1".into()], alice);
    assert!(actions
        .iter()
        .any(|a| matches!(a, BackupAction::Event(BackupEvent::MessageDelivered { .. }))));
    assert_eq!(coord2.store().message_count(), 0);
}

/// Multiple messages for multiple recipients.
#[test]
fn multi_recipient_backup() {
    let relay = node_id(10);
    let alice = node_id(1);
    let bob = node_id(2);
    let charlie = node_id(3);
    let now = 100_000u64;

    let mut coord = BackupCoordinator::new(relay);

    // Messages for different recipients
    coord.store_message("msg-a1".into(), vec![1], alice, bob, now, None);
    coord.store_message("msg-a2".into(), vec![2], alice, charlie, now, None);
    coord.store_message("msg-b1".into(), vec![3], bob, alice, now, None);

    assert_eq!(coord.store().message_count(), 3);
    assert_eq!(coord.store().get_for_recipient(&alice).len(), 2);
    assert_eq!(coord.store().get_for_recipient(&bob).len(), 1);

    // Alice comes online — deliver her messages
    coord.confirm_delivery(&["msg-a1".into(), "msg-a2".into()], alice);
    assert_eq!(coord.store().message_count(), 1); // Only bob's message remains
    assert!(coord.store().has("msg-b1"));
}

/// Replication payload preserves absolute expiry (no TTL drift).
#[test]
fn replication_preserves_expiry() {
    let relay1 = node_id(10);
    let relay2 = node_id(11);
    let alice = node_id(1);
    let bob = node_id(2);
    let now = 100_000u64;

    let mut coord1 = BackupCoordinator::new(relay1);
    let mut coord2 = BackupCoordinator::new(relay2);

    // Store with 10s TTL
    coord1.store_message("msg-1".into(), vec![], alice, bob, now, Some(10_000));

    // Replicate 3 seconds later
    let actions = coord1.replicate_to("msg-1", relay2, now + 3000);
    let BackupAction::Replicate { payload, .. } = &actions[0] else {
        panic!("expected Replicate");
    };

    // Payload uses absolute expiry (110_000), not relative TTL
    assert_eq!(payload.expires_at, 110_000);

    // Relay2 stores — remaining TTL should be 7s, not 10s
    coord2.handle_replication(payload, relay1, now + 3000);
    let entry = coord2.store().get("msg-1").unwrap();
    assert_eq!(entry.remaining_ttl(now + 3000), 7_000);
}
