# Décisions projet

> ⛔ **RÈGLE 1 — ANTI-HALLUCINATION ABSOLUE**
> Une décision non vérifiée n'est pas une décision. Pas d'entrée sans source factuelle.

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## Décisions durables

| Date | Contexte | Décision | Conséquence | Source |
|------|----------|----------|-------------|--------|
| Phase R1 | Transport | **QUIC via relay** (ADR-001) — remplace WebRTC | Relay obligatoire, pas optionnel. MagicSock + hole punching pour upgrade direct | CLAUDE.md ADR-001 |
| Phase R7 | Dépendances | **Fork complet iroh** sous namespace `tom-*` (MIT) | 7 crates forkées, `iroh-quinn-udp` intentionnellement non forké (type compat) | CLAUDE.md §Fork Status |
| Phase R7 | Bootstrap | **Élimination signaling server WebSocket** — remplacé par own relay + Pkarr/DNS | `TOM_RELAY_URL` env var, flag `n0_discovery(bool)` | CLAUDE.md ADR-002 |
| Phase R1 | Wire format | **MessagePack** (rmp-serde) pour enveloppes | `signing_bytes()` exclut `ttl` (muté par relays) | CLAUDE.md ADR-003 |
| Phase R1 | Crypto | **Ed25519 sign + X25519 DH + XChaCha20-Poly1305 + HKDF-SHA256** | `encrypt_and_sign()` = encrypt-then-sign. `ed25519-dalek` épinglé `=3.0.0-pre.1` | CLAUDE.md ADR-004 + note LLM |
| Phase R1 | Identité | **Ed25519 keypair = node identity** — clé publique = adresse réseau | NodeId est la clé publique, pas d'annuaire central | CLAUDE.md ADR-005 |
| Phase R2 | Architecture | **Unified node model** — tout nœud = client + relay potentiel | Rôle assigné par topologie, pas par config | CLAUDE.md ADR-006 |
| Phase R4 | Backup | **Virus metaphor** — messages se répliquent sur nœuds backup, TTL 24h | Purge globale TTL, auto-delete à la livraison | CLAUDE.md ADR-009 |
| Phase R5 | Groupes | **Hub-and-spoke** — hub déterministe (lowest NodeId), failover Primary→Shadow→Candidate | ~3-6s détection failover. Sender key encryption, rotation sur leave | CLAUDE.md §GroupManager |
| Phase R7 | Cargo | **Alias trick** : `quinn = { package = "tom-quinn" }` | Code source inchangé, namespace transparent pour consommateurs | CLAUDE.md §Cargo Alias |
| Phase tvOS | FFI | **xcframework multi-plateforme** (device + simulator) via `tom-protocol-ffi` | `libtom_protocol_ffi.a` + header C + module map Swift | TOM-TVOS-NODE-PLAN.md + build/ existant |
| Phase tvOS | UI | **SwiftUI + MVVM** pour app tvOS | Workflow VSCode 80-95% + Xcode pour signing/device | TOM-TVOS-NODE-PLAN.md |
| 2026-06-07 | Archi Swift tvOS | **Garder le wrapper local** (`TomNodeWrapper` actor + `TomNodeService`) — **ne PAS introduire `TomCoreKit`** | `TomCoreKit` ne débloque aucun problème immédiat : le FFI est déjà entièrement câblé et fonctionnel via le wrapper. Pas de nouvelle archi. | Audit code réel `apps/tom-node-tvos/TomNode/{Models,Services}` |
| 2026-06-07 | Durcissement FFI | **`tom_node_status` sérialisé via serde** (`NodeStatusFFI`) au lieu d'un `format!` manuel | JSON toujours valide/échappé + contrat de clés verrouillé par tests Rust ; supprime le risque de corruption JSON → decode Swift nil → panneau figé | `crates/tom-protocol-ffi/src/{lib,types}.rs` |
| 2026-07-12 | Présence appareil faible (§5 ADR-011, L1-003) | **Quorum ≥N témoins distincts requis pour promotion Online** (N dynamique 2-4, plancher dur 2) ; un témoin unique = `Known` au mieux, jamais `Online` | Ferme l'eclipse par relais unique (kill-shot #3 red-team Fable) — un relais Sybil seul ne peut plus fabriquer un faux `Online` ; appareil faible dégrade sciemment en `Known` plutôt que risquer un faux positif | `docs/plans/L1-003-vue-signee-relais.md` §4, commit `094ec96`, `crates/tom-protocol/src/presence/quorum.rs` |
| 2026-07-12 | Sécurité git | **Gate mécanique email d'auteur gmail** — tout push avec auteur/committer ≠ `karaoui.malik@gmail.com` (hors comptes de service) est refusé par `pre-push-gate.sh` | Suite à l'incident `hmail` (148 commits non attribués sur GitHub) ; ne plus dépendre de la seule vigilance | `scripts/pre-push-gate.sh`, commit `42142b7` |
| 2026-07-12 | Anti-DoS présence (kill-shot #5 Fable) | **Cap global agrégé `RESPONDER_KNOWN_GLOBAL_BUDGET_PER_WINDOW=60`** pour identités `Known`, symétrique au cap stranger existant (120) | Ferme l'amplification Sybil cold-start : sans ce cap, jusqu'à 512 identités Known × 10 sig/identité = 5120 sig/fenêtre (~43x le cap stranger) était possible ; check exécuté avant toute mutation d'état, review sécu indépendante ratifiée | `crates/tom-protocol/src/presence/mod.rs::allow_response()` |
| 2026-07-12 | Rôles réseau (écart BMAD ↔ code) | **Observer abandonné définitivement, pas recodé** — le two-tier `PeerStatus` (Online/Known/Stale/Offline) couvre déjà la présence incertaine/passive | Vérifié : `observer` n'a JAMAIS eu de logique fonctionnelle même dans le TypeScript Phase 1 original (771 tests) — une seule occurrence dans tout `packages/core/src`, juste la déclaration de type, zéro assignation/comportement/test. Pas un abandon en cours de route, un rôle jamais construit. | `packages/core/src/discovery/network-topology.ts:3` vs `crates/tom-protocol/src/relay.rs:35-40` |

## 7 Décisions fondatrices non-négociables

| # | Décision | Règle |
|---|----------|-------|
| 1 | **Delivery** | Message livré ⟺ ACK du destinataire |
| 2 | **TTL** | 24h max, purge globale, aucune exception |
| 3 | **L1 Role** | L1 ancre l'état, n'arbitre jamais |
| 4 | **Reputation** | Dégradation progressive, pas de bans permanents |
| 5 | **Anti-spam** | "Sprinkler gets sprinkled" — charge progressive |
| 6 | **Invisibility** | Protocole invisible pour l'utilisateur final |
| 7 | **Scope** | Fondation universelle (comme TCP/IP), pas un produit |

Source : `_bmad-output/planning-artifacts/design-decisions.md`
