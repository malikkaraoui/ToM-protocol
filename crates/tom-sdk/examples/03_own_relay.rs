//! Exemple relai auto-hébergé : se connecter via son propre `tom-relay`
//! sans aucune dépendance au réseau public n0.
//!
//! Prérequis : un relai joignable, par exemple :
//! ```bash
//! cargo run -p tom-relay -- --dev          # écoute sur http://localhost:3340
//! ```
//! Puis :
//! ```bash
//! RELAY=http://localhost:3340 cargo run -p tom-sdk --example 03_own_relay
//! ```

use tom_sdk::{Event, TomClientBuilder, TomSdkError};

#[tokio::main]
async fn main() -> Result<(), TomSdkError> {
    let relay = std::env::var("RELAY").unwrap_or_else(|_| "http://localhost:3340".to_string());

    let mut client = TomClientBuilder::new()
        .username("nomade")
        .relay_url(&relay)
        .n0_discovery(false)
        .connect()
        .await?;

    println!("identité : {}", client.id());
    println!("ticket   : {}", client.ticket()?);
    println!("en écoute via {relay} — Ctrl-C pour quitter");

    while let Some(event) = client.next_event().await {
        match event {
            Event::MessageReceived(msg) => {
                println!("{} → {}", msg.from, msg.text());
                client.send_text(msg.from, "pong").await?;
            }
            Event::PeerDiscovered { node_id, username } => {
                println!("pair découvert : {username} ({node_id})");
            }
            Event::PathChanged { peer, direct, rtt_ms } => {
                let kind = if direct { "direct" } else { "relai" };
                println!("chemin vers {peer} : {kind} ({rtt_ms} ms)");
            }
            _ => {}
        }
    }
    Ok(())
}
