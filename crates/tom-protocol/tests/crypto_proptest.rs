use proptest::prelude::*;
use tom_protocol::crypto;
use tom_protocol::EncryptedPayload;

/// Generate a deterministic Ed25519 keypair from a seed.
fn ed25519_keypair(seed: u8) -> ([u8; 32], [u8; 32]) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = tom_connect::SecretKey::generate(&mut rng);
    let pk_bytes = *secret.public().as_bytes();
    let sk_bytes = secret.to_bytes();
    (sk_bytes, pk_bytes)
}

proptest! {
    /// Any plaintext should survive encrypt→decrypt roundtrip.
    #[test]
    fn roundtrip_any_payload(
        payload in prop::collection::vec(any::<u8>(), 0..50000),
    ) {
        let (sk, pk) = ed25519_keypair(1);
        let encrypted = crypto::encrypt(&payload, &pk).expect("encrypt");
        let decrypted = crypto::decrypt(&encrypted, &sk).expect("decrypt");
        prop_assert_eq!(&decrypted, &payload);
    }

    /// Ciphertext is always plaintext + 16 bytes (AEAD tag).
    #[test]
    fn ciphertext_size_invariant(
        payload in prop::collection::vec(any::<u8>(), 0..10000),
    ) {
        let (_sk, pk) = ed25519_keypair(2);
        let encrypted = crypto::encrypt(&payload, &pk).expect("encrypt");
        prop_assert_eq!(encrypted.ciphertext.len(), payload.len() + 16);
    }

    /// EncryptedPayload survives MessagePack roundtrip.
    #[test]
    fn encrypted_payload_serde_roundtrip(
        payload in prop::collection::vec(any::<u8>(), 0..10000),
    ) {
        let (_sk, pk) = ed25519_keypair(3);
        let encrypted = crypto::encrypt(&payload, &pk).expect("encrypt");

        let bytes = encrypted.to_bytes().expect("serialize");
        let decoded = EncryptedPayload::from_bytes(&bytes).expect("deserialize");

        prop_assert_eq!(&encrypted, &decoded);
    }

    /// Each encryption produces unique ephemeral keys and nonces.
    #[test]
    fn ephemeral_keys_unique(
        _seed in 0..100u32,
    ) {
        let (_sk, pk) = ed25519_keypair(4);
        let e1 = crypto::encrypt(b"test", &pk).expect("encrypt 1");
        let e2 = crypto::encrypt(b"test", &pk).expect("encrypt 2");
        prop_assert_ne!(e1.ephemeral_pk, e2.ephemeral_pk);
        prop_assert_ne!(e1.nonce, e2.nonce);
    }

    /// Wrong key always fails decryption (different keypairs).
    #[test]
    fn wrong_key_always_fails(
        payload in prop::collection::vec(any::<u8>(), 1..1000),
        sender_seed in 10..50u8,
        wrong_seed in 50..90u8,
    ) {
        let (_sk_sender, pk_sender) = ed25519_keypair(sender_seed);
        let (sk_wrong, _pk_wrong) = ed25519_keypair(wrong_seed);

        let encrypted = crypto::encrypt(&payload, &pk_sender).expect("encrypt");
        let result = crypto::decrypt(&encrypted, &sk_wrong);
        prop_assert!(result.is_err());
    }
}
