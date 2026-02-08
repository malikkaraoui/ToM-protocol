# Routage & reroute (résilience)

Cette page décrit le routage ToM côté SDK :

- comment un message choisit un chemin (relai vs direct)
- quelles sont les limites (profondeur, tentatives)
- comment le reroute s’exécute quand un relai tombe

## Routage : primitives

### Enveloppe et métadonnées

Le routage se fait via `MessageEnvelope` :

- `to` : destinataire final
- `via` : chaîne de relais ordonnée (peut être vide)
- `routeType` : `"relay" | "direct"` (observabilité / UI)
- `hopTimestamps` : timestamps ajoutés à chaque hop

### Relai (chemin “normal”)

Le relai est la voie par défaut :

1. création de l’enveloppe avec `via = [relayId]`
2. émission au relai
3. le relai forward au destinataire

Deux ACK structurent le suivi :

- ACK relai (`ackType = "relay-forwarded"`) → confirme le forward
- ACK destinataire (`ackType = "recipient-received"`) → confirme la delivery

### Direct (optimisation)

Le `DirectPathManager` représente une optimisation : si un chemin “direct” est disponible, le routeur peut bypass le relai.

{% hint style="info" %}
La notion de “direct” dépend de l’implémentation de `TransportLayer.connectToPeer` : dans un PoC, cela peut rester un transport via signaling ; dans une implémentation WebRTC complète, cela devient un DataChannel.
{% endhint %}

## Sélection de relai

Le SDK utilise un `RelaySelector` (résultat: `relayId` ou raison).

Cas typiques :

- relai disponible → sélection du “meilleur” relai
- pas de relai → fallback direct (tentative de connexion au destinataire)
- destinataire = soi-même → erreur (guard)

## Chaînes multi-relai (`via`)

Si un nœud se retrouve dans `via`, il agit en relai intermédiaire.

Limite : une chaîne `via` trop profonde est rejetée (protection contre abus).

## Ce qui déclenche un reroute

Le reroute se déclenche quand l’envoi via un relai échoue.

Exemples concrets :

- pas de connexion active vers le relai (`transport.getPeer(relayId)` absent)
- impossible d’établir une connexion vers le relai (`connectToPeer` rejette)

Dans ce cas, le `Router` émet `onRerouteNeeded(envelope, failedRelayId)`.

## Algorithme de reroute (SDK)

Paramètres importants :

- max tentatives : `MAX_REROUTE_ATTEMPTS = 3`
- mutex : `reroutingInProgress` empêche les reroutes parallèles par message
- set `failedRelaysPerMessage` : mémorise les relais à éviter

Pseudo-code :

```text
onRerouteNeeded(envelope, failedRelay):
  add failedRelay to failedRelays[messageId]
  if failedRelays.size >= 3: queue(message); return

  selection = selectAlternateRelay(to, topology, failedRelays)
  if selection.relayId is null: queue(message); return

  reroutableEnvelope = clone(envelope); reroutableEnvelope.via = [selection.relayId]
  connectToPeer(selection.relayId)
    -> sendViaRelay(reroutableEnvelope, selection.relayId)
    -> on success: cleanup failedRelays
    -> on failure: recurse with new failed relay
```

### Queuing (fallback)

Si aucun relai alternatif n’est disponible, ou si toutes les tentatives échouent :

- le message est “queued” (`emitMessageQueued`) avec une raison

Cela permet de brancher un mécanisme de “backup delivery” / retry plus tard.

## Comportement en panne (ce que tu peux observer)

Côté UI / logs, tu verras typiquement :

- `message:rejected` (relai unreachable)
- `reroute:attempting` → `reroute:alternate-found` → `reroute:success`
- ou `reroute:no-alternate` / `reroute:max-attempts`
- `message:queued`

## Limites et pièges

- Le reroute ne garantit pas la delivery : c’est un mécanisme de résilience “best effort”.
- Un relai peut forwarder puis drop des ACK : la machine à états reste cohérente (pas de faux “delivered”).
- `via` est muté par certaines méthodes ; le SDK clone l’enveloppe avant reroute pour éviter les effets de bord.

## Sources

- Reroute SDK : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/sdk/src/tom-client.ts
- Router (ACK, forward, limites, `sendViaRelay`) : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/routing/router.ts
- Enveloppe (routeType, hopTimestamps) : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/types/envelope.ts
- DirectPathManager : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/transport/direct-path-manager.ts
