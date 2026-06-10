# tom-sdk

SDK haut niveau du **protocole ToM** (The Open Messaging) — transport pair-à-pair décentralisé : chaque appareil est à la fois client et relais, chiffrement de bout en bout (XChaCha20-Poly1305 + Ed25519), relais sans stockage.

Ce crate est la **façade stable** au-dessus de `tom-protocol`. API minimale, aucun type interne exposé. Usages avancés (relay embarqué, rôles, backup) → `tom-protocol` directement.

## Installation

Pas encore publié sur crates.io (pins crypto pre-release en amont). Consommation par git :

```toml
[dependencies]
tom-sdk = { git = "https://github.com/malikkaraoui/ToM-protocol", tag = "v0.3.0" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Démarrage en 10 lignes

```rust,no_run
use tom_sdk::{Event, TomClientBuilder};

#[tokio::main]
async fn main() -> Result<(), tom_sdk::TomSdkError> {
    let mut client = TomClientBuilder::new().username("alice").connect().await?;
    println!("mon identité : {}", client.id());
    while let Some(event) = client.next_event().await {
        if let Event::MessageReceived(msg) = event {
            client.send_text(msg.from, "bien reçu !").await?;
        }
    }
    Ok(())
}
```

## Concepts en 30 secondes

| Concept | Dans le SDK |
|---|---|
| Identité = clé publique | `client.id()` — `NodeId` Ed25519, c'est l'adresse réseau |
| Connexion sans infra | `client.ticket()` ↔ `client.add_peer_ticket(...)` (QR code, copier-coller) |
| Relai auto-hébergé | `.relay_url("http://...:3340")` + `.n0_discovery(false)` |
| Tout arrive au même endroit | `client.next_event()` — messages, statuts, pairs, groupes |
| Livraison | message livré ⟺ ACK du destinataire (`Event::MessageStatusChanged`) |
| Groupes | hub-and-spoke avec failover automatique (`create_group`, invitations) |

## Exemples

```bash
cargo run -p tom-sdk --example 01_send_message   # 2 nœuds, tickets, message E2E
cargo run -p tom-sdk --example 02_group_chat     # groupe, invitation, fan-out
cargo run -p tom-sdk --example 03_own_relay      # relai auto-hébergé, zéro n0
```

## Garanties du protocole (décisions verrouillées)

- Message livré ⟺ le destinataire émet un ACK.
- TTL 24 h maximum, puis purge globale — aucune exception.
- Les relais relaient, ne stockent pas.
- Pas de bans permanents : réputation à décroissance progressive.
- Couche protocole invisible pour l'utilisateur final.

Licence MIT.
