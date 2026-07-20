# R14 Lot C — Re-sondage des candidats inactifs (design, AVANT code)

> Créé le 2026-07-19 (soir), à partir du verdict Lot B (`r14-ipv6-first-class.md` §2.4,
> mémoire `tom-lotb-verdict-failover-sans-resondage`). **Aucun code écrit.**
> À red-teamer + arbitrage Malik avant implémentation.

## §1 Le problème exact (mesuré, pas supposé)

Après un failover (mort du chemin actif), le système reste sur le chemin survivant même
s'il est nettement plus lent que l'ancien chemin redevenu disponible : iPad→Mac resté sur
v6 18 ms pendant ≥ 25 min avec un v4 à 7 ms vivant (gain 11 ms >> hystérésis 5 ms).
Cause : **un chemin non-actif n'est plus sondé** → pas de RTT frais → `select_v4_v6`
(`remote_state.rs:1160-1223`) n'a aucun candidat à comparer. L'hystérésis et l'avantage
v6 fonctionnent — ils n'ont juste plus de matière.

## §2 Ce que le Lot C n'est PAS

- PAS le tri déterministe du probe initial (`iroh_hp.rs`, FxHashMap) : l'aléatoire du
  premier choix est réel mais ne cause pas la non-convergence mesurée — et
  `iroh_hp.rs:196` documente qu'un mécanisme s'appuie sur cet aléatoire. On n'y touche pas.
- PAS un « ping toutes les X ms sur tous les candidats » : le multipath QUIC sonde déjà
  les chemins de l'ensemble ACTIF (§2.3bis : sondes 8 octets vues au tcpdump). Le trou ne
  concerne que les candidats SORTIS de l'ensemble actif (morts, jamais validés).

## §2bis Red-team (7 agents, 19/07 soir) — 3 corrections MAJEURES au design

Le red-team a trouvé 3 défauts de conception réels (pas des peurs) + confirmé par la
cartographie transport (`a2fc686`) que **les briques existent déjà** — on ne construit pas
un mécanisme neuf.

**C1 — Le déclencheur « new_rtt > old_rtt » est le mauvais signal.** Une bascule à perte
ne peut venir QUE d'une fermeture forcée (l'hystérésis interdit de CHOISIR un pire chemin),
donc « rtt monte » est déjà, tautologiquement, « chemin mort remplacé ». Le RTT de l'ancien
chemin (9 ms) n'a aucune valeur prédictive : le chemin est mort parce qu'il a changé
d'identité réseau (port NAT, adresse v6 tournée), pas parce qu'il s'est dégradé. **Correction :
déclencher sur l'ÉVÉNEMENT DE MORT lui-même** — `PathEvent::Abandoned`/`Closed`
(`remote_state.rs:958`, `connection/mod.rs:750 abandoned_paths.insert`), pas sur une
comparaison de RTT. Le candidat mort est déjà horodaté `Inactive(Instant)` (`path_state.rs:51`).

**C2 — Réutiliser PATH_CHALLENGE, ne pas inventer de sonde.** `open_path(addr)`
(`remote_state.rs:832`) ré-ouvre la validation d'une adresse candidate et émet
automatiquement un PATH_CHALLENGE (`tom-quinn-proto/paths.rs:150`). Le « re-probe » = un
appel `open_path` sur un candidat `Inactive`, câblé sur le pattern de backoff tokio DÉJÀ
présent pour le holepunch (`scheduled_holepunch`, `remote_state.rs:280`). Zéro protocole
neuf, zéro trame custom. C'était une ambiguïté du design initial (§3 disait « sondes de
validation QUIC » sans dire lesquelles) — tranché : les existantes.

**C3 — Anti-oscillation + anti-amplification obligatoires (attaques confirmées).** Deux
attaques chiffrées : (a) un pair malveillant annonce 12 adresses pointant vers une victime
puis tue ses chemins → nos re-probes bombardent la victime (cap 12 candidats
`iroh_hp.rs:236` limite à 72 sondes/cycle mais ne l'annule pas) ; (b) cyclage de chemins
toutes les 40 s → 66 failovers en 11 min, batterie. **Corrections :** cooldown post-failover
(ne pas re-basculer vers une adresse quittée < 30 s sans gain > 10 ms) + ≤ 1 re-probe en vol
par adresse-cible. Ces deux garde-fous sont un **fade** (délai/seuil temporaire réversible),
PAS un ban (LOCKED #4 respecté) : l'adresse redevient éligible dès le cooldown écoulé.

**Non-retenu / déjà couvert :**
- « Fade vs ban » (LOCKED #4) : le cooldown réversible EST un fade, pas une blacklist. OK.
- TTL des candidats temporaires v6 : déjà borné par `MAX_INACTIVE_IP_PATHS=10` +
  `prune_ip_paths` (`path_state.rs:244`) ; à vérifier < 24 h mais pas un blocage neuf.
- Power save iOS (§5) : tranché produit (on assume). Le cooldown limite déjà le gaspillage
  de sondes vers un mobile au churn élevé ; pas de guard foreground supplémentaire.

## §3 Proposition RÉVISÉE (post-red-team)

Déclencheur : `PathEvent::Abandoned` d'un chemin qui était ACTIF (pas un candidat jamais
monté). Action : armer un re-probe `open_path(addr_mort)` à T+30 s, backoff ×2 cap 5 min,
6 essais, abandon dès que le chemin revit OU qu'un cooldown anti-oscillation l'interdit.
La sélection existante (`select_v4_v6`) décide au retour du RTT — aucun seuil neuf.
Garde-fous : cooldown 30 s / gain 10 ms post-failover, ≤ 1 re-probe en vol par cible.

## §3bis Proposition INITIALE (conservée pour traçabilité, dépassée par §2bis/§3)

**Re-probe paresseux et borné des candidats connus non-actifs**, dans tom-connect
(`remote_state`), déclenché par une condition de « suspicion de sous-optimalité » plutôt
qu'un timer aveugle :

1. **Déclencheur** : à chaque bascule ENREGISTRÉE comme failover (le nouveau chemin est
   plus lent que l'ancien : `new_rtt > old_rtt`), armer un re-probe du chemin perdu à
   T+30 s, puis backoff ×2 (30 s → 1 min → 2 min → 5 min, cap 5 min), abandon après ~6
   tentatives OU dès que le chemin redevient actif.
2. **Portée** : uniquement les adresses déjà CONNUES du remote (candidats existants de la
   connexion) — jamais de nouvelle découverte, jamais de dial de pair (c'est un
   path-probe QUIC intra-connexion, pas un `get_or_connect`).
3. **Décision** : le re-probe réussi réinjecte un RTT frais → la logique EXISTANTE
   (`select_v4_v6` + `RTT_SWITCHING_MIN_IP` 5 ms + `IPV6_RTT_ADVANTAGE` 3 ms) décide.
   Aucun nouveau seuil de sélection.
4. **Budget** : ≤ 1 re-probe en vol par connexion, sondes = trames de validation QUIC
   (~8-40 octets). Coût plafonné : 6 sondes / chemin perdu / cycle de backoff.

## §4 Risques à red-teamer

- **Oscillation** : chemin qui meurt/revit vite (le cas iPad, cycle ~5-15 min) → le
  re-probe pourrait re-basculer vers un chemin instable. Mitigation candidate : mémoire
  courte de fiabilité par candidat (n morts récentes → exiger un gain > hystérésis
  majorée, p.ex. 2×5 ms) — attention décision LOCKED #4 (fade, pas de ban).
- **Interférence avec le NAT traversal** (`continue_nat_traversal_round`,
  `iroh_hp.rs:196`) : le re-probe ne doit pas relancer de round complet de hole punch.
- **Batterie/veille iOS** : des sondes périodiques peuvent réveiller la radio — le
  backoff cap 5 min et l'abandon après 6 essais bornent ça ; à mesurer sur appareil.
- **Amplification** : sondes vers une adresse morte (peer parti) = trafic vers un tiers
  potentiel (DHCP réattribué). Borné par l'abandon + intra-connexion uniquement (la
  connexion meurt avec le pair).

## §5 Question amont — TRANCHÉE par décision produit (Malik, 19/07 soir)

**On ne creuse pas, on assume.** iOS restreint le réseau même en foreground sans
activité tactile (surtout iPad — la batterie est un point fort du produit). Décision,
extension du non-objectif APNs/background du 15/07 : **ToM n'exploite pas une machine
mobile au-delà de ce que son utilisateur utilise réellement du réseau.** En prod, l'app
mobile est ouverte pendant l'usage réel (envoi/réception/lecture) — pas laissée en
foreground des heures comme au labo.

Conséquences pour ce design :
- Le churn de chemins d'un mobile inactif = comportement ATTENDU, plus une anomalie.
  Aucun keepalive/anti-power-save ne sera ajouté pour le contrer.
- Le Lot C est RECADRÉ, pas invalidé : sa valeur est la **re-convergence des nœuds
  actifs** (NAS, desktop, mobile en usage réel) après un failover — dont le retour au
  meilleur chemin quand l'utilisateur REVIENT dans l'app et que ses chemins revivent.
- Les mesures I9a/I9b doivent séparer les mobiles inactifs de la flotte active, sinon
  elles mesurent le power save d'Apple, pas notre protocole.

## §5bis Implémentation (2026-07-19 soir, feu vert Malik sur §2bis/§3)

Livrée dans `tom-connect` uniquement (AUCUNE modification tom-quinn-proto), tout dans
l'actor mono-tâche `RemoteStateActor` — pas de nouveau verrou, pas d'await sous mutex.

- `path_state.rs` : `abandoned_path()` retourne désormais `bool` (le chemin était-il
  `Open` ?) + getter `status()`.
- `remote_state.rs` :
  - Constantes : `REPROBE_INITIAL_DELAY` 30 s, `REPROBE_MAX_DELAY` 5 min,
    `REPROBE_MAX_ATTEMPTS` 6, `FAILOVER_COOLDOWN` 30 s, `FAILOVER_MIN_GAIN` 10 ms.
  - Déclencheur : `PathEvent::Abandoned` d'un chemin IP qui était `Open` → `arm_reprobe`.
  - Re-probe : bras `scheduled_reprobe` dans le `select!` (pattern `MaybeFuture` du
    holepunch) → `run_due_reprobes` → `open_path(addr)` (PATH_CHALLENGE existant),
    backoff ×2, ≤ 1 entrée par adresse (anti-amplification), abandon si le chemin
    revit (`PathEvent::Opened` retire l'entrée), si l'adresse est élaguée, ou s'il
    n'y a plus de connexion.
  - Cooldown : enregistré dans `PathEvent::Closed` quand le chemin fermé était le
    chemin sélectionné (= failover) ; appliqué dans `select_path` via la fonction pure
    `failover_cooldown_blocks` ; purge des entrées expirées à chaque passe (fade).
  - Nettoyage : tout l'état Lot C est vidé à la dernière connexion fermée ;
    `reprobes` + `recent_failovers` aussi sur changement réseau majeur
    (`deliberately_closed` survit volontairement : les fermetures en vol émettront
    encore leur `Abandoned` et doivent rester exclues).
  - Réversibilité du give-up (LOCKED #4) : l'abandon après 6 essais vaut pour UN
    cycle de mort du chemin — toute nouvelle mort d'un chemin actif ré-arme un cycle
    complet, et un chemin revenu par n'importe quelle voie (holepunch, QNT, dial)
    redevient candidat normal. Rien n'est permanent.

**Raffinement découvert à l'implémentation** (non prévu par le red-team) :
`close_redundant_paths` ferme VOLONTAIREMENT les chemins lents après un upgrade, et ces
fermetures émettent aussi `PathEvent::Abandoned`. Sans discriminant, le re-probe
rouvrirait en boucle un chemin qu'on vient de fermer exprès (probe → reopen → close →
probe…). Ajout : `deliberately_closed` (marqué au `path.close()` volontaire, consommé à
l'`Abandoned` correspondant) — ces chemins ne sont PAS re-sondés.

Tests : backoff (`test_next_reprobe_delay_backoff`), cooldown
(`test_failover_cooldown_blocks`), retour de `abandoned_path` (test path_state étendu).

## §6bis Harnais étage L — état 2026-07-20 (nuit) et DÉCOUVERTE transport

**Infra livrée** (tout gated `cfg(any(test, feature = "test-utils"))`, zéro surface prod) :
- Proxy UDP contrôlable (kill/revive par flag) dans le test.
- `RemoteStateMessage::OpenPath` + plomberie endpoint→socket→actor
  (`Endpoint::open_path(remote, addr)`) : attache un chemin contrôlé (validé :
  PATH_CHALLENGE émis, chemin ouvert et sélectionné).
- `QuicTransportConfigBuilder::disable_nat_traversal()` : sans lui, le wrapper
  du fork IGNORE silencieusement toute valeur < 12 et QNT ouvre des chemins vers
  les adresses RÉELLES (bypass du proxy + fuite d'herméticité).
- `PathInfo::set_max_idle_timeout()` : override par chemin + diagnostic (retourne
  la valeur précédente).
- Constantes Lot C raccourcies en build test (`cfg!(test)`), invariant
  `REPROBE_INITIAL_DELAY > FAILOVER_COOLDOWN` préservé et testé.
- Test `endpoint_reprobes_dead_path_and_recovers` : étape 1 VERTE
  (relais-first, attache proxy, upgrade sélection vers le direct).

**MAJ 2026-07-20 ~04:00 — LOT C VALIDÉ ÉTAGE L (run verte de bout en bout) + 2 findings :**

**Finding 1 (RÉSOLU, le vrai gibier du harnais) — chemin zombie bloquait tout
re-open.** Le re-probe #1 (cible encore morte) crée un chemin, sa validation
échoue (`PathOpen` → `ValidationFailed` → abandon), MAIS l'entrée
`conn_state.path_ids[addr]` réinscrite par ce probe pointe vers le zombie (que
quinn garde en draining jusqu'à `DiscardPath`). Tous les re-probes suivants —
cible VIVANTE — prenaient la branche « chemin déjà connu » d'`open_path` et se
réduisaient à un `set_status` sur le cadavre : plus AUCUN `PATH_CHALLENGE`
émis, jamais. Le même verrou aurait bloqué les upgrades QNT après churn.
**Fix (livré)** : la branche courte exige `open_paths.contains_key(path_id)`
(chemin réellement ouvert), sinon fallthrough vers `open_path_ensure` (dont la
dédup exclut correctement les abandonnés) ; + hygiène `remove_path` (ne retire
`path_ids[addr]` que s'il mappe encore ce path_id). Preuve : run VERTE complète
kill → failover → revive → re-probe (backoff 1,2 s→3 s, ≤1 en vol, deux côtés
armés) → chemin revalidé → retour au direct.

**Finding 2 (RÉSOLU ~04:40, build 134) — `PathIdle` non-déterministe (~2 runs/3).**
CAUSE RACINE : héritage single-path. `packet_builder.rs` resettait l'idle du chemin
d'ÉMISSION quand `permit_idle_reset` (posé par une réception sur N'IMPORTE quel
chemin — global connexion) était armé. Les keep-alives per-path émis sur le chemin
MORT ré-armaient donc son propre PathIdle grâce à la vivacité du RELAIS (~4×/s selon
l'entrelacement — d'où le non-déterminisme). FIX : l'émission ne resette plus que
l'idle CONNEXION (`reset_conn_idle_timeout`) ; le per-path n'est resetté que par la
réception authentifiée sur CE chemin. **Harnais : 10/10 runs vertes → `#[ignore]`
retiré, le test tourne en suite.** Conséquence terrain attendue : la détection de
mort de chemin devient fiable même sous trafic d'émission continu (explique
possiblement la lenteur de failover historique). Constat original :
Quand il tire : +1,44 s après kill, parfait. Quand il ne tire pas : chemin
affamé 20 s, `idle_timeout=Some(1.5s)` confirmé des deux côtés (valeur
précédente via `PathInfo::set_max_idle_timeout`), keep-alives per-path actifs,
LossDetection tire — mais 0 PathIdle, 0 `PathEvent::Closed`, pas de failover.
Armement instrumenté (`arming PathIdle`, trace ajoutée `mod.rs`) : 835
armements observés sur une run qui tire. Zone : `tom-quinn-proto/connection/
mod.rs:3475-3484` (armement), `:2233` (handler), `timer.rs` (TimerTable/
SmallMap). Question protocole associée : en terrain, qu'est-ce qui tue les
chemins si PathIdle est peu fiable ? (`ValidationFailed` QNT observé — les
morts viendraient des revalidations/du pair.) Repro : lancer le test en boucle
(`--ignored`), ~2/3 des runs échouent à « relay failover ».

## §6 Validation prévue

- Étage L : test hermétique loopback (modèle r15_relay_cache) : tuer artificiellement un
  chemin (drop socket v6), vérifier failover PUIS retour ≤ 60 s quand le chemin revit.
  ⚠️ Constat d'implémentation (19/07 soir) : « tuer un chemin sans le fermer » n'a pas
  de brique dans le fork (un `path.close()` est une fermeture VOLONTAIRE, exclue du
  re-probe par design ; un rebind change le port donc le chemin mort ne « revit » jamais
  à l'identique). Le harnais propre = proxies UDP loopback à latence injectable
  (kill = arrêt du forward, revive = reprise), avec la difficulté que QNT peut
  court-circuiter le proxy en découvrant l'adresse réelle. À construire comme chantier
  de validation dédié — PAS bricolé.
- Étage F : rejouer la fenêtre Mac↔iPad avec `path-matrix.py` + logger : I9b (retour au
  meilleur ≤ 60 s après renaissance), I9a (pas d'oscillation accrue).
- Invariants : aucune bascule ne doit plus durer > backoff-cap quand un chemin 5 ms
  meilleur est vivant.
