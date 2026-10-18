---
date: 2026-10-18
mandat: M0016
tags: [genese, vision, inventaire]
base: origin/main aaeee846a1c6ed42d1f9d0f6e4f132cd7cc97473
---

# Genèse de ToM — annexe 0 : inventaire daté des sources de vision

> Annexe de `vault/decisions/2026-10-18-GENESE-vision.md`.
> Périmètre : les deux sources fondatrices (hors dépôt) + tous les `.md` suivis par git sur `origin/main`, sauf `_bmad/`, `.claude/`, `.github/skills/` et `patches/` (outillage). 264 fichiers du dépôt + 2 sources fondatrices.
> Dates : `git log --follow` (date d'auteur ; l'histoire a été réécrite le 2026-07-12, les dates d'auteur sont conservées, les SHA d'avant cette date ont changé). « Commits » = nombre de commits qui touchent le fichier.
> Statut : classement de lecture fait pour ce mandat. « en vigueur » = la source sert encore de référence ; « remplacée » = une source plus récente la contredit, nommée ; « historique » = vraie à sa date, dépassée par le code ou un chantier clos ; « outillage » = sans contenu de vision.
> Aucune IP, aucun nom d'hôte, aucun secret dans ce tableau.

## Repères pour lire le tableau

- **Deux sources fondatrices**, toutes deux du 2026-01-30 : le livre blanc v0.1 (FR, 138 lignes : principes, sous-réseaux, économie d'utilité) et « Step 2 » (393 lignes : L1 purgée, genèse glissante, rôles PoP, Usage/Contribution, arroseur arrosé, double dépense par observateurs). Le PDF de « Step 2 » a été imprimé depuis iOS le 2026-01-30 à 08:06 (métadonnées du PDF).
- **Le livre blanc v1** (`docs/tom-whitepaper-v1.md`) existait dès le 2026-01-30 : le brief produit de ce jour le cite en entrée (`product-brief-tom-protocol-2026-01-30.md:4-5`). Il entre dans git le 2026-02-02 (`2cbf495`) et n'a plus changé (le commit du 2026-03-06 n'est qu'un déplacement, similarité 100 %).
- **Les 7 verrous** (`_bmad-output/planning-artifacts/design-decisions.md`) datent du 2026-02-06 (`9048088`).
- **Le texte qui fait foi aujourd'hui** est `docs/MISSION.md` (créé 2026-07-01, mis à jour 2026-10-17).

## Tableau

| # | Chemin | 1re date git | Dernière date git | Commits | Lignes | Nature | Statut |
|---|---|---|---|---|---|---|---|
| F1 | `sources-fondateur/whitepaper-v0.1-FR.txt` (hors dépôt) | 2026-01-30 (date du PDF, fournie par le mandat) | — | — | 138 | **fondateur** (livre blanc v0.1) | fondatrice — principes repris, détails remplacés |
| F2 | `sources-fondateur/Step2-TheOpenMessaging.txt` (hors dépôt) | 2026-01-30 (PDF imprimé 08:06 CET, métadonnées) | — | — | 393 | **fondateur** (« Step 2 ») | fondatrice — L1/double dépense non construits |
| 1 | `README.md` | 2026-02-02 | 2026-07-16 | 22 | 392 | doc de référence | en vigueur (non réaudité) |
| 2 | `_bmad-output/implementation-artifacts/1-1-monorepo-scaffold.md` | 2026-02-02 | 2026-02-02 | 1 | 243 | story (phase TS) | historique (pile TS remplacée) |
| 3 | `_bmad-output/implementation-artifacts/1-2-node-identity-generation-and-persistence.md` | 2026-02-02 | 2026-02-02 | 1 | 189 | story (phase TS) | historique (pile TS remplacée) |
| 4 | `_bmad-output/implementation-artifacts/1-3-shared-types-and-error-foundation.md` | 2026-02-02 | 2026-02-02 | 1 | 89 | story (phase TS) | historique (pile TS remplacée) |
| 5 | `_bmad-output/implementation-artifacts/3-1-peer-discovery-protocol.md` | 2026-02-02 | 2026-02-02 | 1 | 169 | story (phase TS) | historique (pile TS remplacée) |
| 6 | `_bmad-output/planning-artifacts/architecture.md` | 2026-02-02 | 2026-02-02 | 1 | 687 | cadrage (ADR-001…009) | partiellement remplacée (ADR repris dans CLAUDE.md) |
| 7 | `_bmad-output/planning-artifacts/epics.md` | 2026-02-02 | 2026-02-06 | 3 | 1119 | cadrage (épopées TS) | historique (phase 1 TS livrée) |
| 8 | `_bmad-output/planning-artifacts/implementation-readiness-report-2026-02-02.md` | 2026-02-02 | 2026-02-02 | 1 | 339 | cadrage BMAD | historique |
| 9 | `_bmad-output/planning-artifacts/prd-validation-report.md` | 2026-02-02 | 2026-02-02 | 1 | 427 | cadrage BMAD | historique |
| 10 | `_bmad-output/planning-artifacts/prd.md` | 2026-02-02 | 2026-02-02 | 1 | 438 | cadrage (PRD) | remplacée pour la pile (TS/WebRTC → Rust/QUIC, 2026-02-18) |
| 11 | `_bmad-output/planning-artifacts/product-brief-tom-protocol-2026-01-30.md` | 2026-02-02 | 2026-02-02 | 1 | 204 | cadrage fondateur (brief) | en vigueur via MISSION (source citée MISSION.md:7) |
| 12 | `docs/tom-whitepaper-v1.md` | 2026-02-02 | 2026-03-06 | 2 | 479 | livre blanc | en vigueur sauf conflit avec MISSION (MISSION.md:5) |
| 13 | `_bmad-output/implementation-artifacts/3-2-dynamic-role-assignment.md` | 2026-02-03 | 2026-02-03 | 1 | 191 | story (phase TS) | historique (pile TS remplacée) |
| 14 | `_bmad-output/implementation-artifacts/3-3-automatic-relay-selection.md` | 2026-02-03 | 2026-02-03 | 2 | 166 | story (phase TS) | historique (pile TS remplacée) |
| 15 | `_bmad-output/implementation-artifacts/3-4-dual-role-node.md` | 2026-02-03 | 2026-02-03 | 1 | 112 | story (phase TS) | historique (pile TS remplacée) |
| 16 | `_bmad-output/implementation-artifacts/3-5-bootstrap-participation-vectors.md` | 2026-02-04 | 2026-02-04 | 1 | 127 | story (phase TS) | historique (pile TS remplacée) |
| 17 | `_bmad-output/implementation-artifacts/4-1-direct-path-establishment.md` | 2026-02-04 | 2026-02-04 | 1 | 188 | story (phase TS) | historique (pile TS remplacée) |
| 18 | `_bmad-output/implementation-artifacts/4-2-delivery-confirmation-and-read-receipts.md` | 2026-02-04 | 2026-02-04 | 1 | 228 | story (phase TS) | historique (pile TS remplacée) |
| 19 | `_bmad-output/implementation-artifacts/4-3-message-path-visualization.md` | 2026-02-04 | 2026-02-04 | 1 | 138 | story (phase TS) | historique (pile TS remplacée) |
| 20 | `_bmad-output/implementation-artifacts/4-4-reconnection-and-pending-message-delivery.md` | 2026-02-04 | 2026-02-04 | 1 | 307 | story (phase TS) | historique (pile TS remplacée) |
| 21 | `_bmad-output/implementation-artifacts/4-5-demo-snake-multiplayer-p2p-game.md` | 2026-02-04 | 2026-02-06 | 2 | 299 | story (phase TS) | historique (pile TS remplacée) |
| 22 | `_bmad-output/implementation-artifacts/group-chat-code-review-prompt.md` | 2026-02-04 | 2026-02-04 | 1 | 382 | story (phase TS) | historique (pile TS remplacée) |
| 23 | `tools/signaling-server/README.md` | 2026-02-04 | 2026-02-04 | 1 | 74 | outil | **abandonnée** (DEPRECATED R7) |
| 24 | `CHANGELOG.md` | 2026-02-05 | 2026-07-15 | 6 | 759 | journal | journal (jusqu'au 2026-07-15) |
| 25 | `.github/ISSUE_BACKLOG.md` | 2026-02-06 | 2026-02-06 | 1 | 307 | doc | — |
| 26 | `CLAUDE.md` | 2026-02-06 | 2026-10-17 | 31 | 561 | doc de référence (ADR, verrous, limites) | en vigueur ; ADR-002 « DONE » contesté (ADN:77) |
| 27 | `CONTRIBUTING.md` | 2026-02-06 | 2026-10-17 | 3 | 222 | doc | — |
| 28 | `_bmad-output/implementation-artifacts/gpt-review-prompt-story-4-5.md` | 2026-02-06 | 2026-02-06 | 1 | 107 | story (phase TS) | historique (pile TS remplacée) |
| 29 | `_bmad-output/planning-artifacts/design-decisions.md` | 2026-02-06 | 2026-02-06 | 1 | 272 | décision (7 verrous LOCKED) | **en vigueur** (repris CLAUDE.md, MISSION §6) |
| 30 | `tools/mcp-server/README.md` | 2026-02-06 | 2026-02-06 | 1 | 270 | outil / déploiement / résultat | historique |
| 31 | `tools/vscode-extension/README.md` | 2026-02-06 | 2026-02-06 | 1 | 94 | outil / déploiement / résultat | historique |
| 32 | `_bmad-output/implementation-artifacts/epic-4-8-retro-2026-02-07.md` | 2026-02-07 | 2026-02-07 | 2 | 208 | story (phase TS) | historique (pile TS remplacée) |
| 33 | `scripts/tests-added-log.md` | 2026-02-07 | 2026-02-08 | 6 | 354 | outillage harnais | hors vision |
| 34 | `docs/gitbook/README.md` | 2026-02-08 | 2026-02-08 | 5 | 53 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 35 | `docs/gitbook/SUMMARY.md` | 2026-02-08 | 2026-02-08 | 5 | 34 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 36 | `docs/gitbook/architecture.md` | 2026-02-08 | 2026-02-08 | 5 | 51 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 37 | `docs/gitbook/changelog.md` | 2026-02-08 | 2026-02-08 | 2 | 10 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 38 | `docs/gitbook/concepts.md` | 2026-02-08 | 2026-02-08 | 3 | 44 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 39 | `docs/gitbook/contributing.md` | 2026-02-08 | 2026-02-08 | 2 | 25 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 40 | `docs/gitbook/design-decisions.md` | 2026-02-08 | 2026-02-08 | 2 | 25 | décision (miroir des 7 verrous) | en vigueur (copie résumée) |
| 41 | `docs/gitbook/getting-started.md` | 2026-02-08 | 2026-02-08 | 3 | 45 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 42 | `docs/gitbook/issues-backlog.md` | 2026-02-08 | 2026-02-08 | 2 | 32 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 43 | `docs/gitbook/routage-reroute.md` | 2026-02-08 | 2026-02-08 | 1 | 121 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 44 | `docs/gitbook/sdk-tomclient.md` | 2026-02-08 | 2026-02-08 | 1 | 124 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 45 | `docs/gitbook/securite-modele-menace.md` | 2026-02-08 | 2026-02-08 | 1 | 88 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 46 | `docs/gitbook/signaling-server.md` | 2026-02-08 | 2026-02-08 | 2 | 117 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 47 | `docs/gitbook/spec-groupes.md` | 2026-02-08 | 2026-02-08 | 1 | 148 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 48 | `docs/gitbook/spec-technique.md` | 2026-02-08 | 2026-02-08 | 1 | 137 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 49 | `docs/gitbook/state-machine-delivery.md` | 2026-02-08 | 2026-02-08 | 1 | 120 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 50 | `docs/gitbook/troubleshooting-webrtc-lan-safari.md` | 2026-02-08 | 2026-02-08 | 1 | 124 | doc publique (phase TS) | historique (pile TS/WebRTC) |
| 51 | `docs/gitbook/whitepaper-v1.md` | 2026-02-08 | 2026-02-08 | 2 | 11 | livre blanc (fiche) | renvoi vers v1 |
| 52 | `docs/gitbook/whitepaper-v2.md` | 2026-02-08 | 2026-02-08 | 2 | 29 | livre blanc (plan) | brouillon jamais rédigé |
| 53 | `experiments/iroh-poc/README.md` | 2026-02-09 | 2026-02-12 | 8 | 238 | outil / déploiement / résultat | historique |
| 54 | `experiments/iroh-poc/FORK-ARCHITECTURE.md` | 2026-02-11 | 2026-02-11 | 1 | 321 | PoC | historique (fork réalisé R7) |
| 55 | `experiments/iroh-poc/V2-TEST-CAMPAIGN.md` | 2026-02-11 | 2026-02-11 | 1 | 223 | outil / déploiement / résultat | historique |
| 56 | `_bmad-output/implementation-artifacts/rust-port-kickoff-2026-02-18.md` | 2026-02-18 | 2026-02-18 | 1 | 168 | décision (bascule Rust) | **en vigueur** (pivot TS → Rust) |
| 57 | `docs/archive/AUDIT-2026-02-19.md` | 2026-02-22 | 2026-06-22 | 2 | 392 | revue / rapport archivé | archivé (historique) |
| 58 | `docs/design-runtime-state.md` | 2026-02-23 | 2026-02-23 | 1 | 520 | doc technique | historique ou opérationnel |
| 59 | `docs/plans/2026-02-22-group-e2e-encryption-design.md` | 2026-02-23 | 2026-02-23 | 1 | 280 | plan | historique (chantier clos ou livré) |
| 60 | `docs/plans/2026-02-22-group-e2e-encryption-plan.md` | 2026-02-23 | 2026-02-23 | 1 | 1627 | plan | historique (chantier clos ou livré) |
| 61 | `docs/plans/2026-02-22-runtime-state-refactoring.md` | 2026-02-23 | 2026-02-23 | 1 | 1622 | plan | historique (chantier clos ou livré) |
| 62 | `docs/article-ToM protocol-en.md` | 2026-02-24 | 2026-10-17 | 3 | 1055 | article (traduction) | idem (traduction) |
| 63 | `docs/article-ToM protocol.md` | 2026-02-24 | 2026-10-17 | 3 | 1055 | article | en vigueur comme récit (non normatif) |
| 64 | `docs/medium-article-poc.md` | 2026-02-24 | 2026-03-06 | 2 | 226 | doc technique | historique ou opérationnel |
| 65 | `docs/medium-article-v2.md` | 2026-02-24 | 2026-03-06 | 2 | 186 | doc technique | historique ou opérationnel |
| 66 | `docs/medium-article.md` | 2026-02-24 | 2026-03-06 | 2 | 226 | doc technique | historique ou opérationnel |
| 67 | `docs/plans/2026-02-22-hub-failover-design.md` | 2026-02-24 | 2026-02-24 | 1 | 191 | plan | historique (chantier clos ou livré) |
| 68 | `docs/plans/2026-02-22-hub-failover-plan.md` | 2026-02-24 | 2026-02-24 | 1 | 1105 | plan | historique (chantier clos ou livré) |
| 69 | `docs/plans/2026-02-24-dynamic-roles-bandwidth-design.md` | 2026-02-24 | 2026-02-24 | 1 | 287 | plan | historique (chantier clos ou livré) |
| 70 | `docs/plans/2026-02-24-dynamic-roles-bandwidth-plan.md` | 2026-02-24 | 2026-02-24 | 1 | 1268 | plan | historique (chantier clos ou livré) |
| 71 | `docs/plans/2026-02-24-stress-campaign-design.md` | 2026-02-24 | 2026-02-24 | 1 | 76 | plan | historique (chantier clos ou livré) |
| 72 | `results/CAMPAIGN-REPORT.md` | 2026-02-24 | 2026-02-24 | 1 | 271 | outil / déploiement / résultat | historique |
| 73 | `results/terminal result - 16.02. #1.md` | 2026-02-24 | 2026-02-24 | 1 | 1789 | outil / déploiement / résultat | historique |
| 74 | `docs/plans/2026-02-26-discovery-handler-design.md` | 2026-02-26 | 2026-02-26 | 1 | 274 | plan | historique (chantier clos ou livré) |
| 75 | `docs/plans/2026-02-26-discovery-handler-plan.md` | 2026-02-26 | 2026-02-26 | 1 | 872 | plan | historique (chantier clos ou livré) |
| 76 | `docs/FORK-GOVERNANCE.md` | 2026-02-27 | 2026-03-02 | 2 | 244 | gouvernance du fork | **abandonnée** sur la compatibilité wire iroh (MISSION §1.8) |
| 77 | `docs/archive/CI-RETEX-LOCALHOST-TESTS-2026-02-27.md` | 2026-02-27 | 2026-06-22 | 2 | 132 | revue / rapport archivé | archivé (historique) |
| 78 | `docs/archive/REVIEW-R7.3-FORK-COPILOT.md` | 2026-02-27 | 2026-06-22 | 3 | 596 | revue / rapport archivé | archivé (historique) |
| 79 | `docs/archive/REVIEW-R7.3-FORK.md` | 2026-02-27 | 2026-06-22 | 2 | 346 | revue / rapport archivé | archivé (historique) |
| 80 | `docs/plans/2026-02-26-fork-api-boundaries.md` | 2026-02-27 | 2026-02-27 | 1 | 278 | plan | historique (chantier clos ou livré) |
| 81 | `docs/plans/2026-02-26-phase-r7-dht-bootstrap-elimination-design.md` | 2026-02-27 | 2026-02-27 | 1 | 294 | plan | livré (R7) ; n0 encore actif par défaut (ADN:30) |
| 82 | `docs/plans/2026-02-26-phase-r7-dht-bootstrap-elimination-plan.md` | 2026-02-27 | 2026-02-27 | 1 | 1237 | plan | historique (chantier clos ou livré) |
| 83 | `docs/archive/REPORT-R14-TRANSITION-KEYS-2026-03-04.md` | 2026-03-04 | 2026-06-22 | 2 | 197 | revue / rapport archivé | archivé (historique) |
| 84 | `docs/archive/freebox-bug-report-arp-wifi-vm.md` | 2026-03-04 | 2026-10-17 | 3 | 89 | revue / rapport archivé | archivé (historique) |
| 85 | `docs/archive/rapport-diagnostic-ssh-wan-2026-03-02.md` | 2026-03-04 | 2026-10-17 | 3 | 149 | revue / rapport archivé | archivé (historique) |
| 86 | `docs/plans/R13-offline-delivery.md` | 2026-03-04 | 2026-03-04 | 1 | 247 | plan | historique (chantier clos ou livré) |
| 87 | `deploy/macos/usb-mac-air-relay/README.md` | 2026-03-05 | 2026-03-05 | 1 | 65 | outil / déploiement / résultat | historique |
| 88 | `docs/INFRA-LOCAL-DEPLOYMENT-GUIDE.md` | 2026-03-05 | 2026-10-17 | 3 | 1829 | guide infra | historique (option C) |
| 89 | `docs/archive/RECAP-DEV-2026-03-05.md` | 2026-03-05 | 2026-10-17 | 3 | 133 | revue / rapport archivé | archivé (historique) |
| 90 | `docs/archive/TEST-STRATEGY-2026-03-05.md` | 2026-03-05 | 2026-06-22 | 2 | 131 | revue / rapport archivé | archivé (historique) |
| 91 | `docs/clients/infra-web-client.md` | 2026-03-05 | 2026-03-05 | 1 | 57 | doc technique | historique ou opérationnel |
| 92 | `docs/monitoring/option-c-monitoring.md` | 2026-03-05 | 2026-03-05 | 2 | 69 | doc technique | historique ou opérationnel |
| 93 | `docs/plans/2026-03-04-c1-1-mac-freebox-test-protocol.md` | 2026-03-05 | 2026-03-05 | 1 | 105 | plan | historique (chantier clos ou livré) |
| 94 | `docs/plans/2026-03-04-option-c-infra-execution-plan.md` | 2026-03-05 | 2026-03-05 | 1 | 324 | plan | historique (chantier clos ou livré) |
| 95 | `docs/relay-discovery.md` | 2026-03-05 | 2026-03-05 | 2 | 52 | plan | abandonnée (relais publics codés en dur) |
| 96 | `tools/relay-discovery/README.md` | 2026-03-05 | 2026-03-05 | 5 | 88 | outil / déploiement / résultat | historique |
| 97 | `apps/tom-node-tvos/CLAUDE.md` | 2026-03-06 | 2026-07-12 | 3 | 93 | doc de code | en vigueur |
| 98 | `docs/Brief Infrastructure Option C for GPT 5.3 CODEX.md` | 2026-03-06 | 2026-03-06 | 1 | 473 | plan infra | abandonnée (flotte de relais VPS) — contraire à MISSION §1.1 |
| 99 | `docs/TOM-TVOS-NODE-PLAN.md` | 2026-03-06 | 2026-07-12 | 2 | 64 | doc technique | historique ou opérationnel |
| 100 | `docs/ios_vscode_claude_workflow_v4.md` | 2026-03-06 | 2026-03-06 | 1 | 742 | doc technique | historique ou opérationnel |
| 101 | `docs/archive/REVIEW-PEER-DISCOVERY-2026-03-07.md` | 2026-03-07 | 2026-06-22 | 2 | 1146 | revue / rapport archivé | archivé (historique) |
| 102 | `docs/archive/REVIEW-PEER-DISCOVERY-V2-2026-03-07.md` | 2026-03-07 | 2026-06-22 | 2 | 446 | revue / rapport archivé | archivé (historique) |
| 103 | `docs/archive/REVIEW-PEER-DISCOVERY-V3-2026-03-07.md` | 2026-03-07 | 2026-06-22 | 2 | 406 | revue / rapport archivé | archivé (historique) |
| 104 | `docs/archive/DIRECTIVES-CLAUDE-RELAY-EMBEDE-2026-03-20.md` | 2026-03-20 | 2026-06-22 | 2 | 335 | revue / rapport archivé | archivé (historique) |
| 105 | `docs/archive/VALIDATION-DESIGN-RELAY-EMBEDE-2026-03-20.md` | 2026-03-20 | 2026-06-22 | 2 | 404 | revue / rapport archivé | archivé (historique) |
| 106 | `docs/plans/2026-03-20-embedded-relay-design.md` | 2026-03-20 | 2026-03-20 | 1 | 126 | plan | historique (chantier clos ou livré) |
| 107 | `docs/plans/2026-03-20-relay-registry-design.md` | 2026-03-20 | 2026-03-20 | 1 | 754 | plan | historique (chantier clos ou livré) |
| 108 | `docs/plans/2026-03-22-sprint-1-resultats.md` | 2026-03-22 | 2026-03-22 | 1 | 162 | plan | historique (chantier clos ou livré) |
| 109 | `docs/plans/2026-03-22-sprint-1-wow-moment-honnete.md` | 2026-03-22 | 2026-03-22 | 1 | 288 | plan | historique (chantier clos ou livré) |
| 110 | `docs/plans/s1-runbook.md` | 2026-03-22 | 2026-03-22 | 1 | 107 | plan | historique (chantier clos ou livré) |
| 111 | `docs/archive/PROOF-CROSS-MACHINE-2026-03-27.md` | 2026-03-27 | 2026-10-17 | 3 | 192 | revue / rapport archivé | archivé (historique) |
| 112 | `docs/plans/2026-03-27-cross-machine-proof-lock.md` | 2026-03-27 | 2026-03-27 | 1 | 179 | plan | historique (chantier clos ou livré) |
| 113 | `docs/ADR-M2-reduce-to-3-commands.md` | 2026-03-30 | 2026-03-30 | 1 | 112 | ADR | remplacée (point d'entrée fixe → amorçage tournant, MISSION:34) |
| 114 | `docs/archive/METHODOLOGY-CROSS-CRATE-VALIDATION-2026-03-07.md` | 2026-03-30 | 2026-06-22 | 2 | 408 | revue / rapport archivé | archivé (historique) |
| 115 | `docs/archive/METHODOLOGY-CROSS-CRATE-VALIDATION-SHORT-2026-03-07.md` | 2026-03-30 | 2026-06-22 | 2 | 73 | revue / rapport archivé | archivé (historique) |
| 116 | `docs/archive/PEERPRESENT-REFERENCE-AND-NEXT-CAMPAIGN-2026-03-09.md` | 2026-03-30 | 2026-06-22 | 2 | 345 | revue / rapport archivé | archivé (historique) |
| 117 | `docs/archive/RCA-PEERPRESENT-RACE-2026-03-30.md` | 2026-03-30 | 2026-10-17 | 4 | 35 | revue / rapport archivé | archivé (historique) |
| 118 | `docs/archive/RECAP-DECOUVERTE-2026-03-20.md` | 2026-03-30 | 2026-06-22 | 2 | 248 | revue / rapport archivé | archivé (historique) |
| 119 | `docs/archive/RECAP-METHODOLOGY-EXCHANGE-2026-03-07.md` | 2026-03-30 | 2026-06-22 | 2 | 422 | revue / rapport archivé | archivé (historique) |
| 120 | `docs/archive/REVIEW-PEERPRESENT-IMPLEMENTATION-2026-03-07.md` | 2026-03-30 | 2026-06-22 | 2 | 345 | revue / rapport archivé | archivé (historique) |
| 121 | `docs/plans/2026-03-21-macro-roadmap-realignment.md` | 2026-03-30 | 2026-03-30 | 2 | 484 | plan | historique (chantier clos ou livré) |
| 122 | `docs/plans/2026-03-21-transport-relay-discovery.md` | 2026-03-30 | 2026-03-30 | 1 | 385 | plan | historique (chantier clos ou livré) |
| 123 | `docs/plans/2026-03-30-apple-tv-3rd-node.md` | 2026-03-30 | 2026-10-17 | 3 | 181 | plan | historique (chantier clos ou livré) |
| 124 | `docs/plans/2026-03-30-bootstrap-pump-module.md` | 2026-03-30 | 2026-03-30 | 1 | 215 | plan | en vigueur comme cadre (MISSION:34) |
| 125 | `docs/plans/2026-03-30-keep-the-cap.md` | 2026-03-30 | 2026-03-30 | 1 | 32 | plan | historique |
| 126 | `docs/plans/2026-03-30-organic-seed-handoff-test.md` | 2026-03-30 | 2026-03-30 | 1 | 244 | plan | historique (chantier clos ou livré) |
| 127 | `docs/plans/2026-03-30-rattrapage-bmad-claude-chef-de-projet.md` | 2026-03-30 | 2026-03-30 | 1 | 329 | plan | historique (chantier clos ou livré) |
| 128 | `deploy/peerpresent-k/README.md` | 2026-03-31 | 2026-03-31 | 1 | 33 | outil / déploiement / résultat | historique |
| 129 | `docs/plans/2026-03-30-bootstrap-pump-skeleton.md` | 2026-03-31 | 2026-03-31 | 1 | 360 | plan | historique (chantier clos ou livré) |
| 130 | `docs/plans/2026-03-30-claude-bootstrap-pump-mission.md` | 2026-03-31 | 2026-03-31 | 1 | 125 | plan | historique (chantier clos ou livré) |
| 131 | `docs/plans/2026-03-30-parallel-copilot-claude-worksplit.md` | 2026-03-31 | 2026-03-31 | 1 | 100 | plan | historique (chantier clos ou livré) |
| 132 | `docs/plans/2026-03-30-peerpresent-k-measurement.md` | 2026-03-31 | 2026-03-31 | 1 | 160 | plan | historique (chantier clos ou livré) |
| 133 | `docs/plans/2026-03-31-checkpoint-macos-update.md` | 2026-03-31 | 2026-03-31 | 1 | 144 | plan | historique (chantier clos ou livré) |
| 134 | `docs/plans/2026-03-31-plan-maitre-reseau-vivant-chaos.md` | 2026-04-01 | 2026-04-01 | 1 | 564 | plan | historique (chantier clos ou livré) |
| 135 | `docs/plans/2026-03-31-runbook-campagne-multi-device.md` | 2026-04-01 | 2026-04-01 | 1 | 331 | plan | historique (chantier clos ou livré) |
| 136 | `docs/plans/2026-04-01-plan-maitre-reseau-vivant-v2.md` | 2026-04-01 | 2026-04-01 | 1 | 372 | plan | historique (chantier clos ou livré) |
| 137 | `docs/essai-liberte-durov-2026.md` | 2026-04-14 | 2026-04-14 | 1 | 143 | essai | texte d'opinion |
| 138 | `docs/freebox-sdk-workflow.md` | 2026-04-14 | 2026-10-17 | 2 | 375 | doc technique | historique ou opérationnel |
| 139 | `docs/workflow-vscode-xcode.md` | 2026-04-14 | 2026-04-14 | 1 | 70 | doc technique | historique ou opérationnel |
| 140 | `docs/AUDIT-PROTOCOLE.md` | 2026-06-06 | 2026-06-06 | 2 | 144 | doc technique | historique ou opérationnel |
| 141 | `docs/DEPLOY-APPLE.md` | 2026-06-06 | 2026-10-17 | 3 | 243 | doc technique | historique ou opérationnel |
| 142 | `docs/superpowers/specs/2026-06-08-app-macos-tom-design.md` | 2026-06-08 | 2026-06-08 | 1 | 179 | doc technique | historique ou opérationnel |
| 143 | `vault/00-brief.md` | 2026-06-08 | 2026-06-09 | 3 | 43 | brief vault | **périmé** (ADN:77) |
| 144 | `vault/10-mailbox.md` | 2026-06-08 | 2026-10-17 | 6 | 109 | vault | journal |
| 145 | `vault/20-decisions.md` | 2026-06-08 | 2026-10-17 | 6 | 47 | registre de décisions | **en vigueur** (registre) |
| 146 | `vault/30-discoveries.md` | 2026-06-08 | 2026-10-18 | 57 | 146 | journal | journal (contient des IP — ne pas recopier) |
| 147 | `vault/40-roadmap.md` | 2026-06-08 | 2026-10-17 | 30 | 324 | feuille de route | en vigueur ; R17 périmé (ADN:27) |
| 148 | `docs/handoffs/2026-06-09-deps-claude-atelier-supply-chain.md` | 2026-06-09 | 2026-06-09 | 2 | 175 | journal (passation) | historique |
| 149 | `docs/handoffs/2026-06-09-ffi-nodestatusffi-contrat-swift.md` | 2026-06-09 | 2026-06-09 | 2 | 152 | journal (passation) | historique |
| 150 | `docs/handoffs/2026-06-09-pr39-apple-audit-ffi.md` | 2026-06-09 | 2026-06-09 | 1 | 80 | journal (passation) | historique |
| 151 | `docs/handoffs/2026-06-09-transport-bind-port-quic-fixe.md` | 2026-06-09 | 2026-06-09 | 2 | 183 | journal (passation) | historique |
| 152 | `crates/tom-sdk/README.md` | 2026-06-10 | 2026-06-10 | 1 | 62 | doc de code | en vigueur |
| 153 | `docs/plans/2026-06-09-dev-metrics-dashboard.md` | 2026-06-10 | 2026-10-17 | 2 | 238 | plan | historique (chantier clos ou livré) |
| 154 | `docs/plans/2026-06-10-audit-global-pre-sdk.md` | 2026-06-10 | 2026-06-10 | 1 | 144 | plan | historique (chantier clos ou livré) |
| 155 | `docs/plans/2026-06-10-chantier-s0-suivi.md` | 2026-06-10 | 2026-06-10 | 1 | 136 | plan | historique (chantier clos ou livré) |
| 156 | `docs/plans/2026-06-10-chantier-s1-suivi.md` | 2026-06-10 | 2026-06-10 | 2 | 96 | plan | historique (chantier clos ou livré) |
| 157 | `docs/plans/2026-06-10-roadmap-sdk.md` | 2026-06-10 | 2026-06-10 | 1 | 170 | plan | historique (chantier clos ou livré) |
| 158 | `docs/handoffs/2026-06-11-review-chantiers-sdk-s0-s3.md` | 2026-06-11 | 2026-06-12 | 2 | 263 | journal (passation) | historique |
| 159 | `docs/plans/2026-06-11-chantier-s2-suivi.md` | 2026-06-11 | 2026-06-11 | 1 | 55 | plan | historique (chantier clos ou livré) |
| 160 | `docs/plans/2026-06-11-chantier-s3-suivi.md` | 2026-06-11 | 2026-06-11 | 1 | 56 | plan | historique (chantier clos ou livré) |
| 161 | `docs/spec/README.md` | 2026-06-11 | 2026-06-11 | 1 | 23 | spécification | en vigueur (wire/crypto v1) |
| 162 | `docs/spec/tom-crypto-v1.md` | 2026-06-11 | 2026-06-11 | 1 | 59 | spécification | en vigueur (wire/crypto v1) |
| 163 | `docs/spec/tom-wire-v1.md` | 2026-06-11 | 2026-06-11 | 1 | 82 | spécification | en vigueur (wire/crypto v1) |
| 164 | `sdk/swift/TomProtocolKit/README.md` | 2026-06-11 | 2026-06-11 | 1 | 51 | doc de code | en vigueur |
| 165 | `docs/plans/2026-06-12-chantier-hickory-026.md` | 2026-06-12 | 2026-06-12 | 1 | 90 | plan | historique (chantier clos ou livré) |
| 166 | `docs/plans/2026-06-12-chantier-s24-migration-apps.md` | 2026-06-12 | 2026-06-12 | 2 | 103 | plan | historique (chantier clos ou livré) |
| 167 | `docs/audits/AUDIT-2026-06-26.md` | 2026-06-26 | 2026-06-26 | 1 | 777 | red team / audit | historique (constats corrigés ou suivis) |
| 168 | `docs/MISSION.md` | 2026-07-01 | 2026-10-17 | 3 | 291 | décision (texte maître) | **en vigueur — fait foi** (MISSION.md:5) |
| 169 | `AGENTS.md` | 2026-07-02 | 2026-07-02 | 1 | 77 | doc | — |
| 170 | `docs/journal/JOURNAL.md` | 2026-07-02 | 2026-10-17 | 5 | 213 | journal | journal |
| 171 | `docs/plans/TOM-MASTER-MAP.md` | 2026-07-05 | 2026-07-06 | 4 | 198 | plan / vision | en vigueur (V2) ; portes = propositions |
| 172 | `docs/plans/L1-001-attestation-presence.md` | 2026-07-06 | 2026-07-06 | 3 | 758 | plan | livré (build ~39) |
| 173 | `docs/plans/L1-001-matrice-flotte.md` | 2026-07-06 | 2026-10-17 | 2 | 118 | plan | historique (chantier clos ou livré) |
| 174 | `docs/plans/L1-001-runbook-flotte.md` | 2026-07-06 | 2026-07-06 | 2 | 147 | plan | historique (chantier clos ou livré) |
| 175 | `docs/plans/TOM-PLAN-GLOBAL.md` | 2026-07-06 | 2026-07-06 | 3 | 186 | plan d'exécution | en vigueur partiel (M1.x non faits) |
| 176 | `docs/plans/stories/L1-001-attestation-presence.md` | 2026-07-06 | 2026-07-06 | 1 | 113 | plan | historique (chantier clos ou livré) |
| 177 | `docs/redteam/JOURNAL.md` | 2026-07-06 | 2026-07-10 | 8 | 274 | red team / audit | historique (constats corrigés ou suivis) |
| 178 | `docs/redteam/PROTOCOLE-RED-TEAM.md` | 2026-07-06 | 2026-07-06 | 1 | 207 | red team / audit | historique (constats corrigés ou suivis) |
| 179 | `docs/redteam/R13-VALIDATION.md` | 2026-07-09 | 2026-07-09 | 1 | 66 | red team / audit | historique (constats corrigés ou suivis) |
| 180 | `docs/plans/POP-PROOF-OF-PRESENCE.md` | 2026-07-10 | 2026-07-10 | 1 | 196 | ADR-011 | en vigueur (fondateur / en conception) |
| 181 | `docs/plans/L1-003-vue-signee-relais.md` | 2026-07-11 | 2026-07-12 | 2 | 149 | plan | livré (build 39) |
| 182 | `docs/plans/porte-automatique-self-relay-upnp.md` | 2026-07-12 | 2026-10-17 | 3 | 148 | plan | historique (chantier clos ou livré) |
| 183 | `docs/plans/R-name-via-dht.md` | 2026-07-15 | 2026-07-15 | 1 | 48 | plan | historique (chantier clos ou livré) |
| 184 | `docs/REGEN.md` | 2026-07-16 | 2026-10-17 | 2 | 71 | doc technique | historique ou opérationnel |
| 185 | `docs/plans/CHECKPOINT-2026-07-16.md` | 2026-07-16 | 2026-07-16 | 1 | 43 | plan | historique (chantier clos ou livré) |
| 186 | `docs/plans/FEUILLE-DE-ROUTE-2026-07-16.md` | 2026-07-16 | 2026-07-16 | 1 | 100 | plan | historique (chantier clos ou livré) |
| 187 | `docs/plans/R-sender-queue-antispam-grace.md` | 2026-07-16 | 2026-07-16 | 1 | 42 | plan | historique (chantier clos ou livré) |
| 188 | `docs/plans/R33-path-observability.md` | 2026-07-16 | 2026-10-17 | 2 | 80 | plan | historique (chantier clos ou livré) |
| 189 | `docs/plans/VISION-CONSOLIDEE-2026-07-16.md` | 2026-07-16 | 2026-07-16 | 1 | 106 | plan / vision | en vigueur pour les étages L0-L1-L2 ; L2 conditionnel |
| 190 | `docs/plans/anti-ravivage-fantomes-topologie.md` | 2026-07-17 | 2026-07-18 | 4 | 146 | plan | historique (chantier clos ou livré) |
| 191 | `docs/plans/multipath-path-abandon-protocol-violation.md` | 2026-07-17 | 2026-07-17 | 1 | 95 | plan | historique (chantier clos ou livré) |
| 192 | `vault/50-token-history.md` | 2026-07-17 | 2026-07-21 | 8 | 13 | vault | journal |
| 193 | `docs/plans/PROMPT-REPRISE-R14-R15.md` | 2026-07-18 | 2026-07-19 | 2 | 142 | prompt de reprise | outillage |
| 194 | `docs/plans/RUNBOOK-TESTS.md` | 2026-07-18 | 2026-07-18 | 1 | 139 | plan | historique (chantier clos ou livré) |
| 195 | `docs/plans/banc-test-chaos.md` | 2026-07-18 | 2026-07-18 | 3 | 251 | plan | historique (chantier clos ou livré) |
| 196 | `docs/plans/r14-ipv6-first-class.md` | 2026-07-18 | 2026-10-17 | 6 | 353 | plan | historique (chantier clos ou livré) |
| 197 | `docs/plans/r15-annuaire-local.md` | 2026-07-18 | 2026-07-19 | 2 | 240 | plan | livré réduit (R15-lite, adresses directes écartées) |
| 198 | `docs/plans/redial-presence-attestee-chemin-mort.md` | 2026-07-18 | 2026-07-18 | 3 | 183 | plan | historique (chantier clos ou livré) |
| 199 | `docs/plans/reset-cache-app-sortie-labo.md` | 2026-07-18 | 2026-07-18 | 2 | 156 | plan | historique (chantier clos ou livré) |
| 200 | `docs/plans/PROMPT-REPRISE-I10-LOTC.md` | 2026-07-19 | 2026-07-19 | 2 | 121 | prompt de reprise | outillage |
| 201 | `docs/plans/fix-backup-store-budget-octets.md` | 2026-07-19 | 2026-07-19 | 3 | 181 | plan | historique (chantier clos ou livré) |
| 202 | `docs/plans/protocole-test-r14-r15.md` | 2026-07-19 | 2026-07-20 | 2 | 721 | plan | historique (chantier clos ou livré) |
| 203 | `docs/plans/r14-lot-c-resondage.md` | 2026-07-19 | 2026-07-20 | 8 | 248 | plan | historique (chantier clos ou livré) |
| 204 | `docs/plans/PROMPT-REPRISE-ROLES.md` | 2026-07-20 | 2026-07-21 | 2 | 141 | prompt de reprise | outillage |
| 205 | `docs/plans/audit-instrumentation-phase0.md` | 2026-07-20 | 2026-07-20 | 3 | 106 | plan | historique (chantier clos ou livré) |
| 206 | `docs/plans/banc-courbe-masse.md` | 2026-07-20 | 2026-07-20 | 4 | 218 | plan / banc | livré (phase 1) |
| 207 | `docs/plans/banc-roles-sous-charge.md` | 2026-07-20 | 2026-07-21 | 2 | 267 | plan / banc | livré (phase A) |
| 208 | `docs/plans/charte-cibles-agressives.md` | 2026-07-20 | 2026-07-20 | 2 | 280 | plan / charte | en vigueur (v1, cibles « à ratifier ») |
| 209 | `docs/plans/prisme-des-roles.md` | 2026-07-20 | 2026-07-20 | 1 | 105 | plan / synthèse | en vigueur (grille des rôles) |
| 210 | `docs/article-avancement-2026-07-18.md` | 2026-07-21 | 2026-07-21 | 1 | 340 | article | récit daté |
| 211 | `docs/plans/PROCHAINE-SESSION-carnet-rendezvous.md` | 2026-07-21 | 2026-07-21 | 1 | 47 | prompt de reprise | outillage |
| 212 | `docs/plans/PROMPT-carnet-rendezvous-tournant.md` | 2026-07-21 | 2026-07-21 | 1 | 64 | prompt de reprise | outillage |
| 213 | `docs/plans/PROMPT-fix-freebox-oom.md` | 2026-07-21 | 2026-07-21 | 1 | 98 | prompt de reprise | outillage |
| 214 | `docs/plans/capstone-run-checklist.md` | 2026-07-21 | 2026-10-17 | 2 | 50 | plan | historique (chantier clos ou livré) |
| 215 | `docs/plans/chasse-oom-freebox-2026-07-21.md` | 2026-07-21 | 2026-07-21 | 2 | 110 | plan | historique (chantier clos ou livré) |
| 216 | `docs/plans/design-carnet-rendezvous-tournant.md` | 2026-07-21 | 2026-07-21 | 2 | 166 | plan | remplacée par CARTE (CARTE:4-5) |
| 217 | `docs/plans/hermeticite-etage-l.md` | 2026-07-21 | 2026-10-17 | 2 | 101 | plan | historique (chantier clos ou livré) |
| 218 | `docs/plans/p1-namespace-rendezvous-test.md` | 2026-07-21 | 2026-07-21 | 2 | 80 | plan | historique (chantier clos ou livré) |
| 219 | `docs/plans/verdict-oom-connectioninner-2026-07-22.md` | 2026-07-22 | 2026-07-22 | 2 | 174 | plan | historique (chantier clos ou livré) |
| 220 | `docs/plans/PROMPT-fix-connectioninner-worktree.md` | 2026-07-23 | 2026-10-17 | 2 | 151 | prompt de reprise | outillage |
| 221 | `docs/plans/postmortem-oom-freebox-nuit-22-23-juillet-2026.md` | 2026-07-23 | 2026-07-23 | 1 | 86 | plan | historique (chantier clos ou livré) |
| 222 | `docs/plans/CARTE-rendezvous-tournant.md` | 2026-07-24 | 2026-07-24 | 1 | 124 | plan / vision cadrée | **en vigueur** — pas de code avant go (CARTE:3) |
| 223 | `docs/plans/L1-002-entropie-non-biaisable.md` | 2026-08-07 | 2026-08-07 | 1 | 260 | plan / recherche | en cours — design-first |
| 224 | `docs/plans/PROMPT-reprise-canal-log.md` | 2026-08-11 | 2026-08-11 | 1 | 49 | prompt de reprise | outillage |
| 225 | `docs/plans/etape1-groupe-log-public.md` | 2026-08-11 | 2026-08-11 | 1 | 66 | plan | historique (chantier clos ou livré) |
| 226 | `scripts/harnais-hooks/README.md` | 2026-10-17 | 2026-10-18 | 2 | 382 | outillage harnais | hors vision |
| 227 | `vault/decisions/2026-10-17-ADN-ecarts-et-cible.md` | 2026-10-17 | 2026-10-17 | 2 | 116 | décision / carte des écarts | **en vigueur** (écarts code ↔ MISSION) |
| 228 | `vault/echanges/README.md` | 2026-10-17 | 2026-10-17 | 1 | 82 | outillage harnais | hors vision |
| 229 | `vault/echanges/archive/2026-10-17-F01-M0001-commit-pose-harnais-v1.6.0.md` | 2026-10-17 | 2026-10-17 | 1 | 169 | outillage harnais | hors vision |
| 230 | `vault/echanges/archive/2026-10-17-F01-M0002-commit-pose-harnais-v1.6.0-biome.md` | 2026-10-17 | 2026-10-17 | 1 | 261 | outillage harnais | hors vision |
| 231 | `vault/echanges/archive/2026-10-17-F01-M0003-acces-github-gh-et-issues.md` | 2026-10-17 | 2026-10-17 | 1 | 117 | outillage harnais | hors vision |
| 232 | `vault/echanges/archive/2026-10-17-F01-M0004-retrait-graphify-qmd-first.md` | 2026-10-17 | 2026-10-17 | 1 | 134 | outillage harnais | hors vision |
| 233 | `vault/echanges/archive/2026-10-17-F01-M0006-etat-des-lieux-github-adn.md` | 2026-10-17 | 2026-10-17 | 2 | 137 | outillage harnais | hors vision |
| 234 | `vault/echanges/archive/2026-10-17-F01-M0007-ci-verte-outillage-et-pre-push-miroir.md` | 2026-10-17 | 2026-10-17 | 1 | 207 | outillage harnais | hors vision |
| 235 | `vault/echanges/archive/2026-10-17-F01-M0010-integration-ci-verte-et-pre-push.md` | 2026-10-17 | 2026-10-17 | 1 | 135 | outillage harnais | hors vision |
| 236 | `vault/echanges/archive/2026-10-17-F02-M0005-qmd-imperatif-en-tete.md` | 2026-10-17 | 2026-10-17 | 1 | 106 | outillage harnais | hors vision |
| 237 | `vault/echanges/archive/2026-10-17-F03-M0009-retrait-ip-publique-et-audit-ssh.md` | 2026-10-17 | 2026-10-17 | 1 | 289 | outillage harnais | hors vision |
| 238 | `vault/reprise/00_INDEX.md` | 2026-10-17 | 2026-10-17 | 1 | 41 | journal (reprise) | en vigueur (état au 2026-10-17/18) |
| 239 | `vault/reprise/2026-10-17-ETAT-DES-LIEUX.md` | 2026-10-17 | 2026-10-17 | 2 | 187 | journal (reprise) | en vigueur (état au 2026-10-17/18) |
| 240 | `vault/reprise/CARNET_DE_BORD.md` | 2026-10-17 | 2026-10-17 | 1 | 29 | journal (reprise) | en vigueur (état au 2026-10-17/18) |
| 241 | `vault/reprise/CARTOGRAPHIE_BRANCHES.md` | 2026-10-17 | 2026-10-17 | 1 | 56 | journal (reprise) | en vigueur (état au 2026-10-17/18) |
| 242 | `vault/reprise/F01.md` | 2026-10-17 | 2026-10-18 | 8 | 5 | journal (reprise) | en vigueur (état au 2026-10-17/18) |
| 243 | `vault/reprise/F02.md` | 2026-10-17 | 2026-10-18 | 3 | 5 | journal (reprise) | en vigueur (état au 2026-10-17/18) |
| 244 | `vault/reprise/F03.md` | 2026-10-17 | 2026-10-18 | 3 | 5 | journal (reprise) | en vigueur (état au 2026-10-17/18) |
| 245 | `vault/reprise/etat-technique.md` | 2026-10-17 | 2026-10-17 | 1 | 39 | journal (reprise) | en vigueur (état au 2026-10-17/18) |
| 246 | `vault/revues/2026-10-17-R001-pose-harnais.md` | 2026-10-17 | 2026-10-17 | 1 | 30 | outillage harnais | hors vision |
| 247 | `vault/revues/2026-10-17-R002-retrait-graphify.md` | 2026-10-17 | 2026-10-17 | 1 | 120 | outillage harnais | hors vision |
| 248 | `vault/revues/2026-10-17-R003-qmd-imperatif.md` | 2026-10-17 | 2026-10-17 | 1 | 68 | outillage harnais | hors vision |
| 249 | `vault/revues/2026-10-17-R004-ci-verte-outillage.md` | 2026-10-17 | 2026-10-17 | 1 | 173 | outillage harnais | hors vision |
| 250 | `vault/revues/2026-10-17-R005-advisories-tls.md` | 2026-10-17 | 2026-10-17 | 1 | 93 | outillage harnais | hors vision |
| 251 | `vault/revues/2026-10-17-R006-advisories-tls.md` | 2026-10-17 | 2026-10-17 | 1 | 96 | outillage harnais | hors vision |
| 252 | `vault/revues/2026-10-17-R007-retrait-ip-publique.md` | 2026-10-17 | 2026-10-17 | 1 | 173 | outillage harnais | hors vision |
| 253 | `vault/revues/2026-10-17-R008-pre-push-ci-local.md` | 2026-10-17 | 2026-10-17 | 1 | 48 | outillage harnais | hors vision |
| 254 | `vault/revues/2026-10-17-R009-retrait-ip-publique.md` | 2026-10-17 | 2026-10-17 | 1 | 89 | outillage harnais | hors vision |
| 255 | `vault/revues/2026-10-17-R010-adn-mission.md` | 2026-10-17 | 2026-10-17 | 1 | 45 | outillage harnais | hors vision |
| 256 | `vault/revues/2026-10-17-R011-flake-63.md` | 2026-10-17 | 2026-10-17 | 1 | 53 | outillage harnais | hors vision |
| 257 | `vault/revues/2026-10-17-R012-retrait-ip-publique-integration.md` | 2026-10-17 | 2026-10-17 | 1 | 52 | outillage harnais | hors vision |
| 258 | `vault/runtime/README.md` | 2026-10-17 | 2026-10-17 | 1 | 215 | outillage harnais | hors vision |
| 259 | `vault/echanges/archive/2026-10-17-F01-M0012-adn-mission-decisions-ecarts-cible.md` | 2026-10-18 | 2026-10-18 | 1 | 183 | outillage harnais | hors vision |
| 260 | `vault/echanges/archive/2026-10-17-F02-M0008-advisories-tls.md` | 2026-10-18 | 2026-10-18 | 1 | 149 | outillage harnais | hors vision |
| 261 | `vault/echanges/archive/2026-10-17-F03-M0011-integration-retrait-ip-publique.md` | 2026-10-18 | 2026-10-18 | 1 | 166 | outillage harnais | hors vision |
| 262 | `vault/reprise/PASSATION-COWORK-2026-10-18.md` | 2026-10-18 | 2026-10-18 | 1 | 73 | journal (reprise) | en vigueur (état au 2026-10-17/18) |
| 263 | `vault/revues/2026-10-18-R013-flux-pull-request.md` | 2026-10-18 | 2026-10-18 | 1 | 31 | outillage harnais | hors vision |
| 264 | `vault/revues/2026-10-18-R014-racine-et-pre-push-vault.md` | 2026-10-18 | 2026-10-18 | 1 | 47 | outillage harnais | hors vision |
