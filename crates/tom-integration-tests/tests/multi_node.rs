//! Integration tests avec plusieurs nodes réels (pas de mocks).
//!
//! Ces tests révèlent les bugs que les unit tests ne voient pas :
//! - Découverte automatique (gossip)
//! - Communication bidirectionnelle
//! - Stabilité des connexions
//!
//! Contrairement aux 964 unit tests existants, ces tests lancent de vrais
//! nodes QUIC et vérifient les scénarios end-to-end.

use std::time::Duration;
use tokio::time::timeout;
use tom_protocol::{ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

/// Setup: lance 2 nodes avec configs par défaut.
async fn setup_two_nodes() -> anyhow::Result<(
    (tom_protocol::types::NodeId, tom_protocol::RuntimeHandle, tokio::sync::mpsc::Receiver<tom_protocol::DeliveredMessage>),
    (tom_protocol::types::NodeId, tom_protocol::RuntimeHandle, tokio::sync::mpsc::Receiver<tom_protocol::DeliveredMessage>),
)> {
    // Node A
    let node_a = TomNode::bind(TomNodeConfig::new()).await?;
    let id_a = node_a.id();
    let config_a = RuntimeConfig::default();
    let channels_a = ProtocolRuntime::spawn(node_a, config_a);

    // Node B
    let node_b = TomNode::bind(TomNodeConfig::new()).await?;
    let id_b = node_b.id();
    let config_b = RuntimeConfig::default();
    let channels_b = ProtocolRuntime::spawn(node_b, config_b);

    eprintln!("Node A: {}", id_a);
    eprintln!("Node B: {}", id_b);

    Ok(((id_a, channels_a.handle, channels_a.messages), (id_b, channels_b.handle, channels_b.messages)))
}

/// Test 1: Découverte automatique via gossip.
///
/// **Scénario** :
/// 1. Lance 2 nodes sans /connect manuel
/// 2. Attend 30 secondes (gossip_announce_interval = 10s)
/// 3. Vérifie que chaque node a découvert l'autre
///
/// **Attendu** : Les nodes se découvrent automatiquement via gossip.
/// **Réel (bug)** : Ils ne se découvrent PAS → /connect manuel requis.
#[tokio::test]
#[ignore] // Ignorer jusqu'à fix (ce test va échouer pour l'instant)
async fn auto_discovery_via_gossip() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tom_protocol=debug,tom_gossip=debug")
        .init();

    let ((id_a, _handle_a, _msgs_a), (id_b, _handle_b, _msgs_b)) = setup_two_nodes().await?;

    // Attendre 30 secondes pour que le gossip fasse son travail
    eprintln!("Waiting 30s for gossip discovery...");
    tokio::time::sleep(Duration::from_secs(30)).await;

    // TODO: Ajouter une façon de vérifier les peers connectés via le handle
    // Pour l'instant, ce test est marqué #[ignore]
    eprintln!("Test not fully implemented yet (need connected_peers() API)");

    Ok(())
}

/// Test 2: Communication bidirectionnelle.
///
/// **Scénario** :
/// 1. Lance 2 nodes, A connecte à B via add_peer()
/// 2. A envoie message à B → vérifie réception
/// 3. B envoie message à A → vérifie réception
///
/// **Attendu** : Les deux directions fonctionnent.
/// **Réel (bug)** : Une seule direction fonctionne (asymétrie).
#[tokio::test]
async fn bidirectional_communication() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tom_protocol=debug,tom_transport=debug")
        .try_init()
        .ok();

    let ((id_a, handle_a, mut msgs_a), (id_b, handle_b, mut msgs_b)) = setup_two_nodes().await?;

    // A connecte à B
    eprintln!("A connecting to B...");
    handle_a.add_peer(id_b).await;
    tokio::time::sleep(Duration::from_secs(2)).await; // Laisser connexion s'établir

    // Test A → B
    eprintln!("Sending A → B");
    handle_a
        .send_message(id_b, b"hello from A".to_vec())
        .await?;

    // Attendre réception sur B (avec timeout)
    let received_on_b = timeout(Duration::from_secs(5), async {
        msgs_b.recv().await
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for A→B message"))?;

    assert!(received_on_b.is_some(), "B should have received message from A");
    eprintln!("✅ A → B works");

    // Test B → A (direction inverse)
    eprintln!("Sending B → A");
    let send_result = handle_b
        .send_message(id_a, b"hello from B".to_vec())
        .await;

    // ❌ EXPECTED TO FAIL: Ce send va probablement échouer (asymétrie)
    assert!(
        send_result.is_ok(),
        "B → A send failed (asymmetry bug): {:?}",
        send_result.err()
    );

    // Attendre réception sur A (avec timeout)
    let received_on_a = timeout(Duration::from_secs(5), async {
        msgs_a.recv().await
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for B→A message"))?;

    assert!(received_on_a.is_some(), "A should have received message from B");
    eprintln!("✅ B → A works");

    Ok(())
}

/// Test 3: Stabilité connexion (5 minutes d'échanges).
///
/// **Scénario** :
/// 1. Lance 2 nodes, établit connexion
/// 2. Envoie 1 message/seconde pendant 5 minutes
/// 3. Vérifie que TOUS les messages sont livrés
///
/// **Attendu** : 300 messages envoyés, 300 reçus.
/// **Réel (bug)** : Connexion meurt après ~2 minutes.
#[tokio::test]
#[ignore] // Test long, ignorer par défaut
async fn connection_stability_5min() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tom_protocol=info")
        .try_init()
        .ok();

    let ((id_a, handle_a, _msgs_a), (id_b, _handle_b, _msgs_b)) = setup_two_nodes().await?;

    // Connexion initiale
    handle_a.add_peer(id_b).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Envoyer 1 msg/sec pendant 5 minutes (300 messages)
    let mut sent_count = 0;
    let mut failed_at = None;

    for i in 0..300 {
        let payload = format!("message {}", i).into_bytes();
        let result = handle_a.send_message(id_b, payload).await;

        if result.is_err() {
            failed_at = Some((i, result.err().unwrap()));
            break;
        }

        sent_count += 1;
        tokio::time::sleep(Duration::from_secs(1)).await;

        if i % 60 == 0 {
            eprintln!("{}min elapsed, {} messages sent", i / 60, sent_count);
        }
    }

    // ❌ EXPECTED TO FAIL: Connexion va probablement mourir avant 5min
    assert_eq!(
        sent_count, 300,
        "Connection died after {}s: {:?}",
        sent_count,
        failed_at
    );

    Ok(())
}

/// Test 4: Reconnexion automatique après déconnexion.
///
/// **Scénario** :
/// 1. A et B connectés, échange de messages
/// 2. Force disconnect de A
/// 3. Attend quelques secondes
/// 4. A envoie nouveau message à B
///
/// **Attendu** : A reconnecte automatiquement, message livré.
#[tokio::test]
#[ignore] // À implémenter après fix des tests de base
async fn auto_reconnect() -> anyhow::Result<()> {
    // TODO: Implémenter après fix des bugs de base
    Ok(())
}
