//! ToM Protocol layer.
//!
//! Implements routing, encryption, discovery, and group messaging
//! on top of `tom-transport` (QUIC via iroh).
//!
//! Wire format: MessagePack (compact binary).
//! Crypto: Ed25519 signatures + XChaCha20-Poly1305 encryption.

pub mod envelope;
pub mod error;
pub mod types;

pub use envelope::Envelope;
pub use error::TomProtocolError;
pub use types::{MessageStatus, MessageType, NodeId};
