# ToM Cryptography — v1

> Statut : normatif · Source de vérité : `crates/tom-protocol/src/crypto.rs`, `src/envelope.rs`
> Vérifiable contre : `docs/spec/vectors/tom-vectors-v1.json` (vectors `ed25519_to_x25519`, `e2e_decrypt`, `group_sender_key`, `encrypt_then_sign_order`)

## 1. Primitives

| Usage | Primitive | Détail |
|---|---|---|
| Identité & signature | **Ed25519** | clé publique = NodeId ; vérification stricte |
| Accord de clé | **X25519** (DH éphémère) | un keypair éphémère **par message** (forward secrecy) |
| Dérivation de clé | **HKDF-SHA256** | salt **absent** (None), info = `"tom-protocol-e2e-xchacha20poly1305-v1"` |
| Chiffrement | **XChaCha20-Poly1305** | nonce 24 octets aléatoire, tag d'authentification 16 octets inclus dans le ciphertext |

## 2. Conversion Ed25519 → X25519

Identique à libsodium :
- **Clé publique** : décompression du point Edwards puis carte birationnelle Edwards→Montgomery (`crypto_sign_ed25519_pk_to_curve25519`). Une clé publique non décompressable est rejetée.
- **Clé secrète** : `SHA-512(seed)[0..32]` puis clamping X25519 standard (`b[0] &= 248 ; b[31] &= 127 ; b[31] |= 64`) (`crypto_sign_ed25519_sk_to_curve25519`).

*Preuve : vector `ed25519_to_x25519` (entrées/sorties hex).*

## 3. Chiffrement E2E d'un message direct

Émission (`encrypt`, puis `sign` — **ordre chiffre-puis-signe**, ADR-004) :

1. Convertir la clé publique Ed25519 du destinataire en X25519 (§2).
2. Générer un keypair X25519 **éphémère** (aléatoire OS).
3. `shared = DH(ephemeral_secret, recipient_x25519_pk)`.
4. `key = HKDF-SHA256(salt=None, ikm=shared, info="tom-protocol-e2e-xchacha20poly1305-v1", len=32)`.
5. Nonce aléatoire 24 octets ; `ciphertext = XChaCha20-Poly1305(key, nonce, plaintext)`.
6. Construire `EncryptedPayload { ciphertext, nonce[24], ephemeral_pk[32] }`, le sérialiser en **MessagePack** (mêmes conventions que la spec wire : struct = array positionnel de 3 éléments, `Vec<u8>` = array d'entiers, tableaux fixes `[u8;N]` = array d'entiers).
7. Placer ces octets dans `Envelope.payload`, mettre `encrypted = true`.
8. **Signer l'enveloppe** (spec wire §3) — la signature couvre le **ciphertext**, ce qui permet aux relais de vérifier l'authenticité sans déchiffrer. *Preuve : vector `encrypt_then_sign_order`.*

Réception (`decrypt`) :

1. Convertir son seed Ed25519 en secret X25519 (§2).
2. `shared = DH(recipient_x25519_secret, ephemeral_pk)` (depuis l'`EncryptedPayload`).
3. Dériver `key` (étape 4 ci-dessus) ; déchiffrer ; tout échec d'authentification rejette le message.

*Preuve complète chaînée (DH → HKDF → déchiffrement) : vector `e2e_decrypt` — éphémère et nonce fixés, chaque valeur intermédiaire fournie (shared secret, clé dérivée), validé contre `crypto::decrypt()` de l'implémentation de référence.*

## 4. Chiffrement de groupe (Sender Keys)

- Chaque expéditeur d'un groupe possède une **Sender Key symétrique de 32 octets** (aléatoire OS), distribuée aux membres via messages `GroupSenderKeyDistribution` (chiffrés E2E par paire, §3).
- Message de groupe : `XChaCha20-Poly1305(sender_key, nonce_24_aléatoire, plaintext)`.
- Rotation de clé au départ d'un membre (le quittant ne peut plus lire).

*Preuve : vector `group_sender_key`.*

## 5. Propriétés et limites (v1)

- **Forward secrecy par message** sur les messages directs (éphémère X25519 jeté après usage).
- L'authenticité vient de la **signature d'enveloppe Ed25519** (l'AEAD seul n'authentifie pas l'expéditeur : n'importe qui peut chiffrer vers une clé publique).
- Pas de ratchet (pas de post-compromise security) en v1.
- Métadonnées (`from`, `to`, `via`, `msg_type`, `timestamp`) **en clair** sur le wire — seul `payload` est chiffré.

<!-- Références code : HKDF_INFO crates/tom-protocol/src/crypto.rs:21 ; EncryptedPayload :28-35 ; conversions :53-75 ; derive_key :78-84 ; encrypt :93-127 ; decrypt :134-157 ; sender keys :160-192 ; encrypt_and_sign crates/tom-protocol/src/envelope.rs:259-268 -->
