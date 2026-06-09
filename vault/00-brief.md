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
- **Objectif courant** : Push 9 commits locaux → CI GitHub. Puis : rebuild xcframework + lancer chantier app macOS (5 lots A→E)
- **Prochaine action utile** : `git push origin main` (terminal) → CI → `make ffi && make ffi-device` pour embarquer serde NodeStatusFFI dans l'app tvOS
- **Repo** : https://github.com/malikkaraoui/ToM-protocol
- **Dernière activité** : 2026-06-09 — review Copilot x3 intégrée, fix commentaire CLI --bind-port, 9 commits prêts à pusher

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
- Signaling server WebSocket déprécié mais toujours présent dans `tools/`
- `docs/TOM-TVOS-NODE-PLAN.md` décrit encore beaucoup de "à créer" alors que l'app tvOS + FFI + SwiftUI existent déjà → MAJ nécessaire
- Aucun test Swift/tvOS dans le `.xcodeproj` — couverture absente
- xcframework (414 Mo, artefact binaire) exclu de git — à rebuilder après chaque changement FFI Rust (`make ffi`)
- NAS : IPv6 entrante Freebox port 43925 encore fermée → connexion DIRECT QUIC bloquée (fallback relay actif)
