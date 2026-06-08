# Roadmap vivante

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## Livré

### Rust Phases (R1–R11) — toutes complètes
| Phase | Description | Commits/Artefacts |
|-------|-------------|-------------------|
| ✅ R1 | Foundations (envelope, crypto, types) | crates/tom-protocol/src/{crypto,envelope,types} |
| ✅ R2 | Routing + ProtocolRuntime | crates/tom-protocol/src/{router,runtime} |
| ✅ R3 | Discovery + Keepalive (gossip) | crates/tom-gossip, discovery/ |
| ✅ R4 | Backup + Roles | crates/tom-protocol/src/{backup,roles} |
| ✅ R5 | Groups (hub, failover, sender keys, security) | crates/tom-protocol/src/group |
| ✅ R6 | TUI + Integration + Stress campaigns | crates/tom-tui, tom-stress |
| ✅ R7 | Fork + Bootstrap Elimination | 7 crates tom-* forkées, signaling server déprécié |
| ✅ R8 | Production Hardening | — |
| ✅ R9 | Consolidation (DHT, delivery reliability) | crates/tom-dht |
| ✅ R10 | Group Recovery (rejoin, tracker persistence, liveness reset) | — |
| ✅ R11 | Security & Admin (antispam, nonce anti-replay, group admin controls) | — |

### Phase 1 TypeScript — complète
- ✅ 8/8 epics — 771 tests — packages/core + packages/sdk

### tvOS Node — Phases 1+2 complètes
- ✅ 2026-04-14 : xcframework multi-plateforme buildé (`TomProtocolFFI.xcframework` présent)
- ✅ 2026-04-14 : App Xcode créée — `TomNode.xcodeproj` + structure SwiftUI (Views/ViewModels/Models/Services)
- ✅ 2026-04-15 : observabilité format JSON unifié (appareil + node_id + uptime_s + msgs_sent)
- ✅ 2026-04-15 : `nas-node-ctl.sh` — contrôle NAS depuis Claude Code
- ✅ 2026-04-16 : source_amorcage Swift — reprobe relay si topologie vide
- ✅ 2026-04-16 : auto-reconnect + liveness log
- 🏆 **2026-06-08 : JALON — nœud iOS en 5G cross-réseau** rejoint le réseau ToM **décentralisé** (Pkarr/n0/DHT/IPv6, zéro relais à IP fixe). iPhone 5G (hors-LAN, CGNAT opérateur) ↔ NAS (derrière Freebox) connectés en ~1min30, 0 échec. NAS ajouté comme **nœud unifié** (`tom-node.service`, role Peer — ADR-006). Lien actuel via **fallback relais** (RTT 1856ms) → reste à obtenir le DIRECT (ouvrir IPv6 entrante Freebox + instrumenter `path_kind`).

### Infrastructure
- ✅ NAS relay opérationnel : `tom-relay --dev` port 3340 (local + public `82.67.95.8:3340`)
- ✅ mDNS local discovery activé par défaut
- ✅ `tom-gateway` : CLI auto-config Freebox (crate 0.2.0)

## Sur le feu

### tvOS Node — convergence code ↔ doc ↔ tests
- [x] **Architecture Swift tranchée** (2026-06-07) : on garde le wrapper local `TomNodeWrapper`/`TomNodeService`. `TomCoreKit` abandonné (ne débloque rien). Cf. 20-decisions.
- [x] **Premier filet de sécurité contrat FFI** (2026-06-07) : `tom_node_status` passé sur serde (`NodeStatusFFI`) + 4 tests de contrat Rust verrouillant les clés JSON décodées par Swift (status, peer, message). `cargo clippy`+tests verts.
- [ ] **Rebuild xcframework** (`make ffi` + `make ffi-device`) pour embarquer la sérialisation durcie dans l'app (sortie byte-identique pour valeurs normales → pas de régression).
- [ ] **Mettre `docs/TOM-TVOS-NODE-PLAN.md` à jour** pour refléter l'état réel (app Xcode + FFI + tabs SwiftUI déjà en place)
- [ ] **Tests Swift/tvOS** : nécessite de câbler une cible de test dans le `.xcodeproj` (absente aujourd'hui) — XCTest sur decode `TomModels` (mêmes fixtures que les tests Rust) + lifecycle Service
- [ ] **Durcir la couche tvOS** : flux messages/groupes, persistance, handling reprise après veille, packaging/release

### Dette review
- Handoff Copilot dû : 4 commits · +88 lignes · 12j (signalé hook 2026-05-22)
- Validation globale encore floue côté JS : `runTests` déclenche l'e2e Playwright et échoue si le port 5173 est déjà pris

## Ensuite

### Phase 3 — Convergence TS+Rust (README.md)
- Protocol convergence : stack TypeScript + Rust unifiées (détails non trouvés dans docs scannés)

### Améliorations infra
- Push public relay (`82.67.95.8:3340`) — UDP 3340 déjà forwardé en Freebox
- Clarifier la place de `tom-relay-ffi` (embedding Apple TV / mobile / démo embarquée ?) avant d'ouvrir un nouveau front

## Parking

- `tom-relay-ffi` : crate existante, usage non documenté — probablement pour embedding relay dans app mobile
- `apps/infra-web-client` : client web infra (non exploré)
- Articles Medium présents dans `docs/` — publication potentielle
- `tom-whitepaper-v1.md` — whitepaper existant
