//! Integration tests: two TomNode instances on localhost.

use tom_transport::{MessageEnvelope, TomNode, TomNodeConfig, TomTransportError};

/// Spawn two nodes, send an envelope from A → B, verify it arrives intact.
#[tokio::test]
async fn two_nodes_exchange_envelope() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn")
        .try_init();

    let node_a = TomNode::bind(TomNodeConfig::new()).await.unwrap();
    let mut node_b = TomNode::bind(TomNodeConfig::new()).await.unwrap();

    let id_a = node_a.id();
    let id_b = node_b.id();

    // Share addresses so peers can find each other
    node_a.add_peer_addr(node_b.addr()).await;
    node_b.add_peer_addr(node_a.addr()).await;

    // Send from A → B
    let envelope = MessageEnvelope::new(
        id_a,
        id_b,
        "chat",
        serde_json::json!({"text": "Hello from A!"}),
    );

    // Spawn sender in background (connect + send)
    let send_handle = tokio::spawn(async move {
        node_a.send(id_b, &envelope).await.unwrap();
        node_a
    });

    // Receive on B
    let (from, received) = tokio::time::timeout(std::time::Duration::from_secs(30), node_b.recv())
        .await
        .expect("recv timed out")
        .unwrap();

    assert_eq!(from, id_a);
    assert_eq!(received.msg_type, "chat");
    assert_eq!(received.payload["text"], "Hello from A!");
    assert_eq!(received.from, id_a);
    assert_eq!(received.to, id_b);
    assert!(received.via.is_empty());

    // Cleanup
    let node_a = send_handle.await.unwrap();
    node_a.shutdown().await.unwrap();
    node_b.shutdown().await.unwrap();
}

/// Send raw bytes (not an envelope) and receive them.
#[tokio::test]
async fn raw_bytes_exchange() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn")
        .try_init();

    let node_a = TomNode::bind(TomNodeConfig::new()).await.unwrap();
    let mut node_b = TomNode::bind(TomNodeConfig::new()).await.unwrap();

    let id_a = node_a.id();
    let id_b = node_b.id();

    node_a.add_peer_addr(node_b.addr()).await;
    node_b.add_peer_addr(node_a.addr()).await;

    let payload = b"raw binary payload 123";

    let send_handle = tokio::spawn(async move {
        node_a.send_raw(id_b, payload).await.unwrap();
        node_a
    });

    let (from, data) =
        tokio::time::timeout(std::time::Duration::from_secs(30), node_b.recv_raw())
            .await
            .expect("recv_raw timed out")
            .unwrap();

    assert_eq!(from, id_a);
    assert_eq!(data, payload);

    let node_a = send_handle.await.unwrap();
    node_a.shutdown().await.unwrap();
    node_b.shutdown().await.unwrap();
}

/// Sending a message that exceeds max_message_size should fail.
#[tokio::test]
async fn reject_oversized_message() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn")
        .try_init();

    let config = TomNodeConfig::new().max_message_size(64);
    let node_a = TomNode::bind(config).await.unwrap();
    let node_b = TomNode::bind(TomNodeConfig::new()).await.unwrap();

    let id_b = node_b.id();
    let big_payload = vec![0u8; 128];

    let result = node_a.send_raw(id_b, &big_payload).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        TomTransportError::MessageTooLarge { size, max } => {
            assert_eq!(size, 128);
            assert_eq!(max, 64);
        }
        e => panic!("expected MessageTooLarge, got: {e}"),
    }

    node_a.shutdown().await.unwrap();
    node_b.shutdown().await.unwrap();
}

/// Bidirectional: A sends to B, B responds to A.
#[tokio::test]
async fn bidirectional_exchange() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn")
        .try_init();

    let node_a = TomNode::bind(TomNodeConfig::new()).await.unwrap();
    let mut node_b = TomNode::bind(TomNodeConfig::new()).await.unwrap();

    let id_a = node_a.id();
    let id_b = node_b.id();

    node_a.add_peer_addr(node_b.addr()).await;
    node_b.add_peer_addr(node_a.addr()).await;

    // A → B
    let msg_ab = MessageEnvelope::new(id_a, id_b, "ping", serde_json::json!({"seq": 1}));

    let send_ab = tokio::spawn(async move {
        node_a.send(id_b, &msg_ab).await.unwrap();
        node_a
    });

    let (from, received) =
        tokio::time::timeout(std::time::Duration::from_secs(30), node_b.recv())
            .await
            .expect("recv timed out")
            .unwrap();
    assert_eq!(from, id_a);
    assert_eq!(received.msg_type, "ping");

    let mut node_a = send_ab.await.unwrap();

    // B → A (response)
    let msg_ba = MessageEnvelope::new(id_b, id_a, "pong", serde_json::json!({"seq": 1}));

    let send_ba = tokio::spawn(async move {
        node_b.send(id_a, &msg_ba).await.unwrap();
        node_b
    });

    let (from, received) =
        tokio::time::timeout(std::time::Duration::from_secs(30), node_a.recv())
            .await
            .expect("recv timed out")
            .unwrap();
    assert_eq!(from, id_b);
    assert_eq!(received.msg_type, "pong");

    let node_b = send_ba.await.unwrap();

    node_a.shutdown().await.unwrap();
    node_b.shutdown().await.unwrap();
}
