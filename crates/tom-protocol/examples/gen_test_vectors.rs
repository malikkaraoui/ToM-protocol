//! Générateur de test vectors pour la spec publique (chantier S3.3).
//!
//! Sortie : JSON sur stdout — committé dans `docs/spec/vectors/tom-vectors-v1.json`.
//! Tout est déterministe (clés dérivées de seeds fixes, nonce/éphémère fixés) ;
//! chaque vector est auto-vérifié contre l'implémentation avant émission :
//! si une assertion échoue, le générateur panique et rien n'est émis.
//!
//! Régénération : cargo run -p tom-protocol --example gen_test_vectors
use rand::SeedableRng;
use serde_json::json;
use tom_protocol::{crypto, Envelope, EnvelopeBuilder, MessageType, NodeId};

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Keypair Ed25519 déterministe (seed StdRng u64) — même méthode que les
/// tests du crate.
fn keypair(seed: u64) -> ([u8; 32], [u8; 32], NodeId) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    let secret = tom_connect::SecretKey::generate(&mut rng);
    let pk = *secret.public().as_bytes();
    let sk = secret.to_bytes();
    let node = secret.public().to_string().parse().unwrap();
    (sk, pk, node)
}

fn main() {
    let (alice_sk, alice_pk, alice_id) = keypair(1);
    let (bob_sk, bob_pk, bob_id) = keypair(2);
    let (_, _, relay_id) = keypair(3);

    // ── Vector 1 : identité — seed Ed25519 → clé publique → NodeId ──────
    let v1 = json!({
        "name": "identity",
        "description": "Le NodeId est l'encodage hex minuscule de la clé publique Ed25519 (32 octets).",
        "ed25519_seed": hex(&alice_sk),
        "ed25519_public_key": hex(&alice_pk),
        "node_id": alice_id.to_string(),
    });
    assert_eq!(alice_id.to_string(), hex(&alice_pk), "NodeId == hex(pk)");

    // ── Vector 2 : enveloppe signée (champs fixes) ───────────────────────
    let mut env = Envelope {
        id: "00000000-0000-4000-8000-000000000001".to_string(),
        from: alice_id,
        to: bob_id,
        via: vec![relay_id],
        msg_type: MessageType::Chat,
        payload: b"salut bob".to_vec(),
        timestamp: 1_700_000_000_000,
        signature: Vec::new(),
        ttl: 4,
        encrypted: false,
    };
    let signing_bytes = env.signing_bytes();
    env.sign(&alice_sk);
    env.verify_signature().expect("vector 2: signature valide");
    let wire = env.to_bytes().expect("vector 2: serialize");
    assert_eq!(Envelope::from_bytes(&wire).expect("roundtrip"), env);
    let v2 = json!({
        "name": "signed_envelope",
        "description": "Enveloppe Chat signée. wire_msgpack = rmp_serde::to_vec(Envelope). signing_bytes = rmp_serde::to_vec(SignableEnvelope) — exclut signature et ttl. signature = Ed25519(signing_bytes), vérification stricte (verify_strict).",
        "envelope": {
            "id": env.id,
            "from": env.from.to_string(),
            "to": env.to.to_string(),
            "via": [relay_id.to_string()],
            "msg_type": "Chat",
            "payload_utf8": "salut bob",
            "timestamp": env.timestamp,
            "ttl": env.ttl,
            "encrypted": env.encrypted,
        },
        "signing_bytes_hex": hex(&signing_bytes),
        "signature_hex": hex(&env.signature),
        "wire_msgpack_hex": hex(&wire),
    });

    // ── Vector 3 : mutation TTL en transit (ADR-003) ─────────────────────
    let mut relayed = env.clone();
    relayed.decrement_ttl().expect("ttl 4 -> 3");
    relayed.verify_signature().expect("vector 3: ttl exclu de la signature");
    let v3 = json!({
        "name": "ttl_mutation_in_transit",
        "description": "Le relais décrémente ttl sans casser la signature (ttl exclu de signing_bytes). Mêmes signing_bytes que vector 2, ttl différent sur le wire.",
        "ttl_after_one_hop": relayed.ttl,
        "signing_bytes_hex_unchanged": hex(&relayed.signing_bytes()) == hex(&signing_bytes),
        "wire_msgpack_hex": hex(&relayed.to_bytes().expect("serialize")),
    });

    // ── Vector 4 : conversion Ed25519 → X25519 ──────────────────────────
    let bob_x_pk = crypto::ed25519_to_x25519_public(&bob_pk).expect("conversion pk");
    let bob_x_sk = crypto::ed25519_to_x25519_secret(&bob_sk);
    let v4 = json!({
        "name": "ed25519_to_x25519",
        "description": "Conversions identiques à libsodium : pk = Edwards→Montgomery (birational map) ; sk = SHA-512(seed)[0..32] avec clamping (b[0]&=248, b[31]&=127, b[31]|=64).",
        "ed25519_public_key": hex(&bob_pk),
        "x25519_public_key": hex(&bob_x_pk),
        "ed25519_seed": hex(&bob_sk),
        "x25519_secret": hex(&bob_x_sk),
    });

    // ── Vector 5 : déchiffrement E2E déterministe ────────────────────────
    // encrypt() tire l'éphémère et le nonce de l'OS ; pour un vector
    // reproductible on fixe les deux et on reconstruit EncryptedPayload
    // par les mêmes primitives, puis on prouve que decrypt() du crate
    // restitue le plaintext.
    let plaintext = b"vector e2e v1";
    let eph_secret_bytes: [u8; 32] = {
        // scalaire X25519 fixe (clampé) — uniquement pour le vector
        let mut b = [0x42u8; 32];
        b[0] &= 248;
        b[31] &= 127;
        b[31] |= 64;
        b
    };
    let nonce: [u8; 24] = [0x24; 24];
    let (ciphertext, ephemeral_pk, shared, key) = {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};
        use hkdf::Hkdf;
        use sha2::Sha256;
        use x25519_dalek::{PublicKey, StaticSecret};
        let eph = StaticSecret::from(eph_secret_bytes);
        let eph_pk = PublicKey::from(&eph);
        let shared = eph.diffie_hellman(&PublicKey::from(bob_x_pk));
        let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
        let mut key = [0u8; 32];
        hk.expand(b"tom-protocol-e2e-xchacha20poly1305-v1", &mut key)
            .unwrap();
        let cipher = XChaCha20Poly1305::new(&key.into());
        let ct = cipher
            .encrypt(&XNonce::from(nonce), plaintext.as_ref())
            .unwrap();
        (ct, eph_pk.to_bytes(), *shared.as_bytes(), key)
    };
    let payload = crypto::EncryptedPayload {
        ciphertext: ciphertext.clone(),
        nonce,
        ephemeral_pk,
    };
    // Preuve : l'implémentation du crate déchiffre ce vector.
    let decrypted = crypto::decrypt(&payload, &bob_sk).expect("vector 5: decrypt");
    assert_eq!(decrypted, plaintext, "vector 5: plaintext");
    let payload_bytes = payload.to_bytes().expect("serialize EncryptedPayload");
    let v5 = json!({
        "name": "e2e_decrypt",
        "description": "DH X25519 (éphémère fixé) → HKDF-SHA256 (salt absent, info='tom-protocol-e2e-xchacha20poly1305-v1') → XChaCha20-Poly1305. EncryptedPayload{ciphertext, nonce[24], ephemeral_pk[32]} sérialisé en MessagePack. Vérifié contre crypto::decrypt() du crate.",
        "recipient_ed25519_seed": hex(&bob_sk),
        "ephemeral_x25519_secret": hex(&eph_secret_bytes),
        "ephemeral_x25519_public": hex(&ephemeral_pk),
        "dh_shared_secret": hex(&shared),
        "hkdf_derived_key": hex(&key),
        "nonce": hex(&nonce),
        "plaintext_utf8": String::from_utf8_lossy(plaintext),
        "ciphertext_hex": hex(&ciphertext),
        "encrypted_payload_msgpack_hex": hex(&payload_bytes),
    });

    // ── Vector 6 : message de groupe (sender key symétrique) ────────────
    let sender_key = [0x77u8; 32];
    let group_nonce = [0x07u8; 24];
    let group_ct = {
        use chacha20poly1305::aead::{Aead, KeyInit};
        use chacha20poly1305::{XChaCha20Poly1305, XNonce};
        let cipher = XChaCha20Poly1305::new(&sender_key.into());
        cipher
            .encrypt(&XNonce::from(group_nonce), b"message de groupe".as_ref())
            .unwrap()
    };
    let group_pt = crypto::decrypt_group_message(&group_ct, &group_nonce, &sender_key)
        .expect("vector 6: decrypt group");
    assert_eq!(group_pt, b"message de groupe");
    let v6 = json!({
        "name": "group_sender_key",
        "description": "Chiffrement de groupe : XChaCha20-Poly1305 symétrique avec Sender Key 32 octets, nonce aléatoire 24 octets. Vérifié contre crypto::decrypt_group_message().",
        "sender_key": hex(&sender_key),
        "nonce": hex(&group_nonce),
        "plaintext_utf8": "message de groupe",
        "ciphertext_hex": hex(&group_ct),
    });

    // ── Vector 7 : encrypt_and_sign (ordre chiffre-puis-signe) ──────────
    let env7 = EnvelopeBuilder::new(alice_id, bob_id, MessageType::Chat, b"secret".to_vec())
        .encrypt_and_sign(&alice_sk, &bob_pk)
        .expect("vector 7");
    env7.verify_signature().expect("vector 7: signature couvre le ciphertext");
    let v7 = json!({
        "name": "encrypt_then_sign_order",
        "description": "ADR-004 : encrypt_and_sign chiffre le payload PUIS signe — la signature couvre le ciphertext, un relais vérifie l'authenticité sans déchiffrer. Non déterministe (éphémère/nonce aléatoires) : vector de propriété, pas d'octets attendus.",
        "property": "verify_signature() == ok && encrypted == true",
        "holds": env7.encrypted && env7.is_signed(),
    });

    let out = json!({
        "spec": "tom-protocol test vectors",
        "version": "v1",
        "source": "crates/tom-protocol/examples/gen_test_vectors.rs (déterministe, auto-vérifié)",
        "vectors": [v1, v2, v3, v4, v5, v6, v7],
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
