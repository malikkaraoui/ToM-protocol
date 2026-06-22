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
use sha2::{Digest, Sha256};

/// Salt for BEP-0044 namespace isolation — prevents collisions with other DHT users.
const SALT: &[u8] = b"tom-addr-v1";

/// Max age for DHT records (2 hours). Older records are considered stale.
const MAX_DHT_AGE_MS: u64 = 2 * 3600 * 1000;

// ── Shared rendezvous (zero-config peer discovery) ──────────────────────────
//
// BEP-0044 only resolves a *known* public key — it cannot enumerate unknown
// peers. The rendezvous fills that gap WITHOUT a privileged node: a constant
// namespace derives a fixed set of shared keypairs ("slots"). Every ToM node
// can sign for them (they come from a public constant), so each node publishes
// its own {node_id, addrs} into the slot `hash(node_id) % SLOTS`, and any node
// reads all slots to find live peers it never heard of before.
//
// Multi-writer monotonicity: seq = publication timestamp, so the most recently
// active node in a slot always wins — readers get a fresh, connectable seed.
// One live seed is enough to join the gossip mesh; the rest follows.

/// Shared rendezvous namespace — every ToM node meets here.
const RENDEZVOUS_NAMESPACE: &[u8] = b"tom-protocol-rendezvous-v1";

/// Number of rendezvous slots: spreads writers (fewer same-slot collisions) and
/// gives readers several independent live seeds per discovery round.
pub const RENDEZVOUS_SLOTS: u8 = 8;

/// Salt for rendezvous items — distinct from the per-node address salt.
const RENDEZVOUS_SALT: &[u8] = b"tom-rdv-v1";

/// Derive the shared signing key for rendezvous slot `i` from the public constant.
fn rendezvous_slot_key(i: u8) -> SigningKey {
    let mut hasher = Sha256::new();
    hasher.update(RENDEZVOUS_NAMESPACE);
    hasher.update([i]);
    let seed: [u8; 32] = hasher.finalize().into();
    SigningKey::from_bytes(&seed)
}

/// Deterministic slot index for a node, spreading writers across slots.
fn slot_for_node(node_id: &str) -> u8 {
    let mut hasher = Sha256::new();
    hasher.update(node_id.as_bytes());
    let h: [u8; 32] = hasher.finalize().into();
    h[0] % RENDEZVOUS_SLOTS
}

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

    /// Publish this node into the shared rendezvous so unknown peers can find it.
    ///
    /// Writes `addr` to the slot derived from its `node_id`, using the
    /// publication timestamp as BEP-0044 seq (most recent writer wins).
    pub async fn publish_rendezvous(&self, addr: &DhtNodeAddr) -> Result<()> {
        rendezvous_publish(&self.dht, addr).await
    }

    /// Discover live peers from the shared rendezvous (reads every slot).
    ///
    /// Returns fresh records (< 2h) excluding `own_node_id`. Never errors on a
    /// single bad/missing slot — best-effort enumeration.
    pub async fn discover_rendezvous(&self, own_node_id: &str) -> Vec<DhtNodeAddr> {
        rendezvous_discover(&self.dht, own_node_id).await
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

/// Tolerated clock skew for rendezvous timestamps (1h). Entries dated further in
/// the future than this are rejected: with seq = timestamp a far-future record
/// would otherwise win its slot permanently and never age out (clock-skew bug or
/// deliberate squatting). 1h comfortably covers real NTP/timezone misconfig.
const RENDEZVOUS_FUTURE_SKEW_MS: u64 = 60 * 60 * 1000;

/// Freshness test for a rendezvous entry — rejects both stale and implausibly
/// future timestamps. Uses saturating arithmetic so a future timestamp can never
/// underflow into "fresh".
fn rendezvous_entry_is_fresh(timestamp: u64, now: u64) -> bool {
    if timestamp > now.saturating_add(RENDEZVOUS_FUTURE_SKEW_MS) {
        return false; // implausibly far in the future
    }
    now.saturating_sub(timestamp) <= MAX_DHT_AGE_MS
}

/// Standalone rendezvous publish — for spawned tasks holding a cloned `AsyncDht`.
pub async fn rendezvous_publish(dht: &AsyncDht, addr: &DhtNodeAddr) -> Result<()> {
    let slot = slot_for_node(&addr.node_id);
    let signer = rendezvous_slot_key(slot);
    let value = serde_json::to_vec(addr).context("failed to serialize rendezvous addr")?;
    // seq = timestamp → later writer always wins, no cross-writer seq coordination.
    let seq = addr.timestamp as i64;
    let item = MutableItem::new(signer, &value, seq, Some(RENDEZVOUS_SALT));

    dht.put_mutable(item, None)
        .await
        .map_err(|e| anyhow::anyhow!("rendezvous put_mutable (slot {slot}) failed: {e}"))?;

    tracing::info!(node_id = %addr.node_id, slot, "published to rendezvous");
    Ok(())
}

/// Standalone rendezvous discovery — reads every slot, returns fresh live peers.
///
/// Best-effort: a missing or malformed slot is skipped, never fatal. Records
/// older than [`MAX_DHT_AGE_MS`] and the caller's own `own_node_id` are excluded.
/// The result is de-duplicated by node_id (a node may briefly appear twice if it
/// changed slots after an identity change — keep the freshest).
pub async fn rendezvous_discover(dht: &AsyncDht, own_node_id: &str) -> Vec<DhtNodeAddr> {
    let now = now_ms();
    let mut found: Vec<DhtNodeAddr> = Vec::new();

    for i in 0..RENDEZVOUS_SLOTS {
        let pk = rendezvous_slot_key(i).verifying_key().to_bytes();
        let Some(item) = dht.get_mutable_most_recent(&pk, Some(RENDEZVOUS_SALT)).await else {
            continue;
        };
        let Ok(addr) = serde_json::from_slice::<DhtNodeAddr>(item.value()) else {
            tracing::debug!(slot = i, "rendezvous: malformed slot value, skipping");
            continue;
        };
        if addr.node_id == own_node_id {
            continue;
        }
        if !rendezvous_entry_is_fresh(addr.timestamp, now) {
            tracing::debug!(
                slot = i,
                node_id = %addr.node_id,
                ts = addr.timestamp,
                "rendezvous: stale or implausibly-future entry, skipping"
            );
            continue;
        }
        // De-dup by node_id, keeping the freshest timestamp.
        if let Some(existing) = found.iter_mut().find(|a| a.node_id == addr.node_id) {
            if addr.timestamp > existing.timestamp {
                *existing = addr;
            }
        } else {
            found.push(addr);
        }
    }

    tracing::info!(peers = found.len(), "rendezvous discovery complete");
    found
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

    // ── Rendezvous: slot derivation (pure, no DHT) ───────────────────────────

    fn fresh_addr(node_id: &str) -> DhtNodeAddr {
        DhtNodeAddr {
            node_id: node_id.into(),
            relay_urls: vec![format!("http://relay/{node_id}")],
            direct_addrs: vec!["10.0.0.1:3340".into()],
            timestamp: now_ms(),
        }
    }

    fn stale_addr(node_id: &str) -> DhtNodeAddr {
        DhtNodeAddr {
            timestamp: now_ms() - 3 * 3600 * 1000,
            ..fresh_addr(node_id)
        }
    }

    /// Pick `n` node_ids that each land in a distinct rendezvous slot.
    fn distinct_slot_ids(n: usize) -> Vec<String> {
        let mut out = Vec::new();
        let mut used = std::collections::HashSet::new();
        let mut i = 0u32;
        while out.len() < n {
            let id = format!("node-{i}");
            if used.insert(slot_for_node(&id)) {
                out.push(id);
            }
            i += 1;
        }
        out
    }

    /// Find two distinct node_ids that collide into the same slot.
    fn same_slot_pair() -> (String, String) {
        let first = "collide-0".to_string();
        let s0 = slot_for_node(&first);
        let mut i = 1u32;
        loop {
            let cand = format!("collide-{i}");
            if cand != first && slot_for_node(&cand) == s0 {
                return (first, cand);
            }
            i += 1;
        }
    }

    #[test]
    fn rendezvous_freshness_rejects_stale_and_future() {
        let now = 10 * 3600 * 1000; // arbitrary "now" well above zero
        // Fresh: within max age.
        assert!(rendezvous_entry_is_fresh(now, now));
        assert!(rendezvous_entry_is_fresh(now - 1000, now));
        assert!(rendezvous_entry_is_fresh(now - MAX_DHT_AGE_MS, now));
        // Stale: older than max age.
        assert!(!rendezvous_entry_is_fresh(now - MAX_DHT_AGE_MS - 1, now));
        // Small future skew tolerated (clock differences).
        assert!(rendezvous_entry_is_fresh(now + 1000, now));
        assert!(rendezvous_entry_is_fresh(now + RENDEZVOUS_FUTURE_SKEW_MS, now));
        // Implausibly far future rejected (would otherwise squat a slot forever).
        assert!(!rendezvous_entry_is_fresh(now + RENDEZVOUS_FUTURE_SKEW_MS + 1, now));
        assert!(!rendezvous_entry_is_fresh(u64::MAX, now));
    }

    #[test]
    fn rendezvous_freshness_no_underflow_at_zero() {
        // now near zero must not underflow into "fresh" for future timestamps.
        assert!(rendezvous_entry_is_fresh(0, 0));
        assert!(!rendezvous_entry_is_fresh(u64::MAX, 0));
    }

    #[test]
    fn rendezvous_slot_in_range() {
        for i in 0..1000u32 {
            let s = slot_for_node(&format!("n{i}"));
            assert!(s < RENDEZVOUS_SLOTS, "slot {s} out of range");
        }
    }

    #[test]
    fn rendezvous_slot_deterministic() {
        assert_eq!(slot_for_node("alice"), slot_for_node("alice"));
        // Distinct inputs are allowed to collide, but must each be stable.
        let a = slot_for_node("bob");
        let b = slot_for_node("bob");
        assert_eq!(a, b);
    }

    #[test]
    fn rendezvous_slot_keys_distinct_and_deterministic() {
        let mut keys = std::collections::HashSet::new();
        for i in 0..RENDEZVOUS_SLOTS {
            let k1 = rendezvous_slot_key(i).verifying_key().to_bytes();
            let k2 = rendezvous_slot_key(i).verifying_key().to_bytes();
            assert_eq!(k1, k2, "slot key {i} not deterministic");
            assert!(keys.insert(k1), "slot key {i} collides with another slot");
        }
    }

    #[test]
    fn rendezvous_distinct_slot_ids_helper_works() {
        let ids = distinct_slot_ids(RENDEZVOUS_SLOTS as usize);
        let slots: std::collections::HashSet<u8> = ids.iter().map(|s| slot_for_node(s)).collect();
        assert_eq!(slots.len(), RENDEZVOUS_SLOTS as usize, "should occupy every slot");
    }

    // ── Rendezvous: live DHT (Testnet) ───────────────────────────────────────

    #[test]
    fn rendezvous_empty_returns_nothing() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();
            let reader = make_dht(&testnet);
            let found = reader.discover_rendezvous("whoever").await;
            assert!(found.is_empty(), "empty rendezvous must yield no peers");
        }
        futures_lite::future::block_on(test());
    }

    #[test]
    fn rendezvous_zero_knowledge_discovery() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();
            // 3 publishers in distinct slots so all survive.
            let ids = distinct_slot_ids(3);
            for id in &ids {
                make_dht(&testnet)
                    .publish_rendezvous(&fresh_addr(id))
                    .await
                    .expect("publish_rendezvous");
            }

            // A 4th node that knows NOBODY discovers all three.
            let newcomer = make_dht(&testnet);
            let found = newcomer.discover_rendezvous("newcomer-self").await;

            let found_ids: std::collections::HashSet<_> =
                found.iter().map(|a| a.node_id.clone()).collect();
            for id in &ids {
                assert!(found_ids.contains(id), "newcomer should discover {id}, got {found_ids:?}");
            }
            // Each carries a connectable identity + addr.
            for a in &found {
                assert!(!a.node_id.is_empty());
                assert!(!a.relay_urls.is_empty() || !a.direct_addrs.is_empty());
            }
        }
        futures_lite::future::block_on(test());
    }

    #[test]
    fn rendezvous_excludes_self() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();
            make_dht(&testnet)
                .publish_rendezvous(&fresh_addr("me"))
                .await
                .unwrap();
            let found = make_dht(&testnet).discover_rendezvous("me").await;
            assert!(found.iter().all(|a| a.node_id != "me"), "must not discover self");
        }
        futures_lite::future::block_on(test());
    }

    #[test]
    fn rendezvous_stale_is_filtered() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();
            make_dht(&testnet)
                .publish_rendezvous(&stale_addr("ghost"))
                .await
                .unwrap();
            let found = make_dht(&testnet).discover_rendezvous("reader").await;
            assert!(
                found.iter().all(|a| a.node_id != "ghost"),
                "stale (>2h) rendezvous entry must not be discovered"
            );
        }
        futures_lite::future::block_on(test());
    }

    #[test]
    fn rendezvous_freshest_wins_in_same_slot() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();
            let (a, b) = same_slot_pair();
            assert_eq!(slot_for_node(&a), slot_for_node(&b));

            // Older first, newer second — newer (higher seq) must win the slot.
            let mut older = fresh_addr(&a);
            older.timestamp = now_ms() - 60_000;
            let newer = fresh_addr(&b); // now_ms() > older

            make_dht(&testnet).publish_rendezvous(&older).await.unwrap();
            make_dht(&testnet).publish_rendezvous(&newer).await.unwrap();

            let found = make_dht(&testnet).discover_rendezvous("reader").await;
            let ids: std::collections::HashSet<_> =
                found.iter().map(|x| x.node_id.clone()).collect();
            assert!(ids.contains(&b), "freshest writer {b} should win the slot");
            assert!(!ids.contains(&a), "stale-in-slot {a} should be overwritten");
        }
        futures_lite::future::block_on(test());
    }

    #[test]
    fn rendezvous_republish_updates_address() {
        async fn test() {
            let testnet = Testnet::builder(10).build().unwrap();
            let mut addr = fresh_addr("mover");
            addr.direct_addrs = vec!["1.1.1.1:3340".into()];
            make_dht(&testnet).publish_rendezvous(&addr).await.unwrap();

            // Node moves networks: new addr, newer timestamp.
            addr.direct_addrs = vec!["2.2.2.2:3340".into()];
            addr.timestamp = now_ms() + 1;
            make_dht(&testnet).publish_rendezvous(&addr).await.unwrap();

            let found = make_dht(&testnet).discover_rendezvous("reader").await;
            let mover = found.iter().find(|a| a.node_id == "mover").expect("find mover");
            assert_eq!(mover.direct_addrs, vec!["2.2.2.2:3340".to_string()], "must see updated addr");
        }
        futures_lite::future::block_on(test());
    }

    // ── CHAOS: hardcore churn / disconnection stress ─────────────────────────

    #[test]
    fn rendezvous_chaos_churn_stress() {
        async fn test() {
            let testnet = Testnet::builder(15).build().unwrap();

            // 16 nodes storm the rendezvous: even = alive (fresh), odd = dead (stale).
            let mut alive = std::collections::HashSet::new();
            let mut dead = std::collections::HashSet::new();
            for i in 0..16u32 {
                let id = format!("chaos-{i}");
                let addr = if i % 2 == 0 {
                    alive.insert(id.clone());
                    fresh_addr(&id)
                } else {
                    dead.insert(id.clone());
                    stale_addr(&id)
                };
                // Best-effort: same-slot lower-seq writes may be rejected — that's fine.
                let _ = make_dht(&testnet).publish_rendezvous(&addr).await;
            }

            let found = make_dht(&testnet).discover_rendezvous("chaos-self").await;
            let found_ids: Vec<String> = found.iter().map(|a| a.node_id.clone()).collect();

            // Invariants that MUST hold under chaos:
            // 1. No dead (stale) node ever surfaces.
            for id in &found_ids {
                assert!(!dead.contains(id), "stale node {id} must never be discovered");
                assert!(alive.contains(id), "discovered {id} must be a live publisher");
            }
            // 2. No duplicates.
            let unique: std::collections::HashSet<_> = found_ids.iter().collect();
            assert_eq!(unique.len(), found_ids.len(), "no duplicate node_ids");
            // 3. Never exceed slot capacity.
            assert!(
                found.len() <= RENDEZVOUS_SLOTS as usize,
                "found {} > {} slots",
                found.len(),
                RENDEZVOUS_SLOTS
            );
            // 4. At least one live seed surfaced — a newcomer can always bootstrap.
            assert!(!found.is_empty(), "chaos must still yield at least one live seed");
        }
        futures_lite::future::block_on(test());
    }

    #[test]
    fn rendezvous_recovery_after_total_blackout() {
        async fn test() {
            let testnet = Testnet::builder(12).build().unwrap();

            // Phase 1: everyone is dead (stale). A newcomer finds nothing.
            for i in 0..4u32 {
                let _ = make_dht(&testnet)
                    .publish_rendezvous(&stale_addr(&format!("old-{i}")))
                    .await;
            }
            let blackout = make_dht(&testnet).discover_rendezvous("survivor").await;
            assert!(blackout.is_empty(), "all-stale rendezvous must look empty");

            // Phase 2: ONE node comes back alive → the network is rejoinable again.
            make_dht(&testnet)
                .publish_rendezvous(&fresh_addr("revived"))
                .await
                .unwrap();
            let recovered = make_dht(&testnet).discover_rendezvous("survivor").await;
            assert!(
                recovered.iter().any(|a| a.node_id == "revived"),
                "a single revived node must restore discoverability"
            );
        }
        futures_lite::future::block_on(test());
    }
}
