# Banc « courbe de masse » — prouver l'anti-cyclicité par la mesure

> Compagnon de `charte-cibles-agressives.md` (§0 deux régimes, §4 gap super-additif).
> Design v1 (2026-07-20), issu de la revue « avocat du diable ». **À valider avec Malik
> avant toute exécution** — zéro code tant que le design n'est pas ratifié.

---

## §0 Objectif et critères PRÉ-ENREGISTRÉS

Produire les premières **courbes falsifiables** de la thèse : à **charge par nœud fixe**,
quand N croît —

1. **Courbe capacité** : le débit *livré par nœud* ne s'effondre pas (plat ou mieux).
2. **Courbe latence** : la latence de livraison 1-à-1 (p50/p95) reste bornée
   (charte §0 : au pire sous-logarithmique pour le broadcast, stable pour le 1-à-1).

**Critères fixés AVANT la mesure** (anti-biais — on ne déplace pas les poteaux après) :

| Verdict | Condition (drafts à ratifier) |
|---|---|
| PASS | débit/nœud à N_max ≥ 80 % de celui à N_min ET latence p95 ≤ 2× celle à N_min |
| FAIL | l'un des deux seuils est franchi sur ≥ 2 runs consécutifs |
| INVALIDE | garde-fou contention déclenché (§3) → le point n'existe pas, on ne conclut RIEN |

Un FAIL n'est pas une honte : c'est un **bug architectural localisé** (charte §0), donc un
chantier. Un point INVALIDE publié comme PASS serait, lui, une faute.

## §1 Les 4 pièges que ce design évite (leçons projet + revue adverse)

1. **Contention du banc** (fatal, déjà vécu : « dégradation mesh = contention de mes
   builds », pool-lock 74 s). 100 nœuds sur 1 Mac mesure l'ordonnanceur, pas le réseau.
   → densité par hôte bornée + garde-fou CPU (§3), sinon point INVALIDE.
2. **Compteurs menteurs** (vécu : indicateurs verts sur des proxies pendant des semaines ;
   `messages_recus=0` silencieux ; NAS « connecte » avec 0 pair). → Phase 0 d'audit
   d'instrumentation, BLOQUANTE.
3. **Métrique tautologique** (revue adverse) : « le débit agrégé monte » ne falsifie rien.
   → toutes les métriques sont **par nœud, à charge par nœud fixe**.
4. **LAN ≠ WAN** (validité externe) : un LAN à RTT 2 ms ne dit rien d'un déploiement réel.
   → Phase 2 en WAN simulé (netem), et le rapport sépare les deux mondes sans extrapoler.

## §2 Phase 0 — Audit d'instrumentation (BLOQUANTE)

Avant tout point de courbe, prouver que ce qu'on compte est vrai :

- **Canari comptage** : injecter K messages connus A→B ; vérifier compteur émetteur = K,
  compteur récepteur = K, chacun compté UNE fois (pas de double-comptage à la reconnexion).
- **Latence sans horloge partagée** : mesurer en **aller-retour applicatif** (A→B→écho→A,
  latence = RTT/2) — jamais de timestamps croisés entre machines (horloges non synchro).
- **Vérité terrain croisée** : compteurs :9091/:8085 vs logs collecteur UDP vs comptage
  côté harnais — les trois doivent converger sur le canari.
- **Charge de fond nulle** : `ps aux` avant chaque run, kill des process de stress résiduels
  (leçon : un cas a tourné 2 jours), aucun build en parallèle pendant les mesures.

Sortie : une note « instrumentation auditée le X, écarts trouvés/corrigés ». Sans elle,
aucune courbe n'est publiable.

## §2bis Brique in-process (LIVRÉE 2026-07-20, RÉVISÉE post-oracle le soir même) — hermétique, avant le multi-host

Avant le multi-machines (§3), une brique **in-process** répond au risque #1 de
l'avocat mesure de la façon la plus radicale : les N nœuds sont câblés par
`add_peer_addr` (aucune découverte n0/mDNS/DHT) → **hermétique par
construction**, la vraie flotte ne PEUT pas fausser la courbe. Bonus : horloge
de process UNIQUE → latence par message sans skew.

**Herméticité aussi dans l'AUTRE sens** (leçon de l'incident du 20/07 : la V1
du banc a publié ~150 nœuds fantômes au rendez-vous DHT partagé de la vraie
flotte) : chaque nœud du banc coupe le trio découverte —
`n0_discovery(false).local_discovery(false)` + `enable_dht: false`
(`scenario_courbe.rs:112,127`). Règle générale : un nœud de test ne touche
JAMAIS le rendez-vous partagé.

Commande : `tom-stress courbe --sizes 5,10,20 --duration N` (crate
`scenario_courbe.rs`). Charge fixe/nœud (1 msg/s, 1 Ko, cible aléatoire),
topologie all-pairs directe.

**Ce que la brique JUGE (axe unique, resserré par la revue oracle) :
l'INTÉGRITÉ de livraison.**
- **Perte réelle** : comptage après **drain à quiescence** — le récepteur
  continue de drainer tant qu'il reçoit, ne clôt qu'après ≥ 2 s de silence
  (borne dure de filet). Plus AUCUNE troncature de messages en vol : une
  livraison < 100 % est une perte vraie, pas un artefact de fenêtre.
- **Dédup (I8)** : doublon = même **clé unique `(nœud << 32 | seq)`** revue
  chez un récepteur — incontestable. (La V1 utilisait l'horodatage d'émission
  comme clé : deux envois dans la même milliseconde = faux « doublon ». Les
  doublons vus par la V1 étaient un artefact de MON instrument, pas du
  protocole.)
- **Herméticité** : filtre de signature de payload → les messages non-banc du
  canal applicatif sont isolés, pas comptés.

**Résultats FINAUX (20/07 soir, banc corrigé — drain à quiescence, clé
`(nœud,seq)`, magic d'herméticité, teardown par point ; revue oracle 4 agents
passée. Seed 42, 15 s/point, loopback, build 137) :**

| N | livraison | offert→reçu | livré/nœud | p50 | p95 | max | dérive | dup | état |
|---|---|---|---|---|---|---|---|---|---|
| 5 | 100.0 % | 75→75 | 1.00 Hz | 16.5 ms | 23.4 ms | 32.6 ms | 1.00× | 0 | ok |
| 10 | 100.0 % | 150→150 | 1.00 Hz | 6.8 ms | 172 ms | 1.25 s | 1.01× | 0 | ok |
| 20 | 97.3 % | 300→292 | 0.97 Hz | 504 ms | 2.69 s | 6.81 s | 1.02× | 0 | ok |

→ **VERDICT INTÉGRITÉ : FAIL à N=20** — 8 messages sur 300 non livrés
(2,7 %), 0 doublon partout, 100 % à N=5 ET N=10. Tous les points sont VALIDES
(dérive d'émission ≤ 1.02× : la charge promise a été tenue) → cette perte est
réelle **sur ce banc**, et son ATTRIBUTION reste ouverte : à N=20 le runtime
partagé est saturé côté réception (p50 ×31 vs N=5) — perte protocolaire ou
messages sacrifiés par l'ordonnanceur affamé ? Trancher = multi-process/
multi-host (§3).

**L'histoire de l'instrument (3 artefacts de mesure tués, TOUS accusaient le
protocole à tort)** : la V1 lisait « 74 % + doublons » à N=20 → clé de dédup
ambiguë (horodatage) + troncature du drain. Le 1er run révisé lisait 87,7 % →
3ᵉ artefact découvert par le finding teardown de la revue oracle : **sans
shutdown explicite, les nœuds des points précédents restaient VIVANTS pendant
les points suivants** (le point N=20 tournait avec 15 nœuds parasites des
points N=5/N=10 — keepalives, gossip, contention). Chaque point est désormais
isolé par un teardown borné. Leçon : un banc qui juge le protocole doit être
soupçonné D'ABORD — trois fois de suite, « la perte » était l'instrument.

**Ce que la brique NE juge PAS (limites ASSUMÉES, écrites dans la sortie) :**
- **Débit à saturation : PAS sondé.** À charge fixe 1 msg/s, « livré/nœud
  ×1.00 » est **tautologique** tant que l'intégrité tient — c'est l'absence de
  perte, PAS une preuve d'anti-effondrement. Sonder la saturation exigerait de
  monter `--rate` jusqu'à casser — et sur un runtime partagé on mesurerait le
  Mac, pas le protocole (§3).
- **Latence-vs-N : RAPPORTÉE, pas jugée.** À N=10, p50 ×17 alors que la
  dérive d'ÉMISSION reste ~1.0 → ce n'est pas le réseau, c'est le **runtime
  tokio PARTAGÉ** qui sature côté RÉCEPTION. Un banc mono-runtime ne peut pas
  séparer « latence protocolaire ∝ N » de « ordonnanceur saturé ». ⇒ l'axe
  latence exige l'**isolation par processus** (§3).
- **Validité d'un point** : INVALIDE ⟺ **dérive de cadence à l'émission
  > 1.5×** (la charge promise n'a pas été tenue → on mesure la contention du
  banc, pas le réseau). Le p95 n'est PLUS un critère d'exclusion — le drain à
  quiescence rend la livraison vraie même lente.
- **loopback (RTT ~0)**, pas LAN/WAN → validité externe = §3/§4.
- Plafond du banc local ≈ N≈10-15 avant saturation réception.
- **Ce banc ignore les RÔLES** (recadrage fondateur 20/07) : relais multi-hop,
  backup/gardien, rendez-vous, subnets, rotation de rôles — voir
  `docs/plans/prisme-des-roles.md` (grille R1-R8) ; c'est le prochain banc qui
  les exerce.

**Findings transport (la brique fait fuzzer le teardown de masse) — à instruire,
NON bloquants (post-mesure) :** l'arrêt simultané de ~20 nœuds fait surgir des
races de teardown dans le transport forké : `unreachable: drained connections
always have an error` (`tom-quinn/src/connection.rs:289` — attribution corrigée,
c'est tom-quinn, PAS tom-quinn-proto), `gossip/net.rs:454` panic sur
`JoinError::Cancelled`, `could not close last open path`, `RETIRE_CONNECTION_ID
for unknown path`, `PTO expired while unset`. N'altèrent pas la mesure mais
révèlent des `unwrap`/`unreachable` atteints sous arrêt concurrent massif — zone
deadlocks historique, à durcir séparément avec soin.
**Nouveaux au run révisé (20/07 soir)** : panic `closed without error reason`
(`tom-quinn/src/connection.rs:1723`, ×5 au teardown N=20) ; **boucle chaude de
retry sur `MaxPathIdReached`** (~46 tentatives d'`open_path` dans la MÊME
milliseconde — aucun backoff, spam massif de WARN) ; le multipath sonde
l'**IPv6 globale du Mac** (`2a01:e0a:…`) entre nœuds du même process — pas une
fuite de découverte (le trio isolated tient), mais les nœuds échangent leurs
adresses observées et tentent des chemins hors loopback : bruit + travail
inutile en banc, comportement à borner (backoff) côté transport.

**Chatter du canal applicatif :** ~20 messages/run non-banc arrivent dans le
canal `DeliveredMessage` (filtrés). À élucider : presence/gossip surfacé comme
message applicatif ? (n'affecte pas la mesure, mais l'axe #8 « invisibilité »
aime savoir ce qui traverse la frontière applicative).

## §3 Phase 1 — Courbe LAN multi-machines

- **Hôtes physiques** : Mac, NAS (VM Debian), iPad, iPhone, Apple TV = 5 hôtes.
- **Points de courbe** : N = 5 (1 nœud/hôte) → 10 → 20. Les nœuds supplémentaires sont des
  process headless (tom-tui --bot ou binaire stress) sur Mac + NAS UNIQUEMENT (pas sur les
  devices Apple mobiles).
- **Garde-fou contention (non négociable)** : CPU et loadavg échantillonnés sur chaque hôte
  pendant le run. Si un hôte dépasse **70 % CPU soutenu** (ou loadavg > cœurs), le point est
  **INVALIDE** → réduire la densité, pas la vigilance. C'est le plafond de validité du banc
  local : au-delà de ~20-24 nœuds, il FAUT du matériel distribué (hors scope v1).
- **Charge fixe par nœud** : 1 msg/s, payload 1 Ko, destinataires tirés uniformément parmi
  les N-1 autres. Durée : 15 min/point, 2 runs/point.
- **Mesures par point** : débit livré/nœud, latence écho p50/p95, taux de livraison,
  RssAnon/nœud (l'axe stockage de la charte exige qu'il reste constant), CPU/hôte
  (validité), part du trafic relayé par nœud (→ métrique de **concentration**, charte §4).

## §4 Phase 2 — Courbe WAN-simulée + churn

- **Dégradation réseau** : `tc netem` sur la VM NAS (root dispo) : RTT 40-80 ms, jitter
  10 ms, perte 1 % sur l'interface. Effet : tous les chemins passant par le NAS deviennent
  « WAN ». Piste complémentaire côté Mac : dummynet (`dnctl`/`pfctl`) — à valider avant
  d'en dépendre.
- **Churn contrôlé** : l'orchestrateur étage F (runbook) kill/relance une fraction des
  nœuds headless (10 %/5 min) pendant la mesure — le monde réel n'est pas statique, et la
  charte exige le churn absorbé.
- Mêmes points N, mêmes métriques, mêmes garde-fous que Phase 1. Les courbes LAN et WAN ne
  se mélangent JAMAIS dans un même graphe sans étiquette.

## §5 Phase 3 — Rapport honnête

- Les 2 courbes × 2 mondes (LAN/WAN), avec les points INVALIDES montrés comme tels.
- **Limites d'extrapolation explicites** : N max testé, matériel semi-homogène, un seul
  site physique ; AUCUNE conclusion au-delà de N mesuré, pente ≠ preuve à 10 000.
- La distribution de concentration (part de l'infra par décile) — première photo du
  « nœud-équivalent » réel de la flotte.
- Le rapport devient une annexe de la charte et alimente le tableau de bord juge (§3.2).

## §6 Ce que le banc ne prétend PAS

- Prouver 10 000 nœuds (il mesure une PENTE à petit N, c'est tout).
- Mesurer l'in-censurabilité, la sécurité, ou le hole-punch cross-NAT réel (autres bancs).
- Remplacer le terrain : la flotte réelle (I9/I10) reste le juge de paix des reconnexions.

## §7 Réutilisation de l'existant (pas de réécriture)

- `tom-stress` : campagnes Mac↔NAS existantes (250/250) — base des émetteurs de charge.
- Orchestrateur étage F + runbook (`docs/plans/RUNBOOK-TESTS.md`) : lancement/kill de
  flottes headless, déjà exercé sur un mesh de 8 nœuds.
- Collecteur UDP :9999 + status :9091/:8085 : télémétrie en place (auditée en Phase 0).
- Harnais étage L : inchangé, hors scope ici.

---

> Prochaine étape si ratifié : Phase 0 seule (audit instrumentation), livrable = la note
> d'audit. Aucun point de courbe avant ça.
