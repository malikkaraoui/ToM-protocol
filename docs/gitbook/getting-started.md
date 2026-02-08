Démarrer

Pré-requis

- Node.js >= 20
- pnpm

Installation

- Clone le repo
- Installe les dépendances
- Build + tests

Démo locale

La manière la plus simple est d’utiliser le script de lancement.

- Démarre le signaling server (bootstrap) sur le port 3001
- Démarre la démo Vite sur le port 5173

Référence

- Script : ../../scripts/start-demo.sh
- Code démo : ../../apps/demo
- Signaling server : ../../tools/signaling-server

Notes

- Le signaling WebSocket est temporaire (bootstrap). Il est isolé et explicitement destiné à disparaître dans la roadmap (voir architecture).
