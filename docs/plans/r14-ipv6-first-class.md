# R14 — IPv6 first-class : baseline mesurée + conception

> Créé le 2026-07-18 (soir). Chantier roadmap `vault/40-roadmap.md` R14.
> Méthode : MESURE d'abord (2 sources croisées, leçon observabilité), design-doc AVANT code,
> red-team, canari avant flotte.

## §1 Baseline v4/v6 — flotte build 125 (mesurée 2026-07-18 ~21h)

Sources croisées : `paths_by_peer` (:9091, 2 snapshots à 60 s d'intervalle, T1 19:01 / T2 19:02 UTC)
× collecteur UDP :9999 (`/tmp/tom_collector.log`, fenêtre 2000 lignes ≈ 13 min).
Flotte : Mac (127.0.0.1 / 192.168.0.82), iPhone (.28), iPad (.23), NAS (.83, tom-chat).
ATV retirée (18/07 soir). Aucun nœud TEST-* dans la fenêtre (pollution harnais nulle).

### Matrice des chemins actifs (T1 = T2, 100 % stable sur 60 s)

| De → Vers | Famille | Adresse | Kind | RTT |
|---|---|---|---|---|
| Mac → iPad | **v6-GUA** | `[2a01:e0a:14f:5da0:18f3:…]:60085` | DIRECT | 9 ms |
| Mac → NAS | **v6-GUA** | `[2a01:e0a:14f:5da0:248f:…]:43925` | DIRECT | 5 ms |
| Mac → iPhone | **v6-GUA** | `[2a01:e0a:14f:5da0:49bf:…]:64562` | DIRECT | 30 ms |
| iPad → Mac | **v6-GUA** | `[…:f42f:a9e9:820:861d]:55480` | DIRECT | 10 ms |
| iPad → NAS | **v6-GUA** | `[…:248f:5dff:fea5:8ed1]:43925` | DIRECT | 7 ms |
| iPad → iPhone | v4-LAN | `192.168.0.28:49945` | DIRECT | 9 ms |
| iPhone → iPad | v4-LAN | `192.168.0.23:59455` | DIRECT | 13 ms |
| NAS → Mac | **v6-GUA** | `[…:f42f:a9e9:820:861d]:55480` | DIRECT | 4 ms |
| NAS → iPad | **v6-GUA** | `[…:18f3:a850:f526:8eb3]:60085` | DIRECT | 5 ms |

Collecteur (source 2) : **2000/2000 événements path=DIRECT, 0 RELAY, 0 flap/re-dial** sur ~13 min.
Concordance totale avec :9091.

### ⚠️ Lecture initiale — SUR-INTERPRÉTÉE, corrigée en §1bis

J'avais conclu de ce snapshot : « v6 majoritaire 7/9, l'iPhone est l'exception tout-v4 ».
**C'était faux** : conclure d'une photo instantanée que l'état est stable. Voir §1bis.

## §1bis Ce que 3 séries de mesures montrent VRAIMENT (le vrai résultat de R14 étape 1)

Trois relevés successifs (~21h01, ~21h42-21h45, ~22h10) sur la même flotte inchangée :

| Chemin | Relevé 1 (21h01) | Relevé 2 (21h42→21h45) | Relevé 3 (22h10) |
|---|---|---|---|
| Mac → iPad | **v6** 9 ms | — | **v4** 10 ms |
| Mac → iPhone | **v6** 30 ms | v4 11 ms (stable ×3) | **v6** 10 ms |
| iPhone → iPad | **v4** 13 ms | — | **v6** 15 ms |
| iPhone → Mac | *absent* | — | **v4** 10 ms |
| iPad → iPhone | **v4** 9 ms | v4 11 ms → **bascule v6** 32 ms | **v6** 51 ms |
| iPad → Mac | v6 10 ms | — | v6 9 ms |
| NAS → iPhone | — | v6 12 ms (stable ×3) | — |

**Constat 1 — le choix v4/v6 est instable.** Presque chaque chemin a changé de famille en une
heure, sans que rien ne change dans la flotte (mêmes nœuds, même build 125, même LAN).

**Constat 2 — et il ne converge PAS vers le meilleur chemin.** `iPad → iPhone` est passé de
**v4 à 9 ms** à **v6 à 51 ms**, et y reste. Le système a basculé vers un chemin ~5× plus lent.
C'est l'inverse de ce que la logique de sélection est censée produire.

**Ceci réoriente entièrement R14.** Le problème n'est pas « pas assez d'IPv6 » (il y en a
déjà partout). Le problème est que **le choix de chemin est essentiellement une loterie qui ne
converge pas vers l'optimum** — et forcer une « préférence v6 » par-dessus une loterie ne ferait
que biaiser le tirage, sans traiter la cause.

### La cause, prouvée dans le code (pivot tranché par 4 analyses + vérification personnelle)

L'ordre des adresses candidates **est aléatoire**, ni v4-d'abord ni parallèle :
- `EndpointAddr.addrs` est un `BTreeSet` qui trie effectivement v4 avant v6
  (`tom-base/src/endpoint_addr.rs:44-51`, `Ord` dérivé, `Relay<Ip` et `SocketAddr::V4<V6`)…
- …mais cet ordre est **perdu à l'insertion** : les chemins sont stockés en `FxHashMap`
  (`tom-connect/src/socket/remote_map/remote_state/path_state.rs:38`, itérés par `addrs()` L171).
- Idem côté hole punching : `remote_addresses: FxHashMap<VarInt, (IpPort, bool)>`
  (`tom-quinn-proto/src/iroh_hp.rs:91`), probé par une boucle sur cette map (L152), séquentielle
  et non parallèle. Le code **assume** ce désordre : commentaire L196 « this being random depends
  on iteration not returning always on the same order ».

Puis l'hystérésis fige le résultat du tirage : une fois un chemin établi, la bascule exige
`current_rtt >= new_rtt + RTT_SWITCHING_MIN_IP` (5 ms) — donc un chemin médiocre gagné par
hasard est conservé tant que l'alternative n'est pas 5 ms meilleure. Ça n'explique cependant
pas la bascule v4 9 ms → v6 51 ms observée (dégradation), qui reste **à élucider** : piste la
plus probable, le chemin v4 est mort (changement de port NAT/interface) et le v6 a été
ré-établi en remplacement, le RTT élevé étant celui d'un chemin fraîchement monté.

### Fait mesuré sur les adresses (T2 confirmé)

`ifconfig en0` du Mac : **deux GUA sur le même /64** — une stable (`…:1cfc:2a4e:8593:522`,
flag `secured`) et une temporaire (`…:f42f:a9e9:820:861d`, flag `temporary`), avec
`net.inet6.ip6.use_tempaddr=1`. C'est **la temporaire** qui sort et que les pairs voient
(confirmé : les chemins v6 vers le Mac visent `f42f:…`). Les privacy extensions sont donc
actives et load-bearing — T2 du §2.1 est réel, pas hypothétique.

### Bind : deux sockets séparés, IPv6 optionnel

`tom-connect/src/socket/transports/ip.rs:164-173` + `transports.rs:92-123` : un socket par
famille (v4 **required**, v6 **optional**), pas un dual-stack unique ; si le bind v6 échoue il
est ignoré silencieusement (`ip.rs:415-445`). Un nœud peut donc être v4-only sans le signaler.

### Anomalies consignées (hors périmètre R14, à traiter séparément)

1. **Chemin résiduel vers nœud mort** : `75baa468…@192.168.0.82:52655` présent dans le
   `paths_by_peer` de TOUS les nœuds mais dans aucun `pairs_connectes`. 192.168.0.82 = le Mac
   lui-même (arp `permanent`) ; port 52655 = un nœud de harnais de la nuit (TEST-load, tués).
   → `paths_by_peer` retient des chemins vers des pairs morts ; candidat purge alignée
   TOPOLOGY_TTL/liveness. Idem `iPad → 82.67.95.8:61809` (résiduel via IP publique Freebox).
2. **iPhone → Mac absent de paths_by_peer** alors que Mac ∈ `pairs_connectes` de l'iPhone (et Mac
   voit l'iPhone en v6). Asymétrie d'observabilité (singleton last_path déjà vu au #33 ?) à
   revérifier après redeploy XCFramework ≥123.
3. **NAS `messages_echoues=56` figé** (n'augmente plus ; envoyés/reçus progressent). Probable
   accumulation ancienne (uptime 1885 s) — auditer journalctl au prochain passage NAS.
4. `taille_reseau` Mac 210 / iPhone 348 / iPad 396 : compte topologie gossip/DHT (pollution
   harnais <24 h, élaguée par TTL 24 h posé build 122), PAS les connexions. Rappel : jamais un
   proxy de santé.

### Ce qui manque à l'observabilité pour piloter R14

- `paths_by_peer` ne donne pas la famille explicitement (on la parse de l'addr) — suffisant.
- Le collecteur ne loggue PAS l'adresse du chemin (juste DIRECT/RELAY) → pour mesurer un
  avant/après R14 côté collecteur, ajouter la famille (v4/v6) à l'événement de connexion
  serait utile (1 champ, décision #6 respectée : observabilité interne, pas UI).

## §2 Conception — préférence v6 + publication GUA au rendez-vous

### §2.1 État des lieux du code (cartographie vérifiée sur pièces 2026-07-18)

Beaucoup de R14 existe DÉJÀ — le design part de là, pas de zéro :

| Aspect | État | Preuve (file:line) |
|---|---|---|
| Collecte GUA v6 locales | ✅ collectées (`regular6`), fe80 filtré, ULA en fallback | `patches/netwatch-0.13.0/src/ip.rs:39-101` |
| Publication DHT des v6 | ✅ `extract_node_addrs()` publie TOUTES les addrs, sans filtre | `runtime/loop.rs:1064-1083` |
| Lecture DHT filtrée | ✅ `direct_addr_is_dialable()` + cap `MAX_DHT_ADDRS=32` | `runtime/loop.rs:1089-1110` |
| PeerAnnounce gossip | ✅ ne porte AUCUNE adresse (identité+rôles) — neutre v4/v6 | `runtime/state.rs:717-725`, `discovery/types.rs` |
| Préférence v6 (sélection) | ✅ `select_v4_v6()` : v6 gagne si `rtt_v6 <= rtt_v4 + 3 ms` (`IPV6_RTT_ADVANTAGE`) ; switch de chemin seulement si gain > 5 ms (`RTT_SWITCHING_MIN_IP`) | `tom-connect/src/socket/remote_map/remote_state.rs:78-81,1160-1223` |
| Hole punching v6 | ✅ symétrique v4/v6 (`map_to_local_socket_family`) | `tom-quinn-proto/src/iroh_hp.rs:113-168,516-524` |
| Port 43925 | = option CLI `--bind-addr/--bind-port` du NAS (bind QUIC fixe), pas une constante | `tom-tui/src/main.rs:679-689` |
| `paths_by_peer` | construit côté FFI depuis `PathEvent` transport | `tom-protocol-ffi/src/lib.rs:55,556-572,1353-1389` |

Trous réels :
- **T1 — la préférence v6 ne joue qu'entre chemins DÉJÀ probés** (RTT connus). Rien ne
  garantit qu'un candidat v6 soit probé au dial initial — pas de happy-eyeballs (RFC 8305).
  C'est l'hypothèse n°1 pour expliquer l'iPhone tout-v4 de la baseline (§1) alors que sa GUA
  est publiée et joignable (le Mac le joint en v6).
- **T2 — temporary addresses (RFC 4941 privacy extensions) non filtrées** à la collecte :
  on publie potentiellement des GUA éphémères (valides quelques heures) → chemins qui meurent,
  entrées rendez-vous périmées. iOS en génère agressivement — suspect n°2 pour l'iPhone.
- **T3 — observabilité famille absente** : ni `paths_by_peer` ni le collecteur n'exposent
  v4/v6 explicitement (on parse l'addr). Pas d'avant/après mesurable sans ça.
- **T4 — publication non triée/bornée à la source** : `extract_node_addrs()` envoie tout,
  dans un ordre non maîtrisé ; le tri utile (GUA v6 d'abord) n'existe nulle part.

### §2.1bis ⚠️ Ce plan est DÉPASSÉ par §1bis — voir §2.4 pour la version retenue

Les lots ci-dessous ont été écrits avant les mesures répétées et avant le pivot. Ils partent
d'une prémisse invalidée (« il faut plus de v6 »). Conservés pour la traçabilité du raisonnement.

### §2.2 Ce que R14 change (proposition INITIALE, dépassée)

**Lot A — Observabilité d'abord (aucun risque protocole, mesurable immédiatement)**
1. `PathInfoFFI.addr_family` (`"v4" | "v6" | "relay"`) + même champ dans l'événement
   collecteur de connexion. Décision #6 respectée : observabilité interne/debug, pas UI.
2. Événement `tracing` au dial : liste des candidats par famille + lequel a été probé/choisi.
   → permet de TRANCHER T1 vs T2 sur l'iPhone avec des faits, avant tout changement de dial.

**Lot B — Hygiène des adresses publiées (défensif, faible risque)**
3. Filtrer les temporary addresses à la collecte quand l'OS les marque (macOS/iOS :
   `IN6_IFF_TEMPORARY`) ; à défaut, préférer l'adresse v6 stable si plusieurs GUA du même /64.
4. Trier `extract_node_addrs()` : GUA v6 stables d'abord, puis v4, borner à la source
   (miroir du cap 32 lecture).

**Lot C — Dial initial v6-first (le cœur, APRÈS mesure Lot A)**
5. Au dial d'un pair avec candidats v4+v6 : probe v6 en premier, v4 déclenché ~250 ms plus
   tard si v6 n'a pas répondu (happy-eyeballs allégé, dans tom-connect). Jamais de « v6
   only » : v4 reste le filet.
6. Canari (1 nœud, ex. Mac) avant flotte, mesure avant/après via Lot A + runbook
   (`docs/plans/RUNBOOK-TESTS.md`).

**Lot D — PCP pinhole (étape 3 roadmap, hors périmètre immédiat)** : ouvrir le pare-feu v6
de la box par PCP quand disponible — dépend de la mesure post-règle-43925, doc séparée.

### §2.3 Red-team (à challenger par review indépendante AVANT code)

- **Exposition GUA** : publier des GUA v6 routables au rendez-vous DHT public = déjà le cas
  aujourd'hui (constat §2.1, pas un ajout R14). Risques : scan/tracking des GUA (mitigé par
  rotation privacy ext… qu'on veut justement filtrer — tension T2 : préférer l'adresse STABLE
  augmente la traçabilité long-terme d'un device). À trancher : publier la GUA stable
  (fiabilité) vs temporaire (privacy). Position proposée : stable pour les nœuds
  d'infrastructure (NAS, relais embarqué), temporaire ACCEPTÉE pour les mobiles tant que le
  refresh rendez-vous (cycle existant) réécrit l'entrée plus vite que la rotation d'adresse.
- **Amplification dial** : un attaquant publie une entrée rendez-vous avec des addrs v6 de
  victimes → nos nœuds les dialent. Mitigé existant : signature Ed25519 des entrées
  (rendezvous_entry_authentic, loop.rs:862-877) + cap 32 + un dial QUIC échoué est borné.
  Happy-eyeballs Lot C ne doit PAS multiplier les dials (v6 puis v4 en escalier, pas en
  parallèle ×N).
- **Pare-feu v6 ouvert (règle 43925)** : n'ouvre QUE le port QUIC du NAS ; le service derrière
  est le transport ToM (TLS + NodeId). Surface = celle d'un relais public assumé.
- **Pas de changement wire** : aucun invariant `tom-*` touché ; formats DHT inchangés (les
  addrs sont déjà des `SocketAddr` sérialisés, v6 inclus).

### §2.3bis TRANCHÉ par la mesure (19/07) — pas de « double tunnel »

Question ouverte hier : l'asymétrie observée (A→B en v6, B→A en v4) signifie-t-elle **deux
connexions parallèles** entre les mêmes appareils (donc du gaspillage), ou un artefact de
lecture ? **Tranché par capture réseau (`tcpdump` sur le NAS, 1 274 paquets).**

Pendant un envoi de 12 × 100 Ko NAS→Mac :
- **861 paquets de données sur UN SEUL chemin** v6 (`…248f:…:43925` → `…f42f:…:55480`),
- **0 paquet de données** sur le chemin v4 vers le même pair,
- retour du Mac : 24 paquets de données **sur le même couple d'adresses** (donc symétrique),
- les autres adresses du même pair ne reçoivent que des **sondes de 8 octets**.

**Conclusion : une seule connexion QUIC, plusieurs chemins candidats maintenus et sondés, un
seul actif pour les données.** Il n'y a ni double tunnel, ni double processus, ni gaspillage
de bande passante. Le multipath QUIC fait déjà « observer plusieurs chemins et garder un
secours » — le trou de R14 est donc uniquement la **ré-évaluation** (hystérésis 5 ms qui fige
le premier choix), pas l'absence de mécanisme de choix.
⚠️ Une première tentative de cette mesure a été **polluée** : le NAS était alors étouffé
(§2.3ter) et ses « sondes simultanées vers 3 adresses » étaient des tentatives de reconnexion
d'un nœud isolé, pas du multipath sain. Mesurer sur un nœud dégradé produit une fausse théorie.

Note pour le Lot D : les données passent par la GUA **temporaire** du Mac (`f42f:…`), pas la
stable (`1cfc:…`) — cohérent avec les privacy extensions actives, et confirme qu'il ne faut
PAS filtrer les temporaires (c'est le chemin réellement utilisé).

### §2.3ter Bug CRITIQUE trouvé pendant cette mesure — backup store non borné en octets

Voir mémoire `tom-backup-store-oom-2026-07-19`. Résumé : `backup/store.rs:14`
`MAX_TOTAL_MESSAGES = 10_000` borne la **cardinalité**, pas le **volume** ; `BackupEntry.payload`
est un `Vec<u8>` entier en RAM (`backup/types.rs:50-54`) ; aucun cap en octets.
Terrain : NAS à **688 Mo sur 920** après 13 h, **OOM-killer déjà passé** (771 Mo RSS tués),
**8 366 échecs** (56 la veille), **0 pair** — tout en affichant `phase: "connecte"` et en étant
vu « DIRECT v6 7 ms » par le Mac. Preuve A/B au redémarrage : **688 Mo → 24 Mo, 0 → 5 pairs,
8 366 → 0 échec**, même binaire et même réseau.
Contributeur : les campagnes de charge de la veille (1/2/4/8 Mo) — **un test légitime peut tuer
un nœud de production**. 3ᵉ récidive de la classe « borne par-unité sans budget global »
(cf. large-message-dos, reassembly-memory-dos). Fix à concevoir : budget en octets + éviction
par volume, dimensionné selon la RAM de l'hôte.

### §2.4 Conception RETENUE — « convergence de chemin », pas « préférence v6 »

Fondée sur §1bis (mesures) et le pivot (code). Le chantier change de nom et d'objectif :
faire converger le choix de chemin vers le meilleur lien disponible, dont IPv6 bénéficiera
naturellement (RTT v6 ≤ v4 mesuré partout où les deux existent).

**Lot A — Observabilité (prérequis absolu, aucun risque)**
La bonne nouvelle : `AddrFamily` **existe déjà** (`tom-transport/src/protocol.rs:379,421-440`)
et un `PathEvent` est justement émis sur changement de famille v4↔v6 (L401-410, commentaire :
« on garde le signal v4↔v6, load-bearing pour le diag LAN »). Il ne reste qu'à :
1. exposer la famille dans `paths_by_peer` (aujourd'hui déductible de l'`addr`, mais non explicite) ;
2. **historiser les bascules** : compteur de changements de famille par pair + dernier motif
   (chemin mort ? meilleur RTT ? nouvelle adresse ?). Sans ça on ne peut ni mesurer la stabilité
   ni valider un fix — c'est exactement le trou qui m'a fait sur-interpréter §1.

**→ LIVRÉ (build 128, 2026-07-19).** Réalisation :
- `AddrFamily` promu public dans `tom-transport/src/path.rs` ; `PathEvent` porte
  `family`, `prev_family: Option<_>`, `prev_rtt: Option<_>` — renseignés PAR LE WATCHER,
  par-connexion (`protocol.rs::spawn_path_watcher`). `prev_family = Some` ⟺ l'événement
  est une bascule. C'est la vérité transport, pas un diff de map par-pair (qui mélangerait
  les connexions multiples d'un même pair). Le `last_rtt` est rafraîchi à CHAQUE observation
  du stream (pas seulement aux bascules) pour que `prev_rtt` reflète la dernière mesure du
  chemin quitté.
- `paths_by_peer` expose `family`, `switches` (compteur cumulatif), `last_switch`
  (« v4 9ms → v6 51ms »), `last_switch_at_ms` — côté FFI (`PathInfoFFI`, contrat verrouillé
  par `types::tests::path_info_json_keys_match_swift_decoder`) ET côté NAS
  (`tom-tui/src/main.rs::track_path_event`).
- Ligne collecteur enrichie : `🔀 Chemin xxx → DIRECT v6 [addr] (bascule v4 9ms → v6 51ms)`
  (`TomModels.swift` displayLine ; nouveaux champs `path_family`/`prev_family`/`prev_rtt_ms`
  dans `ProtocolEventFFI` et `TomProtocolEvent`, tous Optional côté Swift — contrat Codable).
- `scripts/path-matrix.py` : Mac en `[::1]` (piège IPv6-only), NAS `:8085` normalisé
  (tableau → dict), et exploitation des compteurs nœud (`BASCULE ×N (vue nœud)`) qui voient
  les bascules survenues ENTRE deux relevés.
- Le « motif » complet (chemin mort ? meilleur RTT ?) reste au Lot B : le watcher n'observe
  que le chemin SÉLECTIONNÉ ; la raison de la re-sélection vit dans tom-connect
  (`remote_state.rs::select_v4_v6`). `prev_rtt` vs `rtt` donne déjà le sens
  (amélioration/dégradation) de chaque bascule — suffisant pour DÉTECTER le cas dégradant
  en direct et déclencher la capture Lot B.

**Lot B — Élucider la bascule dégradante (expérience, pas du code)**
Instrumenter puis observer une bascule v4→v6 dégradante en direct (le cas `iPad → iPhone`
9 ms → 51 ms). Question à trancher : chemin mort remplacé, ou sélection défaillante ?
La réponse détermine si le fix est dans la détection de mort de chemin ou dans la comparaison
de RTT. **Aucun code de sélection ne doit être écrit avant cette réponse.**

**→ VERDICT (19/07 après-midi, données du Lot A — logger `/tmp/lotb-paths.jsonl`, 40 min,
flotte 129) : CHEMIN MORT REMPLACÉ + ABSENCE DE RE-SONDAGE, pas une sélection défaillante.**
Trois faits mesurés, croisés sur les deux sens Mac↔iPad :
1. **Les bascules à perte sont des failovers.** Observé « v6 10ms → v4 12ms » (Mac→iPad, ×7)
   et « v4 7ms → v6 18ms » (iPad→Mac) : l'hystérésis (`RTT_SWITCHING_MIN_IP` = 5 ms,
   `remote_state.rs:78-81`) interdit une bascule-comparaison vers un chemin plus lent —
   si elle a lieu, le chemin courant était invalidé. Le suivi PAR-CONNEXION du watcher
   (Lot A) exclut l'artefact multi-connexions.
2. **Le chemin v6 renaît sur des adresses DIFFÉRENTES** : 3 IID v6 distincts de l'iPad en
   40 min (`18f3:…`, `b1a2:…`, `f13d:…`, port 60706 constant) — le pair tourne entre ses
   adresses v6 (stable + temporaires) ; chaque cycle mort/renaissance passe par un
   re-probe d'un candidat différent. Le chemin v4, lui, n'est JAMAIS mort
   (`192.168.0.23:64466` constant sur toute la fenêtre).
3. **Après un failover vers un chemin pire, RIEN ne re-sonde le chemin perdu** :
   iPad→Mac est resté à v6 18 ms ≥ 25 min alors que le v4 valait 7 ms. Un chemin mort
   n'est plus mesuré → la comparaison RTT n'a plus de candidat → l'hystérésis n'a rien à
   comparer. **C'est le mécanisme exact de la non-convergence** (et l'explication probable
   du cas originel 9 ms → 51 ms resté).

Résiduel (assumé) : pas de capture paquets au moment des morts (tcpdump indisponible sur
le Mac en session autonome) — l'inférence « bascule à perte ⇒ failover » repose sur la
règle d'hystérésis lue dans le code, pas sur l'observation du silence du chemin. Et le
POURQUOI des morts fréquentes de chemins v6 vers un appareil iOS (suspect n°1 : power
save WiFi / ND) reste à élucider — possiblement hors de notre code.

**Conséquence pour le Lot C — il change de nature** : le tri déterministe des candidats
(`iroh_hp.rs`) n'adresse PAS ce mécanisme. Le fix candidat devient : **re-sonder
périodiquement les candidats inactifs** (dont les morts récents) pour redonner à la
comparaison RTT + hystérésis la matière pour rebasculer vers le meilleur lien. À
design-first avant toute ligne (décision requise : cadence de re-probe vs coût sondes).

**Lot C — Déterminisme du probe (conditionné au Lot B)**
Si le Lot B montre que le tirage aléatoire coûte des chemins médiocres : trier les candidats
avant probe (`iroh_hp.rs:152` — collecter puis `sort_by` sur une clé stable, ou remplacer la
`FxHashMap` par une `BTreeMap`), plutôt qu'ajouter une préférence v6 par-dessus le hasard.
⚠️ Le commentaire `iroh_hp.rs:196` indique qu'un autre mécanisme **s'appuie** sur l'aléatoire —
à ne pas casser : vérifier `continue_nat_traversal_round` avant de toucher à l'ordre.

**Lot D — Hygiène des adresses (indépendant, faible risque)**
Les privacy extensions sont actives et fonctionnent (§1bis). Ne PAS filtrer les temporaires :
elles sont ce qui sort réellement, et le /64 est de toute façon commun aux deux (donc filtrer
n'apporterait aucune privacy — argument tranché). Le vrai risque est une adresse temporaire
**périmée** publiée au rendez-vous : vérifier que le cycle de republication est plus court que
la rotation RFC 4941 (24-48 h) — il l'est (tick 60 s), donc **rien à faire**. Lot clos.

**Abandonné** : préférence v6 forcée, happy-eyeballs, filtrage IN6_IFF_TEMPORARY, règle
pare-feu Freebox. Aucun n'est justifié par les mesures.

### §2.5 Ce que le red-team avait FAUX (pour ne pas propager de peur infondée)

- « BTreeSet ⇒ v4 dialé en premier, BLOQUANT » → **faux** : l'ordre est perdu en `FxHashMap`,
  le probe est aléatoire. Le problème existe mais n'est pas celui-là.
- « Filtrer IN6_IFF_TEMPORARY infaisable ⇒ Lot B s'effondre » → **sans objet** : le filtrage
  ne doit pas être fait du tout (§2.4 Lot D).
- « Baseline insuffisante » → **fondé sur le principe**, et confirmé au-delà : la baseline
  n'était pas seulement incomplète, elle était **trompeuse** (photo instantanée d'un état
  instable). Leçon [[observability-must-reflect-ground-truth]] rejouée à mes dépens.

## §2bis Décisions LOCKED — conformité

Invisibilité (#6) : tout est interne/debug. L1 (#3) : non concerné. Pas d'état user-visible,
pas de ban, pas de persistance > TTL. Le tri/préférence v6 est topologie-driven (#6 réseau).

## §3 Règle pare-feu Freebox 43925 — ABANDONNÉE (décision Malik, 2026-07-18)

La roadmap listait « règle pare-feu Freebox 43925 » comme étape 1 de R14. **Écartée** : on ne
touche pas à la configuration de la box (le réseau fonctionne, on ne prend pas ce risque pour un
gain non prouvé). Ce n'est pas un blocage : la VM NAS est sous notre contrôle total, et la mesure
hors-LAN qui manque (§2.1 T1) se fait sans la box — **iPhone en cellulaire pur** (sort par
l'opérateur, ne dépend pas du pare-feu entrant Freebox).

Conséquence pour R14 : le volet « joignabilité entrante v6 de la maison depuis l'extérieur » sort
du périmètre. R14 se concentre sur ce qui est sous notre contrôle : ordre/choix des adresses au
dial, hygiène des adresses publiées, observabilité de la famille.
