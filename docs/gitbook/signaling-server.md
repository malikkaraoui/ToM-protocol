# Serveur de signaling (WebSocket)

## Contexte

Le signaling server est un composant temporaire (PoC) pour permettre la découverte des pairs et l’échange des informations nécessaires à l’établissement WebRTC.
Il est explicitement marqué “TEMPORARY” dans le code et sera remplacé à terme par un mécanisme distribué (DHT / discovery).

## Endpoints

- WebSocket : `ws://<host>:3001`
- Healthcheck : `http://<host>:3001/health`

{% hint style="warning" %}
Ce serveur est un **bootstrap**. Il est volontairement minimal et destiné à disparaître à terme.
{% endhint %}

## Modèle de messages

Tous les messages sont des JSON avec un champ `type`.
Le server agit principalement comme relais:
- il maintient une liste de participants connectés
- il broadcast des événements de présence
- il forward les messages `signal` sans inspection

## Types de messages

### `register`

**Client → Serveur**

Champs :
- `nodeId` (string, requis)
- `username` (string, requis)
- `publicKey` (string, optionnel)
- `encryptionKey` (string, optionnel)

Exemple :

```json
{"type":"register","nodeId":"aaa","username":"alice"}
```

### `participants`

**Serveur → Clients (broadcast)**

Champs :
- `participants`: `[{ nodeId, username, encryptionKey? }]`

### `presence`

**Serveur → Clients (broadcast aux autres)**

Champs :
- `action`: `"join" | "leave"`
- `nodeId`
- `username`
- `publicKey`
- `encryptionKey?`

### `heartbeat`

**Client → Serveur**

```json
{"type":"heartbeat"}
```

**Serveur → Clients (broadcast aux autres)**

- `type`: `"heartbeat"`
- `from`: `nodeId` de l’émetteur (uniquement si l’émetteur s’est enregistré)

### `role-assign`

**Client → Serveur**

Champs :
- `nodeId` (string, requis)
- `roles` (string[], requis)

**Serveur → Clients (broadcast à tous)**

### `signal`

**Client → Serveur**

Champs :
- `from` (string, requis)
- `to` (string, requis)
- `payload` (unknown)

Le serveur transmet ce message **tel quel** vers le pair cible.

### `error`

**Serveur → Client**

Champs :
- `error` (string)

Cas typiques:
- JSON invalide
- register incomplet
- signal sans `to`/`from`
- peer non trouvé

## Notes de sécurité

- Le signaling server est un relais “best effort”; il n’authentifie pas (encore) les identités.
- Toute sécurité “forte” est supposée venir du protocole (E2E) et des mécanismes de confiance, pas du serveur.
- L’endpoint /health est volontairement minimal (pas de détails sensibles).

## Code source

- Serveur : https://github.com/malikkaraoui/ToM-protocol/blob/main/tools/signaling-server/src/server.ts
- Types : https://github.com/malikkaraoui/ToM-protocol/blob/main/tools/signaling-server/src/index.ts
