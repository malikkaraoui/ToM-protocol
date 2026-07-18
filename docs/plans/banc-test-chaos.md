# Banc de test chaos ToM — « le test qui doit passer »

> 🏃 **Pour LANCER les tests (routine reproductible, tout LLM) → `docs/plans/RUNBOOK-TESTS.md`.**
> Ce document-ci décrit le POURQUOI (invariants, scénarios, conception) ; le runbook décrit le COMMENT.

> Statut : **PROPOSITION de conception — à valider avant de coder l'orchestrateur.** Design-first.
> Origine : constat Malik (18/07) — « nos campagnes valident l'envoi/réception + quelques
> chronos, c'est trop peu ». Objectif : un banc qui **injecte des fautes réelles** et **vérifie
> des invariants durs**, durcissable au fil du dev. Ancres capacités vérifiées par 3 explorations
> du 18/07 (tom-stress, points d'injection, API control) — file:line à re-confirmer avant code.

## 1. Le principe qui change tout : INVARIANT, pas chrono

Aujourd'hui un test « réussit » si les messages arrivent et que le RTT est faible. C'est le
**chemin heureux**. Un vrai banc pose la question inverse :

> **Sous cette faute précise, est-ce que les GARANTIES du protocole tiennent encore ?**

Un scénario PASSE ssi **tous ses invariants durs** tiennent — pas parce que « ça avait l'air
d'aller ». Un scénario qui livre 100 % des messages mais laisse une connexion zombie, ou gèle
une boucle 45 s, ou double une livraison : **ÉCHOUE**. C'est le renversement demandé.

## 2. Les 8 invariants durs (vérifiés à CHAQUE scénario)

Chiffres = seuils réels du code (ordres de grandeur, à re-confirmer file:line avant d'asserter).

| # | Invariant | Mesure | Seuil |
|---|-----------|--------|-------|
| **I1** | **Zéro perte finale** | tout message émis finit `livré` (ACK) OU explicitement `expiré` (TTL) | 0 perte silencieuse |
| **I2** | **ACK ⟺ livraison à l'app** | pas d'ACK émis par un nœud zombie (violerait décision #1) | 0 ACK fantôme |
| **I3** | **Reconvergence bornée** | délai perte-de-pair → re-mesh complet | post-kill < 15 s ; isolation 15 s ; failover hub : **shadow ~8 s** (watchdog actif, `SHADOW_PING_INTERVAL 3s × THRESHOLD 2`), **candidat orphelin < 35 s** (pire cas, `CANDIDATE_ORPHAN_TIMEOUT 30s`) |
| **I4** | **Zéro zombie** | connexion QUIC ouverte mais silencieuse | jamais > 45 s (`LIVENESS_STALE_MS`) |
| **I5** | **Zéro gel de boucle** | intervalle réel des ticks « 60 s » / « 15 s » | jamais > 2× la période (profileur `timed()`) |
| **I6** | **Zéro crash non compté** | PID stable, ou restart systemd/watchdog **explicitement attendu** | 0 crash surprise |
| **I7** | **Mémoire bornée** | RSS sur endurance | pas de croissance monotone (fuite) |
| **I8** | **Zéro double-livraison** | un rejeu ne produit pas 2 livraisons app (nonce anti-replay 24 h) | idempotence stricte |

Ces 8 invariants sont le **contrat**. Le reste (RTT, débit) sont des métriques de qualité, pas
des critères de réussite.

## 3. Deux étages complémentaires

### Étage L — Logique (in-process, `tom-stress`, existe déjà, à étendre)
- Spawn N nœuds dans UN processus, adresses échangées en mémoire (`n0_discovery(false)`).
- **Fautes disponibles** : KILL/REVIVE/SKEW (`chaos-monkey`, seed reproductible), partition
  logique (`RemovePeer`/`AddPeerAddr`), gossip malformé (`InjectGossipBytes`), latence gossip.
- **Force** : rapide, déterministe (seed), tourne en CI, aucun appareil. **Régression.**
- **Limite** : ne teste PAS le vrai transport QUIC/UDP, ni l'OS lifecycle, ni le réseau réel.

### Étage F — Flotte réelle (orchestrateur externe, À CONSTRUIRE) ⭐ le cœur de la demande
- Pilote les VRAIS nœuds (Mac, NAS, iPhone×2, iPad, ATV) via API HTTP + kill OS + collecteur.
- **Ce que Malik veut vraiment** : appareils physiques, réseaux réels, suspension iOS, handover
  cellulaire. Là où les vrais bugs vivent (le pool-lock du 17/07, le wedge transport, la
  surdité UDP n'étaient visibles QU'EN réel).
- **Autonome** au maximum (kill, API, réseau simulable) ; **semi-auto** pour l'irréductiblement
  physique (bascule 3G/4G/avion d'un iPhone = main humaine — l'orchestrateur dit « bascule
  maintenant » et mesure l'invariant autour).

## 4. Taxonomie des fautes — levier exact, autonome vs physique

| Faute | Levier | Étage | Autonome ? |
|-------|--------|-------|-----------|
| Kill brutal (défaut utilisateur) | `pkill` (Mac) · `systemctl stop`/kill (NAS) · `devicectl process terminate` (iOS/tvOS) | L+F | ✅ |
| Arrêt propre (bouton stop) | `POST :9300/stop` · bouton UI (traqueur) | F | ✅ (Mac/NAS) |
| Retour d'un nœud | `systemctl start` · `devicectl process launch` · relance process | L+F | ✅ |
| Destinataire hors ligne | tuer B puis envoyer à B | L+F | ✅ |
| Message lourd (Mo) | `POST :9300/send?size=8000000&count=N` | L+F | ✅ |
| Répétition / rejeu | renvoyer le même payload/seq N fois | L+F | ✅ |
| Clock skew ±90 s | `set_presence_clock_offset` / chaos-monkey SKEW | L | ✅ |
| Partition | logique (`RemovePeer`) · **réelle** (`pfctl` bloque l'IP du pair) | L / F | ✅ |
| Latence / perte réseau (mobiles) | **Network Link Conditioner** (outil Apple, profils 3G/LTE/DSL/loss) sur iPhone/iPad · `dnctl`+`pfctl` côté Mac/NAS | F | ⚠️ semi-auto iOS / ✅ Mac |
| Coupure réseau / hors-ligne | interface down (`ifconfig`/`networksetup`) Mac · `pfctl` block-all | F | ✅ (Mac/NAS) |
| Mode avion (iPhone/iPad) | **physique** OU Network Link Conditioner profil « 100% loss » | F | ⚠️ semi-auto |
| Changement 3G/4G/5G↔WiFi | **iPhone/iPad UNIQUEMENT** (le réseau adverse ne concerne que les mobiles, pas NAS/ATV fixes) — physique OU profils NLC | F | ⚠️ semi-auto |
| Background / foreground iOS | `devicectl` launch d'une autre app pour backgrounder · retour physique | F | ⚠️ semi-auto |
| Veille système (Mac) | `pmset sleepnow` (⚠️ tuer `caffeinate` + relâcher l'anti-veille app d'abord) | F | ✅ (Mac) |
| Vidage cache / reset | **API `/reset` (chantier à shipper)** · en attendant : kill + `devicectl copy` fichier vide sur state.db | F | ✅ après reset |

**Trou à construire (dépendance)** : l'injection réseau réelle (latence/perte/coupure) n'existe
PAS dans le repo — c'est un module d'orchestrateur à écrire (`pfctl`/`dnctl` sur Mac & NAS,
profils NLC pour iOS). Idem l'orchestrateur flotte lui-même, et l'API `/reset`.

## 5. Les scénarios gradués — #1 → #21, de plus en plus durs

Chaque scénario : **préconditions → faute(s) → trafic → invariants vérifiés → durcissement**.
Un niveau ne se débloque que si le précédent PASSE (les invariants durs I1-I8 s'appliquent partout).

### Niveau 0 — Socle (doit passer les yeux fermés)
- **#1 Sanity unicast** — A→B, 20 msgs. Inv : I1,I2. Baseline RTT. *Durcir : +débit, +tailles.*
- **#2 Broadcast** — A→tous via `/sendall`. Inv : I1,I2 sur chaque pair.

### Niveau 1 — Charge & contenu
- **#3 Message lourd** — 1→8 Mo, ladder géométrique. Inv : I1 + réassemblage correct + plafond
  anti-DoS respecté (pas d'OOM, cf. `tom-reassembly-memory-dos`). *Durcir : 8→64 Mo, N en parallèle.*
- **#4 Rafale soutenue** — 250 msgs/s pendant 60 s. Inv : I1,I5 (pas de gel sous backpressure).
- **#5 Répétition / idempotence** — même message rejoué 100×. Inv : **I8** (1 seule livraison
  app), ACK dupliqué non recompté. *Durcir : rejeu concurrent multi-source.*

### Niveau 2 — Absence & retour
- **#6 Destinataire hors ligne** — A→B(tué), 20 msgs, B revient à t+30 s. Inv : I1 (backup rejoué
  à 100 %), I2. *Durcir : B absent 1 h ; volume backup élevé ; TTL proche.*
- **#7 Kill brutal d'un pair** — SIGKILL sur un nœud en plein trafic. Inv : I3 (<15 s), I1, I4.
- **#8 Retour d'un nœud** — relance après #7. Inv : re-mesh, rattrapage backup, compteurs cohérents.
- **#9 Arrêt propre (bouton stop)** — stop puis restart. Inv : I4 (pas de zombie au restart),
  I1 (re-livraison). *Durcir : stop/start en boucle rapide (flapping).*

### Niveau 3 — Cycle de vie mobile (iOS/tvOS)
- **#10 Background court** — app en arrière-plan < 18 s puis retour. Inv : PAS de restart (grâce),
  I1, I4. *Durcir : aller-retours répétés.*
- **#11 Suspension prolongée** — écran verrouillé plusieurs minutes. Inv : backup couvre (I1),
  reprise propre au réveil, I4 (pas de zombie post-réveil).

### Niveau 4 — Réseau adverse (cible : MOBILES iPhone/iPad — pas NAS/ATV, réseaux fixes)
> Le réseau adverse (cellulaire, handover, avion) est un problème de **mobiles**. Apple fournit
> les outils sur iPhone/iPad (Network Link Conditioner dans Réglages > Développeur : profils 3G,
> LTE, très-mauvais-réseau, 100 % perte). NAS et Apple TV sont sur réseau fixe → pour eux,
> seules la coupure d'interface et la latence `pfctl` (côté Mac/NAS) s'appliquent.
- **#12 Handover réseau** — WiFi→cellulaire→WiFi sur iPhone/iPad. Inv : continuité par les autres
  pairs (I1), re-upgrade DIRECT/RELAY, pas de fausse mort. *Durcir : bascules rapides répétées.*
- **#13 Latence + perte injectées** — profil NLC (LTE dégradé, 10 % perte) sur iPhone/iPad ;
  `dnctl` +200 ms côté Mac. Inv : QUIC survit, I3, pas de faux offline. *Durcir : 40 % perte, jitter.*
- **#14 Hors-réseau / mode avion** — mode avion iPhone/iPad 60 s (ou NLC 100 % perte). Inv :
  isolation recovery (I3), I4, reprise + backup à la reconnexion (I1). *Durcir : rafales.*
- **#15 Coupure du relais** — tuer le NAS/relais. Inv : bascule DIRECT ou re-seed, I1 (0 perte).

### Niveau 5 — Cold contact (« sortir du labo » — le cœur de la vision)
- **#16 Vidage cache réseau** — reset « oublier le réseau » (garde l'identité). Inv : re-découverte
  à froid < 60 s, I1.
- **#17 Reset usine** — node_id neuf. Inv : les autres voient un nouveau nœud, convergence < 60 s,
  ancien node_id évincé au TTL (recoupe anti-ravivage 24 h).
- **#18 ⭐ Contact INCONNU et OFFLINE** (le test-signature de Malik) — A fait reset usine, saisit le
  node_id de B **jamais rencontré** ET **hors ligne à l'instant de l'envoi**, écrit. B se connecte
  plus tard. Inv : **I1** (le message atteint B via backup/découverte sans qu'ils se soient JAMAIS
  écrit), I2, I8. *Durcir : A ET B tous deux offline par intermittence ; via relais uniquement.*

### Niveau 6 — Chaos & combinatoire (le « bien hard »)
- **#19 Chaos aléatoire** — chaos-monkey durci : séquence seedée de KILL/REVIVE/SKEW/partition +
  (étage F) coupures réseau, 30–60 min. Inv : I1-I8 en continu, MIN_ALIVE respecté, jamais mort.
  *Durcir : baisser MIN_ALIVE, densifier les événements, allonger.*
- **#20 Cascade combinatoire** — fautes SIMULTANÉES : kill d'un pair + message lourd en vol +
  handover réseau d'un autre + skew d'un 3ᵉ. Inv : I1-I8. *C'est là que les couplages cachés
  sortent (le pool-lock du 17/07 était un couplage kill×statut).*
- **#21 Endurance sous chaos** — 6→24 h, injection périodique + trafic continu. Inv : I1 cumulée
  (0 perte sur des dizaines de milliers de msgs), I6 (0 crash surprise), **I7** (RSS borné),
  I5 (0 gel). *Durcir : monter à 72 h ; superposer #20 toutes les heures.*

## 6. L'orchestrateur (étage F) — structure

Un script/binaire pilote pilote (Rust dans `tom-stress`, ou script sous `scripts/chaos/`) :

```
Pour chaque scénario :
  1. PRÉPARER   — état à froid reproductible (via /reset quand dispo ; sinon kill+wipe).
                  Démarrer les nœuds cibles, attendre convergence (poll /peers).
  2. BASELINE   — capturer compteurs de départ (/metrics sur chaque nœud) + t0 collecteur.
  3. TRAFIC     — générer la charge (/send /sendall, tailles/débits du scénario).
  4. INJECTER   — appliquer la/les faute(s) au timing prévu (kill, pfctl, pmset, prompt humain).
  5. OBSERVER   — collecteur UDP :9999 (horloge Mac = référentiel unique) + poll /metrics /peers
                  /inbox /paths_by_peer.
  6. VÉRIFIER   — asserter les 8 invariants durs sur les traces. PASS/FAIL par invariant.
  7. RAPPORT    — JSON + markdown : scénario, seed, faute, invariants ✓/✗, métriques, timeline.
```

**Reproductibilité** : chaque run porte un `seed` ; la séquence de fautes en dérive. Un échec est
rejouable à l'identique (impératif — cf. le flake 1/256, un chaos non reproductible est inutile).

**Garde-fous hérités (leçons 17-18/07, gravés dans l'orchestrateur)** :
- Marqueur `EXIT=$?` en dernière ligne de chaque log + relecture shell principal (notifications
  background menteuses).
- Silence UDP ≠ mort : toujours croiser `:9091`/`/metrics` (le collecteur perd des paquets).
- Chemins absolus dans les boucles longues (PATH se corrompt).
- `ps`/PID avant tout verdict ; tuer les process de stress AVANT et APRÈS chaque scénario.
- Jamais deux scénarios réseau lourds en parallèle non prévus (contention = faux bug).

## 7. Rapport & recette du banc lui-même

- Sortie par scénario : `PASS` / `FAIL(invariant, preuve)` — jamais « ça a l'air d'aller ».
- Un `FAIL` doit nommer **l'invariant violé + la trace** (fichier collecteur, ligne, timing).
- Le banc est **durci progressivement** : à chaque feature (P0-1, reset, re-dial, anti-ravivage),
  on ajoute le scénario qui la stresse ET on monte le curseur des scénarios voisins (colonnes
  *Durcir* ci-dessus). Le banc grandit avec le code — il n'est jamais « fini ».
- Intégration CI : l'étage L (in-process) tourne en CI à chaque push (rapide, seedé). L'étage F
  tourne à la demande (campagne), semi-auto quand il faut une main humaine.

## 8. Dépendances à construire (ordre proposé)

1. **Invariants automatiques sur l'étage L existant** (chaos-monkey → asserter I1-I8, pas juste
   « responsive ») — petit, immédiat, en CI. Première brique.
2. **Squelette orchestrateur étage F** — préparer/baseline/trafic/observer/vérifier/rapport, avec
   les leviers DÉJÀ dispo (kill, API, collecteur). Couvre #1-#9 sans rien de neuf.
3. **Module injection réseau** (`pfctl`/`dnctl` Mac+NAS, profils NLC iOS) — débloque #12-#15.
4. **API `/reset`** (déjà conçue, `reset-cache-app-sortie-labo.md`) — débloque #16-#18.
5. **Protocole semi-auto** (l'orchestrateur invite Malik : « bascule l'iPhone en 4G », attend,
   mesure) — débloque le physique irréductible (#12 réel, mode avion).

## 9. Décisions (tranchées par Malik, 18/07)

1. **Étage L d'abord** ✅ : démarrer par les invariants automatiques sur le chaos-monkey
   in-process existant (asserter I1-I8, full-auto, CI, seedé). Première brique, avant l'étage F.
2. **Réseau adverse = mobiles** ✅ : les fautes cellulaire/handover/avion ne concernent que
   iPhone/iPad (pas NAS/ATV, réseaux fixes). Apple fournit les outils (Network Link Conditioner).
   Semi-auto guidé pour les bascules physiques irréductibles ; scriptabilité exacte du NLC iOS à
   confirmer au moment de coder l'étage F (ne pas surpromettre : NLC s'active dans les Réglages
   Développeur de l'appareil — activation par profil de config à investiguer).
3. **Failover hub — invariant corrigé** (review-oracle 18/07) : « < 25 s » était faux. Vrai :
   shadow ~8 s (watchdog), candidat orphelin < 35 s (`CANDIDATE_ORPHAN_TIMEOUT 30s`,
   `group/types.rs:51`). I3 mis à jour.

## 10. Prochaine étape immédiate (validée)
Coder l'**étage L** : transformer `scenario_chaos_monkey.rs` (ou un nouveau `scenario_invariants.rs`)
pour vérifier les 8 invariants durs au lieu du seul « le nœud répond », avec sortie PASS/FAIL par
invariant et seed reproductible. En CI. TDD (un invariant = un test). Puis on monte vers l'étage F.

## 11. Transparence des nœuds de test — REGISTRE (décision Malik 18/07)

**On n'isole plus, on marque.** L'herméticité parfaite des harnais n'est pas atteignable à coût
raisonnable (canal résiduel non identifié, cf. incident fantômes `invariants-N` du 18/07) et n'est
pas nécessaire en phase test : les nœuds de test sont NÔTRES, donc on les **marque**, on les
**liste**, et les vrais nœuds **avertissent** de leur présence.

### Convention de marquage
- **Préfixe de username : `TEST-`** — constante partagée `tom_protocol::TEST_NODE_PREFIX`
  (`crates/tom-protocol/src/types.rs`) + helper `is_test_node_username()`. Miroir Swift :
  `TomNodeService.testNodePrefix` (les deux DOIVENT rester identiques).
- **Obligation** : tout harnais qui spawn un `ProtocolRuntime` DOIT préfixer son username via la
  constante. Aucune exception — même un scénario « local » (l'incident du 18/07 a prouvé que
  l'isolation annoncée peut fuir).

### Comportement des vrais nœuds face à un `TEST-*`
| Surface | Comportement |
|---|---|
| tom-tui (interactif) | affiché « Nœud de test éphémère … — ignoré comme cible », jamais auto-connecté |
| tom-tui (bot NAS) | événement collecteur distinct `pair_test_trouve` (au lieu de `pair_trouve`), jamais cible ping (`select_ping_target`) |
| Apps Swift (iOS/tvOS/macOS) | badge `🧪 Nom (test éphémère)` via `displayName(for:)` + picker Messages (`TomPeer.isTestNode`) |

Le protocole lui-même ne discrimine PAS les nœuds de test (pas de refus de routage — décision #6,
couche invisible ; c'est de l'affichage et de la sélection de cible, pas du protocole).

### Registre des harnais émetteurs (tom-stress, tous préfixés)
| Harnais | Usernames émis |
|---|---|
| `scenario_invariants.rs` (banc étage L) | `TEST-invariants-{n}` |
| `scenario_chaos.rs` / `scenario_chaos_monkey.rs` | `TEST-chaos-*` |
| `scenario_presence_storm.rs` | `TEST-storm-{n}` |
| `campaign.rs` / `responder.rs` (Mac ↔ NAS) | `TEST-{nom de config}` |
| `fleet_probe.rs` | `TEST-fleet-probe` |
| autres scénarios (backup, churn, e2e, failover, group, partition, presence, presence_attack, roles, endurance) | `TEST-alice`, `TEST-bob`, … |

⚠️ Règle d'exécution : le banc `invariants` reste exécutable près de la vraie flotte UNIQUEMENT
depuis le build où la flotte affiche les badges (build ≥ 117) — avant ça, les nœuds de test
apparaissent comme des pairs anonymes/fantômes.
