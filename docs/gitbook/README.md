# ToM — The Open Messaging

ToM est un protocole de transport P2P (pas une blockchain) : chaque appareil est à la fois **client** et **relais**.

{% hint style="info" %}
Objectif : rendre la messagerie décentralisée aussi « banale » qu’un tuyau réseau — sans serveurs applicatifs à opérer.
{% endhint %}

## Promesse

- **Zéro serveur applicatif** : pas de backend central qui stocke et route les messages.
- **Relais sans stockage** : pass‑through uniquement.
- **Chiffrement bout‑en‑bout (E2E)** : seuls l’émetteur et le destinataire lisent le contenu.
- **Auto‑organisation** : découverte gossip, subnets éphémères, rôles dynamiques.

## Ce que la démo prouve déjà

- un message traverse **réellement** un relais
- groupes, invitations, visualisation de chemin
- résilience multi‑participants (failover hub)
- Snake multijoueur P2P

## Démarrer

- [Démarrage rapide](getting-started.md)
- [Concepts clés](concepts.md)

## Repères projet

- [Architecture](architecture.md)
- [Décisions verrouillées (les « 7 locks »)](design-decisions.md)
- [Serveur de signaling (bootstrap WebSocket)](signaling-server.md)

## Contribuer

- [Contribuer](contributing.md)
- [Backlog d’issues](issues-backlog.md)

## Liens

- Dépôt GitHub : https://github.com/malikkaraoui/ToM-protocol
- Référence rapide (LLM) : https://github.com/malikkaraoui/ToM-protocol/blob/main/llms.txt
- Guide dev (LLM) : https://github.com/malikkaraoui/ToM-protocol/blob/main/CLAUDE.md
