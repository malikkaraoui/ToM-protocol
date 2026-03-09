/// Integration test: PeerPresent auto-discovery via relay.
///
/// Validates the core product value of relay-assisted discovery:
/// - Two nodes on the same relay, ZERO manual bootstrap
/// - PeerPresent frame triggers gossip join_peers()
/// - Real GossipNeighborUp event observed
/// - Message delivered end-to-end
///
/// NOTE: This test covers the PeerPresent path only.
/// It does NOT replace separate tests for AddPeerAddr / DhtLookupResult ordering.
use std::{net::Ipv4Addr, time::Duration};

use anyhow::{Context, Result};
use tokio::time::timeout;
use tom_protocol::{AntiSpamConfig, ProtocolEvent, ProtocolRuntime, RuntimeConfig};
use tom_relay::server::{
    AccessConfig, RelayConfig as RelayServerConfig, Server, ServerConfig, SpawnError,
};
use tom_transport::{RelayUrl, TomNode, TomNodeConfig};

struct TestNode {
    id: tom_protocol::NodeId,
    handle: tom_protocol::RuntimeHandle,
    messages: tokio::sync::mpsc::Receiver<tom_protocol::DeliveredMessage>,
    events: tokio::sync::mpsc::Receiver<ProtocolEvent>,
}

/// Local helper — HTTP-only relay (no TLS, avoids cert verification issues in tests).
async fn run_test_relay() -> Result<(RelayUrl, Server), SpawnError> {
    let config = ServerConfig::<(), ()> {
        relay: Some(RelayServerConfig {
            http_bind_addr: (Ipv4Addr::LOCALHOST, 0).into(),
            tls: None,
            limits: Default::default(),
            key_cache_capacity: Some(1024),
            access: AccessConfig::Everyone,
        }),
        quic: None,
        ..Default::default()
    };

    let server = Server::spawn(config).await?;
    let relay_url: RelayUrl = format!("http://{}", server.http_addr().expect("configured"))
        .parse()
        .expect("invalid relay url");

    Ok((relay_url, server))
}

async fn spawn_runtime_node(relay_url: RelayUrl) -> Result<TestNode> {
    let antispam = AntiSpamConfig {
        min_rate: 1000.0,
        ..AntiSpamConfig::default()
    };

    let node = TomNode::bind(
        TomNodeConfig::new()
            .relay_url(relay_url)
            .n0_discovery(false),
    )
    .await?;

    let id = node.id();

    let channels = ProtocolRuntime::spawn(
        node,
        RuntimeConfig {
            enable_dht: false,
            gossip_bootstrap_peers: Vec::new(),
            antispam_config: antispam,
            ..RuntimeConfig::default()
        },
    );

    Ok(TestNode {
        id,
        handle: channels.handle,
        messages: channels.messages,
        events: channels.events,
    })
}

async fn wait_for_neighbor_up(
    events: &mut tokio::sync::mpsc::Receiver<ProtocolEvent>,
    target: tom_protocol::NodeId,
    label: &str,
) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);

    loop {
        let remaining = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .context("neighbor-up timeout expired")?;

        let event = timeout(remaining, events.recv())
            .await
            .context(format!(
                "[{label}] timed out waiting for GossipNeighborUp({target})"
            ))?
            .context("event channel closed")?;

        match event {
            ProtocolEvent::GossipNeighborUp { node_id } if node_id == target => {
                eprintln!("[{label}] NeighborUp for {node_id}");
                return Ok(());
            }
            other => {
                eprintln!("[{label}] ignoring event: {other:?}");
            }
        }
    }
}

/// Core product test: two nodes, same relay, zero bootstrap → auto-discovery → message delivered.
///
/// Ignored in workspace runs: this test uses real relay+gossip networking and hangs
/// when other tests saturate UDP ports. Run solo: `cargo test -p tom-integration-tests`
#[tokio::test]
#[ignore]
async fn peer_present_auto_discovery_leads_to_neighbor_up_and_delivery() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            "tom_protocol=debug,tom_transport=debug,tom_connect=info,tom_relay=info,tom_gossip=debug",
        )
        .try_init();

    let (relay_url, _relay_server) = run_test_relay().await?;

    // Two nodes on the same relay, NO manual address exchange, NO bootstrap.
    let mut alice = spawn_runtime_node(relay_url.clone()).await?;
    let mut bob = spawn_runtime_node(relay_url).await?;

    eprintln!("Alice: {}", alice.id);
    eprintln!("Bob:   {}", bob.id);

    // PeerPresent -> join_peers -> NeighborUp: the product value of this feature.
    wait_for_neighbor_up(&mut alice.events, bob.id, "alice").await?;
    wait_for_neighbor_up(&mut bob.events, alice.id, "bob").await?;

    // Once gossip neighborhood is established, a real message must be delivered.
    alice
        .handle
        .send_message(bob.id, b"hello via peer_present".to_vec())
        .await?;

    let msg = timeout(Duration::from_secs(10), bob.messages.recv())
        .await
        .context("timed out waiting for delivered message")?
        .context("messages channel closed")?;

    assert_eq!(msg.from, alice.id);
    assert_eq!(msg.payload, b"hello via peer_present");

    eprintln!("Alice -> Bob via PeerPresent: OK");

    Ok(())
}
