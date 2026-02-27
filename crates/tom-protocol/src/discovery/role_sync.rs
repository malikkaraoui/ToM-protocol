//! Role change announcements via gossip.

use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};

use crate::relay::PeerRole;
use crate::types::NodeId;

/// Broadcast when a peer's role changes (Peer <-> Relay).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleChangeAnnounce {
    pub node_id: NodeId,
    pub new_role: PeerRole,
    pub score: f64,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl RoleChangeAnnounce {
    /// Create and sign a role change announcement.
    pub fn new(
        node_id: NodeId,
        new_role: PeerRole,
        score: f64,
        timestamp: u64,
        secret_seed: &[u8; 32],
    ) -> Self {
        let mut announce = Self {
            node_id,
            new_role,
            score,
            timestamp,
            signature: Vec::new(),
        };
        announce.sign(secret_seed);
        announce
    }

    /// Sign the announcement.
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

    /// Get bytes to sign (excludes signature field).
    fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.node_id.as_bytes());
        bytes.push(match self.new_role {
            PeerRole::Peer => 0,
            PeerRole::Relay => 1,
        });
        bytes.extend_from_slice(&self.score.to_le_bytes());
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

    #[test]
    fn sign_and_verify_role_announce() {
        let (node_id, seed) = test_node_id();

        let announce =
            RoleChangeAnnounce::new(node_id, PeerRole::Relay, 15.5, 1000, &seed);

        assert!(announce.verify_signature(), "Signature should be valid");
    }

    #[test]
    fn tampered_announce_fails_verification() {
        let (node_id, seed) = test_node_id();

        let mut announce =
            RoleChangeAnnounce::new(node_id, PeerRole::Relay, 15.5, 1000, &seed);

        // Tamper with score
        announce.score = 100.0;

        assert!(
            !announce.verify_signature(),
            "Tampered signature should fail"
        );
    }
}
