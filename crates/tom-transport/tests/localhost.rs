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

/// Depuis la segmentation, les gros messages sont chunkés (jusqu'à 64 Mo),
/// donc `max_message_size` ne rejette plus les envois — seul le plafond de
/// réassemblage (MAX_REASSEMBLED = 64 Mo) fait échouer un envoi trop gros.
#[tokio::test]
async fn reject_message_above_reassembly_ceiling() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn")
        .try_init();

    let node_a = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await.unwrap();
    let node_b = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await.unwrap();

    let id_b = node_b.id();
    // 64 Mo + 1 octet : juste au-dessus du plafond de réassemblage.
    let ceiling = 64 * 1024 * 1024;
    let too_big = vec![0u8; ceiling + 1];

    let result = node_a.send_raw(id_b, &too_big).await;
    assert!(result.is_err(), "un message > 64 Mo doit être rejeté");
    match result.unwrap_err() {
        TomTransportError::MessageTooLarge { size, max } => {
            assert_eq!(size, ceiling + 1);
            assert_eq!(max, ceiling);
        }
        e => panic!("expected MessageTooLarge, got: {e}"),
    }

    node_a.shutdown().await.unwrap();
    node_b.shutdown().await.unwrap();
}

/// Fix fuite `ConnectionInner` : évincer une connexion du pool la FERME
/// activement (CONNECTION_CLOSE) au lieu de l'abandonner vivante. Observable
/// côté pair : l'accept-loop de B se termine → B ne compte plus A parmi ses
/// pairs connectés bien avant l'idle timeout (~10 s). Avant le fix, la
/// connexion évincée restait un zombie vivant côté B (drop sans close) et ce
/// test échouait au timeout de 8 s.
#[tokio::test]
async fn disconnect_actively_closes_connection_for_peer() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn")
        .try_init();

    let node_a = TomNode::bind(TomNodeConfig::new().hermetic()).await.unwrap();
    let mut node_b = TomNode::bind(TomNodeConfig::new().hermetic()).await.unwrap();

    let id_a = node_a.id();
    let id_b = node_b.id();
    node_a.add_peer_addr(node_b.addr()).await;
    node_b.add_peer_addr(node_a.addr()).await;

    // Établit la connexion A→B (B l'enregistre côté inbound).
    let env = MessageEnvelope::new(id_a, id_b, "ping", serde_json::json!({}));
    node_a.send(id_b, &env).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(30), node_b.recv())
        .await
        .expect("recv timed out")
        .unwrap();
    assert!(
        node_b.connected_peers().await.contains(&id_a),
        "B doit voir A connecté après réception"
    );

    // Éviction côté A → le CONNECTION_CLOSE doit se propager à B bien avant
    // l'idle timeout (~10 s) : borne à 8 s pour discriminer close actif vs
    // abandon passif.
    node_a.disconnect(id_b).await;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    while node_b.connected_peers().await.contains(&id_a) {
        assert!(
            std::time::Instant::now() < deadline,
            "B compte encore A 8 s après disconnect — la connexion évincée n'a pas été fermée activement"
        );
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
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
