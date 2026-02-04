# ToM Protocol

**The Open Messaging** — Un protocole de transport P2P décentralisé où chaque appareil fait partie du réseau.

## Qu'est-ce que ToM ?

ToM est une couche de communication peer-to-peer conçue pour fonctionner **sans serveur central**. Chaque participant devient à la fois client et relais, formant un réseau organique auto-organisé.

### Principes clés

- **Décentralisé** : Aucune dépendance à un serveur central obligatoire
- **P2P automatique** : Connexion directe entre pairs, avec relais intelligent en fallback
- **Éphémère** : Bus de données sans historique permanent (pas de blockchain)
- **Chiffré** : Chiffrement de bout en bout via TweetNaCl
- **Rôles dynamiques** : Les nœuds se voient attribuer des rôles selon leur capacité réseau

### Fonctionnalités

- Messagerie directe et de groupe
- Gestion automatique des relais
- Système de backup temporaire des messages
- Découverte de pairs via heartbeat
- Métriques de latence et visualisation des chemins

## Quick Start

```bash
pnpm install
pnpm build
pnpm test
```

## Structure du projet

```
packages/
├── core/     → Primitives du protocole (transport, routing, identité, groupes)
└── sdk/      → API simplifiée pour développeurs (TomClient)

apps/
└── demo/     → Application de démonstration avec jeu Snake multijoueur P2P

tools/
└── signaling-server/  → Serveur WebSocket de bootstrap (temporaire)
```

## Roadmap

Le serveur de signalisation est une béquille temporaire. L'objectif est de l'éliminer progressivement :

1. **Actuel** : Serveur WebSocket unique pour le bootstrap
2. **Phase 2** : Serveurs de seed redondants
3. **Phase 3** : Table de hachage distribuée (DHT)
4. **Phase 4** : Découverte 100% P2P, zéro infrastructure fixe

## License

MIT
