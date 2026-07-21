# Banc « rôles sous charge » — exercer l'organisme, pas le tuyau

> Design-first (2026-07-20 soir). AUCUN code tant que ce doc n'est pas stabilisé
> (règle projet : doc de conception avant featur protocolaire).
> Fondation : `docs/plans/prisme-des-roles.md` (relecture intégrale des notes,
> grille R1-R8) + mandat verbatim de Malik (`PROMPT-REPRISE-ROLES.md` §0).
> Juge : `docs/plans/charte-cibles-agressives.md` (§3 — critères pré-enregistrés).

## §0 Pourquoi ce banc

Le banc « courbe de masse » (§2bis de `banc-courbe-masse.md`) ne juge que
l'intégrité send/ACK entre nœuds homogènes : il est **aveugle aux rôles** —
relais multi-hop, backup, rendez-vous, subnets, promotion/rétrogradation,
failover, PoP, anti-spam. Or c'est LÀ que vit l'organisme (whitepaper §3-§6,
Master Map §4). Ce banc-ci prouve que **les rôles font leur travail SOUS
charge, ensemble, sans se casser mutuellement** — la thèse « organisme vivant »
de la charte (§1).

**Périmètre honnête** : on exerce les rôles QUI EXISTENT dans le code (L0 +
primitifs L1-001/003). La sélection en cascade, l'entropie, les
observateurs/validateurs pleins et le carnet de rendez-vous TOURNANT ne sont
pas construits (jalons M1.2-M1.6) — le banc ne peut pas les tester, il le dit.

## §1 Principes hérités (non négociables)

1. **Hermétique par défaut** : tout nœud de banc coupe le trio découverte
   (`n0_discovery(false)`, `local_discovery(false)`, `enable_dht: false`).
   Exception UNIQUE : R4 (rendez-vous), qui exige un **namespace DHT de test
   dédié** (§5-P1) — jamais celui de la production.
   ⚠️ **Trou d'herméticité ENTRANT découvert par R7 (21/07)** : le trio coupe
   la découverte **sortante** du nœud de banc, mais PAS l'acceptation de
   connexions **entrantes**. Quand des apps de la flotte tournent sur la
   **même machine**, leur mDNS actif trouve le nœud de banc et lui pousse leur
   carnet par gossip → il les liste en `Known` (jamais `Online` : pas de
   travail constaté, le PoP tient). **Même cause que le « chatter du canal
   applicatif » du banc courbe** (`banc-courbe-masse.md` §2bis : 24-43 msg
   non-banc filtrés = la flotte locale). Conséquence : ne jamais fonder un
   oracle sur un **compte ABSOLU** (`online_count`, nombre de pairs) — juger
   RELATIVEMENT aux nœuds du banc identifiés. À traiter pour l'étage L : soit
   kill la flotte avant un run, soit refuser les connexions non-câblées (à
   investiguer : `local_discovery(false)` coupe-t-il l'écoute mDNS entrante ou
   seulement la publication ?).
2. **La vérité = le collecteur + oracles in-process**, pas les impressions
   (findings Phase 0 : compteurs :9300/:9091 = ENVELOPES pas messages ; :9091
   retardé < 2 min ; vérité = seq collecteur + /inbox).
3. **Critères pré-enregistrés** : chaque scénario fixe son verdict AVANT le run
   (pattern charte §3). Un run sans critère pré-enregistré n'est pas publiable.
4. **Événements orchestrés marqués `TEST-*`** (transparence : jamais confondre
   une panne orchestrée avec un événement spontané — leçon du 18/07).
5. **Kill discipline** : `ps aux` avant, kill explicite après, jamais de
   process de banc laissé vivant.
6. **Chaque scénario déclare ce qu'il NE prouve PAS** (leçon courbe : le
   « ×1.00 = PASS » tautologique).

## §2 Les huit scénarios

Notation : **[L]** = étage in-process hermétique (CI-able), **[F]** = étage
flotte réelle multi-host (orchestrateur + collecteur).

### R1 [L] — Relais multi-hop : le facteur fait passer, ne stocke pas

- **Montage** : 3 nœuds in-process A, R, B. A⇄R et R⇄B câblés
  (`add_peer_addr`) ; A n'a AUCUNE adresse directe de B — il n'a que
  l'existence de B dans sa topologie (via annonce portée par R). L'émission
  A→B doit produire un `RoutingAction::Forward` chez R (relais PROTOCOLAIRE,
  `PeerRole::Relay` — pas le relais transport tom-relay).
- **Oracle** : (a) B reçoit le message intact (E2E : R n'a jamais le clair) ;
  (b) le compteur de relais de R a crédité exactement ce forward
  (`ContributionMetrics`) ; (c) R est pass-through : aucun résidu du message
  chez R après livraison (pas d'entrée backup, pas de rétention) ; (d) preuve
  wire du multi-hop : TTL décrémenté d'un hop chez B.
- **Ne prouve pas** : la sélection de relais sous topologie riche (un seul R
  candidat), le hole punching réel (loopback).

### R2 [L+F] — Backup/gardien : le virus garde, restitue, s'efface

- **Montage [L]** : A, G, B in-process. B éteint. A envoie → le message se
  réplique en backup chez G (ADR-009). B revient → livraison différée → purge.
  Variante TTL : B ne revient jamais → purge à expiration (TTL court légal,
  toujours ≤ 24 h — jamais toucher le plafond LOCKED #2).
- **Oracle** : (a) pendant l'absence : 1 copie chez G (inspection
  `BackupCoordinator`), 0 chez A au-delà du pending borné ; (b) au retour :
  livraison exactement-une-fois (clé (nœud,seq) du banc courbe réutilisée) ;
  (c) après ACK : store de G **vide** (auto-suppression) ; (d) variante TTL :
  store vide à l'échéance, message JAMAIS livré ensuite.
- **Montage [F]** : même logique sur flotte réelle : un device réel éteint
  (action orchestrée `TEST-*`), messages envoyés pendant l'absence SOUS charge
  de fond, rallumage, mesure de la livraison différée au collecteur.
- **Ne prouve pas** : le choix du MEILLEUR hôte backup (survival_score) sous
  churn massif — scénario dédié plus tard si besoin.

### R3 [L+F] — Promotion/rétrogradation : le rôle vient de la contribution

- **Montage [L]** : faire transiter du travail réel par un nœud N (relais
  crédités) jusqu'au franchissement du seuil → vérifier la promotion
  Peer→Relay et son ANNONCE (`RoleChangeAnnounce`) ; puis stopper le travail
  et vieillir le score (decay 5 %/h, `roles/scoring.rs` — temps injecté, pas
  d'attente réelle) → rétrogradation.
- **Oracle** : (a) promotion au seuil exact, visible des autres nœuds ;
  (b) rétrogradation en FADE (progressive), (c) **jamais de ban** : à score
  plancher le nœud envoie/reçoit toujours (LOCKED #4) ; (d) le rôle est
  réseau-imposé : aucune API « je veux être relais » n'existe dans le chemin.
- **Montage [F]** : observer sur la flotte une promotion RÉELLE (champ `role`
  des logs collecteur) après une période de relais soutenu (NAS attendu Relay).
- **Ne prouve pas** : la rotation imprévisible en cascade (M1.3, pas
  construite) — ici le rôle suit le score, c'est le mécanisme LIVRÉ.

### R4 [F] — Rendez-vous : deux inconnus se trouvent (le carnet)

- **Prérequis code** : §5-P1 (namespace de rendez-vous de test).
- **Montage** : 2 process headless sur 2 HÔTES distincts (Mac + NAS),
  identités neuves, state.db vierges, ZÉRO pair configuré, mDNS coupé, aucun
  relais commun configuré (si un relais est requis par le transport : relais
  leurre anti faux-vert — leçon `RelayMode::Disabled`). Seul canal : slots DHT
  du namespace TEST.
- **Oracle** : (a) connexion établie ≤ 120 s (cible provisoire, à calibrer —
  la DHT Mainline réelle a sa latence propre ; c'est la PREMIÈRE mesure de
  baseline froide réelle, l'I10 LAN étant non mesurable) ; (b) la découverte
  vient BIEN du rendez-vous (source d'amorçage tracée dans les logs — champ
  `source_amorcage` du principe des logs, Plan Maître V2) ; (c) entrée signée
  vérifiée (anti-squat : `rendezvous_entry_authentic`).
- **Ne prouve pas** : la rotation du carnet (slots statiques aujourd'hui —
  écart #2 du prisme). Quand le carnet tournant sera conçu, R4 devient son
  banc de non-régression.

### R5 [L] — Subnets : le territoire se forme et se dissout

- **Montage** : 6 nœuds in-process. Trio A-B-C converse dense (≥ 3 msg/arête
  → `MIN_EDGE_WEIGHT`), trio D-E-F quasi silencieux. Le manager est branché au
  runtime (`state.rs:92`, événements surfacés `state.rs:3217`).
- **Oracle** : (a) un subnet {A,B,C} se FORME (événement Formed observé) et
  n'inclut JAMAIS D/E/F ; (b) après silence > timeout d'inactivité, il se
  DISSOUT (Dissolved, raison inactivité) ; (c) la dissolution ne laisse aucun
  état résiduel (auto-purge, wp §3.3).
- **Prérequis** : §5-P3 (timeout d'inactivité paramétrable pour ne pas
  attendre 5 min réelles en CI ; défaut prod inchangé).
- **Ne prouve pas** : le fork de subnet surchargé (« mécanisme de
  respiration », wp §3.3) — pas de mécanique de fork observable aujourd'hui.

### R6 [F] — Failover de groupe : le roi meurt, vive le roi

- **Montage** : groupe de 4+ sur la flotte réelle, trafic de groupe continu ;
  kill orchestré (`TEST-*`) du Responsable (hub).
- **Oracle** : (a) le Remplaçant (Shadow) est promu ≤ 10 s (nominal ~6 s +
  marge) ; (b) AUCUN trou de séquence au collecteur pendant la transition ;
  (c) un seul hub après convergence (pas de split-brain) ; (d) au retour de
  l'ancien hub : il REJOINT sans reprendre le trône de force.
- **Ne prouve pas** : la cascade double (hub ET shadow morts en < 6 s) — durci
  ailleurs (tests failover existants), à intégrer au capstone plus tard.

### R7 [L+F] — PoP : les fantômes ne votent pas

- **Montage [L]** : trafic réel entre vivants + injection de ré-annonces
  gossip d'un nœud MORT (fantôme).
- **Oracle** : (a) le fantôme reste `Known`, ne passe JAMAIS `Online` (la
  présence exige un travail constaté — ADR-011) ; (b) `online_count` = les
  vivants réels, exactement ; (c) sous charge, aucun gonflement de la vue.
- **Montage [F]** : comparer le `online_count` de chaque nœud (:9091) à la
  réalité de la flotte (5) pendant la charge du capstone.
- **Ne prouve pas** : la vue signée L1-003 sous attaque de témoin unique
  (red-teamé à part), l'attestation challenge-réponse L1-001 en profondeur.

### R8 [L] — Arroseur arrosé : le spammeur s'épuise, les autres ne voient rien

- **Montage** : 4 nœuds : S spamme (cadence >> budget), V1-V3 conversent
  normalement, tous vers des cibles croisées.
- **Oracle** : (a) S est progressivement freiné (`SenderThrottled`,
  `roles/antispam.rs`) ; (b) le débit et la latence de V1-V3 restent dans
  ±10 % de leur baseline sans spammeur (« les utilisateurs normaux ne voient
  rien », design-decisions D5) ; (c) S n'est JAMAIS exclu : à cadence réduite
  ses messages passent encore (pas de ban, pas d'état binaire) ; (d) S
  redevenu sage → retour progressif à la normale (rédemption, LOCKED #4).
- **Ne prouve pas** : la micro-preuve-de-travail (wp §6.5, non implémentée) ni
  la sur-assignation de tâches — seul le budget progressif existe.

## §3 Le capstone [F] — « la vie de l'organisme »

Sur la flotte réelle (5 nœuds + 2-3 headless Mac/NAS), UNE séquence continue
de ~15 min avec charge de fond permanente (send/ACK multi-host — le §3 du banc
courbe sert de bruit de fond, pas de juge) :

| t | Événement orchestré (`TEST-*`) | Rôle exercé |
|---|---|---|
| t+0 | charge de fond démarre | tuyau (baseline) |
| t+2 min | kill du hub de groupe | R6 failover |
| t+4 min | extinction d'un device ; messages vers lui | R2 backup |
| t+6 min | un inconnu (identité neuve) rejoint | R4 rendez-vous* |
| t+8 min | un headless spamme | R8 arroseur |
| t+10 min | rallumage du device éteint | R2 livraison différée |
| t+15 min | fin de charge, drain, relevés | — |

\* au capstone, l'inconnu utilise le rendez-vous RÉEL de la flotte (c'est la
flotte de test de Malik, pas la prod mondiale) — le namespace test ne sert
qu'à l'étage R4 isolé. **Discipline anti-pollution** (leçon 20/07) : l'inconnu
est UN nœud, marqué `TEST-*`, et le capstone se termine par son retrait +
purge (`/reset?level=network` au besoin) — jamais de fantôme laissé au
carnet réel. Le capstone est conditionné à P1 livré (revue DOCTRINE
2026-07-20) : sans P1 exercé d'abord en R4 isolé, pas de Phase C.

**Oracle global (les 3 verdicts du capstone)** :
1. **Aucune interférence destructrice** : la livraison de fond ne tombe jamais
   sous sa cible pré-enregistrée pendant TOUTE la séquence (chaque événement
   perturbe localement, l'organisme absorbe).
2. **Chaque mécanisme s'est déclenché avec preuve** (logs collecteur : élection
   hub, réplication backup, source d'amorçage de l'inconnu, throttle du
   spammeur, livraison différée au retour).
3. **PoP reste vrai de bout en bout** (R7-F : online_count = réalité, pendant
   le chaos).

C'est la matérialisation du mandat : **plusieurs rôles, en même temps, sous
charge — et le réseau continue de faire TOUT son travail, pas juste send/ACK.**

## §4 Étages et phases d'exécution

- **Phase A [L]** : R1, R2-L, R3-L, R5, R7-L, R8 — in-process, hermétiques,
  déterministes autant que possible ; intégrables en CI (les lourds en
  `#[ignore]` + job dédié).
- **Phase B [F]** : R4 (après §5-P1), R2-F, R6, R3-F, R7-F — orchestrateur
  étage F existant (`scripts/chaos/orchestrator.py`) + collecteur versionné.
- **Phase C [F]** : capstone §3. Publiable dans le rapport charte.

Chaque phase : critères pré-enregistrés AVANT le run, rapport honnête
(PASS/FAIL/INVALIDE par oracle), findings séparés des verdicts.

## §5 Prérequis code identifiés (petits, chacun design-first si protocolaire)

- **P1 — Namespace rendez-vous de test** (`tom-dht/src/lib.rs:40` :
  `RENDEZVOUS_NAMESPACE` est une constante ; vérifié absent du code au
  2026-07-20). Ajouter un override **test-only** (variable d'env
  `TOM_RENDEZVOUS_NAMESPACE`, défaut inchangé, jamais exposée dans les apps).
  Touche le cœur du rendez-vous → mini-doc + red-team du mésusage
  (fragmentation du réseau si une app la définissait) AVANT le code.
  **P1 est CRITIQUE avant TOUTE Phase F touchant le rendez-vous (R4) et avant
  le capstone** — sans elle, chaque run risque de re-polluer le carnet réel
  (incident 20/07). Ordre imposé : P1 → R4 isolé → capstone.
- **P2 — Oracles lisibles** — état vérifié (revue DOCTRINE 20/07) : DÉJÀ
  exposés en `ProtocolEvent` : `SubnetFormed`/`SubnetDissolved`
  (`runtime/mod.rs:381-386`), `SenderThrottled` (`runtime/mod.rs:433`),
  `RolePromoted`/`RoleDemoted` ; `online_count()` (`relay.rs:167`).
  **Manquant** : les crédits de forward (`ContributionMetrics`) ne sont pas
  surfacés en événement — l'oracle R1 lira l'état in-process ou :9091, sinon
  petit ajout à concevoir.
- **P3 — Timeout d'inactivité subnet paramétrable**
  (`discovery/subnet.rs:24` : `INACTIVITY_TIMEOUT_MS` est une const ; vérifié
  non configurable au 2026-07-20) : const → config avec défaut 5 min inchangé,
  pour R5 en CI.
- **P4 — TTL backup court pour R2-L** : vérifier que l'API accepte un TTL
  custom ≤ 24 h (le clamp `.min(MAX_TTL_MS)` n'interdit pas un TTL bas).

## §6 Ce que ce banc ne prétend PAS

- Pas de sélection en cascade ni d'entropie non-biaisable (M1.2-M1.3 : pas
  construits — mur #1 Fable ouvert).
- Pas d'observateurs/validateurs pleins ni d'ancrage L1 (M1.5-M1.6).
- Pas de rotation du carnet de rendez-vous (écart #2 du prisme : à concevoir).
- Pas de saturation de débit ni de latence-vs-N (repris par le banc courbe
  multi-host, §3 de `banc-courbe-masse.md`).
- Pas de résistance à l'analyse de trafic (routage onion non implémenté).

## §7 Critères de sortie du chantier

1. Phase A verte en CI (ou #[ignore] documentés).
2. Phase B : chaque scénario F exécuté ≥ 1 fois avec rapport.
3. Capstone : 1 run complet publié (verdicts + findings), intégré au rapport
   charte comme première preuve « organisme sous charge ».
4. MAJ `prisme-des-roles.md` (grille R1-R8 : statut exercé/pas exercé) et
   `vault/30-discoveries.md`.
