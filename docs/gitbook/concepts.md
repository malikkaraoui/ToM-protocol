Concepts

ToM est une couche de transport. Les applications s’en servent, mais les utilisateurs finaux ne devraient pas “voir” ToM.

No central server

Il n’y a pas de serveur applicatif qui route et stocke les messages.
Un serveur de signaling existe aujourd’hui uniquement pour le bootstrap WebRTC, et il est marqué comme temporaire.

Relay

Un relais est un nœud qui transmet des messages pour d’autres.
Le relais ne doit pas stocker le contenu.
Le relais est un mécanisme de contribution au réseau, pas une optimisation optionnelle.

ACK

Un message est considéré comme délivré si et seulement si le destinataire final émet un ACK.
C’est une règle de définition (pas une métrique UI).

TTL et purge

Les messages ont une durée de vie maximale de 24h.
Passé ce délai, purge globale, même si non délivré.
Le protocole promet un effort maximal borné dans le temps, pas une conservation infinie.

Rôles dynamiques

Les rôles (client, relay, etc.) peuvent évoluer.
L’idée n’est pas de “configurer un rôle”, mais de laisser le réseau attribuer du travail.

E2E

Le contenu des messages est chiffré bout-en-bout.
Les relais ne peuvent pas lire le contenu, seulement le routage.

Références

- Décisions verrouillées : ../../_bmad-output/planning-artifacts/design-decisions.md
- Architecture / ADRs : ../../_bmad-output/planning-artifacts/architecture.md
