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
- ✅ NAS relay opérationnel : `tom-relay --dev` port 3340 (local + public `82.67.95.8:3340`) — ⚠️ **IP LAN DYNAMIQUE** (bail DHCP renouvelé à chaque redémarrage VM, ex. `.21`→`.83` le 2026-07-12) : `ping`/IP en dur ne sont PAS fiables pour vérifier la joignabilité, toujours tester le VRAI service (SSH/port relais) ou résoudre par hostname mDNS (`chk3wej...home`), jamais présumer le NAS mort sur un seul ping raté
- ✅ mDNS local discovery activé par défaut
- ✅ `tom-gateway` : CLI auto-config Freebox (crate 0.2.0)
- ⚠️ **Diagnostic devices Apple** : `xcrun xctrace list devices` peut afficher à tort "Offline" des devices réellement appairés en WiFi (bug/lag de cet outil sur les devices réseau, pas USB) — utiliser `xcrun devicectl list devices` (le vrai outil derrière la fenêtre Xcode Devices) pour un état fiable.
- ✅ **Team de signature Xcode corrigée (2026-07-12)** : `DEVELOPMENT_TEAM` pointait vers `K22558HU63` (Personal Team GRATUITE, plafond 3 apps/7 jours) au lieu de `UPES5479T5` (Apple Developer Program payant) — corrigé dans `project.yml`. Cause exacte de l'échec d'install sur iPhone de Malik pendant le déploiement flotte. Nécessite une action manuelle Xcode (Settings→Accounts→vérifier le compte payant→Download Manual Profiles) pour générer le certificat, pas faisable en CLI seule.

## Sur le feu

### 🚧 L1-003 — Vue signée du relais (§5 ADR-011, présence scopée appareil faible)
> Doc de conception : `docs/plans/L1-003-vue-signee-relais.md`. Suite du red-team Fable PoP (voir section suivante) — ferme le kill-shot #3 (eclipse témoin unique).
- [x] Étape 1 — type wire `RelayPresenceView`/`PresenceEntry` (`0c78e70`, build 34)
- [x] Étape 2a/2b — `WitnessLog` côté témoin (observations bornées) + câblage depuis les ACK relayés (`f5fa9dc`, `eee6a1f`)
- [x] Étape 2c — abonnement + tick push + vue signée vérifiable (`54dcad4`)
- [x] Étape 3 — **quorum consommateur** (cœur défensif) : `presence/quorum.rs::QuorumAggregator`, promotion Known→Online seulement si ≥N témoins distincts concordent, N dynamique 2-4 (`094ec96`, build 35)
- [x] Étape 4 — durcissement anti-abus : **cap par témoin** `MAX_PEERS_PER_WITNESS=64` (`QuorumAggregator::witness_peers`, éviction FIFO intra-témoin, un Sybil ne peut plus évincer les attestations d'un autre témoin) + **spot-check crypto réel** `proof_ref` — `PresenceEntry.ack_proof: Vec<u8>` embarque l'`Envelope` Ack signé brut réellement relayé, vérifié côté consommateur (`state.rs::verify_presence_entry_proof`, 7 checks : parse, msg_type, from==peer_id, signature Ed25519, cohérence AckPayload, fraîcheur TTL). Limite connue documentée (pas un bug) : Sybil MULTI-identité peut contourner le cap par témoin en répartissant sur N witness_id (coût = N keypairs, pas fermé ici).
- [x] Étape 5 — tests adversariaux : flood par témoin (2 tests), ACK forgé/signature invalide, ACK périmé, incohérence proof_ref/ack_proof, mélange valide/invalide dans une vue, témoins complices sur réseau clairsemé (documenté comme accepté, pas un bug). **Bug de sécurité trouvé PENDANT l'implémentation et corrigé avant merge** : le premier jet avait un bypass `if ack_proof.is_empty() { return true }` qui annulait toute la vérification — supprimé + test de régression `empty_ack_proof_never_promotes` ajouté.
- [x] **Red-team post-livraison (2026-07-12)** : 2 failles réelles trouvées et corrigées — (a) aucune dégradation Online→Stale quand le quorum de témoins s'effondre (pair promu restait Online indéfiniment, exploitable par un Sybil qui construit un quorum temporaire puis se tait) ; (b) aucune borne sur `PresenceScope::Peers` déclaré par un abonné (DoS asymétrique consommateur→relais, coût O(scope) payé par le relais à chaque tick 30s). Fix (a) : sweep périodique dans `tick_presence_cleanup`, `PEER_ONLINE_STALE_MS=60s`, dégrade vers `Stale` (PAS `Known` — bug trouvé en cours de review : `HeartbeatTracker` ne remonte Online que depuis Stale/Offline, jamais depuis Known, un pair dégradé vers Known serait resté bloqué). Fix (b) : scope `Peers` borné à `MAX_VIEW_ENTRIES=256` à l'abonnement, rejet complet (pas de troncature silencieuse) si dépassé. 625 tests verts, clippy 0 warning, 2 reviews sécu indépendantes passées.
- [x] **Étape 6 — validation flotte réelle FAITE (2026-07-12)** — build déployé et lancé sur Apple TV, iPad Air, iPhone Laura, macOS local (build+run réel, pas juste compilé). iPhone de Malik bloqué par une limite Apple réelle (profil dev gratuit = 3 apps/7 jours), pas un bug — ancienne version continue de tourner dessus et participe quand même au maillage. **Preuve réseau réelle** (status HTTP local de chaque device, port 9091) : les 4 devices sont `phase: Converged`, se voient mutuellement dans `pairs_connectes`, l'iPad utilise le NAS comme `relay_url_active` (`http://192.168.0.83:3340`) — maillage P2P réel confirmé, pas une supposition. **2 obstacles trouvés et corrigés pendant le déploiement, pas avant** : (1) ma détection initiale "NAS+devices injoignables" était fausse — `ping`/`xctrace` donnaient un faux négatif (ICMP bloqué, `xctrace` ne voit pas les devices appairés en WiFi) ; `devicectl` (le vrai outil utilisé par Xcode) et un test du VRAI service (SSH, port relais) ont montré que tout était joignable — le NAS avait juste changé d'IP (`.21`→`.83`, bail DHCP renouvelé après redémarrage VM). (2) bug réel de config Xcode : le target `TomNode` (universel tvOS+iOS) référençait l'icône brandasset tvOS (`App Icon & Top Shelf Image`) pour TOUTES les plateformes, cassant tout build iPhone/iPad (`CompileAssetCatalogVariant` échoue, iOS n'a pas ce type d'asset) — corrigé par des clés `ASSETCATALOG_COMPILER_APPICON_NAME[sdk=...]` conditionnelles dans `project.yml` (l'`AppIcon.appiconset` iOS existait déjà, juste jamais référencé).
- [x] **Gap versioning rattrapé** : build 35→36 (L1-003 4-5)→37 (fix downgrade+scope)

### ✅ Red-team Fable ADR-011 PoP à 1M — 6/6 kill-shots fermés (2026-07-11/12)
Verdict : "survit comme DIRECTION, pas comme état actuel" avant fixes. Ordre de reconstruction imposé respecté :
- [x] **#1** heartbeat déclaratif non gaté sur signature → fermé (`ecee492`, build 33)
- [x] **#4** anti-Sybil KNOWN faux (14h quasi-gratuit sur 1 relais) → `Known` séparé d'`Online` (travail soutenu prouvé) (`aadc8bd` + `76bd63a`, build 33)
- [x] **#3** eclipse témoin unique → quorum (voir L1-003 étape 3 ci-dessus)
- [x] **#2** fermé (vérifié file:line 2026-07-12, code déjà en place — pas un nouveau fix) — le sweep de challenge actif (`state.rs::tick_presence_probe`) est **opt-in** (`config.presence_probe_interval` défaut `None`, `runtime/mod.rs:136`) — le tick handler retourne `Vec::new()` immédiatement si désactivé (`state.rs:2209`). Même activé manuellement ("fleet observability"), il est plafonné à **8 pairs max** (`.take(8)`, `state.rs:2217`), jamais O(N) sur tous les Online. L1-003 (`witness.rs`/`relay_view.rs`/`quorum.rs`) fournit désormais le chemin passif dérivé du flux ACK que l'audit demandait. **Trou trouvé pendant la vérification** : zéro test ne figeait ce comportement (opt-in + cap 8) — corrigé, voir ci-dessous.
- [x] **#5** fermé (2026-07-12) — cap agrégé global `RESPONDER_KNOWN_GLOBAL_BUDGET_PER_WINDOW=60` ajouté dans `presence/mod.rs::allow_response()`, symétrique au cap stranger existant (`RESPONDER_GLOBAL_BUDGET_PER_WINDOW=120`). Avant : un Sybil qui promeut jusqu'à 512 identités à `Known` (5 relais soutenus/identité) pouvait extraire 512×10=5120 signatures/fenêtre (~43x le cap stranger) sans aucune borne agrégée. Vérifié check exécuté AVANT toute mutation d'état partagé (pas d'effet de bord sur rejet), review sécu indépendante RATIFIÉE + red-team indépendant (piste "now non-monotone" vérifiée et écartée : `presence_now()` = horloge locale + offset de config, jamais dérivé d'un message hostile). 62 tests presence verts (2 nouveaux), clippy+tests workspace verts.
- [x] **#6** fermé (même vérification que #2, conséquence directe) — sans sweep périodique par défaut, et plafonné à 8 pairs même activé, une guérison de partition ne peut pas déclencher de burst O(N) de challenges synchronisés.
- [x] **Régression tests #2/#6** (2026-07-12) — `tick_presence_probe_noop_when_disabled_by_default` + `tick_presence_probe_caps_at_eight_peers_when_enabled` (`state.rs`) : la fermeture de #2/#6 reposait sur un comportement NON testé — un futur changement du default ou du `.take(8)` aurait pu rouvrir silencieusement les deux kill-shots sans qu'aucun test rouge.

### ✅ Salve de correctifs post-audit — SOLDÉE (revérifié dans le code 2026-07-05)
Les 5 criticals de l'audit 6-agents sont corrigés (file:line revérifiés) :
- [x] **verrou #2 — purge SQLite hub** : `state.rs:549` `cleanup_hub_messages(now.saturating_sub(TTL_MS))`.
- [x] **verrou #1 — ACK entrant** : `state.rs:894` `if !signature_valid` gate l'arm `RoutingAction::Ack`.
- [x] **Hub hijack** : `handle_hub_migration` garde `from == new_hub_id && shadow_id == new_hub_id`.
- [x] **Failover hub mort** : `record_ping_failure` câblé hors tests (`manager.rs:1018`).
- [x] **dalek** : `ed25519-dalek = "=3.0.0-pre.1"` aligné (`Cargo.toml:24`).
- [x] **PR #53** MERGÉE (docs audit).
- [x] **Déploiement** : **build 18** sur TOUTE la flotte (iPad/iPhone/AppleTV/macOS/NAS) le 2026-07-05 — inclut fixes audit + CPU/watchdog iOS + 4 DoS anti-pair-malveillant. (Remplace l'ancien « build 4→5 ».)

### Points ouverts (2026-07-05)
- [x] **Écart rôles BMAD ↔ code — TRANCHÉ (2026-07-12) : Observer abandonné, documenté, pas recodé.** Vérifié : dans le TypeScript Phase 1 (source des rôles fondateurs, 771 tests), `observer` n'apparaît **qu'une fois dans tout `packages/core/src`** — juste dans la déclaration du type union `NodeRole` (`network-topology.ts:3`) — **zéro logique d'assignation, zéro branche de comportement, zéro test l'exerçant**. Ce n'est pas un rôle abandonné en cours de route : il n'a **jamais été fonctionnel**, même dans la référence originale. Le port Rust (`PeerRole` = Peer/Relay, `relay.rs:35-40`) a donc porté tout le comportement réel et correctement laissé de côté un type mort. Décision : ne pas recoder Observer — le modèle two-tier `PeerStatus` (Online/Known/Stale/Offline, ADR-011) couvre déjà la notion de "présence incertaine/passive" que le rôle Observer était censé représenter, sans avoir besoin d'un rôle réseau séparé. Guardian/Validator restent explicitement "DO NOT implement" (futur), inchangé.
- [ ] **Stall récurrent harness test** : `tom-chat --bot` se fige ~15 min (log gelé) — serveur HTTP TCP brut probablement bloqueur. Harness uniquement, pas protocole. Fiabilité test à améliorer.

### tvOS Node — convergence code ↔ doc ↔ tests
- [x] **Architecture Swift tranchée** (2026-06-07) : on garde le wrapper local `TomNodeWrapper`/`TomNodeService`. `TomCoreKit` abandonné.
- [x] **Premier filet de sécurité contrat FFI** (2026-06-07/09) : `tom_node_status` sur serde (`NodeStatusFFI`) + tests de contrat. **Review Copilot confirmée** : contrat clés correct, zone grise `u64→Int Swift` documentée.
- [x] **Fix commentaire CLI `--bind-port`** (2026-06-09) : "dual-stack IPv6+IPv4" remplacé par la réalité (IPv4 reste éphémère).
- [x] **Review Copilot x3 soldée** (2026-06-09) : handoffs FFI + transport + deps intégrés. Dette §25 effacée.
- [x] **Push commits** — routine désormais (voir §Règle "commit push" CLAUDE.md), plus un item isolé
- [x] **Rebuild xcframework** (2026-07-12) — 2 rebuilds ce jour (round 1 + round 2 red-team L1-003), synced vers `sdk/swift/TomProtocolKit/Artifacts/`, build 37
- [ ] **Ouvrir IPv6 entrante Freebox** — port 43925 → `2a01:e0a:14f:5da0:248f:5dff:fea5:8ed1` (Freebox OS, règle pare-feu manuelle). Débloque connexion DIRECT QUIC NAS. **Action manuelle utilisateur, pas exécutable en autonomie.**
- [x] **Mettre `docs/TOM-TVOS-NODE-PLAN.md` à jour** — refléter l'état réel (2026-07-12, `c8ec8d2`) : doc réécrite, phases 1-5 marquées livrées, écart architecture TomCoreKit→wrapper local documenté
- [ ] **Tests Swift/tvOS** — câbler XCTest dans `.xcodeproj` (fixtures identiques aux tests Rust)
- [ ] **Durcir couche tvOS** : messages/groupes, persistance, reprise après veille

### ✅ Chantier macOS — SOLDÉ (vérifié + rebuild + lancé réel 2026-07-12, la doc disait "prêt à démarrer" à tort)

> Spec complète : `docs/superpowers/specs/2026-06-08-app-macos-tom-design.md` — **périmée**, décrivait les 5 lots comme "à faire" alors qu'ils étaient déjà tous livrés.

- [x] **Lot A** — Slice Rust FFI `aarch64-apple-darwin` : déjà dans `build-tom-protocol-ffi-xcframework.sh`, xcframework contient `macos-arm64_x86_64`.
- [x] **Lot B** — Cible Xcode macOS : `TomNode-macOS` existe (scheme + target), `apps/tom-node-tvos/TomNode-macOS/` (Assets.xcassets) présent.
- [x] **Lot C** — Portage `TomNodeService.swift` : `#if os(macOS) ... ProcessInfo.processInfo.beginActivity(...)` déjà en place (anti-veille macOS).
- [x] **Lot D** — Entitlements : `TomNode-macOS.entitlements` présent, exact contenu spec (`app-sandbox` + `network.client` + `network.server`).
- [x] **Lot E** — Makefile : `macbuild`/`macrun`/`macdoctor` déjà dans `apps/tom-node-tvos/Makefile`.
- [x] **Validation réelle 2026-07-12** : `make macdoctor` tout vert, `make macbuild` → `BUILD SUCCEEDED`, `.app` produit et **lancé réellement** sur le MacBook Pro (process natif confirmé `ps aux`), fermé proprement ensuite. Pas de test réseau P2P (NAS injoignable à cette date) — juste la preuve build+launch. À refaire avec le NAS up pour valider la connexion réelle (§6 de la spec).

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
  - [x] **Étape 1 VÉRIFIÉE EMPIRIQUEMENT (2026-07-12)** — mapping UPnP obtenu
    en conditions réelles sur la Freebox de l'utilisatrice, PAS une supposition :
    nœud macOS relancé avec `RUST_LOG=debug` (le niveau par défaut masquait ce
    log), trace exacte : `getting a port mapping for 192.168.0.70:49850` →
    `new port mapping Some(Upnp(Mapping { gateway: http://192.168.0.254:5678/control/wan_ip_connection, external_ip: 82.67.95.8, external_port: 58870 }))`.
    La Freebox répond bien à UPnP IGD par défaut, un port UDP public est
    obtenu automatiquement pour le port QUIC principal, zéro manip manuelle.
    Confirme l'hypothèse de départ de R13 sur du matériel réel, pas en théorie.
  - [x] **Étape 2 IMPLÉMENTÉE (2026-07-13)** — mapping UPnP du port du relais
    embarqué câblé (`embedded_relay.rs` + `loop.rs` + `state.rs`), doc
    `docs/plans/porte-automatique-self-relay-upnp.md` §Statut pour le détail.
    2 bugs de robustesse trouvés en review (regénération de watcher à chaque
    tick, ordre d'assignation vs un await) et corrigés avant commit. 632 tests
    tom-protocol verts, workspace complet vert (dont `stability_2min` rejoué
    isolément avant/après pour écarter un faux hang lié à un incident réseau
    nocturne, pas au code). Build 40.
  - [ ] Étape 3 (test d'acceptation réel : iPhone data ↔ maison sans le NAS).
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
