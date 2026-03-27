/// Terrain validation: transport relay discovery via real gossip.
///
/// 2 nodes, 1 process:
///   - Node A: embedded relay + publication (publisher)
///   - Node B: transport relay discovery enabled (observer)
///
/// Success criteria (all must pass for exit 0):
///   1. TransportRelayInserted received by Node B
///   2. No spurious PeerDiscovered on Node B (only real peers allowed)
///   3. TransportRelayRemoved after TTL expiration
///   4. Refresh (same URL, different node) → RelayReadyReceived but NOT TransportRelayInserted
use std::net::SocketAddr;
use std::time::Duration;

use tom_protocol::{
    now_ms, ProtocolEvent, ProtocolRuntime, RuntimeConfig,
    runtime::EmbeddedRelayConfig,
};
use tom_transport::{TomNode, TomNodeConfig};

const TIMEOUT: Duration = Duration::from_secs(30);
const TTL: Duration = Duration::from_secs(5);
const HEARTBEAT: Duration = Duration::from_millis(500);

fn make_keypair(seed: u8) -> (tom_protocol::NodeId, [u8; 32]) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = tom_connect::SecretKey::generate(&mut rng);
    let node_id: tom_protocol::NodeId = secret.public().to_string().parse().unwrap();
    (node_id, secret.to_bytes())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("tom_protocol::runtime=debug,validate_transport_relay=info")
        .init();

    let deadline = tokio::time::Instant::now() + TIMEOUT;

    // ── Spawn Node A (publisher) ──

    let node_a = TomNode::bind(TomNodeConfig::new().n0_discovery(false))
        .await
        .expect("node_a bind failed");
    let id_a = node_a.id();
    let addr_a = node_a.addr();

    let config_a = RuntimeConfig {
        username: "publisher".into(),
        encryption: false,
        enable_dht: false,
        enable_embedded_relay: true,
        enable_embedded_relay_publication: true,
        ..Default::default()
    };
    let mut channels_a = ProtocolRuntime::spawn(node_a, config_a);
    let handle_a = channels_a.handle.clone();

    // ── Spawn Node B (observer) ──

    let node_b = TomNode::bind(TomNodeConfig::new().n0_discovery(false))
        .await
        .expect("node_b bind failed");
    let id_b = node_b.id();
    let addr_b = node_b.addr();

    // Known real peer IDs — PeerDiscovered for these is expected (gossip handshake)
    let known_peers = [id_a, id_b];

    let config_b = RuntimeConfig {
        username: "observer".into(),
        encryption: false,
        enable_dht: false,
        enable_transport_relay_discovery: true,
        relay_registry_ttl: TTL,
        heartbeat_interval: HEARTBEAT,
        ..Default::default()
    };
    let mut channels_b = ProtocolRuntime::spawn(node_b, config_b);
    let handle_b = channels_b.handle.clone();

    // ── Connect peers ──

    handle_a.add_peer_addr(addr_b).await;
    handle_b.add_peer_addr(addr_a).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // ── Start embedded relay on Node A ──

    handle_a
        .start_embedded_relay(EmbeddedRelayConfig {
            bind_addr: "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
            advertise_addr: None,
        })
        .await;

    // Wait for EmbeddedRelayStarted on Node A to learn the relay URL
    let relay_url = loop {
        match tokio::time::timeout_at(deadline, channels_a.events.recv()).await {
            Ok(Some(ProtocolEvent::EmbeddedRelayStarted { url })) => {
                tracing::info!(%url, "node_a: embedded relay started");
                break url;
            }
            Ok(Some(_)) => continue,
            Ok(None) => panic!("node_a: event channel closed"),
            Err(_) => panic!("TIMEOUT waiting for EmbeddedRelayStarted"),
        }
    };

    // ══════════════════════════════════════════════════════════════════════
    // Criterion 1: TransportRelayInserted received by Node B
    // ══════════════════════════════════════════════════════════════════════

    let mut got_inserted = false;
    let mut no_peer_discovered = true;
    let insert_deadline = tokio::time::Instant::now() + Duration::from_secs(15);

    while !got_inserted {
        match tokio::time::timeout_at(insert_deadline, channels_b.events.recv()).await {
            Ok(Some(ProtocolEvent::TransportRelayInserted { relay_url: ref url }))
                if *url == relay_url =>
            {
                tracing::info!(%url, "PASS criterion 1: TransportRelayInserted");
                got_inserted = true;
            }
            Ok(Some(ProtocolEvent::PeerDiscovered { node_id, .. }))
                if !known_peers.contains(&node_id) =>
            {
                tracing::error!(%node_id, "FAIL criterion 2: spurious PeerDiscovered");
                no_peer_discovered = false;
            }
            Ok(Some(_)) => continue,
            Ok(None) => panic!("node_b: event channel closed"),
            Err(_) => panic!("FAIL criterion 1: TIMEOUT waiting for TransportRelayInserted"),
        }
    }

    // ══════════════════════════════════════════════════════════════════════
    // Criterion 2: No spurious PeerDiscovered (drain briefly)
    // ══════════════════════════════════════════════════════════════════════

    let drain_until = tokio::time::Instant::now() + Duration::from_millis(500);
    loop {
        match tokio::time::timeout_at(drain_until, channels_b.events.recv()).await {
            Ok(Some(ProtocolEvent::PeerDiscovered { node_id, .. }))
                if !known_peers.contains(&node_id) =>
            {
                tracing::error!(%node_id, "FAIL criterion 2: spurious PeerDiscovered");
                no_peer_discovered = false;
            }
            Ok(Some(_)) => continue,
            Ok(None) | Err(_) => break,
        }
    }
    if no_peer_discovered {
        tracing::info!("PASS criterion 2: no spurious PeerDiscovered on observer");
    }

    // ══════════════════════════════════════════════════════════════════════
    // Criterion 4: Refresh same URL → no duplicate TransportRelayInserted
    // (done BEFORE TTL expiration — URL still in discovered set)
    // ══════════════════════════════════════════════════════════════════════

    let (fake_node_id, fake_seed) = make_keypair(42);
    let refresh_announce = tom_protocol::discovery::RelayReadyAnnounce::new(
        fake_node_id,
        relay_url.clone(),
        now_ms(),
        &fake_seed,
    );
    let refresh_bytes = rmp_serde::to_vec(&refresh_announce).unwrap();
    handle_b.inject_gossip_bytes(refresh_bytes).await;

    let mut got_relay_received = false;
    let mut got_dup_insert = false;
    let refresh_deadline = tokio::time::Instant::now() + Duration::from_secs(2);

    loop {
        match tokio::time::timeout_at(refresh_deadline, channels_b.events.recv()).await {
            Ok(Some(ProtocolEvent::RelayReadyReceived { .. })) => {
                got_relay_received = true;
            }
            Ok(Some(ProtocolEvent::TransportRelayInserted { .. })) => {
                got_dup_insert = true;
            }
            Ok(Some(_)) => continue,
            Ok(None) | Err(_) => break,
        }
    }

    if got_relay_received && !got_dup_insert {
        tracing::info!("PASS criterion 4: refresh → RelayReadyReceived, no dup insert");
    } else {
        tracing::error!(
            got_relay_received,
            got_dup_insert,
            "FAIL criterion 4: refresh dedup"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // Criterion 3: TTL expiration → TransportRelayRemoved
    // ══════════════════════════════════════════════════════════════════════

    let mut got_removed = false;
    let ttl_deadline = tokio::time::Instant::now() + Duration::from_secs(15);

    while !got_removed {
        match tokio::time::timeout_at(ttl_deadline, channels_b.events.recv()).await {
            Ok(Some(ProtocolEvent::TransportRelayRemoved { relay_url: ref url }))
                if *url == relay_url =>
            {
                tracing::info!(%url, "PASS criterion 3: TransportRelayRemoved after TTL");
                got_removed = true;
            }
            Ok(Some(_)) => continue,
            Ok(None) => panic!("node_b: event channel closed"),
            Err(_) => panic!("FAIL criterion 3: TIMEOUT waiting for TransportRelayRemoved"),
        }
    }

    // ── Verdict ──

    handle_a.shutdown().await;
    handle_b.shutdown().await;

    let all_pass = got_inserted && no_peer_discovered && got_removed && got_relay_received && !got_dup_insert;
    if all_pass {
        tracing::info!("ALL 4 CRITERIA PASSED — transport relay discovery validated");
        std::process::exit(0);
    } else {
        tracing::error!(
            criterion_1 = got_inserted,
            criterion_2 = no_peer_discovered,
            criterion_3 = got_removed,
            criterion_4_received = got_relay_received,
            criterion_4_no_dup = !got_dup_insert,
            "SOME CRITERIA FAILED"
        );
        std::process::exit(1);
    }
}
