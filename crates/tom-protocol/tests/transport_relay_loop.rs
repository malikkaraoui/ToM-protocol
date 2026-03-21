/// Async runtime-loop integration test for transport relay discovery.
///
/// Exercises the REAL async loop: ProtocolRuntime::spawn → InjectGossipBytes →
/// state → effects → loop interceptor → node.insert_relay/remove_relay →
/// ProtocolEvent observable on channels.events.
///
/// NOT a pure state/effect test — this proves the loop async pipeline.
use std::time::Duration;

use tom_protocol::{ProtocolEvent, ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

fn make_keypair(seed: u8) -> (tom_protocol::NodeId, [u8; 32]) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = tom_connect::SecretKey::generate(&mut rng);
    let node_id: tom_protocol::NodeId = secret.public().to_string().parse().unwrap();
    (node_id, secret.to_bytes())
}

/// Drain events from channel until we find the expected one or timeout.
async fn expect_event(
    events: &mut tokio::sync::mpsc::Receiver<ProtocolEvent>,
    check: impl Fn(&ProtocolEvent) -> bool,
    label: &str,
    timeout: Duration,
) {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        match tokio::time::timeout_at(deadline, events.recv()).await {
            Ok(Some(event)) if check(&event) => return,
            Ok(Some(_)) => continue, // not the one we want, keep draining
            Ok(None) => panic!("{label}: event channel closed"),
            Err(_) => panic!("{label}: timed out after {timeout:?}"),
        }
    }
}

/// Assert that a specific event does NOT appear within a short window.
async fn expect_no_event(
    events: &mut tokio::sync::mpsc::Receiver<ProtocolEvent>,
    check: impl Fn(&ProtocolEvent) -> bool,
    label: &str,
    window: Duration,
) {
    let deadline = tokio::time::Instant::now() + window;
    loop {
        match tokio::time::timeout_at(deadline, events.recv()).await {
            Ok(Some(event)) if check(&event) => {
                panic!("{label}: unexpected event found: {event:?}")
            }
            Ok(Some(_)) => continue, // irrelevant event, keep draining
            Ok(None) => return,      // channel closed, no match
            Err(_) => return,        // timeout, no match — success
        }
    }
}

#[tokio::test]
async fn transport_relay_discovery_via_runtime_loop() {
    // Enable tracing for debug
    let _ = tracing_subscriber::fmt()
        .with_env_filter("tom_protocol::runtime=debug")
        .with_test_writer()
        .try_init();

    // ── Setup: spawn one node with transport relay discovery enabled ──

    let node = TomNode::bind(TomNodeConfig::new().n0_discovery(false))
        .await
        .expect("bind failed");

    let config = RuntimeConfig {
        username: "test-node".into(),
        encryption: false,
        enable_dht: false,              // no DHT — avoid blocking publish
        enable_transport_relay_discovery: true,
        relay_registry_ttl: Duration::from_millis(500), // short enough for prune test, long enough for phases 1-3
        heartbeat_interval: Duration::from_millis(50),  // fast ticks for prune test
        ..Default::default()
    };

    let mut channels = ProtocolRuntime::spawn(node, config);
    let handle = channels.handle.clone();

    // Give the loop a moment to initialize
    tokio::time::sleep(Duration::from_millis(50)).await;

    let timeout = Duration::from_secs(5);

    // ── Phase 1: Announce → RelayReadyReceived + TransportRelayInserted ──

    let (relay_node_id, relay_seed) = make_keypair(200);
    let relay_url: tom_connect::RelayUrl = "http://10.0.0.99:3340".parse().unwrap();

    let announce = tom_protocol::discovery::RelayReadyAnnounce::new(
        relay_node_id,
        relay_url.clone(),
        tom_protocol::now_ms(),
        &relay_seed,
    );
    let gossip_bytes = rmp_serde::to_vec(&announce).unwrap();
    handle.inject_gossip_bytes(gossip_bytes).await;

    // Must observe TransportRelayInserted (interceptor emits BEFORE executor runs Emit effects)
    // AND RelayReadyReceived — order depends on interceptor vs executor pipeline.
    // Collect both without assuming order.
    let mut got_received = false;
    let mut got_inserted = false;
    let deadline = tokio::time::Instant::now() + timeout;
    while !got_received || !got_inserted {
        match tokio::time::timeout_at(deadline, channels.events.recv()).await {
            Ok(Some(ProtocolEvent::RelayReadyReceived { node_id, .. }))
                if node_id == relay_node_id =>
            {
                got_received = true;
            }
            Ok(Some(ProtocolEvent::TransportRelayInserted { relay_url: ref url }))
                if *url == relay_url =>
            {
                got_inserted = true;
            }
            Ok(Some(_)) => continue,
            Ok(None) => panic!("phase1: event channel closed"),
            Err(_) => panic!(
                "phase1: timed out (got_received={got_received}, got_inserted={got_inserted})"
            ),
        }
    }

    // ── Phase 2: Refresh same URL → no TransportRelayInserted ──

    let announce2 = tom_protocol::discovery::RelayReadyAnnounce::new(
        relay_node_id,
        relay_url.clone(),
        tom_protocol::now_ms(),
        &relay_seed,
    );
    let gossip_bytes2 = rmp_serde::to_vec(&announce2).unwrap();
    handle.inject_gossip_bytes(gossip_bytes2).await;

    // RelayReadyReceived should still fire
    expect_event(
        &mut channels.events,
        |e| matches!(e, ProtocolEvent::RelayReadyReceived { .. }),
        "phase2: RelayReadyReceived",
        timeout,
    )
    .await;

    // But NO TransportRelayInserted (same URL = refresh, no duplicate)
    expect_no_event(
        &mut channels.events,
        |e| matches!(e, ProtocolEvent::TransportRelayInserted { .. }),
        "phase2: no duplicate TransportRelayInserted",
        Duration::from_millis(100),
    )
    .await;

    // ── Phase 3: URL change → InsertTransportRelay(new) + RemoveTransportRelay(old) ──

    let new_url: tom_connect::RelayUrl = "http://10.0.0.100:4444".parse().unwrap();
    let announce3 = tom_protocol::discovery::RelayReadyAnnounce::new(
        relay_node_id,
        new_url.clone(),
        tom_protocol::now_ms(),
        &relay_seed,
    );
    let gossip_bytes3 = rmp_serde::to_vec(&announce3).unwrap();
    handle.inject_gossip_bytes(gossip_bytes3).await;

    expect_event(
        &mut channels.events,
        |e| matches!(e, ProtocolEvent::TransportRelayInserted { relay_url: url } if url == &new_url),
        "phase3: TransportRelayInserted(new_url)",
        timeout,
    )
    .await;

    expect_event(
        &mut channels.events,
        |e| matches!(e, ProtocolEvent::TransportRelayRemoved { relay_url: url } if url == &relay_url),
        "phase3: TransportRelayRemoved(old_url)",
        timeout,
    )
    .await;

    // ── Phase 4: TTL expiration → RelayRegistryExpired + TransportRelayRemoved ──

    // TTL is 500ms, heartbeat ticks every 50ms.
    // Collect both events without assuming order (interceptor vs executor).
    let mut got_expired = false;
    let mut got_removed = false;
    let deadline4 = tokio::time::Instant::now() + Duration::from_secs(3);
    while !got_expired || !got_removed {
        match tokio::time::timeout_at(deadline4, channels.events.recv()).await {
            Ok(Some(ProtocolEvent::RelayRegistryExpired { node_id, .. }))
                if node_id == relay_node_id =>
            {
                got_expired = true;
            }
            Ok(Some(ProtocolEvent::TransportRelayRemoved { relay_url: ref url }))
                if *url == new_url =>
            {
                got_removed = true;
            }
            Ok(Some(_)) => continue,
            Ok(None) => panic!("phase4: event channel closed"),
            Err(_) => panic!(
                "phase4: timed out (got_expired={got_expired}, got_removed={got_removed})"
            ),
        }
    }

    // ── Cleanup ──

    handle.shutdown().await;
}
