use proptest::prelude::*;
use tom_protocol::{Envelope, EnvelopeBuilder, MessageType};

/// Generate a deterministic Ed25519 keypair (seed, public_key, NodeId).
fn keypair(seed: u8) -> ([u8; 32], [u8; 32], tom_protocol::NodeId) {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = iroh::SecretKey::generate(&mut rng);
    let pk_bytes = *secret.public().as_bytes();
    let sk_bytes = secret.to_bytes();
    let node = secret.public().to_string().parse().unwrap();
    (sk_bytes, pk_bytes, node)
}

proptest! {
    /// Signed envelopes always verify.
    #[test]
    fn sign_verify_roundtrip(
        payload in prop::collection::vec(any::<u8>(), 0..10000),
        ttl in 0..10u32,
    ) {
        let (sk, _, from) = keypair(1);
        let (_, _, to) = keypair(2);

        let env = EnvelopeBuilder::new(from, to, MessageType::Chat, payload)
            .ttl(ttl)
            .sign(&sk);

        prop_assert!(env.verify_signature().is_ok());
    }

    /// Signed envelope survives wire roundtrip (serialize → deserialize → verify).
    #[test]
    fn sign_wire_roundtrip(
        payload in prop::collection::vec(any::<u8>(), 0..5000),
    ) {
        let (sk, _, from) = keypair(3);
        let (_, _, to) = keypair(4);

        let env = EnvelopeBuilder::new(from, to, MessageType::Chat, payload)
            .sign(&sk);

        let bytes = env.to_bytes().expect("serialize");
        let decoded = Envelope::from_bytes(&bytes).expect("deserialize");

        prop_assert!(decoded.verify_signature().is_ok());
    }

    /// Tampered payload always breaks signature.
    #[test]
    fn tampered_payload_breaks_sig(
        payload in prop::collection::vec(any::<u8>(), 1..5000),
        tamper_pos in 0..5000usize,
    ) {
        let (sk, _, from) = keypair(5);
        let (_, _, to) = keypair(6);

        let mut env = EnvelopeBuilder::new(from, to, MessageType::Chat, payload.clone())
            .sign(&sk);

        // Tamper one byte
        let pos = tamper_pos % env.payload.len();
        env.payload[pos] ^= 0xFF;

        prop_assert!(env.verify_signature().is_err());
    }

    /// Encrypt-then-sign → verify → decrypt always works.
    #[test]
    fn encrypt_sign_decrypt_roundtrip(
        payload in prop::collection::vec(any::<u8>(), 0..10000),
    ) {
        let (sk_sender, _, from) = keypair(7);
        let (sk_recipient, pk_recipient, to) = keypair(8);

        let env = EnvelopeBuilder::new(from, to, MessageType::Chat, payload.clone())
            .encrypt_and_sign(&sk_sender, &pk_recipient)
            .expect("encrypt_and_sign");

        // Verify signature (covers encrypted payload)
        prop_assert!(env.verify_signature().is_ok());
        prop_assert!(env.encrypted);

        // Decrypt
        let mut decrypted = env;
        decrypted.decrypt_payload(&sk_recipient).expect("decrypt");
        prop_assert!(!decrypted.encrypted);
        prop_assert_eq!(&decrypted.payload, &payload);
    }
}
