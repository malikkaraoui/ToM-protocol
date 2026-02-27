//! E2E test: DHT-only peer discovery
//!
//! Phase R7.1 PoC: Validates DHT integration in the protocol stack.
//! Note: Actual DHT lookup returns None (stub) - tests fallback path.
//! Full DHT implementation in Phase R7.4.

use tokio::time::Duration;
use tom_protocol::{ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

#[tokio::test]
#[ignore] // Manual run: cargo test -p tom-protocol --test dht_e2e -- --ignored --nocapture
async fn test_dht_integration_with_fallback() {
    tracing_subscriber::fmt()
        .with_env_filter("info,tom_dht=debug")
        .with_test_writer()
        .init();

    tracing::info!("=== Starting DHT E2E test ===");

    // ── Node A ───────────────────────────────────────────────────────
    let config_a = TomNodeConfig::default();
    let node_a = TomNode::bind(config_a).await.expect("Failed to bind node A");
    let id_a = node_a.id();

    let runtime_config_a = RuntimeConfig {
        username: "alice".into(),
        encryption: true,
        enable_dht: true, // DHT enabled
        ..Default::default()
    };

    let channels_a = ProtocolRuntime::spawn(node_a, runtime_config_a);
    tracing::info!("Node A spawned: {id_a}");

    // ── Node B ───────────────────────────────────────────────────────
    let config_b = TomNodeConfig::default();
    let node_b = TomNode::bind(config_b).await.expect("Failed to bind node B");
    let id_b = node_b.id();

    let runtime_config_b = RuntimeConfig {
        username: "bob".into(),
        encryption: true,
        enable_dht: true, // DHT enabled
        gossip_bootstrap_peers: vec![id_a], // Bootstrap via A for fallback
        ..Default::default()
    };

    let mut channels_b = ProtocolRuntime::spawn(node_b, runtime_config_b);
    tracing::info!("Node B spawned: {id_b}");

    // ── Wait for DHT publish (both nodes) ────────────────────────────
    tokio::time::sleep(Duration::from_secs(2)).await;
    tracing::info!("DHT publish window complete");

    // ── Wait for gossip discovery (fallback path) ────────────────────
    tokio::time::sleep(Duration::from_secs(5)).await;
    tracing::info!("Gossip discovery window complete");

    // ── A sends message to B ─────────────────────────────────────────
    // DHT lookup will return None (stub), fallback to gossip discovery
    tracing::info!("Sending message from A to B");
    channels_a
        .handle
        .send_message(id_b, b"hello via DHT fallback".to_vec())
        .await
        .expect("Failed to send message");

    // ── B receives message ───────────────────────────────────────────
    tracing::info!("Waiting for message delivery...");
    let msg = tokio::time::timeout(Duration::from_secs(15), channels_b.messages.recv())
        .await
        .expect("timeout waiting for message")
        .expect("channel closed");

    tracing::info!("Message received!");

    // ── Assertions ───────────────────────────────────────────────────
    assert_eq!(msg.payload, b"hello via DHT fallback");
    assert!(msg.was_encrypted, "message should be encrypted");
    assert!(msg.signature_valid, "signature should be valid");
    assert_eq!(msg.from, id_a, "sender should be node A");

    tracing::info!("✅ DHT integration test passed!");
    tracing::info!("   - DHT enabled on both nodes");
    tracing::info!("   - DHT publish called at startup");
    tracing::info!("   - DHT lookup attempted (returned None - expected)");
    tracing::info!("   - Fallback to gossip discovery successful");
    tracing::info!("   - Message delivered encrypted + signed");
}

#[tokio::test]
#[ignore] // Manual run: cargo test -p tom-protocol --test dht_e2e -- --ignored --nocapture
async fn test_dht_disabled_still_works() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_test_writer()
        .init();

    tracing::info!("=== Starting DHT disabled test ===");

    // ── Node A (DHT disabled) ────────────────────────────────────────
    let config_a = TomNodeConfig::default();
    let node_a = TomNode::bind(config_a).await.expect("Failed to bind node A");
    let id_a = node_a.id();

    let runtime_config_a = RuntimeConfig {
        username: "alice".into(),
        encryption: true,
        enable_dht: false, // DHT DISABLED
        ..Default::default()
    };

    let channels_a = ProtocolRuntime::spawn(node_a, runtime_config_a);
    tracing::info!("Node A spawned (DHT disabled): {id_a}");

    // ── Node B (DHT disabled) ────────────────────────────────────────
    let config_b = TomNodeConfig::default();
    let node_b = TomNode::bind(config_b).await.expect("Failed to bind node B");
    let id_b = node_b.id();

    let runtime_config_b = RuntimeConfig {
        username: "bob".into(),
        encryption: true,
        enable_dht: false, // DHT DISABLED
        gossip_bootstrap_peers: vec![id_a],
        ..Default::default()
    };

    let mut channels_b = ProtocolRuntime::spawn(node_b, runtime_config_b);
    tracing::info!("Node B spawned (DHT disabled): {id_b}");

    // ── Wait for gossip discovery ────────────────────────────────────
    tokio::time::sleep(Duration::from_secs(5)).await;

    // ── A sends message to B ─────────────────────────────────────────
    channels_a
        .handle
        .send_message(id_b, b"hello without DHT".to_vec())
        .await
        .expect("Failed to send message");

    // ── B receives message ───────────────────────────────────────────
    let msg = tokio::time::timeout(Duration::from_secs(10), channels_b.messages.recv())
        .await
        .expect("timeout waiting for message")
        .expect("channel closed");

    // ── Assertions ───────────────────────────────────────────────────
    assert_eq!(msg.payload, b"hello without DHT");
    assert!(msg.was_encrypted);
    assert!(msg.signature_valid);

    tracing::info!("✅ DHT disabled test passed!");
    tracing::info!("   - Protocol works without DHT (backward compat)");
}
