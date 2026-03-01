//! ToM Protocol transport layer.
//!
//! Wraps QUIC connectivity (hole punching, relay fallback, E2E encryption)
//! via tom-connect behind a stable API.
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
pub use envelope::{now_ms, MessageEnvelope};
pub use error::TomTransportError;
pub use node::TomNode;
pub use path::{PathEvent, PathKind};

// Re-export gossip types for protocol layer
pub use tom_gossip;

// Re-export connect types for custom relay configuration and address exchange
pub use tom_connect::{EndpointAddr, RelayUrl};

use std::fmt;
use std::str::FromStr;

/// ToM network identity — Ed25519 public key.
///
/// Wraps `EndpointId`. Displayed and parsed as hex string.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(tom_connect::EndpointId);

impl NodeId {
    /// Create from an EndpointId.
    pub fn from_endpoint_id(id: tom_connect::EndpointId) -> Self {
        Self(id)
    }

    /// Access the underlying EndpointId.
    pub fn as_endpoint_id(&self) -> &tom_connect::EndpointId {
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
        let id: tom_connect::EndpointId = s
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
