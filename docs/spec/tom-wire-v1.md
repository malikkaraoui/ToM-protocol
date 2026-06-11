# ToM Wire Format — v1

> Statut : normatif · Source de vérité : `crates/tom-protocol/src/envelope.rs`, `src/types.rs`
> Vérifiable contre : `docs/spec/vectors/tom-vectors-v1.json` (générés et auto-vérifiés par `crates/tom-protocol/examples/gen_test_vectors.rs`)
> Les 7 décisions de design verrouillées du protocole s'appliquent : `_bmad-output/planning-artifacts/design-decisions.md`.

## 1. Identité

- L'identité d'un nœud est une paire de clés **Ed25519**. La clé publique EST l'adresse réseau (`NodeId`).
- Sur le wire, un `NodeId` s'encode en **string MessagePack de 64 caractères hex minuscules** (l'encodage hex de la clé publique 32 octets). *Preuve : vector `identity` — `node_id == hex(ed25519_public_key)`.*

## 2. Envelope

L'unité de communication est l'`Envelope`, sérialisée en **MessagePack** via `rmp_serde` en représentation **array positionnelle** (PAS une map : pas de noms de champs sur le wire).

**⚠️ Pièges d'implémentation (constatés sur les octets, vector `signed_envelope`) :**
- L'enveloppe est un **fixarray de 10 éléments** (premier octet `0x9a`).
- `payload` et `signature` (`Vec<u8>` côté Rust) s'encodent en **array MessagePack d'entiers** (`0x9X`/`0xdc…`), PAS au format `bin` (`0xc4…`). Un encodeur tiers qui émet du `bin` sera incompatible.
- `msg_type` s'encode comme la **string du nom de variante** (ex. `0xa4 "Chat"`).

Ordre et types des 10 champs :

| # | Champ | Type MessagePack | Description |
|---|---|---|---|
| 1 | `id` | str (36) | UUID v4, identifiant unique du message |
| 2 | `from` | str (64) | NodeId expéditeur (hex) |
| 3 | `to` | str (64) | NodeId destinataire final (hex) |
| 4 | `via` | array de str | Chaîne de relais (NodeIds intermédiaires) |
| 5 | `msg_type` | str | Nom de variante (cf. §4) |
| 6 | `payload` | array d'uint | Octets opaques (clair ou chiffré, cf. spec crypto) |
| 7 | `timestamp` | uint64 | Création, Unix epoch **millisecondes** |
| 8 | `signature` | array d'uint | 64 octets Ed25519, vide si non signée |
| 9 | `ttl` | uint | Compteur de **sauts** restants (cf. §5) |
| 10 | `encrypted` | bool | `payload` chiffré E2E ou non |

Vector de référence : `signed_envelope.wire_msgpack_hex`.

## 3. Signature

- Les octets signés (`signing_bytes`) sont la sérialisation MessagePack d'un **array de 8 éléments** (premier octet `0x98`) : les champs 1-7 et 10 ci-dessus, dans le même ordre, **en excluant `signature`** (circularité) **et `ttl`** (muté par les relais en transit — ADR-003).
- Signature : **Ed25519** (64 octets) sur `signing_bytes`, clé = identité de `from`.
- Vérification : **stricte** (`verify_strict` — les signatures non canoniques sont rejetées). Une signature de longueur ≠ 64 est rejetée.
- *Preuves : vectors `signed_envelope` (octets + signature attendus) et `ttl_mutation_in_transit` (signing_bytes inchangés après décrément du ttl).*

## 4. Types de message

36 variantes (string du nom exact, sensible à la casse) — `src/types.rs` :

`Chat`, `Ack`, `ReadReceipt`, `Heartbeat`,
`GroupCreate`, `GroupCreated`, `GroupInvite`, `GroupJoin`, `GroupSync`, `GroupMessage`, `GroupLeave`,
`GroupMemberJoined`, `GroupMemberLeft`, `GroupHubMigration`, `GroupDeliveryAck`, `GroupHubHeartbeat`,
`GroupSenderKeyDistribution`, `GroupHubPing`, `GroupHubPong`, `GroupHubShadowSync`,
`GroupCandidateAssigned`, `GroupHubUnreachable`,
`GroupKickMember`, `GroupUpdateMemberRole`, `GroupMemberRoleChanged`, `GroupInviteMember`,
`GroupSyncRequest`, `GroupSyncResponse`,
`BackupStore`, `BackupDeliver`, `BackupReplicate`, `BackupReplicateAck`, `BackupQuery`,
`BackupQueryResponse`, `BackupConfirmDelivery`,
`PeerAnnounce`

Statuts de livraison (hors wire, état local ; progression stricte) :
`Pending → Sent → Relayed → Delivered → Read`, plus `Failed` (terminal, après épuisement des retries d'ACK).

## 5. TTL — deux notions distinctes

1. **`ttl` de l'enveloppe = compteur de sauts** : défaut et maximum **4** (`DEFAULT_TTL`/`MAX_TTL`, types.rs). Chaque relais décrémente avant de transmettre ; à 0, le message est rejeté (pas de décrément possible). N'invalide jamais la signature (§3).
2. **Durée de vie d'un message = 24 h maximum** (décision verrouillée n°2) : tout message non livré est purgé après 24 h, sans exception — appliqué par la couche backup (ADR-009), pas par le champ `ttl`.

## 6. Règles de relais (stateless)

- Un relais **ne stocke pas** : il vérifie la signature, décrémente `ttl`, et transmet au saut suivant (`via` ou destinataire).
- Délivré ⟺ le **destinataire** émet un `Ack` (décision verrouillée n°1). Un relais n'émet jamais d'ACK de livraison pour autrui.
- Un relais peut vérifier l'authenticité d'une enveloppe chiffrée sans la déchiffrer (la signature couvre le ciphertext — cf. spec crypto §3).
- Entrées malformées : tout flux non décodable doit produire une erreur propre, jamais un crash (l'implémentation de référence rejette troncatures et bytes arbitraires).

## 7. Validation d'une implémentation tierce

1. Recalculer le vector `identity` : seed → clé publique → NodeId.
2. Réencoder l'enveloppe du vector `signed_envelope` champ à champ et comparer à `wire_msgpack_hex` (octet près).
3. Recalculer `signing_bytes_hex`, vérifier `signature_hex` avec la clé publique de `from`.
4. Vérifier que le décrément de `ttl` produit `ttl_mutation_in_transit.wire_msgpack_hex` et que la signature reste valide.

<!-- Références code : Envelope crates/tom-protocol/src/envelope.rs:14-35 ; signing_bytes :90-103 ; SignableEnvelope :274-284 ; sign/verify :129-152 ; DEFAULT_TTL/MAX_TTL crates/tom-protocol/src/types.rs ; NodeId serde crates/tom-transport/src/lib.rs:102-113 -->
