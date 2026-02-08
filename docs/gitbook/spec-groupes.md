# Spécification groupes (payloads `group-*`, hub, failover)

Cette page documente le protocole de groupes tel qu’implémenté dans `packages/core/src/groups/*` et consommé par le SDK.

## Modèle

- Un **relai** peut agir comme **hub temporaire** d’un groupe.
- Le hub :
  - maintient la membership
  - fait le fanout des messages
  - fournit un `group-sync` aux nouveaux membres
  - émet des heartbeats
- Chaque nœud membre maintient un état local via `GroupManager`.

{% hint style="warning" %}
Ce design n’est pas un “serveur central”. Le hub est un rôle temporaire porté par un nœud relai, avec une logique de migration.
{% endhint %}

## Identité et state

### `GroupInfo`

Champs clés :

- `groupId: string`
- `name: string`
- `hubRelayId: NodeId`
- `backupHubId?: NodeId` (optionnel)
- `members: GroupMember[]`
- `maxMembers: number`

### `GroupMember`

- `nodeId`, `username`, `joinedAt`
- `role: "admin" | "member"`

## Table des payloads `group-*`

Tous les payloads partagent :

- `type: string`
- `groupId: string`

### Lifecycle / membership

- `group-create` : créer un groupe sur un hub
- `group-created` : confirmation (contient `groupInfo`)
- `group-invite` : invitation (inclut `hubRelayId`)
- `group-invite-ack` : ack de réception d’invite
- `group-join` : rejoindre (avec `nodeId`, `username`)
- `group-member-joined` : broadcast membership
- `group-leave` : quitter
- `group-member-left` : broadcast départ
- `group-sync` : synchronisation d’état (nouveau membre / reconnexion)

### Messages

- `group-message` : message de groupe
  - contient `messageId`, `senderId`, `senderUsername`, `text`, `sentAt`
  - peut contenir `signature?` et `nonce?` (sécurité anti-replay)

### Hub / résilience

- `group-hub-heartbeat` : heartbeat périodique hub → membres
- `group-hub-migration` : annonce de migration (newHubId / oldHubId / reason)

### Delivery / read receipts (group)

- `group-delivery-ack`
- `group-read-receipt`

## Flux (end-to-end)

### 1) Création

1. le créateur sélectionne un relai “hub” (ou lui-même s’il est relai)
2. envoie `group-create`
3. le hub répond `group-created` avec `groupInfo`

### 2) Invitation

1. admin envoie `group-invite` vers l’invité
2. l’invité répond `group-invite-ack` (robustesse)

{% hint style="info" %}
L’ACK d’invite est un mécanisme “anti zone grise” : l’inviteur sait si l’invite est arrivée.
{% endhint %}

### 3) Join + Sync

1. l’invité envoie `group-join` au hub
2. le hub répond `group-sync` (state complet + messages récents optionnels)
3. le hub broadcast `group-member-joined`

### 4) Message

1. un membre envoie `group-message` au hub
2. le hub fanout à tous les membres
3. acks/receipts optionnels (selon intégration)

## Hub heartbeats & détection de panne

### Heartbeats côté hub

Le hub émet régulièrement `group-hub-heartbeat` vers les membres.

### Health monitoring côté membre

Chaque membre suit :

- `lastHeartbeat`
- `missedHeartbeats`

Si le nombre de heartbeats manqués dépasse un seuil (`HUB_FAILURE_THRESHOLD`), le membre déclenche un événement de panne.

## Failover hub (migration)

Quand un hub est considéré comme “down” :

1. le SDK construit une liste de candidats = relais online
2. exécute une élection déterministe (`HubElection`) pour choisir `newHubId`
3. met à jour l’état local (`handleHubMigration`)
4. si le nœud courant est élu hub :
   - il initialise `GroupHub`
   - importe l’état (membres + messages récents)
   - notifie les autres via `group-hub-migration`

Objectif : éviter le **split-brain** (deux hubs simultanés).

## Invariants & limites (implémentation)

Côté hub (`GroupHub`) :

- limites mémoire et capacité (max groups, max members, max messages, etc.)
- rate limiting par sender
- anti-replay via `NonceTracker` (si nonces requis)
- vérification de signature de messages (si signatures requises)

{% hint style="warning" %}
Les groupes actuels sont conçus comme “privés via invitation” ; l’annonce publique de groupes est mentionnée dans le code mais désactivée côté SDK.
{% endhint %}

## Sources

- Types `group-*` : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/groups/group-types.ts
- Hub : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/groups/group-hub.ts
- GroupManager (état membre + health) : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/groups/group-manager.ts
- SDK wiring (invite/join/sync/migration) : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/sdk/src/tom-client.ts
