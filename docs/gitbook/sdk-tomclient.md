# SDK — `TomClient` (tom-sdk)

Cette page documente l’API “plug-and-play” exposée par `tom-sdk` (package `packages/sdk`).

## Philosophie

- Tu fournis un `signalingUrl` et un `username`.
- Le client gère le bootstrap (WebSocket), le transport (WebRTC), le routage (relai/direct), la découverte, et les événements.

{% hint style="warning" %}
Le signaling WebSocket est un **bootstrap temporaire** (voir ADR-002). Le SDK cache cette complexité pour pouvoir migrer vers une découverte autonome (DHT) sans casser l’API applicative.
{% endhint %}

## Instanciation

Options principales :

- `signalingUrl: string`
- `username: string`
- `encryption?: boolean` — E2E activé par défaut
- `storage?: IdentityStorage` — persistance d’identité

## Connexion

- `await client.connect()`
- `client.disconnect()`

À la connexion, le client :

- initialise l’identité (`nodeId`)
- se registre côté signaling (présence + clé de chiffrement)
- initialise transport + router + sélection de relais + direct-path
- démarre heartbeat, gossip, subnets (selon configuration)

### Récupérer son identité

- `client.getNodeId()`

## Envoi de messages

### Message texte

- `await client.sendMessage(to: NodeId, text: string, relayId?: NodeId)`

Comportement :

- sélection automatique d’un relai si `relayId` n’est pas fourni
- chiffrement E2E si une clé de chiffrement du destinataire est connue
- tracking de statut (pending → sent → relayed → delivered → read)

### Payload arbitraire (app / jeu)

- `await client.sendPayload(to: NodeId, payload: object, relayId?: NodeId)`

Utile pour des messages applicatifs typés (ex: Snake).

## Réception et événements

### Recevoir les messages

- `client.onMessage((envelope) => { ... })`

Le SDK tente de déchiffrer automatiquement le payload si le message est chiffré.

### Participants (présence)

- `client.onParticipants((participants) => { ... })`

### Statut / debug

- `client.onStatus((status, detail?) => { ... })`

Le statut est volontairement verbeux : il sert d’outil de debug protocolaire et UI.

## ACK, delivery, read receipts

### Statut d’un message

- `client.onMessageStatusChanged((messageId, prev, next) => { ... })`
- `client.getMessageStatus(messageId)`

### Marquer un message comme lu

- `client.markAsRead(messageId)`

C’est à l’application d’appeler cette méthode au bon moment (affichage UI, scroll, etc.).

## Topologie, rôles et connexions

- `client.getTopology()` — liste de peers atteignables
- `client.getPeerRoles(nodeId)` — rôles d’un peer
- `client.onRoleChanged((nodeId, roles) => { ... })`
- `client.onConnectionTypeChanged((peerId, connectionType) => { ... })` (`direct` / `relay`)

## Visualisation de chemin (path info)

Pour un message reçu, le SDK garde une trace de l’enveloppe afin d’extraire un chemin lisible :

- `client.getPathInfo(messageId)`

Ce chemin est dérivé des champs `via`, `routeType`, `timestamp` et d’un `receivedAt` interne.

## Groupes (chat de groupe)

Le SDK expose des primitives haut niveau pour créer et faire vivre des groupes.

### Créer un groupe

- `await client.createGroup(name, initialMembers?)`

### Invitations

- `await client.inviteToGroup(groupId, inviteeNodeId, inviteeUsername)`
- `await client.acceptGroupInvite(groupId)`
- `client.declineGroupInvite(groupId)`

### Envoyer dans un groupe

- `await client.sendGroupMessage(groupId, text)`

## Sources

- TomClient : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/sdk/src/tom-client.ts
- SDK package : https://github.com/malikkaraoui/ToM-protocol/tree/main/packages/sdk
