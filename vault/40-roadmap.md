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
- [x] **Architecture Swift tranchée** (2026-06-07) : on garde le wrapper local `TomNodeWrapper`/`TomNodeService`. `TomCoreKit` abandonné.
- [x] **Premier filet de sécurité contrat FFI** (2026-06-07/09) : `tom_node_status` sur serde (`NodeStatusFFI`) + tests de contrat. **Review Copilot confirmée** : contrat clés correct, zone grise `u64→Int Swift` documentée.
- [x] **Fix commentaire CLI `--bind-port`** (2026-06-09) : "dual-stack IPv6+IPv4" remplacé par la réalité (IPv4 reste éphémère).
- [x] **Review Copilot x3 soldée** (2026-06-09) : handoffs FFI + transport + deps intégrés. Dette §25 effacée.
- [ ] **Push 9 commits** → `git push origin main` (terminal) → surveiller CI GitHub
- [ ] **Rebuild xcframework** (`make ffi && make ffi-device`) pour embarquer serde NodeStatusFFI dans l'app tvOS
- [ ] **Ouvrir IPv6 entrante Freebox** — port 43925 → `2a01:e0a:14f:5da0:248f:5dff:fea5:8ed1` (Freebox OS, règle pare-feu manuelle). Débloque connexion DIRECT QUIC NAS.
- [ ] **Mettre `docs/TOM-TVOS-NODE-PLAN.md` à jour** — refléter l'état réel
- [ ] **Tests Swift/tvOS** — câbler XCTest dans `.xcodeproj` (fixtures identiques aux tests Rust)
- [ ] **Durcir couche tvOS** : messages/groupes, persistance, reprise après veille

### Chantier macOS (5 lots — prêt à démarrer)

> Spec complète : `docs/superpowers/specs/2026-06-08-app-macos-tom-design.md`

- [ ] **Lot A** — Slice Rust FFI `aarch64-apple-darwin` + mise à jour `build-ffi.sh`
- [ ] **Lot B** — Cible Xcode macOS (1 passage Xcode obligatoire — signing + entitlements)
- [ ] **Lot C** — Portage `TomNodeService.swift` (UIKit conditionnel → `ProcessInfo.beginActivity` anti-veille)
- [ ] **Lot D** — App Sandbox entitlements réseau (`network.client` + `network.server`)
- [ ] **Lot E** — Makefile cibles `ffi-macos`, `build-macos`

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
