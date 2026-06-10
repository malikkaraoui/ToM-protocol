//! Exemple minimal : deux nœuds en mémoire échangent un message via tickets.
//!
//! ```bash
//! cargo run -p tom-sdk --example 01_send_message
//! ```

use tom_sdk::{Event, TomClientBuilder, TomSdkError};

#[tokio::main]
async fn main() -> Result<(), TomSdkError> {
    // Deux clients locaux, sans discovery publique ni relai.
    let alice = TomClientBuilder::new()
        .username("alice")
        .n0_discovery(false)
        .dht(false)
        .connect()
        .await?;
    let mut bob = TomClientBuilder::new()
        .username("bob")
        .n0_discovery(false)
        .dht(false)
        .connect()
        .await?;

    println!("alice : {}", alice.id());
    println!("bob   : {}", bob.id());

    // Échange de tickets (en conditions réelles : QR code, copier-coller…).
    alice.add_peer_ticket(&bob.ticket()?).await?;
    bob.add_peer_ticket(&alice.ticket()?).await?;

    alice.send_text(bob.id(), "salut bob, ici alice").await?;

    while let Some(event) = bob.next_event().await {
        if let Event::MessageReceived(msg) = event {
            println!(
                "bob a reçu de {} : « {} » (chiffré: {}, signature: {})",
                msg.from,
                msg.text(),
                msg.was_encrypted,
                msg.signature_valid
            );
            break;
        }
    }

    alice.shutdown().await;
    bob.shutdown().await;
    Ok(())
}
