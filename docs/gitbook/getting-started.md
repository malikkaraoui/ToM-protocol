# Démarrage rapide

## Pré‑requis

- Node.js >= 20
- pnpm

## Installation (local)

1. Clone le repo
2. Installe les dépendances
3. Build + tests

```bash
pnpm install
pnpm build
pnpm test
```

## Lancer la démo

La manière la plus simple est d’utiliser le script de lancement.

Le script lance 2 services :

- Signaling server (bootstrap) : `ws://localhost:3001` (+ healthcheck `http://localhost:3001/health`)
- Demo Vite : `http://localhost:5173`

```bash
./scripts/start-demo.sh
```

## Références

- Script : https://github.com/malikkaraoui/ToM-protocol/blob/main/scripts/start-demo.sh
- Code démo : https://github.com/malikkaraoui/ToM-protocol/tree/main/apps/demo
- Signaling server : https://github.com/malikkaraoui/ToM-protocol/tree/main/tools/signaling-server

## Notes

- Le signaling WebSocket est **temporaire** (bootstrap). Il est isolé et explicitement destiné à disparaître dans la roadmap (voir architecture).

{% hint style="info" %}
Si tu testes sur un autre device (LAN/Wi‑Fi), pense à exposer Vite avec `--host 0.0.0.0`.
{% endhint %}
