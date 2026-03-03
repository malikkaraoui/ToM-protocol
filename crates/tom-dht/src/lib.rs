//! DHT-based peer discovery for ToM Protocol.
//!
//! Uses Mainline DHT [BEP-0044](https://www.bittorrent.org/beps/bep_0044.html)
//! mutable storage for distributed peer address publication and lookup.
//!
//! Each node publishes its network coordinates (relay URLs, direct addresses)
//! signed with its ed25519 identity key. Any node can look it up by
//! its public key — no central server required.

use std::sync::atomic::{AtomicI64, Ordering};

use anyhow::{Context, Result};
pub use mainline::async_dht::AsyncDht;
use mainline::{Dht, MutableItem, SigningKey};
use serde::{Deserialize, Serialize};

/// Salt for BEP-0044 namespace isolation — prevents collisions with other DHT users.
const SALT: &[u8] = b"tom-addr-v1";

/// Max age for DHT records (2 hours). Older records are considered stale.
const MAX_DHT_AGE_MS: u64 = 2 * 3600 * 1000;

/// Node address stored in the DHT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DhtNodeAddr {
    /// Ed25519 public key as base32 string.
    pub node_id: String,
    /// Relay server URLs (e.g. "https://relay.example.com").
    pub relay_urls: Vec<String>,
    /// Direct network addresses (e.g. "192.168.1.100:3340").
    pub direct_addrs: Vec<String>,
    /// Publication timestamp (Unix ms).
    pub timestamp: u64,
}

/// DHT discovery service — publish and lookup node addresses via BEP-0044.
///
/// Uses ed25519-signed mutable items so only the key owner can update their record.
/// The DHT client runs in a background thread (mainline actor); all public methods are async.
pub struct DhtDiscovery {
    dht: AsyncDht,
    /// Monotonically increasing sequence number for BEP-0044 versioning.
    seq: AtomicI64,
}

impl DhtDiscovery {
    /// Create a new DHT discovery client.
    ///
    /// Bootstraps from well-known mainline DHT nodes. The client runs in
    /// the background — no listening port required.
    pub fn new() -> Result<Self> {
        let dht = Dht::client()
            .context("failed to create mainline DHT client")?
            .as_async();
        tracing::info!("DHT discovery client created (BEP-0044)");
        Ok(Self {
            dht,
            seq: AtomicI64::new(0),
        })
    }

    /// Create a DHT discovery client from a builder-configured DHT.
    ///
    /// Useful for tests (local testnet) or custom bootstrap nodes.
    pub fn from_dht(dht: Dht) -> Self {
        Self {
            dht: dht.as_async(),
            seq: AtomicI64::new(0),
        }
    }

    /// Publish this node's address to the DHT.
    ///
    /// The record is signed with the node's ed25519 key and stored as a
    /// BEP-0044 mutable item. Other nodes can look it up by public key.
    ///
    /// `signing_key_bytes` is the 32-byte ed25519 secret key seed.
    pub async fn publish(&self, signing_key_bytes: &[u8; 32], addr: &DhtNodeAddr) -> Result<()> {
        let value = serde_json::to_vec(addr).context("failed to serialize DhtNodeAddr")?;
        let seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        let signer = SigningKey::from_bytes(signing_key_bytes);

        let item = MutableItem::new(signer, &value, seq, Some(SALT));

        self.dht
            .put_mutable(item, None)
            .await
            .map_err(|e| anyhow::anyhow!("DHT put_mutable failed: {e}"))?;

        tracing::info!(
            node_id = %addr.node_id,
            seq,
            relays = addr.relay_urls.len(),
            addrs = addr.direct_addrs.len(),
            "published to DHT"
        );
        Ok(())
    }

    /// Get a clonable handle to the async DHT client.
    ///
    /// Useful for spawning lookup tasks that run concurrently with the main loop.
    pub fn async_dht(&self) -> AsyncDht {
        self.dht.clone()
    }

    /// Look up a node's address by its ed25519 public key.
    ///
    /// Returns `None` if the node hasn't published to the DHT or if the
    /// record is too old (> 2 hours).
    pub async fn lookup(&self, public_key: &[u8; 32]) -> Result<Option<DhtNodeAddr>> {
        tracing::debug!("DHT lookup for key {}", hex_encode(public_key));

        let result = self
            .dht
            .get_mutable_most_recent(public_key, Some(SALT))
            .await;

        let item = match result {
            Some(item) => item,
            None => {
                tracing::debug!("DHT lookup: no record found");
                return Ok(None);
            }
        };

        let addr: DhtNodeAddr = serde_json::from_slice(item.value())
            .context("failed to deserialize DHT record")?;

        // Validate freshness
        let now = now_ms();
        if now > addr.timestamp && now - addr.timestamp > MAX_DHT_AGE_MS {
            tracing::debug!(
                age_ms = now - addr.timestamp,
                "DHT record too old, ignoring"
            );
            return Ok(None);
        }

        tracing::info!(
            node_id = %addr.node_id,
            seq = item.seq(),
            relays = addr.relay_urls.len(),
            addrs = addr.direct_addrs.len(),
            "DHT lookup success"
        );
        Ok(Some(addr))
    }
}

/// Standalone DHT lookup — for use in spawned tasks.
///
/// Takes a cloned `AsyncDht` (from `DhtDiscovery::async_dht()`) so it can
/// run concurrently without borrowing the DhtDiscovery.
pub async fn dht_lookup(dht: &AsyncDht, public_key: &[u8; 32]) -> Result<Option<DhtNodeAddr>> {
    let result = dht
        .get_mutable_most_recent(public_key, Some(SALT))
        .await;

    let item = match result {
        Some(item) => item,
        None => return Ok(None),
    };

    let addr: DhtNodeAddr = serde_json::from_slice(item.value())
        .context("failed to deserialize DHT record")?;

    let now = now_ms();
    if now > addr.timestamp && now - addr.timestamp > MAX_DHT_AGE_MS {
        return Ok(None);
    }

    Ok(Some(addr))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mainline::Testnet;
    use std::net::Ipv4Addr;

    fn make_dht(testnet: &Testnet) -> DhtDiscovery {
        let dht = Dht::builder()
            .bootstrap(&testnet.bootstrap)
            .bind_address(Ipv4Addr::LOCALHOST)
            .build()
            .unwrap();
        DhtDiscovery::from_dht(dht)
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
        assert_eq!(decoded, addr);
    }

    #[test]
    fn test_dht_discovery_creation() {
        // May fail in environments without network access — that's OK.
        let _ = DhtDiscovery::new();
    }

    #[test]
    fn test_publish_and_lookup_roundtrip() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();
            let publisher = make_dht(&testnet);
            let reader = make_dht(&testnet);

            let signing_key_bytes = [42u8; 32];
            let signer = SigningKey::from_bytes(&signing_key_bytes);
            let public_key = signer.verifying_key().to_bytes();

            let addr = DhtNodeAddr {
                node_id: "test-node-roundtrip".into(),
                relay_urls: vec!["http://relay.test:3340".into()],
                direct_addrs: vec!["10.0.0.1:3340".into()],
                timestamp: now_ms(),
            };

            publisher
                .publish(&signing_key_bytes, &addr)
                .await
                .expect("publish failed");

            let found = reader
                .lookup(&public_key)
                .await
                .expect("lookup failed")
                .expect("should find published record");

            assert_eq!(found.node_id, "test-node-roundtrip");
            assert_eq!(found.relay_urls, vec!["http://relay.test:3340"]);
            assert_eq!(found.direct_addrs, vec!["10.0.0.1:3340"]);
        }

        futures_lite::future::block_on(test());
    }

    #[test]
    fn test_lookup_nonexistent() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();
            let reader = make_dht(&testnet);

            let random_key = SigningKey::from_bytes(&[99u8; 32])
                .verifying_key()
                .to_bytes();

            let result = reader.lookup(&random_key).await.expect("lookup failed");
            assert!(result.is_none());
        }

        futures_lite::future::block_on(test());
    }

    #[test]
    fn test_publish_increments_seq() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();
            let dht = make_dht(&testnet);

            let addr = DhtNodeAddr {
                node_id: "seq-test".into(),
                relay_urls: vec![],
                direct_addrs: vec![],
                timestamp: now_ms(),
            };

            dht.publish(&[7u8; 32], &addr).await.unwrap();
            assert_eq!(dht.seq.load(Ordering::Relaxed), 1);

            dht.publish(&[7u8; 32], &addr).await.unwrap();
            assert_eq!(dht.seq.load(Ordering::Relaxed), 2);
        }

        futures_lite::future::block_on(test());
    }

    #[test]
    fn test_stale_record_filtered() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();
            let publisher = make_dht(&testnet);
            let reader = make_dht(&testnet);

            let signing_key_bytes = [55u8; 32];
            let signer = SigningKey::from_bytes(&signing_key_bytes);
            let public_key = signer.verifying_key().to_bytes();

            // Publish with a timestamp 3 hours in the past
            let addr = DhtNodeAddr {
                node_id: "stale-node".into(),
                relay_urls: vec![],
                direct_addrs: vec![],
                timestamp: now_ms() - 3 * 3600 * 1000,
            };

            publisher.publish(&signing_key_bytes, &addr).await.unwrap();

            let result = reader.lookup(&public_key).await.expect("lookup failed");
            assert!(result.is_none(), "stale record should be filtered");
        }

        futures_lite::future::block_on(test());
    }
}
