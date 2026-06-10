//! # tom-sdk — SDK haut niveau du protocole ToM
//!
//! ToM (The Open Messaging) est un transport pair-à-pair décentralisé :
//! chaque appareil est à la fois client et relais, les messages sont
//! chiffrés de bout en bout (XChaCha20-Poly1305) et signés (Ed25519),
//! les relais ne stockent rien.
//!
//! Ce crate est la **façade stable** au-dessus de `tom-protocol` : une API
//! minimale, sans aucun type des crates internes. Pour les usages avancés
//! (relay embarqué, rôles, backup), consommez `tom-protocol` directement.
//!
//! ## Démarrage rapide
//!
//! ```no_run
//! use tom_sdk::{Event, TomClientBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), tom_sdk::TomSdkError> {
//!     let mut client = TomClientBuilder::new()
//!         .username("alice")
//!         .connect()
//!         .await?;
//!
//!     println!("mon identité : {}", client.id());
//!
//!     while let Some(event) = client.next_event().await {
//!         if let Event::MessageReceived(msg) = event {
//!             println!("{} → {}", msg.from, msg.text());
//!             client.send_text(msg.from, "bien reçu !").await?;
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Connexion sans infrastructure
//!
//! Deux nœuds s'échangent leurs [`TomClient::ticket`] (hors bande : QR code,
//! copier-coller) puis s'enregistrent mutuellement via
//! [`TomClient::add_peer_ticket`] — aucun serveur requis sur un même LAN.

#![deny(missing_docs)]

mod builder;
mod client;
mod error;
mod event;

pub use builder::TomClientBuilder;
pub use client::TomClient;
pub use error::TomSdkError;
pub use event::{Event, Message};

// Types du protocole volontairement re-exportés (crates ToM originaux,
// aucun type de fork) — nécessaires pour consommer les événements.
pub use tom_protocol::{
    GroupId, GroupInfo, GroupInvite, GroupMember, GroupMemberRole, GroupMessage, LeaveReason,
    MessageStatus, MetricsSnapshot, NodeId,
};
