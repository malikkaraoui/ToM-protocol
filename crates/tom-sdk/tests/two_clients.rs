//! Intégration : deux clients SDK échangent un message en local,
//! sans relai ni discovery externe (tickets uniquement).

use std::time::Duration;

use tom_sdk::{Event, TomClientBuilder};

#[tokio::test(flavor = "multi_thread")]
async fn two_clients_exchange_message_via_tickets() {
    let alice = TomClientBuilder::new()
        .username("alice")
        .n0_discovery(false)
        .dht(false)
        .connect()
        .await
        .expect("alice bind");
    let mut bob = TomClientBuilder::new()
        .username("bob")
        .n0_discovery(false)
        .dht(false)
        .connect()
        .await
        .expect("bob bind");

    // Échange de tickets hors bande (ici : en mémoire).
    alice
        .add_peer_ticket(&bob.ticket().expect("bob ticket"))
        .await
        .expect("alice add bob");
    bob.add_peer_ticket(&alice.ticket().expect("alice ticket"))
        .await
        .expect("bob add alice");

    alice
        .send_text(bob.id(), "salut bob")
        .await
        .expect("send");

    let received = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(event) = bob.next_event().await {
            if let Event::MessageReceived(msg) = event {
                return Some(msg);
            }
        }
        None
    })
    .await
    .expect("timeout : message jamais reçu")
    .expect("flux d'événements fermé prématurément");

    assert_eq!(received.from, alice.id());
    assert_eq!(received.text(), "salut bob");
    assert!(received.signature_valid, "signature invalide");
    assert!(received.was_encrypted, "message non chiffré");

    alice.shutdown().await;
    bob.shutdown().await;
}
