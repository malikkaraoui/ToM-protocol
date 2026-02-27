//! tom-connect — ToM transport layer (forked from iroh)
//!
//! Phase R7.2: Skeleton only. MagicSock copy happens in R7.3.

/// Node identity (Ed25519 public key).
/// Will replace iroh::PublicKey in R7.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId([u8; 32]);

impl NodeId {
    /// Create from 32-byte Ed25519 public key.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Get bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Node address (ID + relay URLs + direct addresses).
/// Will replace iroh::NodeAddr in R7.3.
#[derive(Debug, Clone)]
pub struct NodeAddr {
    /// Node ID.
    pub node_id: NodeId,
    /// Relay server URLs.
    pub relay_urls: Vec<String>,
    /// Direct IP addresses.
    pub direct_addrs: Vec<std::net::SocketAddr>,
}

/// Endpoint placeholder.
/// Will contain MagicSock + Quinn in R7.3.
pub struct Endpoint;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_roundtrip() {
        let bytes = [42u8; 32];
        let id = NodeId::from_bytes(bytes);
        assert_eq!(id.as_bytes(), &bytes);
    }

    #[test]
    fn test_node_addr_creation() {
        let id = NodeId::from_bytes([1u8; 32]);
        let addr = NodeAddr {
            node_id: id,
            relay_urls: vec!["https://relay.example.com".into()],
            direct_addrs: vec!["192.168.1.100:12345".parse().unwrap()],
        };

        assert_eq!(addr.relay_urls.len(), 1);
        assert_eq!(addr.direct_addrs.len(), 1);
    }
}
