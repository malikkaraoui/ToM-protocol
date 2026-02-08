Signaling server (WebSocket)

Contexte

Le signaling server est un composant temporaire (PoC) pour permettre la découverte des pairs et l’échange des informations nécessaires à l’établissement WebRTC.
Il est explicitement marqué “TEMPORARY” dans le code et sera remplacé à terme par un mécanisme distribué (DHT / discovery).

URL

- WebSocket: ws://<host>:3001
- Healthcheck: http://<host>:3001/health

Modèle de messages

Tous les messages sont des JSON avec un champ `type`.
Le server agit principalement comme relais:
- il maintient une liste de participants connectés
- il broadcast des événements de présence
- il forward les messages `signal` sans inspection

Types de messages

register

Client → Server

Champs:
- nodeId (string, requis)
- username (string, requis)
- publicKey (string, optionnel)
- encryptionKey (string, optionnel)

Exemple:

{"type":"register","nodeId":"aaa","username":"alice"}

participants

Server → Client (broadcast)

Champs:
- participants: [{ nodeId, username, encryptionKey? }]

presence

Server → Client (broadcast aux autres)

Champs:
- action: "join" | "leave"
- nodeId
- username
- publicKey
- encryptionKey?

heartbeat

Client → Server: {"type":"heartbeat"}

Server → Clients (broadcast aux autres):
- type: "heartbeat"
- from: nodeId de l’émetteur (uniquement si l’émetteur s’est enregistré)

role-assign

Client → Server

Champs:
- nodeId (string, requis)
- roles (string[], requis)

Server → Clients (broadcast à tous)

signal

Client → Server

Champs:
- from (string, requis)
- to (string, requis)
- payload (unknown)

Le server forward ce message tel quel vers le pair cible.

error

Server → Client

Champs:
- error (string)

Cas typiques:
- JSON invalide
- register incomplet
- signal sans `to`/`from`
- peer non trouvé

Notes de sécurité

- Le signaling server est un relais “best effort”; il n’authentifie pas (encore) les identités.
- Toute sécurité “forte” est supposée venir du protocole (E2E) et des mécanismes de confiance, pas du serveur.
- L’endpoint /health est volontairement minimal (pas de détails sensibles).
