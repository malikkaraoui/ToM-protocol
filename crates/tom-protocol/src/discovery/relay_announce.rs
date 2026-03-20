//! Relay-ready announcements via gossip.
//!
//! Separate from PeerAnnounce: this is a dedicated, signed announcement
//! that a node has an embedded relay ready for use.
//! Consumers will use this to discover available relays — but that's a future chantier.

use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use tom_connect::RelayUrl;

use crate::types::NodeId;

/// Tag byte to disambiguate from PeerAnnounce/RoleChangeAnnounce in gossip.
///
/// Gossip messages are tried in order: PeerAnnounce, RoleChangeAnnounce, RelayReadyAnnounce.
/// This tag makes deserialization unambiguous.
pub const RELAY_READY_TAG: u8 = 0x52; // 'R'

/// Announce that this node has a healthy embedded relay available.
///
/// - Signed by the announcing node (Ed25519)
/// - Contains the relay URL
/// - Separate from PeerAnnounce (no coupling)
/// - NOT an auto-selection trigger — just a publication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayReadyAnnounce {
    /// Tag for disambiguation in gossip deserialization.
    pub tag: u8,
    /// The node hosting the embedded relay.
    pub node_id: NodeId,
    /// The URL where the relay is listening.
    pub relay_url: RelayUrl,
    /// Unix timestamp (ms) when this announce was created.
    pub timestamp: u64,
    /// Ed25519 signature over signing_bytes().
    pub signature: Vec<u8>,
}

impl RelayReadyAnnounce {
    /// Create and sign a relay-ready announcement.
    pub fn new(
        node_id: NodeId,
        relay_url: RelayUrl,
        timestamp: u64,
        secret_seed: &[u8; 32],
    ) -> Self {
        let mut announce = Self {
            tag: RELAY_READY_TAG,
            node_id,
            relay_url,
            timestamp,
            signature: Vec::new(),
        };
        announce.sign(secret_seed);
        announce
    }

    /// Sign the announcement with the node's secret key.
    fn sign(&mut self, secret_seed: &[u8; 32]) {
        let signing_key = SigningKey::from_bytes(secret_seed);
        let sig = signing_key.sign(&self.signing_bytes());
        self.signature = sig.to_bytes().to_vec();
    }

    /// Verify the signature against the node_id (public key).
    pub fn verify_signature(&self) -> bool {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let pk_bytes: [u8; 32] = self.node_id.as_bytes();

        let verifying_key = match VerifyingKey::from_bytes(&pk_bytes) {
            Ok(k) => k,
            Err(_) => return false,
        };

        if self.signature.len() != 64 {
            return false;
        }

        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&self.signature);
        let signature = Signature::from_bytes(&sig_bytes);

        verifying_key
            .verify(&self.signing_bytes(), &signature)
            .is_ok()
    }

    /// Check if the timestamp is reasonably fresh (within 5 min future drift + 1 hour past).
    pub fn is_fresh(&self, now_ms: u64) -> bool {
        let max_future = 5 * 60 * 1000; // 5 min
        let max_past = 60 * 60 * 1000; // 1 hour
        self.timestamp <= now_ms + max_future && self.timestamp + max_past >= now_ms
    }

    /// Bytes used for signing (excludes signature field).
    fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.push(RELAY_READY_TAG);
        bytes.extend_from_slice(&self.node_id.as_bytes());
        bytes.extend_from_slice(self.relay_url.as_str().as_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn test_node_id() -> (NodeId, [u8; 32]) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        let seed = secret.to_bytes();
        let node_id = secret.public().to_string().parse().unwrap();
        (node_id, seed)
    }

    fn test_relay_url() -> RelayUrl {
        "http://127.0.0.1:3340".parse().unwrap()
    }

    #[test]
    fn sign_and_verify_relay_announce() {
        let (node_id, seed) = test_node_id();
        let url = test_relay_url();

        let announce = RelayReadyAnnounce::new(node_id, url, 1000, &seed);

        assert_eq!(announce.tag, RELAY_READY_TAG);
        assert!(announce.verify_signature(), "Signature should be valid");
    }

    #[test]
    fn tampered_url_fails_verification() {
        let (node_id, seed) = test_node_id();
        let url = test_relay_url();

        let mut announce = RelayReadyAnnounce::new(node_id, url, 1000, &seed);

        // Tamper with URL
        announce.relay_url = "http://evil.example.com:9999".parse().unwrap();

        assert!(
            !announce.verify_signature(),
            "Tampered URL should fail verification"
        );
    }

    #[test]
    fn tampered_timestamp_fails_verification() {
        let (node_id, seed) = test_node_id();
        let url = test_relay_url();

        let mut announce = RelayReadyAnnounce::new(node_id, url, 1000, &seed);

        announce.timestamp = 9999;

        assert!(
            !announce.verify_signature(),
            "Tampered timestamp should fail"
        );
    }

    #[test]
    fn freshness_check() {
        let (node_id, seed) = test_node_id();
        let url = test_relay_url();

        let now = 1_000_000;
        let announce = RelayReadyAnnounce::new(node_id, url, now, &seed);

        assert!(announce.is_fresh(now));
        assert!(announce.is_fresh(now + 30_000)); // 30s later: still fresh
        assert!(!announce.is_fresh(now + 3_700_000)); // >1h later: stale
    }

    #[test]
    fn wrong_signer_fails_verification() {
        let (node_id, _seed) = test_node_id();
        let url = test_relay_url();

        // Sign with a different key
        let mut rng = rand::rngs::StdRng::seed_from_u64(99);
        let other_secret = tom_connect::SecretKey::generate(&mut rng);
        let other_seed = other_secret.to_bytes();

        let announce = RelayReadyAnnounce::new(node_id, url, 1000, &other_seed);

        assert!(
            !announce.verify_signature(),
            "Wrong signer should fail verification"
        );
    }
}
