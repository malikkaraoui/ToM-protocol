//! ToM Protocol transport layer.
//!
//! Wraps iroh's QUIC connectivity (hole punching, relay fallback, E2E encryption)
//! behind a stable API. If iroh changes or disappears, swap the internals
//! without touching the public surface.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use tom_transport::{TomNode, TomNodeConfig, MessageEnvelope, NodeId};
//!
//! # async fn example() -> Result<(), tom_transport::TomTransportError> {
//! let mut node = TomNode::bind(TomNodeConfig::new()).await?;
//! println!("My ID: {}", node.id());
//!
//! // Send a message
//! let target: NodeId = "abc123...".parse()?;
//! let envelope = MessageEnvelope::new(node.id(), target, "chat", serde_json::json!({"text": "Hello!"}));
//! node.send(target, &envelope).await?;
//!
//! // Receive messages
//! let (from, msg) = node.recv().await?;
//! println!("From {from}: {:?}", msg.payload);
//!
//! node.shutdown().await?;
//! # Ok(())
//! # }
//! ```

mod config;
mod connection;
mod envelope;
mod error;
mod node;
mod path;
mod protocol;

pub use config::TomNodeConfig;
pub use envelope::MessageEnvelope;
pub use error::TomTransportError;
pub use node::TomNode;
pub use path::{PathEvent, PathKind};

use std::fmt;
use std::str::FromStr;

/// ToM network identity — Ed25519 public key.
///
/// Wraps iroh's `EndpointId`. Displayed and parsed as hex string.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(iroh::EndpointId);

impl NodeId {
    /// Create from an iroh EndpointId.
    pub(crate) fn from_endpoint_id(id: iroh::EndpointId) -> Self {
        Self(id)
    }

    /// Access the underlying iroh EndpointId.
    pub(crate) fn as_endpoint_id(&self) -> &iroh::EndpointId {
        &self.0
    }

    /// Get the raw 32-byte public key.
    pub fn as_bytes(&self) -> [u8; 32] {
        *self.0.as_bytes()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex = self.0.to_string();
        let short = if hex.len() > 12 { &hex[..12] } else { &hex };
        write!(f, "NodeId({short}...)")
    }
}

impl FromStr for NodeId {
    type Err = TomTransportError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id: iroh::EndpointId = s
            .parse()
            .map_err(|_| TomTransportError::InvalidNodeId(s.to_string()))?;
        Ok(Self(id))
    }
}

impl serde::Serialize for NodeId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for NodeId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// ALPN protocol identifier for ToM transport.
pub const TOM_ALPN: &[u8] = b"tom-protocol/transport/0";
