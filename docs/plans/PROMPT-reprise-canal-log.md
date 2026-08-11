# PROMPT DE REPRISE — Canal log (étape 1 posée, reprendre à S2)

> À coller à l'ouverture d'une nouvelle session. Contexte : ToM Protocol,
> messagerie P2P décentralisée (Rust `crates/` + apps Swift iOS/tvOS/macOS + NAS Freebox).

## Ce qui est fait (commité + poussé le 11/08)
Le **canal log** = voir les logs de tous les devices via NOTRE réseau (dogfooding des
groupes). **Étape 1 (S1)** est faite (`0e41628`) : orchestration runtime **rejoins-ou-crée**
un groupe PUBLIC d'id réservé `__tomlog__` au démarrage (broadcast `Join` → timeout 3 s →
`create` `invite_only=false`). Design complet : `docs/plans/etape1-groupe-log-public.md`.

## À FAIRE — reprendre ici
**S2 — émission des logs vers le groupe** :
- App Swift (`apps/tom-node-tvos/TomNode/Services/TomNodeService.swift`) : hook sur
  `appendLog` → buffer → flush **~2 s** → `tom_node_send_group_message(<id __tomlog__>, batch)`.
  **Filtrer ces messages de l'UI Messages** (ne jamais les afficher).
- FFI (`crates/tom-protocol-ffi`) : exposer le group_id `__tomlog__` (ou un helper) à Swift.
  `tom_node_send_group_message` existe déjà.
- (option) NAS Rust (`tom-tui`) émet aussi ses logs.

**S3 — lecture côté bot** :
- `crates/tom-tui/src/main.rs` : le bot rejoint le groupe log + route
  `GET :9300/group/inbox?group=__tomlog__&contains=&limit=` (miroir de `/inbox` ~l.515).
  Le bot ne doit PAS echo les messages de groupe.

**Puis** : 1 build/déploiement flotte (FFI xcframework → apps iPhone/iPad/ATV/Mac + bot NAS
musl) → test LIVE : les 4 devices déversent dans « log », lire via `:9300/group/inbox`.

## Contraintes projet NON négociables
- Avant push : REJOUER les commandes CI exactes (`.github/workflows/ci.yml`, par crate) —
  `cargo build/clippy/test -p <crate>` + `bash scripts/check-ffi.sh` (FFI hors workspace).
  Tests réseau → `--test-threads=1`. Push AUTO quand fini (ne pas redemander).
- `pnpm install` si `node_modules` absent (hooks git : biome). Email auteur = gmail.
- loop-master pour toute feature >1 fichier (mais il stalle sur les longs builds Rust →
  repli sur reprise à la main + relecture Fable). Design-first.

## Pièges connus (relecture Fable, TODO documentés)
- Convergence double-création (deux nœuds créent `__tomlog__` en <3 s) : atténuée par
  l'election déterministe + `rens()`, réconciliation propre = à faire.
- Barrière DURE anti-collision (refuser au protocole toute création user d'un name
  préfixé `__`) : aujourd'hui réservation par convention.
- `group_actions_to_effects` rendu `pub` (plumbing) : à ré-encapsuler.

## Après le canal log
**Étape 2 — génome immortel** : répliquer les paramètres du groupe via le **backup
service** (ADR-009) → portage collectif (le groupe survit à son créateur). Design + red-team.

Voir mémoires : `session-handoff-2026-08-11`, `push-after-finished-chantier`,
`reseau-organisme-roles-pas-tuyau`, `tom-rendezvous-tournant-design`.
