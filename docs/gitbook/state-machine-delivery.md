# State machine de delivery (pending → sent → relayed → delivered → read)

Cette page décrit la **machine à états** utilisée pour suivre la vie d’un message “direct” (1:1) côté SDK.

Elle sert à :

- afficher un statut cohérent dans l’UI
- mesurer des timings (latence perçue, délais d’ACK)
- éviter les régressions d’état (ex: `delivered → relayed`)

## Les états

| État | Signification | Intuition UI |
|---|---|---|
| `pending` | message créé et tracké, pas encore “hand-off” au transport | “en attente” |
| `sent` | message remis à la couche transport | “envoyé” |
| `relayed` | ACK de relai reçu (le relai confirme le forward) | “transmis” |
| `delivered` | ACK destinataire reçu (arrivé au nœud cible) | “délivré” |
| `read` | read receipt reçu (le destinataire a vu) | “lu” |

{% hint style="info" %}
La règle “Delivery = ACK” est alignée avec les décisions verrouillées : **un message est livré si et seulement si le destinataire final ACK**.
{% endhint %}

## Diagramme (vue d’ensemble)

```text
track()                    markSent()         markRelayed()         markDelivered()        markRead()
  │                            │                  │                     │                    │
  ▼                            ▼                  ▼                     ▼                    ▼
pending  ───────────────▶     sent  ─────────▶  relayed  ─────────▶  delivered  ─────────▶   read
```

## Déclencheurs (source of truth dans le code)

### 1) Création du tracking (`pending`)

Au moment où `TomClient` envoie, il crée l’enveloppe puis enregistre le message :

- `MessageTracker.track(messageId, to)` ⇒ status = `pending`

### 2) Passage à `sent`

Après l’émission vers le transport, le SDK marque :

- `MessageTracker.markSent(messageId)`

### 3) Passage à `relayed`

Quand un relai forwarde, il émet un ACK `ackType = "relay-forwarded"`.
Le `Router` reçoit cet ACK et déclenche :

- `onRelayAckReceived(messageId)` côté `TomClient`
- `MessageTracker.markRelayed(messageId)`

### 4) Passage à `delivered`

Quand le destinataire final reçoit le message, il auto-envoie un ACK `ackType = "recipient-received"`.
Le `Router` reçoit cet ACK et déclenche :

- `onDeliveryAckReceived(messageId)` côté `TomClient`
- `MessageTracker.markDelivered(messageId)`

### 5) Passage à `read`

Le read receipt est déclenché par l’application (ex: quand l’UI affiche le message) :

- `TomClient.markAsRead(messageId)` côté destinataire

Puis côté émetteur :

- `Router` reçoit `type = "read-receipt"`
- `onReadReceiptReceived(messageId, readAt, from)`
- `MessageTracker.markRead(messageId)`

## Timings (timestamps et deltas)

Chaque transition stocke un timestamp dans `MessageStatusEntry.timestamps` :

- `pending`, `sent`, `relayed`, `delivered`, `read`

Les timings utiles (approximations observables) :

- délai de hand-off transport :
  $$\Delta_{handoff} = t_{sent} - t_{pending}$$
- temps jusqu’au relai :
  $$\Delta_{relayAck} = t_{relayed} - t_{sent}$$
- temps E2E (définition “delivery=ACK”) :
  $$\Delta_{delivery} = t_{delivered} - t_{sent}$$
- temps jusqu’à lecture (UX) :
  $$\Delta_{read} = t_{read} - t_{delivered}$$

{% hint style="warning" %}
Ces timings sont *best effort* : ils dépendent d’horloges locales (timestamp côté destinataire pour `readAt`), et du fait que l’app appelle `markAsRead`.
{% endhint %}

## Garanties et anti-bugs

### Pas de régression d’état

Le tracker impose un ordre strict et ignore les transitions invalides (ex: `delivered → relayed`).

### Anti-DoS / mémoire

- nombre max de messages trackés : 10 000 (éviction)
- cleanup :
  - messages `read` purgeables après un délai (ex: 10 minutes)
  - messages “stuck” purgeables après 24h

## Ce que l’UI doit faire (conseils)

- Afficher “délivré” uniquement après `delivered`.
- Appeler `markAsRead` au bon moment (message réellement vu), sinon `read` ne montera jamais.
- Exposer un état “stuck” si `pending/sent` restent trop longtemps.

## Sources

- `MessageTracker` (state machine + timestamps + cleanup) : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/routing/message-tracker.ts
- `Router` (ACK `relay-forwarded` / `recipient-received`, read receipts) : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/core/src/routing/router.ts
- `TomClient` (wiring des événements tracker) : https://github.com/malikkaraoui/ToM-protocol/blob/main/packages/sdk/src/tom-client.ts
