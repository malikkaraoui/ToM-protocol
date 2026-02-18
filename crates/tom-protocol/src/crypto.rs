/// End-to-end encryption for ToM protocol.
///
/// Uses ephemeral X25519 Diffie-Hellman + XChaCha20-Poly1305 AEAD.
/// Each message gets a fresh ephemeral keypair for forward secrecy.
///
/// Key derivation: Ed25519 (iroh NodeId) → X25519 via standard
/// Edwards→Montgomery conversion (same as libsodium).
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use curve25519_dalek::edwards::CompressedEdwardsY;
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519Secret};

use crate::TomProtocolError;

/// HKDF info string for domain separation.
const HKDF_INFO: &[u8] = b"tom-protocol-e2e-xchacha20poly1305-v1";

/// Encrypted payload with ephemeral key exchange metadata.
///
/// Contains everything needed to decrypt: ciphertext, nonce, and the
/// sender's ephemeral X25519 public key for DH key recovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// XChaCha20-Poly1305 ciphertext (includes 16-byte auth tag).
    pub ciphertext: Vec<u8>,
    /// 24-byte nonce (XChaCha20 extended nonce — safe to generate randomly).
    pub nonce: [u8; 24],
    /// Sender's ephemeral X25519 public key (32 bytes).
    pub ephemeral_pk: [u8; 32],
}

impl EncryptedPayload {
    /// Serialize to MessagePack bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, crate::TomProtocolError> {
        rmp_serde::to_vec(self).map_err(Into::into)
    }

    /// Deserialize from MessagePack bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, crate::TomProtocolError> {
        rmp_serde::from_slice(data).map_err(Into::into)
    }
}

/// Convert an Ed25519 public key to an X25519 public key.
///
/// Uses the birational map from the Edwards curve to Montgomery form.
/// Equivalent to libsodium's `crypto_sign_ed25519_pk_to_curve25519`.
pub fn ed25519_to_x25519_public(ed25519_pk: &[u8; 32]) -> Result<[u8; 32], TomProtocolError> {
    let compressed = CompressedEdwardsY(*ed25519_pk);
    let edwards = compressed.decompress().ok_or_else(|| {
        TomProtocolError::Crypto("invalid Ed25519 public key: decompression failed".into())
    })?;
    let montgomery = edwards.to_montgomery();
    Ok(montgomery.to_bytes())
}

/// Convert an Ed25519 secret key (32-byte seed) to an X25519 secret key.
///
/// Mirrors libsodium's `crypto_sign_ed25519_sk_to_curve25519`:
/// SHA-512(seed), take first 32 bytes, clamp.
pub fn ed25519_to_x25519_secret(ed25519_seed: &[u8; 32]) -> [u8; 32] {
    let hash = Sha512::digest(ed25519_seed);
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&hash[..32]);
    // Standard X25519 clamping
    secret[0] &= 248;
    secret[31] &= 127;
    secret[31] |= 64;
    secret
}

/// Derive a 32-byte encryption key from a DH shared secret using HKDF-SHA256.
fn derive_key(shared_secret: &[u8; 32]) -> [u8; 32] {
    let hkdf = Hkdf::<Sha256>::new(None, shared_secret);
    let mut key = [0u8; 32];
    hkdf.expand(HKDF_INFO, &mut key)
        .expect("HKDF-SHA256 expand to 32 bytes always succeeds");
    key
}

/// Encrypt plaintext for a recipient identified by their Ed25519 public key.
///
/// Generates an ephemeral X25519 keypair, performs DH with the recipient's
/// converted X25519 public key, derives an encryption key via HKDF,
/// and encrypts with XChaCha20-Poly1305.
///
/// The `recipient_ed25519_pk` is typically obtained via `NodeId::as_bytes()`.
pub fn encrypt(
    plaintext: &[u8],
    recipient_ed25519_pk: &[u8; 32],
) -> Result<EncryptedPayload, TomProtocolError> {
    use chacha20poly1305::aead::rand_core::{OsRng, RngCore};

    // Convert recipient's Ed25519 pubkey to X25519
    let recipient_x25519_bytes = ed25519_to_x25519_public(recipient_ed25519_pk)?;
    let recipient_x25519 = X25519PublicKey::from(recipient_x25519_bytes);

    // Generate ephemeral X25519 keypair
    let ephemeral_secret = X25519Secret::random_from_rng(OsRng);
    let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);

    // Diffie-Hellman → HKDF key derivation
    let shared_secret = ephemeral_secret.diffie_hellman(&recipient_x25519);
    let key = derive_key(shared_secret.as_bytes());
    let cipher = XChaCha20Poly1305::new(&key.into());

    // Random 24-byte nonce (safe for random generation with XChaCha20)
    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from(nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| TomProtocolError::Crypto(format!("encryption failed: {e}")))?;

    Ok(EncryptedPayload {
        ciphertext,
        nonce: nonce_bytes,
        ephemeral_pk: ephemeral_public.to_bytes(),
    })
}

/// Decrypt an `EncryptedPayload` using the recipient's Ed25519 secret key (32-byte seed).
///
/// Converts the seed to an X25519 secret, performs DH with the sender's
/// ephemeral public key, derives the decryption key via HKDF,
/// and decrypts with XChaCha20-Poly1305.
pub fn decrypt(
    payload: &EncryptedPayload,
    recipient_ed25519_seed: &[u8; 32],
) -> Result<Vec<u8>, TomProtocolError> {
    // Convert recipient's Ed25519 secret to X25519
    let x25519_secret_bytes = ed25519_to_x25519_secret(recipient_ed25519_seed);
    let x25519_secret = X25519Secret::from(x25519_secret_bytes);

    // Recover sender's ephemeral X25519 public key
    let ephemeral_pk = X25519PublicKey::from(payload.ephemeral_pk);

    // Diffie-Hellman → HKDF key derivation
    let shared_secret = x25519_secret.diffie_hellman(&ephemeral_pk);
    let key = derive_key(shared_secret.as_bytes());
    let cipher = XChaCha20Poly1305::new(&key.into());

    // Decrypt
    let nonce = XNonce::from(payload.nonce);
    let plaintext = cipher
        .decrypt(&nonce, payload.ciphertext.as_ref())
        .map_err(|_| TomProtocolError::Crypto("decryption failed: authentication error".into()))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a deterministic Ed25519 keypair (seed, public_key) from a seed byte.
    fn ed25519_keypair(seed_byte: u8) -> ([u8; 32], [u8; 32]) {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed_byte as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        let pk_bytes = *secret.public().as_bytes();
        let sk_bytes = secret.to_bytes();
        (sk_bytes, pk_bytes)
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let (sk, pk) = ed25519_keypair(42);
        let plaintext = b"Hello, ToM protocol!";

        let encrypted = encrypt(plaintext, &pk).unwrap();
        let decrypted = decrypt(&encrypted, &sk).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_decrypt_empty_payload() {
        let (sk, pk) = ed25519_keypair(1);
        let encrypted = encrypt(b"", &pk).unwrap();
        let decrypted = decrypt(&encrypted, &sk).unwrap();
        assert_eq!(decrypted, b"");
    }

    #[test]
    fn encrypt_decrypt_large_payload() {
        let (sk, pk) = ed25519_keypair(2);
        let plaintext = vec![0xAB; 100_000];
        let encrypted = encrypt(&plaintext, &pk).unwrap();
        let decrypted = decrypt(&encrypted, &sk).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_key_fails() {
        let (_sk1, pk1) = ed25519_keypair(1);
        let (sk2, _pk2) = ed25519_keypair(2);

        let encrypted = encrypt(b"secret", &pk1).unwrap();
        let result = decrypt(&encrypted, &sk2);
        assert!(result.is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let (sk, pk) = ed25519_keypair(3);
        let mut encrypted = encrypt(b"secret", &pk).unwrap();

        if let Some(byte) = encrypted.ciphertext.first_mut() {
            *byte ^= 0xFF;
        }

        let result = decrypt(&encrypted, &sk);
        assert!(result.is_err());
    }

    #[test]
    fn tampered_nonce_fails() {
        let (sk, pk) = ed25519_keypair(4);
        let mut encrypted = encrypt(b"secret", &pk).unwrap();
        encrypted.nonce[0] ^= 0xFF;

        let result = decrypt(&encrypted, &sk);
        assert!(result.is_err());
    }

    #[test]
    fn tampered_ephemeral_pk_fails() {
        let (sk, pk) = ed25519_keypair(5);
        let mut encrypted = encrypt(b"secret", &pk).unwrap();
        encrypted.ephemeral_pk[0] ^= 0xFF;

        let result = decrypt(&encrypted, &sk);
        assert!(result.is_err());
    }

    #[test]
    fn different_encryptions_differ() {
        let (_sk, pk) = ed25519_keypair(6);
        let e1 = encrypt(b"same message", &pk).unwrap();
        let e2 = encrypt(b"same message", &pk).unwrap();

        // Different ephemeral keys → different everything
        assert_ne!(e1.ephemeral_pk, e2.ephemeral_pk);
        assert_ne!(e1.nonce, e2.nonce);
        assert_ne!(e1.ciphertext, e2.ciphertext);
    }

    #[test]
    fn ed25519_to_x25519_public_produces_valid_key() {
        let (_sk, pk) = ed25519_keypair(7);
        let x25519_pk = ed25519_to_x25519_public(&pk).unwrap();
        // Should not be all zeros (identity point)
        assert_ne!(x25519_pk, [0u8; 32]);
    }

    #[test]
    fn ed25519_to_x25519_public_deterministic() {
        let (_sk, pk) = ed25519_keypair(8);
        let x1 = ed25519_to_x25519_public(&pk).unwrap();
        let x2 = ed25519_to_x25519_public(&pk).unwrap();
        assert_eq!(x1, x2);
    }

    #[test]
    fn ed25519_to_x25519_secret_deterministic() {
        let seed = [42u8; 32];
        let x1 = ed25519_to_x25519_secret(&seed);
        let x2 = ed25519_to_x25519_secret(&seed);
        assert_eq!(x1, x2);
    }

    #[test]
    fn x25519_secret_is_clamped() {
        let seed = [0xFF; 32];
        let secret = ed25519_to_x25519_secret(&seed);
        assert_eq!(secret[0] & 7, 0, "low 3 bits should be cleared");
        assert_eq!(secret[31] & 128, 0, "high bit should be cleared");
        assert_eq!(secret[31] & 64, 64, "bit 6 should be set");
    }

    #[test]
    fn encrypted_payload_msgpack_roundtrip() {
        let (_sk, pk) = ed25519_keypair(9);
        let encrypted = encrypt(b"roundtrip test", &pk).unwrap();

        let bytes = encrypted.to_bytes().unwrap();
        let decoded = EncryptedPayload::from_bytes(&bytes).unwrap();

        assert_eq!(encrypted, decoded);
    }

    #[test]
    fn ciphertext_overhead() {
        let (_sk, pk) = ed25519_keypair(10);
        let plaintext = b"test payload";
        let encrypted = encrypt(plaintext, &pk).unwrap();

        // XChaCha20-Poly1305 adds 16 bytes auth tag
        assert_eq!(
            encrypted.ciphertext.len(),
            plaintext.len() + 16,
            "ciphertext should be plaintext + 16 bytes auth tag"
        );
    }

    #[test]
    fn dh_symmetry() {
        // Verify that DH(a_secret, B_public) == DH(b_secret, A_public)
        let (sk_a, pk_a) = ed25519_keypair(20);
        let (sk_b, pk_b) = ed25519_keypair(21);

        let x_sk_a = X25519Secret::from(ed25519_to_x25519_secret(&sk_a));
        let x_pk_a = X25519PublicKey::from(ed25519_to_x25519_public(&pk_a).unwrap());
        let x_sk_b = X25519Secret::from(ed25519_to_x25519_secret(&sk_b));
        let x_pk_b = X25519PublicKey::from(ed25519_to_x25519_public(&pk_b).unwrap());

        let shared_ab = x_sk_a.diffie_hellman(&x_pk_b);
        let shared_ba = x_sk_b.diffie_hellman(&x_pk_a);

        assert_eq!(shared_ab.as_bytes(), shared_ba.as_bytes());
    }
}
