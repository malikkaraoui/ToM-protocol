//! ToM Protocol layer.
//!
//! Implements routing, encryption, discovery, and group messaging
//! on top of `tom-transport` (QUIC via iroh).
//!
//! Wire format: MessagePack (compact binary).
//! Crypto: Ed25519 signatures + XChaCha20-Poly1305 encryption.

pub mod crypto;
pub mod envelope;
pub mod error;
pub mod relay;
pub mod router;
pub mod tracker;
pub mod types;

pub use crypto::EncryptedPayload;
pub use envelope::{Envelope, EnvelopeBuilder};
pub use error::TomProtocolError;
pub use relay::{PeerInfo, PeerRole, PeerStatus, RelaySelector, Topology};
pub use router::{AckPayload, AckType, ReadReceiptPayload, Router, RoutingAction};
pub use tracker::{MessageTracker, StatusChange};
pub use types::{MessageStatus, MessageType, NodeId};
