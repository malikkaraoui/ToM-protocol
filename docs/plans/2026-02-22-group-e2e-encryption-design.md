# Chiffrement E2E des Messages de Groupe — Design Document

> **Pour Claude :** REQUIRED SUB-SKILL : Utiliser superpowers:writing-plans pour créer le plan d'implémentation.

**Objectif :** Rendre les messages de groupe illisibles par le hub relay. Seuls les membres du groupe peuvent déchiffrer le contenu.

**Architecture :** Sender Keys — chaque membre génère sa propre clé symétrique d'envoi. Les autres membres reçoivent une copie chiffrée (via le crypto 1-to-1 existant) pour déchiffrer.

**Stack :** XChaCha20-Poly1305 (symétrique, même lib que le 1-to-1), X25519 DH pour la distribution des clés, Ed25519 pour les signatures.

---

## 1. Contexte et Problème

### Situation actuelle

Les messages de groupe sont **signés** (Ed25519) mais **pas chiffrés**. Le hub relay :
- Lit le contenu des messages en clair
- Stocke l'historique en clair
- Vérifie les signatures sur le texte en clair

Le hub est un **relay de confiance** — il voit tout.

### Objectif

Transformer le hub en relay **aveugle** :
- ✅ Vérifie les signatures (sur le ciphertext)
- ✅ Vérifie le membership (par NodeId)
- ✅ Rate limiting et déduplication
- ❌ Ne peut PAS lire le contenu des messages
- ❌ Ne peut PAS lire le username de l'envoyeur

## 2. Architecture : Sender Keys

### Principe

Chaque membre du groupe génère une **Sender Key** — une clé symétrique XChaCha20-Poly1305 de 32 bytes. Cette clé est utilisée exclusivement pour chiffrer les messages envoyés par ce membre.

Quand Alice envoie un message :
1. Elle chiffre `{username, text}` avec **sa** Sender Key
2. Elle signe le ciphertext avec Ed25519
3. Le hub reçoit du ciphertext opaque, vérifie la signature, fan-out aux membres
4. Bob possède une copie de la Sender Key d'Alice → il déchiffre

### Avantages vs clé partagée unique

| Aspect | Clé partagée | Sender Keys |
|--------|-------------|-------------|
| Compromission d'une clé | TOUS les messages exposés | Seulement les messages d'UN membre |
| Rotation au départ | Une seule clé à redistribuer | Chaque membre re-key indépendamment |
| Authentification | Clé ne prouve pas l'identité | Sender Key + signature = identité forte |

### Pourquoi pas Double Ratchet ?

Le Double Ratchet offre un forward secrecy par message, mais ajoute une complexité significative (chaînes de clés, état par paire de participants). Pour un PoC, Sender Keys avec rotation au départ offrent un bon compromis sécurité/simplicité. L'architecture permet d'évoluer vers un ratchet plus tard.

## 3. Structures de Données

### Nouvelles structures

```rust
/// Sender Key d'un membre — stocké par tous les autres membres du groupe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenderKeyEntry {
    pub owner_id: NodeId,     // Qui a généré cette clé
    pub key: [u8; 32],        // Clé symétrique XChaCha20-Poly1305
    pub epoch: u32,           // Incrémenté à chaque rotation
    pub created_at: u64,
}

/// Bundle chiffré pour distribuer une Sender Key à un membre spécifique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSenderKey {
    pub recipient_id: NodeId,
    pub encrypted_key: EncryptedPayload,  // Réutilise le crypto 1-to-1 existant
}
```

### Modifications aux structures existantes

**GroupMessage** — le texte devient ciphertext :

```rust
pub struct GroupMessage {
    pub group_id: GroupId,
    pub message_id: String,
    pub sender_id: NodeId,
    // sender_username SUPPRIMÉ du clair — inclus dans le ciphertext
    pub ciphertext: Vec<u8>,        // {username, text} chiffré avec sender_key
    pub nonce: [u8; 24],            // Nonce unique par message
    pub key_epoch: u32,             // Version de la sender key utilisée
    pub sent_at: u64,
    pub sender_signature: Vec<u8>,  // Signature Ed25519 sur le ciphertext
    pub encrypted: bool,            // true = chiffré, false = legacy clair
}
```

**Note backward compat :** Le champ `encrypted: bool` permet de supporter les anciens messages non chiffrés pendant la transition. Si `encrypted == false`, le comportement est identique à aujourd'hui.

**GroupPayload** — nouveau variant :

```rust
pub enum GroupPayload {
    // ... variantes existantes ...

    /// Distribution de Sender Key (après join ou rotation)
    SenderKeyDistribution {
        group_id: GroupId,
        from: NodeId,
        epoch: u32,
        encrypted_keys: Vec<EncryptedSenderKey>,
    },
}
```

**GroupManager** — stockage des sender keys :

```rust
pub struct GroupManager {
    // ... champs existants ...
    sender_keys: HashMap<GroupId, HashMap<NodeId, SenderKeyEntry>>,
    local_sender_keys: HashMap<GroupId, SenderKeyEntry>,  // Nos propres clés
}
```

**Contenu chiffré du message :**

```rust
/// Sérialisé en MessagePack avant chiffrement
#[derive(Serialize, Deserialize)]
struct GroupMessageContent {
    pub username: String,
    pub text: String,
}
```

## 4. Protocole

### 4.1 Création de groupe

```
Admin → génère sa sender_key locale
Admin → envoie GroupPayload::Create au hub (comme aujourd'hui)
Hub → crée le groupe, répond GroupPayload::Created
→ Pas de distribution (aucun autre membre)
```

### 4.2 Invitation + Join + Échange de clés

```
1. Admin invite Bob → hub envoie GroupPayload::Invite à Bob
2. Bob accepte → envoie GroupPayload::Join au hub
3. Hub confirme → envoie GroupPayload::MemberJoined à tous

4. Bob génère SA sender_key
5. Bob chiffre sa sender_key pour chaque membre existant (1-to-1 crypto)
6. Bob envoie GroupPayload::SenderKeyDistribution au hub
7. Hub fan-out à tous les membres (opaque)
8. Chaque membre existant reçoit la sender_key de Bob

9. Chaque membre existant chiffre SA sender_key pour Bob seul
10. Chaque membre envoie GroupPayload::SenderKeyDistribution (1 EncryptedSenderKey pour Bob)
11. Hub fan-out à Bob
12. Bob reçoit les sender_keys de tous les membres
→ Bob peut maintenant déchiffrer les messages de tous
```

**Timing :** Entre l'étape 3 (join confirmé) et l'étape 12 (toutes les clés reçues), Bob ne peut pas déchiffrer les messages des autres. Les messages reçus pendant cette fenêtre sont mis en attente et déchiffrés quand la clé correspondante arrive.

### 4.3 Envoi de message chiffré

```
1. Alice sérialise {username: "alice", text: "Hello"} en MessagePack
2. Alice chiffre avec sa sender_key + nonce aléatoire → ciphertext
3. Alice crée GroupMessage { ciphertext, nonce, key_epoch, sender_id, encrypted: true }
4. Alice signe le ciphertext (signing_bytes = group_id + message_id + ciphertext + nonce + epoch)
5. Alice enveloppe dans GroupPayload::Message → Envelope → hub
6. Hub vérifie signature sur ciphertext + membership → fan-out
7. Bob reçoit, lookup sender_keys[alice][epoch] → déchiffre → affiche
```

### 4.4 Départ de membre + Rotation

```
1. Eve quitte (ou est expulsée)
2. Hub envoie GroupPayload::MemberLeft à tous les membres restants
3. Chaque membre restant :
   a. Génère une NOUVELLE sender_key (epoch += 1)
   b. Chiffre pour chaque membre restant (SAUF Eve)
   c. Envoie GroupPayload::SenderKeyDistribution
4. Eve ne possède pas les nouvelles clés → forward secrecy post-départ
```

### 4.5 Messages en attente (buffering)

Quand un membre reçoit un message chiffré mais ne possède pas (encore) la sender_key de l'envoyeur :
1. Le message est stocké dans un buffer local `pending_messages: Vec<GroupMessage>`
2. Quand une `SenderKeyDistribution` arrive avec la clé manquante, on déchiffre les messages en attente
3. Si aucune clé n'arrive après un timeout (30s), les messages en attente sont supprimés (l'envoyeur peut les renvoyer)

## 5. Impact sur les Modules

### `crypto.rs` — Nouvelles fonctions

```rust
/// Chiffrement symétrique pour messages de groupe (pas de DH)
pub fn encrypt_group_message(plaintext: &[u8], key: &[u8; 32]) -> (Vec<u8>, [u8; 24]);
pub fn decrypt_group_message(ciphertext: &[u8], nonce: &[u8; 24], key: &[u8; 32]) -> Result<Vec<u8>>;

/// Génération d'une Sender Key aléatoire
pub fn generate_sender_key() -> [u8; 32];
```

### `group/types.rs` — Nouvelles structures + modification GroupMessage

- Ajout : `SenderKeyEntry`, `EncryptedSenderKey`, `GroupMessageContent`
- Modif : `GroupMessage` (ciphertext + nonce + key_epoch + encrypted)
- Modif : `GroupPayload::SenderKeyDistribution`
- Modif : `GroupMessage::sign()` / `verify_signature()` — signing bytes couvrent le ciphertext
- Ajout : `GroupMessage::encrypt()` / `decrypt()` — chiffrement/déchiffrement avec sender key

### `group/manager.rs` — Gestion des sender keys

- Stockage `sender_keys` et `local_sender_keys`
- `generate_sender_key_distribution()` — génère les bundles chiffrés pour tous les membres
- `handle_sender_key_distribution()` — reçoit et stocke les sender keys des autres
- `encrypt_message()` — chiffre un message avec notre sender key
- `decrypt_message()` — déchiffre un message avec la sender key de l'envoyeur
- `rotate_sender_key()` — appelé au départ d'un membre
- Buffer de messages en attente

### `group/hub.rs` — Minimal

- Fan-out `SenderKeyDistribution` comme n'importe quel payload (déjà supporté par le pattern Broadcast)
- Suppression de la lecture du contenu message (hub aveugle)
- La vérification de signature continue de fonctionner (sur ciphertext)

### `runtime/state.rs` — Orchestration

- `handle_send_group_message()` : chiffre le message avant envoi
- `handle_incoming_group()` : dispatch `SenderKeyDistribution`, déchiffre les messages reçus
- `MemberLeft` déclenche la rotation des sender keys

### `envelope.rs` — Aucun changement

L'envelope transporte du payload opaque en MessagePack. Pas de modification nécessaire.

## 6. Plan de Test

| # | Test | Type |
|---|------|------|
| 1 | `encrypt_group_message` / `decrypt_group_message` roundtrip | Unit |
| 2 | Sender Key generation (aléatoire, 32 bytes) | Unit |
| 3 | Sender Key distribution : chiffrement 1-to-1 pour chaque membre | Unit |
| 4 | Hub ne peut PAS déchiffrer (test négatif — mauvaise clé) | Unit |
| 5 | Rotation au départ — ancien membre ne peut plus déchiffrer | Unit |
| 6 | Message avec mauvais epoch → erreur gracieuse | Unit |
| 7 | Message en attente (pas de clé) → déchiffré quand clé arrive | Unit |
| 8 | Backward compat : message `encrypted: false` → lu comme avant | Unit |
| 9 | E2E intégration : create → join → key exchange → send encrypted → receive decrypted | Integration |
| 10 | Signature sur ciphertext valide → hub accept | Integration |
| 11 | 3 membres, rotation après départ, messages post-rotation OK | Integration |

## 7. Risques et Mitigations

| Risque | Impact | Mitigation |
|--------|--------|------------|
| Fenêtre de vulnérabilité au join (pas encore de clés) | Messages perdus ou illisibles temporairement | Buffer de messages en attente + retry |
| Explosion de messages SenderKeyDistribution (N² au join) | Surcharge réseau dans grands groupes | Acceptable pour PoC (groupes < 20 membres) |
| Pas de forward secrecy intra-session | Messages avant rotation lisibles par ancien membre | Acceptable — rotation au départ suffit pour le PoC |
| Sender Key stockée en mémoire | Vol si process compromis | Acceptable — même risque que les clés Ed25519 |

## 8. Non-Scope (YAGNI)

- ❌ Double Ratchet / forward secrecy par message
- ❌ Chiffrement du nom de groupe
- ❌ Rotation périodique automatique
- ❌ Multi-device key sync
- ❌ Key escrow ou recovery
- ❌ Chiffrement des metadata de contrôle (Create, Join, Leave)
