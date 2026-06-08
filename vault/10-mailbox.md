# Mailbox projet

> Géré automatiquement par Claude. Markdown vivant, pas document gravé.

## Courrier entrant

### 2026-06-08 — 🏆 JALON : nœud iOS en 5G sur réseau ToM décentralisé [auto]

- Source : Claude Code — session test hardware réel (Apple TV + iPad + iPhone + NAS Freebox)
- Statut : traité
- Résumé :
  - **Réussite** : iPhone en **5G hors-LAN** (CGNAT opérateur) rejoint le réseau ToM **décentralisé** (Pkarr/n0/DHT/IPv6, zéro relais à IP fixe) et échange avec le NAS **derrière la Freebox** — 0 échec.
  - **NAS = nœud unifié** : service systemd `tom-node.service` (`tom-chat --bot`, role Peer, pas relais exclusif — ADR-006). Relais `tom-relay.service` séparé. IP publique `82.67.95.8`, forward NAT UDP+TCP 3340 OK (vérifié via SDK `tom-gateway`).
  - **Fix clé** : défaut relais vide dans l'app (`TomNodeService.swift:50`) — le `.70` forcé bloquait le rendez-vous public. Rebuild iPhone → découverte organique OK.
  - **Limite actuelle** : lien via **fallback relais** (RTT 1.8–3.6s), pas de DIRECT. Cause : CGNAT 5G symétrique + firewall IPv6 Freebox. Hole-punch sans chemin direct viable.
- Prochaine action : ouvrir IPv6 entrante Freebox (SDK `tom-gateway`) + instrumenter `path_kind` dans status `:8085` pour viser le DIRECT ; figer une identité persistante pour le nœud NAS.


### 2026-05-22 — Session initialisation vault [auto]

- Source : Claude Code — exploration complète projet
- Statut : traité
- Résumé :
  - **Projet** : ToM Protocol — protocole P2P décentralisé (pas blockchain), transport QUIC, chiffrement E2E
  - **Phase active** : tvOS Node en cours — xcframework Rust buildé, app Xcode créée, UI SwiftUI partielle
  - **Stack** : 17 crates Rust · 2 packages TS · 1 app Swift tvOS — version 0.2.0
  - **Dernière activité significative** : 2026-04-16, série de commits observabilité (source_amorcage Swift, format JSON unifié, reprobe relay topologie vide)
  - **Phases Rust R1–R11** : toutes complètes (stress tests 100%)
  - **Phase 3 convergence TS+Rust** : en cours — nœud tvOS est le front actif
  - **Infrastructure** : relay NAS Freebox ARM64 opérationnel (port 3340), mDNS local discovery actif
  - **Nouveaux crates découverts** : `tom-gateway` (CLI config Freebox), `tom-relay-ffi`, `tom-integration-tests`
- Prochaine action : compléter Swift TomCoreKit + UI tvOS (Phases 3-4 TOM-TVOS-NODE-PLAN.md)

### 2026-05-22 — Vault initialisé [auto]

- Source : setup-project-vaults.py
- Statut : archivé
- Résumé : Vault créé pour le projet Tom-Protocol. Les sessions futures doivent alimenter ce fichier à chaque clôture significative.
- Prochaine action : première session → compléter vault/00-brief.md + vault/40-roadmap.md

### 2026-06-07 — Audit repo / état de lieu [auto]

- Source : GitHub Copilot — lecture vault + code + git + validation build
- Statut : traité
- Résumé :
  - **Direction générale** : le cœur Rust avance bien et compile (`cargo check --workspace` vert), l'infra relay et les briques FFI sont réelles
  - **Écart majeur** : la doc tvOS (`docs/TOM-TVOS-NODE-PLAN.md`) est en retard sur le code ; elle parle encore de création de briques déjà présentes
  - **Risque principal** : la couche tvOS a du vrai code productif (`TomNodeWrapper`, `TomNodeService`, tabs SwiftUI) mais **aucun test Swift détecté**
  - **Signal de dette** : working tree local chargé et validation JS confuse (`runTests` tombe sur un e2e Playwright dépendant du port 5173)
  - **Conclusion** : oui, le repo avance, mais il faut maintenant consolider, clarifier et tester plus qu'ajouter de nouvelles briques
- Prochaine action : remettre la roadmap tvOS en phase avec le code réel puis ajouter un filet de sécurité Swift + clarifier le split tests unitaires/e2e

### 2026-06-07 — Chantier tvOS : audit + durcissement contrat FFI [auto]

- Source : Claude Code — audit code réel (Swift + FFI) puis implémentation ciblée
- Statut : traité
- Résumé :
  - **Constat** : FFI entièrement câblé (`TomNodeWrapper`/`TomNodeService`), vues SwiftUI réelles (StatusView 399L…), aucun test Swift, `tom-protocol-ffi` exclu du workspace, pas de cible de test dans le `.xcodeproj`.
  - **Décision archi** : garder le wrapper local, abandonner `TomCoreKit` (ne débloque rien).
  - **Avance livrée** : durcissement du point le plus fragile — `tom_node_status` passé d'un `format!` manuel à serde (`NodeStatusFFI`) + 4 tests de contrat Rust verrouillant les clés JSON décodées par Swift. `cargo clippy -D warnings` + tests verts.
  - **Fichiers** : `crates/tom-protocol-ffi/src/types.rs`, `crates/tom-protocol-ffi/src/lib.rs`.
- Prochaine action : `make ffi` + `make ffi-device` pour embarquer la sérialisation durcie ; puis câbler une cible XCTest dans le `.xcodeproj` réutilisant les mêmes fixtures que les tests Rust.
