# Brief projet

> ⛔ **RÈGLE 1 — ANTI-HALLUCINATION ABSOLUE**
> Interdiction totale d'inventer, de mentir, d'halluciner.
> Si incertain → « Je ne peux pas l'affirmer » + 2-3 hypothèses + comment vérifier.

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## État court

- **Projet** : ToM Protocol (The Open Messaging)
- **Phase** : Phase 3 — Convergence TS+Rust + tvOS Node (actif)
- **Stack** : Rust 2021 · TypeScript · Swift/SwiftUI · QUIC (iroh-forked)
- **Version** : 0.2.0 (tous crates natifs, bumped 2026-04-15)
- **Objectif courant** : Consolider le nœud tvOS existant (FFI Rust → Swift déjà branché) et réaligner code, tests et documentation
- **Prochaine action utile** : Décider explicitement entre package `TomCoreKit` prévu au plan et wrapper Swift local existant, puis ajouter des tests Swift + remettre `TOM-TVOS-NODE-PLAN.md` à jour
- **Repo** : https://github.com/malikkaraoui/ToM-protocol
- **Dernière activité** : 2026-06-07 — audit dépôt : `cargo check --workspace` vert, plan tvOS partiellement obsolète, absence de tests Swift confirmée

## À lire en priorité

- `docs/TOM-TVOS-NODE-PLAN.md` — plan en cours (tvOS node)
- `.claude/CLAUDE.md §0` — contexte session courant
- `vault/40-roadmap.md` — prochaines phases

## Décisions actives

- Fork complet des dépendances iroh sous namespace `tom-*` (R7 ✅)
- Relay embarqué NAS ARM64 sur port 3340 (http://192.168.0.21:3340)
- FFI Rust→Swift via xcframework (`TomProtocolFFI.xcframework` buildé ✅)
- Cargo alias trick pour transparence crates forkées
- `ed25519-dalek` épinglé `=3.0.0-pre.1` (compat crypto quinn)

## Risques / angles morts

- `tom-relay-ffi` et `tom-gateway` présents dans crates mais absents de CLAUDE.md (documentation à jour incertaine)
- `docs/TOM-TVOS-NODE-PLAN.md` décrit encore beaucoup de "à créer" alors que l'app tvOS, le FFI Rust et plusieurs vues SwiftUI existent déjà
- Aucun fichier `*Tests*.swift` trouvé sous `apps/tom-node-tvos/` — couverture Swift/tvOS absente à ce stade
- Working tree local chargé : changements `.claude`, `pnpm-lock.yaml`, `package.json`, `project.pbxproj`, artefacts build tvOS et `vault/` non commités
- Signaling server WebSocket déprécié mais toujours présent dans `tools/`
- Handoff Copilot dû : 4 commits · +88 lignes · 12j (signalé par hook)
