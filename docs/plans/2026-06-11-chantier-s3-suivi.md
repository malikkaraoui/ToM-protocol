# Chantier S3 — Spec protocole publique + test vectors · Suivi d'exécution

> Démarré : 2026-06-11 12:34 · Référence : `2026-06-10-roadmap-sdk.md` (Phase S3) · Décision D4
> Objectif : un implémenteur tiers (Go, Python…) peut implémenter le wire format ToM sans lire le code Rust, et **vérifier** son implémentation contre des test vectors.
> Règle de rédaction : chaque affirmation de la spec est dérivée du code source (référence fichier:ligne en commentaire HTML dans la spec). Zéro invention.

## Tableau de bord

| Tâche | Description | Statut | Commit |
|---|---|---|---|
| S3.1 | `docs/spec/tom-wire-v1.md` | ✅ | `fdd6c06` |
| S3.2 | `docs/spec/tom-crypto-v1.md` | ✅ | `fdd6c06` |
| S3.3 | Test vectors (générateur + JSON committés) | ✅ | `25da859` |
| S3.4 | `tom-discovery-v1.md` | 🔁 reporté (backlog, périmètre annoncé dans spec README) | — |
| S3.V | Validation + clôture | ✅ | (docs) |

## Journal de chantier

### 2026-06-11 12:34 — Ouverture

### Méthode anti-invention (§5) — vectors d'abord, spec ensuite

Plutôt que décrire l'encodage de mémoire, le générateur a été écrit et exécuté **avant** la rédaction ; les specs décrivent ce que les octets prouvent. Découvertes factuelles sur le wire (aucune n'était documentée) :
- `Envelope` = **fixarray MessagePack de 10 éléments positionnels** (`0x9a`), pas une map.
- `Vec<u8>` (payload, signature) = **array d'entiers**, PAS le format `bin` — piège majeur pour tout implémenteur tiers, désormais signalé en tête de spec.
- `MessageType` = string du nom de variante (`0xa4 "Chat"`).
- `SignableEnvelope` = fixarray de 8 (`0x98`) — exclusions signature+ttl confirmées aux octets (vector `ttl_mutation_in_transit`).
- `NodeId` = string hex minuscule 64 chars (== hex de la clé publique, prouvé vector `identity`).
- `EncryptedPayload` = array de 3 (`0x93`), vérifié.

### S3.3 ✅ — commit `25da859`

- `crates/tom-protocol/examples/gen_test_vectors.rs` : 7 vectors déterministes (seeds fixes, éphémère/nonce fixés pour le vector E2E), **chaque vector auto-vérifié contre l'implémentation avant émission** (panic sinon → impossible de committer des vectors faux).
- Vector E2E chaîné : chaque valeur intermédiaire exposée (secret DH, clé HKDF) pour déboguer une implémentation tierce étape par étape.
- `encrypt()` de prod étant non déterministe (OsRng), le vector E2E reconstruit le chiffrement avec les mêmes primitives et prouve que `crypto::decrypt()` du crate l'accepte.
- biome : `docs/spec/vectors/**` exclu (fichier généré).

### S3.1 + S3.2 ✅ — commit `fdd6c06`

- `tom-wire-v1.md` : identité, les 10 champs et leurs encodages exacts, signature stricte, 36 MessageTypes, **distinction des deux TTL** (4 sauts wire vs 24 h de durée de vie — confusion possible levée), règles relais stateless, protocole de validation en 4 étapes pour implémenteur tiers.
- `tom-crypto-v1.md` : primitives, conversions libsodium-compat, HKDF (salt None + info-string exacte), ordre chiffre-puis-signe et sa justification (relais vérifient sans déchiffrer), sender keys, **limites v1 explicites** (pas de ratchet, métadonnées en clair).
- Références code en commentaires HTML (fichier:ligne) dans chaque spec.
- `docs/spec/README.md` : index + commande de régénération + politique de gel v1.

### 🔁 S3.4 reporté — discovery

Périmètre annoncé dans le README de spec ; nécessite une lecture approfondie de discovery/ et gossip (PeerAnnounce, RelayReady, DHT) — chantier propre.

## Clôture S3 — bilan

**Décision D4 honorée** : un implémenteur Go/Python peut encoder/signer/chiffrer ToM sans lire le Rust, et se valider octet par octet contre les vectors. Avec S1 (tom-sdk) et S2 (TomProtocolKit), les trois portes d'entrée du protocole annoncées dans la roadmap existent.

### Backlog généré
1. S3.4 spec discovery (chantier propre).
2. CI : job qui exécute le générateur et diff contre le JSON committé (drift de vectors = échec, comme le header cbindgen).
3. Lier les specs depuis le README racine (porte « Integrate ToM »).
