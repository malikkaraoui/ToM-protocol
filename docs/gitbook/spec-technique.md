# Spécification technique (v0.0.1)

Cette page décrit le **socle technique observable dans le code** (wire format, routage, ACK, E2E). Elle vise à être assez précise pour implémenter un client compatible, sans prétendre couvrir tous les cas métier.

{% hint style="info" %}
La version exposée côté code est `TOM_PROTOCOL_VERSION = "0.0.1"`.
{% endhint %}

## Format des messages (wire format)

Le protocole échange des enveloppes JSON appelées `MessageEnvelope`.

### `MessageEnvelope`

Champs **requis** :

- `id: string` — identifiant unique du message (UUID si disponible, sinon aléatoire hex)
- `from: NodeId` — identifiant du nœud émetteur (hex, généralement 64 caractères)
- `to: NodeId` — identifiant du nœud destinataire final
- `via: NodeId[]` — chemin de relais **ordonné** (peut être vide)
- `type: string` — type applicatif/protocolaire (ex: `"chat"`, `"app"`, `"ack"`, `"read-receipt"`)
- `payload: unknown` — contenu (en clair ou chiffré)
- `timestamp: number` — epoch ms côté émetteur
- `signature: string` — signature de l’enveloppe (hex). Peut être vide selon le contexte.

Champs **optionnels** (observabilité / crypto) :

- `encryptedPayload?: string` — pipeline crypto (héritage / compat), peut être utilisé pour porter le ciphertext
- `ephemeralPublicKey?: string` — clé publique éphémère (X25519) pour l’E2E
- `nonce?: string` — nonce (XSalsa20-Poly1305)
- `hopTimestamps?: number[]` — timestamps ajoutés à chaque hop pour mesurer la latence
- `routeType?: "relay" | "direct"` — utilisé pour la visualisation de chemin

### Signature

Quand une signature est produite, l’implémentation signe la sérialisation JSON de l’enveloppe **avec `signature` mise à vide**, puis injecte la signature (hex) dans le champ `signature`.

Intuition :

- on signe l’enveloppe complète (métadonnées + payload)
- l’auto-référence (`signature` qui signerait elle-même) est évitée en signant la variante `signature: ""`

{% hint style="warning" %}
La signature protège l’intégrité de l’enveloppe **au niveau transport**. L’E2E protège le contenu **au niveau payload**. Les deux peuvent coexister.
{% endhint %}

## Routage et relais

### Direct vs relais

- **Direct** : `via = []` et/ou `routeType = "direct"`.
- **Relais** : `via` contient au moins un NodeId et/ou `routeType = "relay"`.

Le routeur peut préférer une route directe lorsqu’un chemin WebRTC est établi, et retomber sur un relais si nécessaire.

### Chaîne multi-relai (`via`)

Si un nœud trouve son identifiant dans `via`, il se considère comme un **relai intermédiaire** et forward vers :

- le prochain relai dans `via`, ou
- le destinataire final `to` si c’est le dernier relai.

Une protection existe contre les chaînes trop longues : le routeur rejette si `via.length` dépasse une profondeur maximale.

### Latence et `hopTimestamps`

À chaque forward, un relai peut enrichir `hopTimestamps` en ajoutant `Date.now()`.
Cela sert à :

- observer la latence par hop,
- alimenter la visualisation du chemin.

## Accusés de réception (ACK) et read receipts

Deux types “système” sont utilisés :

- `type = "ack"`
- `type = "read-receipt"`

### ACK (`type = "ack"`)

Le payload suit la forme :

- `originalMessageId: string`
- `ackType: "relay-forwarded" | "recipient-received" | "recipient-read"`

Sémantique :

- `relay-forwarded` : un relai confirme qu’il a forwardé
- `recipient-received` : le destinataire final confirme la réception
- `recipient-read` : réservé/compatible, la lecture est principalement matérialisée via `read-receipt`

Anti-replay : les ACK sont filtrés via un cache à durée courte, avec une clé composite de la forme :

$$k = originalMessageId : from : ackType$$

### Read receipt (`type = "read-receipt"`)

Le payload attend :

- `originalMessageId: string`
- `readAt?: number`

Le `readAt` est **clampé** côté réception (pas dans le futur, pas trop ancien) afin d’éviter une manipulation temporelle grossière.

## Chiffrement E2E (payload)

Quand l’E2E est activé, le `payload` peut être un objet de type `EncryptedPayload` (hex) :

- `ciphertext: string`
- `nonce: string`
- `ephemeralPublicKey: string`

Le chiffrement utilise `tweetnacl` via `nacl.box` : X25519 + XSalsa20-Poly1305.

Propriétés :

- seuls l’émetteur et le destinataire peuvent déchiffrer
- les relais ne voient que les métadonnées de routage (pas le contenu)

Limite : la distribution des clés publiques de chiffrement dépend aujourd’hui du bootstrap (signaling) et doit être considérée comme **non fiable** tant qu’un mécanisme d’authentification/attestation n’est pas établi.

## Observabilité / debug (chemins)

Une extraction de chemin peut être faite à partir de l’enveloppe reçue (sans requête réseau), en combinant :

- `routeType` (direct/relay)
- `via`
- `timestamp` + `receivedAt`

C’est la base de la feature de visualisation de chemin et des métriques “golden path”.

## Sources

- Type `MessageEnvelope` : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/types/envelope.ts
- Routeur (ACK, read receipts, dedup, depth) : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/routing/router.ts
- E2E (TweetNaCl `box`) : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/crypto/encryption.ts
