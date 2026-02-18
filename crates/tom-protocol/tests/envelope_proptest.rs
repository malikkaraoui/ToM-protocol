use proptest::prelude::*;
use tom_protocol::{Envelope, MessageType, NodeId};

/// Generate a deterministic NodeId from a seed.
fn node_id(seed: u8) -> NodeId {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
    let secret = iroh::SecretKey::generate(&mut rng);
    let id_str = secret.public().to_string();
    id_str.parse().unwrap()
}

/// Strategy for generating random MessageType variants.
fn arb_message_type() -> impl Strategy<Value = MessageType> {
    prop_oneof![
        Just(MessageType::Chat),
        Just(MessageType::Ack),
        Just(MessageType::ReadReceipt),
        Just(MessageType::Heartbeat),
        Just(MessageType::GroupCreate),
        Just(MessageType::GroupInvite),
        Just(MessageType::GroupMessage),
        Just(MessageType::GroupLeave),
        Just(MessageType::PeerAnnounce),
        Just(MessageType::BackupStore),
        Just(MessageType::BackupDeliver),
    ]
}

proptest! {
    /// Any envelope should survive a MessagePack roundtrip.
    #[test]
    fn roundtrip_envelope(
        payload in prop::collection::vec(any::<u8>(), 0..10000),
        ttl in 0..10u32,
        msg_type in arb_message_type(),
        encrypted in any::<bool>(),
        sig_len in 0..128usize,
    ) {
        let from = node_id(1);
        let to = node_id(2);

        let env = Envelope {
            id: "proptest-id".to_string(),
            from,
            to,
            via: Vec::new(),
            msg_type,
            payload,
            timestamp: 1708000000000,
            signature: vec![0xAA; sig_len],
            ttl,
            encrypted,
        };

        let bytes = env.to_bytes().expect("serialize");
        let decoded = Envelope::from_bytes(&bytes).expect("deserialize");

        prop_assert_eq!(&env, &decoded);
    }

    /// Envelopes with relay chains should roundtrip correctly.
    #[test]
    fn roundtrip_with_via_chain(
        via_count in 0..5usize,
        payload in prop::collection::vec(any::<u8>(), 0..1000),
    ) {
        let from = node_id(1);
        let to = node_id(2);
        let via: Vec<NodeId> = (0..via_count)
            .map(|i| node_id(10 + i as u8))
            .collect();

        let env = Envelope {
            id: "proptest-via".to_string(),
            from,
            to,
            via,
            msg_type: MessageType::Chat,
            payload,
            timestamp: 1708000000000,
            signature: Vec::new(),
            ttl: 4,
            encrypted: false,
        };

        let bytes = env.to_bytes().expect("serialize");
        let decoded = Envelope::from_bytes(&bytes).expect("deserialize");

        prop_assert_eq!(&env, &decoded);
    }

    /// signing_bytes must be deterministic: same input → same output.
    #[test]
    fn signing_bytes_deterministic(
        payload in prop::collection::vec(any::<u8>(), 0..5000),
        ttl in 0..10u32,
    ) {
        let env = Envelope {
            id: "proptest-sign".to_string(),
            from: node_id(1),
            to: node_id(2),
            via: Vec::new(),
            msg_type: MessageType::Chat,
            payload,
            timestamp: 1708000000000,
            signature: Vec::new(),
            ttl,
            encrypted: false,
        };

        let sb1 = env.signing_bytes();
        let sb2 = env.signing_bytes();
        prop_assert_eq!(&sb1, &sb2);
    }

    /// signing_bytes must not change when signature is modified.
    #[test]
    fn signing_bytes_ignores_signature(
        sig in prop::collection::vec(any::<u8>(), 0..128),
    ) {
        let mut env = Envelope {
            id: "proptest-sig".to_string(),
            from: node_id(1),
            to: node_id(2),
            via: Vec::new(),
            msg_type: MessageType::Chat,
            payload: b"test".to_vec(),
            timestamp: 1708000000000,
            signature: Vec::new(),
            ttl: 4,
            encrypted: false,
        };

        let sb_before = env.signing_bytes();
        env.signature = sig;
        let sb_after = env.signing_bytes();

        prop_assert_eq!(&sb_before, &sb_after);
    }
}
