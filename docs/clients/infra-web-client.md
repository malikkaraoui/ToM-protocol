# C4 MVP — Infra Web Client

Date: 2026-03-05

Client web minimal pour valider l'infrastructure Option C (relay + discovery), sans dépendre du signaling legacy.

## Emplacement

- `apps/infra-web-client/`

## Ce que valide le client

Endpoints relay:

- `GET /ready`
- `GET /health`
- `GET /healthz`

Endpoints discovery:

- `GET /health`
- `GET /relays`
- `GET /metrics`
- `GET /status`

Le client affiche:

- état global relay/discovery
- latence des probes
- liste des relays découverts
- snapshot JSON consolidé

## Lancement

Depuis la racine workspace:

- `pnpm dev:infra-client`

Build:

- `pnpm build:infra-client`

## Paramétrage

L’UI permet de saisir:

- `Relay URL`
- `Discovery URL`

Support query params pour partage rapide:

- `?relay=http://127.0.0.1:3340&discovery=http://127.0.0.1:8080`

## Scope

Ce MVP est volontairement orienté **validation infra**.
Il ne remplace pas le client chat/demo produit.
