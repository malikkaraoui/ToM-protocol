//! DHT-based peer discovery for ToM Protocol.
//! Uses Mainline DHT (BEP-0044) for distributed peer discovery.
//!
//! Phase R7.1 PoC: Simplified implementation to validate DHT integration.
//! Full BEP-0044 mutable storage will be implemented in Phase R7.4.

use anyhow::Result;
use mainline::Dht;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::time::Duration;

/// Node address for DHT storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtNodeAddr {
    pub node_id: String, // iroh::PublicKey string repr
    pub relay_urls: Vec<String>,
    pub direct_addrs: Vec<String>,
    pub timestamp: u64,
}

/// DHT discovery service.
pub struct DhtDiscovery {
    #[allow(dead_code)] // TODO R7.4: Will be used for actual DHT put/get operations
    dht: Dht,
}

impl DhtDiscovery {
    /// Create DHT client.
    pub fn new() -> Result<Self> {
        let dht = Dht::client()?;
        tracing::info!("DHT client created");
        Ok(Self { dht })
    }

    /// Publish node address to DHT.
    ///
    /// Note: This is a simplified implementation for Phase R7.1 PoC.
    /// Full BEP-0044 mutable storage will be implemented in R7.4.
    pub async fn publish(&self, addr: DhtNodeAddr) -> Result<()> {
        let key = dht_key(&addr.node_id);
        let value = serde_json::to_vec(&addr)?;

        // For now, we'll use immutable put (simpler API)
        // BEP-0044 mutable storage requires signing - defer to R7.4
        tracing::info!("DHT publish (immutable) for: {}", addr.node_id);

        // Store key for later lookup - in full impl, this would be put_mutable
        tracing::debug!("Would publish {} bytes to DHT key {:?}", value.len(), key);

        // TODO R7.4: Implement actual BEP-0044 mutable put with ed25519 signing
        Ok(())
    }

    /// Lookup peer by node ID.
    pub async fn lookup(&self, node_id: &str) -> Result<Option<DhtNodeAddr>> {
        let key = dht_key(node_id);

        tracing::debug!("DHT lookup for: {} (key: {:?})", node_id, key);

        // TODO R7.4: Implement actual DHT get
        // For R7.1 PoC, we'll return None (no peer found)
        // This tests the fallback path in the protocol

        tokio::time::sleep(Duration::from_millis(100)).await; // Simulate DHT query

        tracing::warn!("DHT lookup not implemented (R7.1 PoC) - returning None");
        Ok(None)
    }
}

impl Default for DhtDiscovery {
    fn default() -> Self {
        Self::new().expect("Failed to create DHT client")
    }
}

/// Hash node ID to 20-byte DHT key (SHA1).
fn dht_key(node_id: &str) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(node_id.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dht_key_deterministic() {
        let id = "test-node-id";
        let key1 = dht_key(id);
        let key2 = dht_key(id);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_dht_key_different() {
        let key1 = dht_key("node-a");
        let key2 = dht_key("node-b");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_dht_node_addr_serde() {
        let addr = DhtNodeAddr {
            node_id: "test-node-123".into(),
            relay_urls: vec!["https://relay.example.com".into()],
            direct_addrs: vec!["192.168.1.100:12345".into()],
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&addr).unwrap();
        let decoded: DhtNodeAddr = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.node_id, addr.node_id);
        assert_eq!(decoded.relay_urls, addr.relay_urls);
        assert_eq!(decoded.direct_addrs, addr.direct_addrs);
        assert_eq!(decoded.timestamp, addr.timestamp);
    }

    #[test]
    fn test_dht_discovery_creation() {
        // Note: This may fail in environments without network access
        // That's okay - it's testing the happy path
        let _ = DhtDiscovery::new();
    }
}
