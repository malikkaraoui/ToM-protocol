//! Tests simulant le scénario utilisateur : 2 machines distinctes.
//!
//! Le test `bidirectional_communication` en localhost PASSE, mais le user
//! rapporte un échec en LAN. Ce test explore pourquoi.

use std::time::Duration;
use tokio::time::timeout;
use tom_protocol::{ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

/// Test : Connection manuelle (comme /connect dans TUI).
///
/// **Différence vs test multi_node** :
/// - multi_node : `add_peer(id)` qui fait add + connect
/// - Ce test : seulement ID, pas d'adresse EndpointAddr
///
/// **Hypothèse** : B ne peut pas envoyer à A car il ne connaît pas l'adresse de A.
#[tokio::test]
#[ignore] // Test d'exploration, pas encore fixé
async fn manual_connect_like_tui() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tom_protocol=debug,tom_transport=debug")
        .try_init()
        .ok();

    // Setup 2 nodes
    let node_a = TomNode::bind(TomNodeConfig::new()).await?;
    let id_a = node_a.id();
    let channels_a = ProtocolRuntime::spawn(node_a, RuntimeConfig::default());

    let node_b = TomNode::bind(TomNodeConfig::new()).await?;
    let id_b = node_b.id();
    let channels_b = ProtocolRuntime::spawn(node_b, RuntimeConfig::default());

    eprintln!("Node A: {}", id_a);
    eprintln!("Node B: {}", id_b);

    // ❓ Question : Que fait /connect dans TUI ?
    // Regarder tom-tui/src/main.rs ligne ~153
    //
    // Réponse : `handle.add_peer(peer_id).await`
    //
    // Mais add_peer() fait quoi exactement ? Il appelle probablement
    // RuntimeState::add_peer() qui stocke juste l'ID, SANS adresse.
    //
    // Donc quand B veut envoyer à A, il n'a pas l'EndpointAddr de A !
    //
    // Solution possible : Quand A envoie à B, B devrait automatiquement
    // stocker l'adresse de A (via envelope.from + connection remote_addr).

    // Simuler /connect : A "connecte" à B (stocke juste l'ID)
    eprintln!("A 'connecting' to B (store ID only, no address)...");
    channels_a.handle.add_peer(id_b).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    // A → B (devrait marcher car A a initié la connexion)
    eprintln!("Sending A → B");
    channels_a
        .handle
        .send_message(id_b, b"hello from A".to_vec())
        .await?;

    let mut msgs_b = channels_b.messages;
    let received_on_b = timeout(Duration::from_secs(5), msgs_b.recv())
        .await
        .map_err(|_| anyhow::anyhow!("Timeout waiting for A→B message"))?;
    assert!(received_on_b.is_some(), "B should have received message from A");
    eprintln!("✅ A → B works");

    // B → A (va probablement échouer : B n'a pas l'adresse de A)
    eprintln!("Sending B → A (expected to fail: B has no address for A)");
    let send_result = channels_b
        .handle
        .send_message(id_a, b"hello from B".to_vec())
        .await;

    // Ce test est marqué #[ignore] car on s'attend à ce qu'il fail
    // Une fois le bug fixé (auto address discovery), ce test devrait passer.
    eprintln!("Send result: {:?}", send_result);

    Ok(())
}
