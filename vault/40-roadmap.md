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
| ✅ R12 | Zero-config DHT rendezvous + resilience (isolation recovery, anti-sleep, bounded stop) | Livré, gaps résiduels identifiés (Known Limitations) |

### Phase 1 TypeScript — complète
- ✅ 8/8 epics — 771 tests — packages/core + packages/sdk

### Durci protocolaire (post-audit Fable, 2026-07-05)
- ✅ **4 bugs DoS corrigés** : backup `replicated_to` cap, HubShadowSync `members` truncate, réassemblage cap+TTL, DHT addrs cap. +6 tests régression. Commits 1656abb, 3d8cdfb.
- ✅ **DoS mémoire réassemblage** (amplification 1-paquet) : BTreeMap + MAX_CHUNKS=100k + budget global MAX_TOTAL_REASSEMBLY=128Mo. Commit 347421b.

### tvOS Node — Phases 1+2 complètes + Durcissement (2026-07-05)
- ✅ 2026-04-14 : xcframework multi-plateforme buildé (`TomProtocolFFI.xcframework` présent)
- ✅ 2026-04-14 : App Xcode créée — `TomNode.xcodeproj` + structure SwiftUI (Views/ViewModels/Models/Services)
- ✅ 2026-04-15 : observabilité format JSON unifié (appareil + node_id + uptime_s + msgs_sent)
- ✅ 2026-04-15 : `nas-node-ctl.sh` — contrôle NAS depuis Claude Code
- ✅ 2026-04-16 : source_amorcage Swift — reprobe relay si topologie vide
- ✅ 2026-04-16 : auto-reconnect + liveness log
- 🏆 **2026-06-08 : JALON — nœud iOS en 5G cross-réseau** rejoint le réseau ToM **décentralisé** (Pkarr/n0/DHT/IPv6, zéro relais à IP fixe). iPhone 5G (hors-LAN, CGNAT opérateur) ↔ NAS (derrière Freebox) connectés en ~1min30, 0 échec. NAS ajouté comme **nœud unifié** (`tom-node.service`, role Peer — ADR-006). Lien actuel via **fallback relais** (RTT 1856ms) → reste à obtenir le DIRECT (ouvrir IPv6 entrante Freebox + instrumenter `path_kind`).
- ✅ **2026-07-05 : Build 18 déployé flotte complète** (iPad, iPhone, Apple TV, macOS, NAS) — 4 fixes DoS + fixes watchdog 0x8BADF00D (Text() lazy-decode) + fixes CPU 100% (tokio::select! busy-spin). Perf validée : LAN ~6 Mo/s jusqu'à 64 Mo (100%), WiFi/relais ~5 Mo/s, FOREGROUND seul (iOS suspension = contrainte OS, chantier R18 APNs).

### Infrastructure
- ✅ NAS relay opérationnel : `tom-relay --dev` port 3340 (local + public `82.67.95.8:3340`)
- ✅ mDNS local discovery activé par défaut
- ✅ `tom-gateway` : CLI auto-config Freebox (crate 0.2.0)

## Sur le feu

### 🔴 Salve de correctifs post-audit (2026-07-01) — PRIORITÉ, pas encore faits
Confirmés file:line par l'audit 6-agents (`docs/audits/AUDIT-2026-06-26.md`) + revérifiés. Ordre : trivial → délicat.
- [ ] **verrou #2 — purge SQLite hub** (trivial) : `state.rs:536` `cleanup_hub_messages(now - TTL_MS)` au lieu de `TTL_MS`.
- [ ] **verrou #1 — ACK entrant** (faible) : gater l'arm `RoutingAction::Ack` sur `signature_valid` (`state.rs`~872).
- [ ] **Hub hijack** (moyen) : authentifier l'émetteur du `HubMigration` (`manager.rs:449-465`).
- [ ] **Failover hub mort** (élevé) : câbler timeout HubPong manquant → `record_ping_failure` ; corriger `should_promote` (`manager.rs:846,880-887`).
- [ ] **dalek double-version** (délicat) : aligner tom-protocol sur `=3.0.0-pre.1` (`Cargo.toml:23`), tester API 2.x→3.0-pre.
- [ ] 🟠 chat non signé livré ; pre-push-gate ignore Rust ; tom-connect/dht hors CI ; tom-quinn-udp orphelin.
- [ ] **Déploiement** : les fixes d'audit (main `e6d3501`) NE sont PAS sur les appareils (apps = build 4, NAS = pré-audit). Rebuild xcframework + iPad/iPhone/Mac/AppleTV + binaire NAS + `TomVersion` → build 5.
- [ ] **PR #53** (branche `claude/tom-protocol-audit-yf42jz`, docs vérifiées : wire-invariants + rapport) → à merger dans main.

### Points ouverts (2026-07-05)
- [ ] **Écart rôles BMAD ↔ code** : BMAD definit client/relay/observer ; code PeerRole = Peer/Relay seulement. Trancher : réintégrer observer ou documenter abandon. Sources : `_bmad-output/implementation-artifacts/3-2-dynamic-role-assignment.md` vs `crates/tom-protocol/src/relay.rs:19`.
- [ ] **Stall récurrent harness test** : `tom-chat --bot` se fige ~15 min (log gelé) — serveur HTTP TCP brut probablement bloqueur. Harness uniquement, pas protocole. Fiabilité test à améliorer.

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

### R13→R18 — Cible : réseau distribué viable, zéro friction (arbitré 2026-07-03)

> Constat : protocole correct (R1-R12, audits soldés) mais UNE seule porte
> publique (NAS perso) = SPOF de fait. Exigences utilisateur : zéro friction
> (« si mon beau-père doit ouvrir un port à la main, c'est mort »), relais
> TOURNANTS, aucun point central. La rotation existe déjà (rôles réseau ADR-006 +
> gate ADR-010) — ces chantiers élargissent le VIVIER de portes éligibles.
> Chaque step validé en tom-stress + campagne multi-devices réelle.

- [ ] **R13 — Porte d'entrée automatique (zéro friction)** — LE multiplicateur.
  VÉRIFIÉ 2026-07-03 : `portmapper` v0.13 (UPnP IGD + NAT-PMP + PCP) hérité
  d'iroh, DÉJÀ câblé dans MagicSock (`procure_mapping()` — socket.rs:610) pour
  le port QUIC. Steps : (1) instrumenter le mapping obtenu sur la flotte réelle
  (Freebox = UPnP actif par défaut) ; (2) étendre le mapping au port du relais
  embarqué (self-relay) → tout nœud installé devient porte complète, publiée et
  recrutée par le gate ADR-010 sans aucune manip ; (3) test d'acceptation :
  iPhone en data ↔ maison SANS le NAS (le Mac/iPad devient la porte tout seul).
- [ ] **R14 — IPv6 first-class** : (1) règle pare-feu Freebox 43925 (déjà
  identifiée) + mesurer le DIRECT v6 ; (2) publier les GUA v6 au rendez-vous,
  préférence v6 au dial, hole-punch v6 (quasi 100% vs NAT v4) ; (3) pinhole
  automatique via PCP quand la box le permet (zéro friction v6).
- [ ] **R15 — Annuaire local (mémoire des pairs)** : persister `node_id → relais
  habituel + dernières addrs (LAN/publique/v6) + path_kind` ; dial parallèle
  cache + lookup frais ; expiration douce (décision #4). Gain : reconnexion
  quasi instantanée famille/amis, moins de pression DHT. Zéro config (décision #6).
- [ ] **R16 — Nœud léger multi-plateformes (distribution du vivier)** :
  binaire statique musl (chaîne de cross-compil déjà en place). Canaux vérifiés 2026-07-03 :
  (a) **Raspberry Pi — 2 canaux officiels-adjacents** :
      · **Raspberry Pi Imager** (l'outil de flash officiel) accepte les OS tiers
        dans sa liste (guide « How to add your own images to Imager », repo JSON
        + cloud-init NoCloud pour la personnalisation WiFi/SSH dans l'UI).
        Flow rêvé : choisir « ToM Node OS » dans l'Imager → flasher → brancher → fini.
      · **Pi-Apps** (store communautaire, 1M+ utilisateurs, 200+ apps, GPL-3) :
        soumission par script shell + rubrique d'éligibilité — installe le nœud
        sur un Raspberry Pi OS existant.
      · Complément : one-liner curl (pattern Pi-hole) + Docker.
      Gagnant-gagnant : héberger un nœud = profiter du réseau (crypté, gratuit).
  (b) **Docker** Synology/QNAP/Unraid/home-servers ;
  (c) **VM Freebox** (Delta/Ultra) — flow sans friction pour fan de forum :
      qcow2 ARM64 prêt (nœud préinstallé + autostart + tom-gateway embarqué) →
      Freebox OS → VM → image personnalisée (~5 clics) → au 1er boot, tom-gateway
      demande l'autorisation API → l'utilisateur VALIDE SUR L'ÉCRAN DE LA BOX
      (flow de pairing natif Freebox) → redirection de port auto → porte ouverte.
      Monitoring : status server du nœud (8085) déjà exposé en LAN.
  (d) **FreeStore** = app PLAYER (Qt/QML, store TV vérifié) → canal de
  VISIBILITÉ/compagnon : dashboard de monitoring du nœud VM sur la TV, pairing QR.
  PAS un hôte de démon (apps TV, pas de background fiable). R13 ouvre la porte partout.
- [ ] **R17 — Seeds optionnels + rotation observée** : 1-2 VPS derrière
  `relay-eu/us.tom-protocol.org` (URLs déjà en défaut committé), explicitement
  RETIRABLES ; valider en réel la rotation (publication/dé-publication gate,
  répartition RelaySelector). Amorçage de confort, pas d'infrastructure sacrée.
- [ ] **R18 — Wake-up adapters (BONUS, hors cœur)** : hook « sonnette » neutre
  dans le SDK ; adaptateurs par plateforme côté app (APNs d'abord, FCM ensuite,
  rien sur headless). La dépendance centralisée reste cantonnée à l'app.

**Stratégie d'adoption (décision Malik 2026-07-03) : PAR LE BAS.** Le premier
public est le geek souverainiste — celui qui veut couper ses chaînes, retrouver
de la liberté, ne plus dépendre des USA ou d'autres puissances. C'est lui qui
porte la « vraie parole » (bouche-à-oreille authentique, crédibilité technique).
Les canaux R16 (Pi Imager, Pi-Apps, forums Freebox, self-hosters) sont le
vecteur ; la liberté est le message ; le deal BitTorrent est le pitch (héberger
un nœud = messagerie chiffrée gratuite, sans serveur à payer). Pas de grand
public avant ce socle. ⚠️ PAS PRÊT à distribuer : R13 (porte automatique) est
le prérequis du « brancher et oublier » — d'abord taffer.

**Horizon (étoile polaire, pas un chantier)** : tout terminal toujours-allumé
comme nœud — terminaux de paiement, IoT, box. Un réseau de transport neutre,
gratuit, chiffré, sans blockchain ni fees (décision #7 : fondation universelle).
- Protocol convergence : stack TypeScript + Rust unifiées (détails non trouvés dans docs scannés)

### Améliorations infra
- Push public relay (`82.67.95.8:3340`) — UDP 3340 déjà forwardé en Freebox
- Clarifier la place de `tom-relay-ffi` (embedding Apple TV / mobile / démo embarquée ?) avant d'ouvrir un nouveau front

## Parking

- `tom-relay-ffi` : crate existante, usage non documenté — probablement pour embedding relay dans app mobile
- `apps/infra-web-client` : client web infra (non exploré)
- Articles Medium présents dans `docs/` — publication potentielle
- `tom-whitepaper-v1.md` — whitepaper existant
