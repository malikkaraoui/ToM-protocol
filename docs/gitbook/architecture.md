# Architecture

## Vue d’ensemble

Le repo est structuré comme un protocole, pas comme une application.

- packages/core (tom-protocol) : primitives du protocole
- packages/sdk (tom-sdk) : API TomClient “plug-and-play”
- apps/demo : démo browser (chat + Snake)
- tools/signaling-server : bootstrap temporaire (WebSocket)
- tools/vscode-extension : extension VS Code (WIP sur certaines parties)

## Décisions clés (ADRs)

- WebRTC DataChannel comme transport browser-first
- Signaling WebSocket comme bootstrap temporaire
- Format d’enveloppe JSON (id, from, to, via, type, payload, timestamp, signature)
- Chiffrement E2E via TweetNaCl
- Modèle de nœud unifié (un même code, rôle déterminé par le réseau)

## Où lire la source

- Document d’architecture (ADRs) : https://github.com/malikkaraoui/ToM-protocol/blob/main/_bmad-output/planning-artifacts/architecture.md
- PRD : https://github.com/malikkaraoui/ToM-protocol/blob/main/_bmad-output/planning-artifacts/prd.md
- Guide “comment naviguer le code” : https://github.com/malikkaraoui/ToM-protocol/blob/main/CLAUDE.md

## Pour les développeurs (détails techniques)

- [Spécification technique (v0.0.1)](spec-technique.md)
- [SDK — TomClient](sdk-tomclient.md)
- [Sécurité & modèle de menace](securite-modele-menace.md)

## État d’avancement

Le sprint-status indique Epics 1 à 8 comme livrés.
La rétro Epics 4-8 détaille une phase de consolidation (failover hub, invitations robustes, UI réactive, E2E tests).

{% hint style="warning" %}
Le signaling server est un **bootstrap**. Il aide à démarrer, mais la roadmap vise à l’éliminer.
{% endhint %}

- Sprint status : https://github.com/malikkaraoui/ToM-protocol/blob/main/_bmad-output/implementation-artifacts/sprint-status.yaml
- Rétro : https://github.com/malikkaraoui/ToM-protocol/blob/main/_bmad-output/implementation-artifacts/epic-4-8-retro-2026-02-07.md

{% hint style="info" %}
Ces documents sont versionnés dans le dépôt GitHub (liens ci‑dessus). La doc GitBook est volontairement un **résumé lisible**.
{% endhint %}
